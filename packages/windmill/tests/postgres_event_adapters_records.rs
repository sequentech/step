// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The application, secret, template, certificate authority, phone blacklist
//! and preview adapters against the migrated schema. Every test writes its
//! own rows in a transaction, checks them with independent SQL and rolls the
//! transaction back.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local, TimeZone, Utc};
use deadpool_postgres::{Object, Transaction};
use sequent_core::types::hasura::core::{Application, PhoneBlacklistEntry, Preview, Template};
use serde_json::{json, Value};
use std::cell::Cell;
use std::collections::HashMap;
use tokio_postgres::types::{FromSql, ToSql};
use uuid::Uuid;
use windmill::postgres::application::{self, EnrollmentFilters};
use windmill::postgres::certificate_authority::{self, CertificateAuthorityRecord};
use windmill::postgres::{phone_blacklist, preview, secret, template};
use windmill::services::application::ApplicationAnnotations;
use windmill::types::application::{ApplicationStatus, ApplicationType};

const BAD_UUID: &str = "not-a-uuid";

async fn connect() -> Object {
    schema::pool().await.get().await.unwrap()
}

/// Noon (UTC) of the given day of January 2026.
fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0).unwrap()
}

fn local(instant: DateTime<Utc>) -> DateTime<Local> {
    instant.with_timezone(&Local)
}

fn strings(ids: &[Uuid]) -> Vec<String> {
    ids.iter().map(Uuid::to_string).collect()
}

fn sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items
}

fn assert_invalid_uuid(result: anyhow::Result<impl std::fmt::Debug>) {
    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("invalid UUID 'not-a-uuid'"), "{message}");
}

async fn scalar<T: for<'r> FromSql<'r>>(
    tx: &Transaction<'_>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> T {
    tx.query_one(sql, params).await.unwrap().get(0)
}

async fn now(tx: &Transaction<'_>) -> DateTime<Utc> {
    scalar(tx, "SELECT now()", &[]).await
}

/// A tenant and one of its election events.
#[derive(Clone, Copy, Debug)]
struct Scope {
    tenant: Uuid,
    event: Uuid,
}

impl Scope {
    fn tenant_id(&self) -> String {
        self.tenant.to_string()
    }

    fn event_id(&self) -> String {
        self.event.to_string()
    }
}

/// Row builders with fixed identifiers. `seed` keeps the keys of concurrently
/// running tests apart; pass `line!()`, which is unique per test in this file.
struct Fixture<'t, 'c> {
    tx: &'t Transaction<'c>,
    seed: u32,
    next: Cell<u32>,
}

impl<'t, 'c> Fixture<'t, 'c> {
    fn new(tx: &'t Transaction<'c>, seed: u32) -> Self {
        Self {
            tx,
            seed,
            next: Cell::new(0),
        }
    }

    /// The next fixed v4 UUID of this test.
    fn id(&self) -> Uuid {
        let n = self.next.get() + 1;
        self.next.set(n);
        Uuid::parse_str(&format!("{:08x}-0000-4000-8000-{n:012x}", self.seed)).unwrap()
    }

    async fn execute(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) {
        self.tx.execute(sql, params).await.unwrap();
    }

    async fn tenant(&self) -> Uuid {
        let tenant = self.id();
        self.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await;
        tenant
    }

    /// A new tenant with one election event.
    async fn scope(&self) -> Scope {
        let tenant = self.tenant().await;
        self.event_in(tenant).await
    }

    /// Another election event of `tenant`.
    async fn event_in(&self, tenant: Uuid) -> Scope {
        let event = self.id();
        self.execute(
            "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1, $2, 'RSA256')",
            &[&event, &tenant],
        )
        .await;
        Scope { tenant, event }
    }

    async fn area(&self, scope: Scope, name: &str, description: &str) -> Uuid {
        let id = self.id();
        self.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name, description)
             VALUES ($1, $2, $3, $4, $5)",
            &[&id, &scope.tenant, &scope.event, &name, &description],
        )
        .await;
        id
    }

    /// Links `area` to a contest of a new election with `permission_label`.
    async fn labelled_election_for(&self, scope: Scope, area: Uuid, permission_label: &str) {
        let (election, contest) = (self.id(), self.id());
        self.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, permission_label)
             VALUES ($1, $2, $3, $4)",
            &[&election, &scope.tenant, &scope.event, &permission_label],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
             VALUES ($1, $2, $3, $4)",
            &[&contest, &scope.tenant, &scope.event, &election],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.area_contest
                 (tenant_id, election_event_id, area_id, contest_id)
             VALUES ($1, $2, $3, $4)",
            &[&scope.tenant, &scope.event, &area, &contest],
        )
        .await;
    }

    async fn election(&self, scope: Scope, permission_label: Option<&str>) -> Uuid {
        let id = self.id();
        self.execute(
            "INSERT INTO sequent_backend.election
                 (id, tenant_id, election_event_id, permission_label)
             VALUES ($1, $2, $3, $4)",
            &[&id, &scope.tenant, &scope.event, &permission_label],
        )
        .await;
        id
    }

    /// An application written with SQL.
    async fn application(&self, scope: Scope, row: ApplicationRow) -> Uuid {
        let id = self.id();
        self.execute(
            "INSERT INTO sequent_backend.applications
                 (id, tenant_id, election_event_id, area_id, applicant_id, status,
                  verification_type, applicant_data, annotations, created_at,
                  permission_label)
             VALUES ($1, $2, $3, $4, $5, $6, $7, '{}', $8, $9, $10)",
            &[
                &id,
                &scope.tenant,
                &scope.event,
                &row.area,
                &format!("applicant-{id}"),
                &row.status,
                &row.verification_type,
                &row.annotations,
                &row.created_at,
                &row.permission_label,
            ],
        )
        .await;
        id
    }
}

struct ApplicationRow {
    area: Option<Uuid>,
    status: &'static str,
    verification_type: &'static str,
    annotations: Option<Value>,
    created_at: DateTime<Utc>,
    permission_label: Option<&'static str>,
}

fn pending(area: Uuid, day: u32) -> ApplicationRow {
    ApplicationRow {
        area: Some(area),
        status: "PENDING",
        verification_type: "MANUAL",
        annotations: None,
        created_at: at(day),
        permission_label: None,
    }
}

// ---------------------------------------------------------------------------
// application
// ---------------------------------------------------------------------------

fn annotations(value: Value) -> ApplicationAnnotations {
    serde_json::from_value(value).unwrap()
}

async fn application_row(tx: &Transaction<'_>, scope: Scope, id: Uuid) -> Value {
    scalar(
        tx,
        "SELECT to_jsonb(t) - 'created_at' - 'updated_at' FROM sequent_backend.applications t
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
        &[&scope.tenant, &scope.event, &id],
    )
    .await
}

fn application_ids(applications: &[Application]) -> Vec<String> {
    applications.iter().map(|a| a.id.clone()).collect()
}

async fn count(
    tx: &Transaction<'_>,
    scope: Scope,
    area: Option<&str>,
    filter: Option<&EnrollmentFilters>,
    role: Option<&str>,
) -> i64 {
    application::count_applications(
        tx,
        &scope.tenant_id(),
        &scope.event_id(),
        area,
        filter,
        role,
    )
    .await
    .unwrap()
}

fn filters(
    status: ApplicationStatus,
    verification_type: Option<ApplicationType>,
) -> EnrollmentFilters {
    EnrollmentFilters {
        status,
        verification_type,
    }
}

#[tokio::test]
async fn get_permission_label_from_post_matches_the_area_name_and_description_case_insensitively() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "North District", "Post 12 Madrid").await;
    f.labelled_election_for(a, area, "label-north").await;

    let found = application::get_permission_label_from_post(
        &tx,
        "north",
        "madrid",
        &a.tenant_id(),
        &a.event_id(),
    )
    .await
    .unwrap();
    let unmatched = application::get_permission_label_from_post(
        &tx,
        "north",
        "barcelona",
        &a.tenant_id(),
        &a.event_id(),
    )
    .await
    .unwrap();

    assert_eq!(found, (Some("label-north".to_string()), Some(area)));
    assert_eq!(unmatched, (None, None));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_permission_label_from_post_needs_an_election_linked_in_the_same_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    f.area(a, "Unlinked", "Post 1").await;
    let linked = f.area(sibling, "Linked", "Post 2").await;
    f.labelled_election_for(sibling, linked, "label-sibling")
        .await;
    let lookup = |name: &'static str, description: &'static str| {
        let tx = &tx;
        async move {
            application::get_permission_label_from_post(
                tx,
                name,
                description,
                &a.tenant_id(),
                &a.event_id(),
            )
            .await
            .unwrap()
        }
    };

    assert_eq!(lookup("Unlinked", "Post 1").await, (None, None));
    assert_eq!(lookup("Linked", "Post 2").await, (None, None));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_application_writes_the_application_with_its_enum_and_json_columns() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "Area", "Post").await;

    application::insert_application(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &Some(area),
        "applicant-1",
        &HashMap::from([("first_name".to_string(), "Ada".to_string())]),
        &Some(json!({"label": 1})),
        &annotations(json!({"session_id": "session-1", "mismatches": 2})),
        &ApplicationType::AUTOMATIC,
        &ApplicationStatus::ACCEPTED,
        &Some("label-a".to_string()),
    )
    .await
    .unwrap();

    let id: Uuid = scalar(
        &tx,
        "SELECT id FROM sequent_backend.applications WHERE tenant_id = $1",
        &[&a.tenant],
    )
    .await;
    assert_eq!(
        application_row(&tx, a, id).await,
        json!({
            "id": id.to_string(),
            "tenant_id": a.tenant_id(),
            "election_event_id": a.event_id(),
            "area_id": area.to_string(),
            "applicant_id": "applicant-1",
            "applicant_data": {"first_name": "Ada"},
            "labels": {"label": 1},
            "annotations": {
                "session_id": "session-1",
                "credentials": null,
                "verified_by": null,
                "rejection_reason": null,
                "rejection_message": null,
                "unset-attributes": null,
                "search-attributes": null,
                "update-attributes": null,
                "mismatches": 2,
                "fields_match": null,
                "manual_verify_reason": null,
            },
            "verification_type": "AUTOMATIC",
            "status": "ACCEPTED",
            "permission_label": "label-a",
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_application_status_records_the_decision_and_who_verified_it() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "Area", "Post").await;
    let id = f
        .application(
            a,
            ApplicationRow {
                annotations: Some(json!({"session_id": "kept"})),
                ..pending(area, 1)
            },
        )
        .await;

    let updated = application::update_application_status(
        &tx,
        &id.to_string(),
        &a.tenant_id(),
        &a.event_id(),
        "voter-7",
        ApplicationStatus::REJECTED,
        ApplicationType::MANUAL,
        Some("no-matching-voter".to_string()),
        Some("Not in the census".to_string()),
        "Admin Name",
        &vec!["admins".to_string(), "reviewers".to_string()],
    )
    .await
    .unwrap();

    // verified_by_role holds the JSON array as a string.
    let expected_annotations = json!({
        "session_id": "kept",
        "verified_by": "Admin Name",
        "verified_by_role": "[\"admins\",\"reviewers\"]",
        "rejection_reason": "no-matching-voter",
        "rejection_message": "Not in the census",
    });
    assert_eq!(updated.id, id.to_string());
    assert_eq!(updated.applicant_id, "voter-7");
    assert_eq!(updated.status, "REJECTED");
    assert_eq!(updated.verification_type, "MANUAL");
    assert_eq!(updated.annotations, Some(expected_annotations.clone()));
    assert_eq!(updated.area_id, Some(area.to_string()));
    assert_eq!(updated.updated_at, Some(local(now(&tx).await)));
    let row = application_row(&tx, a, id).await;
    assert_eq!(row["annotations"], expected_annotations);
    assert_eq!(row["status"], json!("REJECTED"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_application_status_without_rejection_details_only_records_the_verifier() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "Area", "Post").await;
    let id = f.application(a, pending(area, 1)).await;

    let updated = application::update_application_status(
        &tx,
        &id.to_string(),
        &a.tenant_id(),
        &a.event_id(),
        "voter-7",
        ApplicationStatus::ACCEPTED,
        ApplicationType::AUTOMATIC,
        None,
        None,
        "Admin Name",
        &vec![],
    )
    .await
    .unwrap();

    assert_eq!(
        updated.annotations,
        Some(json!({"verified_by": "Admin Name", "verified_by_role": "[]"}))
    );
    assert_eq!(updated.status, "ACCEPTED");
    assert_eq!(updated.verification_type, "AUTOMATIC");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_application_status_fails_for_a_rejection_message_without_a_reason() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "Area", "Post").await;
    let id = f.application(a, pending(area, 1)).await;

    // The message is bound as $10 while no $9 is referenced, so PostgreSQL
    // cannot prepare the statement.
    let error = application::update_application_status(
        &tx,
        &id.to_string(),
        &a.tenant_id(),
        &a.event_id(),
        "voter-7",
        ApplicationStatus::REJECTED,
        ApplicationType::MANUAL,
        None,
        Some("Not in the census".to_string()),
        "Admin Name",
        &vec![],
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Error preparing the update query: db error"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_application_status_reports_an_application_outside_the_scope() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let area = f.area(a, "Area", "Post").await;
    let id = f.application(a, pending(area, 1)).await;

    let error = application::update_application_status(
        &tx,
        &id.to_string(),
        &a.tenant_id(),
        &sibling.event_id(),
        "voter-7",
        ApplicationStatus::ACCEPTED,
        ApplicationType::MANUAL,
        None,
        None,
        "Admin Name",
        &vec![],
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!("Error updating application: No application with id {id} found.")
    );
    assert_eq!(
        application_row(&tx, a, id).await["status"],
        json!("PENDING")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_applications_pages_the_applications_of_an_area_in_creation_order() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (area, other_area) = (
        f.area(a, "Area", "Post").await,
        f.area(a, "Other", "Post").await,
    );
    let third = f.application(a, pending(area, 3)).await;
    let first = f.application(a, pending(area, 1)).await;
    let second = f.application(a, pending(area, 2)).await;
    f.application(a, pending(other_area, 1)).await;
    f.application(
        a,
        ApplicationRow {
            area: None,
            ..pending(area, 1)
        },
    )
    .await;
    f.application(sibling, pending(area, 1)).await;
    let page = |limit: Option<i64>, offset: Option<i64>| {
        let tx = &tx;
        async move {
            let (applications, last_offset) = application::get_applications(
                tx,
                &a.tenant_id(),
                &a.event_id(),
                &area.to_string(),
                None,
                limit,
                offset,
            )
            .await
            .unwrap();
            (application_ids(&applications), last_offset)
        }
    };

    assert_eq!(
        page(None, None).await,
        (strings(&[first, second, third]), Some(3))
    );
    assert_eq!(
        page(Some(2), None).await,
        (strings(&[first, second]), Some(2))
    );
    assert_eq!(page(Some(2), Some(2)).await, (strings(&[third]), Some(3)));
    assert_eq!(page(Some(2), Some(3)).await, (vec![], None));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_applications_breaks_creation_time_ties_by_id() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "Area", "Post").await;
    let first = f.application(a, pending(area, 1)).await;
    let second = f.application(a, pending(area, 1)).await;

    let (applications, _) = application::get_applications(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &area.to_string(),
        None,
        None,
        None,
    )
    .await
    .unwrap();

    assert!(first < second);
    assert_eq!(application_ids(&applications), strings(&[first, second]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_applications_filters_by_status_and_verification_type() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "Area", "Post").await;
    let manual = f.application(a, pending(area, 1)).await;
    let automatic = f
        .application(
            a,
            ApplicationRow {
                verification_type: "AUTOMATIC",
                ..pending(area, 2)
            },
        )
        .await;
    f.application(
        a,
        ApplicationRow {
            status: "ACCEPTED",
            ..pending(area, 3)
        },
    )
    .await;
    let filtered = |filter: EnrollmentFilters| {
        let tx = &tx;
        async move {
            let (applications, _) = application::get_applications(
                tx,
                &a.tenant_id(),
                &a.event_id(),
                &area.to_string(),
                Some(&filter),
                Some(10),
                Some(0),
            )
            .await
            .unwrap();
            application_ids(&applications)
        }
    };

    assert_eq!(
        filtered(filters(ApplicationStatus::PENDING, None)).await,
        strings(&[manual, automatic])
    );
    assert_eq!(
        filtered(filters(
            ApplicationStatus::PENDING,
            Some(ApplicationType::AUTOMATIC)
        ))
        .await,
        strings(&[automatic])
    );
    assert!(filtered(filters(ApplicationStatus::REJECTED, None))
        .await
        .is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn count_applications_counts_by_area_status_type_and_verifier_role() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (area, other_area) = (
        f.area(a, "Area", "Post").await,
        f.area(a, "Other", "Post").await,
    );
    let by_admin = Some(json!({"verified_by_role": "[\"admin\",\"reviewer\"]"}));
    f.application(
        a,
        ApplicationRow {
            status: "ACCEPTED",
            annotations: by_admin.clone(),
            ..pending(area, 1)
        },
    )
    .await;
    f.application(
        a,
        ApplicationRow {
            status: "ACCEPTED",
            verification_type: "AUTOMATIC",
            annotations: Some(json!({"verified_by_role": "[\"reviewer\"]"})),
            ..pending(area, 1)
        },
    )
    .await;
    f.application(a, pending(area, 1)).await;
    f.application(
        a,
        ApplicationRow {
            status: "ACCEPTED",
            annotations: by_admin,
            ..pending(other_area, 1)
        },
    )
    .await;
    f.application(sibling, pending(area, 1)).await;
    let area_id = area.to_string();
    let accepted = filters(ApplicationStatus::ACCEPTED, None);
    let accepted_manual = filters(ApplicationStatus::ACCEPTED, Some(ApplicationType::MANUAL));

    assert_eq!(count(&tx, a, None, None, None).await, 4);
    assert_eq!(count(&tx, a, Some(&area_id), None, None).await, 3);
    assert_eq!(count(&tx, a, None, None, Some("admin")).await, 2);
    assert_eq!(
        count(&tx, a, Some(&area_id), None, Some("reviewer")).await,
        2
    );
    assert_eq!(
        count(&tx, a, Some(&area_id), Some(&accepted), None).await,
        2
    );
    assert_eq!(
        count(&tx, a, None, Some(&accepted_manual), Some("admin")).await,
        2
    );
    assert_eq!(
        count(
            &tx,
            a,
            Some(&area_id),
            Some(&accepted_manual),
            Some("reviewer")
        )
        .await,
        1
    );
    tx.rollback().await.unwrap();
}

/// Exports the applications of `election`, or of the whole event, sorted by id.
async fn exported(tx: &Transaction<'_>, scope: Scope, election: Option<Uuid>) -> Vec<String> {
    let election = election.map(|id| id.to_string());
    let applications = application::get_applications_by_election(
        tx,
        &scope.tenant_id(),
        &scope.event_id(),
        election.as_deref(),
    )
    .await
    .unwrap();
    sorted(application_ids(&applications))
}

fn labelled(area: Uuid, permission_label: &'static str) -> ApplicationRow {
    ApplicationRow {
        permission_label: Some(permission_label),
        ..pending(area, 1)
    }
}

#[tokio::test]
async fn get_applications_by_election_returns_the_applications_with_the_elections_label() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let area = f.area(a, "Area", "Post").await;
    let (north, south) = (
        f.election(a, Some("north")).await,
        f.election(a, Some("south")).await,
    );
    let north_applicants = [
        f.application(a, labelled(area, "north")).await,
        f.application(a, labelled(area, "north")).await,
    ];
    let south_applicant = f.application(a, labelled(area, "south")).await;
    f.application(a, pending(area, 1)).await;
    f.application(sibling, labelled(area, "north")).await;

    // The election's Approvals tab filters on its permission label.
    assert_eq!(
        exported(&tx, a, Some(north)).await,
        sorted(strings(&north_applicants))
    );
    assert_eq!(
        exported(&tx, a, Some(south)).await,
        strings(&[south_applicant])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_applications_by_election_returns_the_whole_event_for_an_election_without_a_label() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let area = f.area(a, "Area", "Post").await;
    let (unlabelled, blank) = (f.election(a, None).await, f.election(a, Some("")).await);
    f.election(a, Some("north")).await;
    let applicants = [
        f.application(a, labelled(area, "north")).await,
        f.application(a, pending(area, 1)).await,
    ];
    f.application(sibling, pending(area, 1)).await;

    // The tab adds no label filter when the label is missing or empty.
    for election in [unlabelled, blank] {
        assert_eq!(
            exported(&tx, a, Some(election)).await,
            sorted(strings(&applicants))
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_applications_by_election_without_an_election_returns_every_application_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (area, other_area) = (
        f.area(a, "Area", "Post").await,
        f.area(a, "Other", "Post").await,
    );
    f.election(a, Some("north")).await;
    let applicants = [
        f.application(a, labelled(area, "north")).await,
        f.application(a, pending(other_area, 2)).await,
    ];
    f.application(sibling, pending(area, 1)).await;

    assert_eq!(exported(&tx, a, None).await, sorted(strings(&applicants)));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_applications_by_election_fails_for_an_election_outside_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let area = f.area(a, "Area", "Post").await;
    let elsewhere = f.election(sibling, Some("north")).await;
    f.application(a, labelled(area, "north")).await;

    let error = application::get_applications_by_election(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        Some(&elsewhere.to_string()),
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "No election found");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_applications_keeps_the_imported_ids_and_timestamps() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "Area", "Post").await;
    let (with_area, without_area) = (f.id(), f.id());
    let imported = |id: Uuid, area: Option<Uuid>| Application {
        id: id.to_string(),
        created_at: Some(local(at(1))),
        updated_at: Some(local(at(2))),
        tenant_id: a.tenant_id(),
        election_event_id: a.event_id(),
        area_id: area.map(|area| area.to_string()),
        applicant_id: format!("applicant-{id}"),
        applicant_data: json!({"email": "voter@example.test"}),
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"verified_by": "importer"})),
        verification_type: "MANUAL".to_string(),
        status: "ACCEPTED".to_string(),
    };

    application::insert_applications(
        &tx,
        &[
            imported(with_area, Some(area)),
            imported(without_area, None),
        ],
    )
    .await
    .unwrap();

    let mut loaded =
        application::get_applications_by_election(&tx, &a.tenant_id(), &a.event_id(), None)
            .await
            .unwrap();
    loaded.sort_by(|left, right| left.id.cmp(&right.id));
    assert_eq!(
        loaded,
        vec![
            imported(with_area, Some(area)),
            imported(without_area, None)
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_applications_rejects_an_application_without_timestamps() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.id();

    // The explicit NULL overrides the column default of the NOT NULL column.
    let error = application::insert_applications(
        &tx,
        &[Application {
            id: id.to_string(),
            created_at: None,
            updated_at: None,
            tenant_id: a.tenant_id(),
            election_event_id: a.event_id(),
            area_id: None,
            applicant_id: "applicant".to_string(),
            applicant_data: json!({}),
            labels: None,
            annotations: None,
            verification_type: "MANUAL".to_string(),
            status: "PENDING".to_string(),
        }],
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "Error inserting application: db error");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn application_adapters_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let area = f.area(a, "Area", "Post").await;
    let id = f.application(a, pending(area, 1)).await.to_string();
    let (tenant, event, area) = (a.tenant_id(), a.event_id(), area.to_string());

    for (tenant, event) in [(BAD_UUID, event.as_str()), (tenant.as_str(), BAD_UUID)] {
        assert_invalid_uuid(
            application::get_permission_label_from_post(&tx, "a", "b", tenant, event).await,
        );
        assert_invalid_uuid(
            application::get_applications(&tx, tenant, event, &area, None, None, None).await,
        );
        assert_invalid_uuid(
            application::count_applications(&tx, tenant, event, None, None, None).await,
        );
        assert_invalid_uuid(
            application::get_applications_by_election(&tx, tenant, event, None).await,
        );
        assert_invalid_uuid(
            application::insert_application(
                &tx,
                tenant,
                event,
                &None,
                "applicant",
                &HashMap::new(),
                &None,
                &annotations(json!({})),
                &ApplicationType::MANUAL,
                &ApplicationStatus::PENDING,
                &None,
            )
            .await,
        );
        assert_invalid_uuid(
            application::update_application_status(
                &tx,
                &id,
                tenant,
                event,
                "applicant",
                ApplicationStatus::ACCEPTED,
                ApplicationType::MANUAL,
                None,
                None,
                "admin",
                &vec![],
            )
            .await,
        );
    }
    assert_invalid_uuid(
        application::get_applications(&tx, &tenant, &event, BAD_UUID, None, None, None).await,
    );
    assert_invalid_uuid(
        application::count_applications(&tx, &tenant, &event, Some(BAD_UUID), None, None).await,
    );
    assert_eq!(
        application_row(&tx, a, id.parse().unwrap()).await["status"],
        json!("PENDING")
    );
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// secret
// ---------------------------------------------------------------------------

async fn insert_raw_secret(
    tx: &Transaction<'_>,
    id: Uuid,
    tenant: Uuid,
    event: Option<Uuid>,
    key: &str,
) {
    tx.execute(
        "INSERT INTO sequent_backend.secret (id, tenant_id, election_event_id, key, value)
         VALUES ($1, $2, $3, $4, $5)",
        &[&id, &tenant, &event, &key, &key.as_bytes()],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn insert_secret_writes_and_returns_the_secret() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let key = format!("key-{}", f.id());

    let inserted = secret::insert_secret(
        &tx,
        &a.tenant_id(),
        Some(&a.event_id()),
        &key,
        &vec![0, 159, 255],
    )
    .await
    .unwrap();

    assert_eq!(
        inserted,
        secret::Secret {
            id: inserted.id.clone(),
            tenant_id: a.tenant_id(),
            election_event_id: Some(a.event_id()),
            labels: None,
            annotations: None,
            key: key.clone(),
            value: vec![0, 159, 255],
            created_at: Some(local(now(&tx).await)),
        }
    );
    let stored: Vec<u8> = scalar(
        &tx,
        "SELECT value FROM sequent_backend.secret WHERE tenant_id = $1 AND key = $2",
        &[&a.tenant, &key],
    )
    .await;
    assert_eq!(stored, vec![0, 159, 255]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_secret_rejects_a_key_used_by_any_tenant() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let key = format!("key-{}", f.id());
    insert_raw_secret(&tx, f.id(), other.tenant, None, &key).await;

    // Keys are unique across tenants.
    let error = secret::insert_secret(&tx, &a.tenant_id(), None, &key, &vec![1])
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Error inserting secret: db error");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_secret_by_key_finds_the_tenant_secret_with_that_key() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let (id, key) = (f.id(), format!("key-{}", f.id()));
    insert_raw_secret(&tx, id, a.tenant, None, &key).await;

    let found = secret::get_secret_by_key(&tx, &a.tenant_id(), None, &key)
        .await
        .unwrap()
        .unwrap();
    let foreign = secret::get_secret_by_key(&tx, &other.tenant_id(), None, &key)
        .await
        .unwrap();

    assert_eq!(found.id, id.to_string());
    assert_eq!(found.election_event_id, None);
    assert_eq!(found.value, key.as_bytes());
    assert_eq!(foreign, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_secret_by_key_without_an_event_also_finds_event_secrets() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (id, key) = (f.id(), format!("key-{}", f.id()));
    insert_raw_secret(&tx, id, a.tenant, Some(a.event), &key).await;
    let get = |event: Option<String>| {
        let (tx, key) = (&tx, key.clone());
        async move {
            secret::get_secret_by_key(tx, &a.tenant_id(), event.as_deref(), &key)
                .await
                .unwrap()
                .map(|secret| secret.id)
        }
    };

    assert_eq!(get(None).await, Some(id.to_string()));
    assert_eq!(get(Some(a.event_id())).await, Some(id.to_string()));
    assert_eq!(get(Some(sibling.event_id())).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_secret_by_id_without_an_event_only_finds_tenant_level_secrets() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let (tenant_level, event_level) = (f.id(), f.id());
    insert_raw_secret(
        &tx,
        tenant_level,
        a.tenant,
        None,
        &format!("key-{tenant_level}"),
    )
    .await;
    insert_raw_secret(
        &tx,
        event_level,
        a.tenant,
        Some(a.event),
        &format!("key-{event_level}"),
    )
    .await;
    let get = |tenant: Uuid, event: Option<Uuid>, id: Uuid| {
        let tx = &tx;
        async move {
            secret::get_secret_by_id(
                tx,
                &tenant.to_string(),
                event.map(|event| event.to_string()).as_deref(),
                &id.to_string(),
            )
            .await
            .unwrap()
            .map(|secret| secret.key)
        }
    };

    assert_eq!(
        get(a.tenant, None, tenant_level).await,
        Some(format!("key-{tenant_level}"))
    );
    assert_eq!(get(a.tenant, None, event_level).await, None);
    assert_eq!(
        get(a.tenant, Some(a.event), event_level).await,
        Some(format!("key-{event_level}"))
    );
    assert_eq!(get(a.tenant, Some(a.event), tenant_level).await, None);
    assert_eq!(get(other.tenant, None, tenant_level).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_secret_by_id_rejects_an_id_shared_by_several_keys() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.id();
    // The primary key is (id, tenant_id, key).
    insert_raw_secret(&tx, id, a.tenant, None, &format!("key-a-{id}")).await;
    insert_raw_secret(&tx, id, a.tenant, None, &format!("key-b-{id}")).await;

    let error = secret::get_secret_by_id(&tx, &a.tenant_id(), None, &id.to_string())
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Found too many secrets: 2");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn secret_adapters_reject_invalid_uuids_naming_the_field() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (tenant, event, id) = (a.tenant_id(), a.event_id(), f.id().to_string());
    let starts_with = |error: anyhow::Error, prefix: &str| {
        assert!(error.to_string().starts_with(prefix), "{error}");
    };

    starts_with(
        secret::get_secret_by_key(&tx, BAD_UUID, None, "key")
            .await
            .unwrap_err(),
        "Error parsing tenant_id as UUID: invalid UUID 'not-a-uuid'",
    );
    starts_with(
        secret::get_secret_by_key(&tx, &tenant, Some(BAD_UUID), "key")
            .await
            .unwrap_err(),
        "Error parsing election_event_id as UUID: invalid UUID 'not-a-uuid'",
    );
    starts_with(
        secret::get_secret_by_id(&tx, BAD_UUID, None, &id)
            .await
            .unwrap_err(),
        "Error parsing tenant_id as UUID",
    );
    starts_with(
        secret::get_secret_by_id(&tx, &tenant, Some(BAD_UUID), &id)
            .await
            .unwrap_err(),
        "Error parsing election_event_id as UUID",
    );
    starts_with(
        secret::get_secret_by_id(&tx, &tenant, Some(&event), BAD_UUID)
            .await
            .unwrap_err(),
        "Error parsing secret_id as UUID",
    );
    starts_with(
        secret::insert_secret(&tx, BAD_UUID, None, "key", &vec![])
            .await
            .unwrap_err(),
        "Error parsing tenant_id as UUID",
    );
    starts_with(
        secret::insert_secret(&tx, &tenant, Some(BAD_UUID), "key", &vec![])
            .await
            .unwrap_err(),
        "Error parsing election_event_id as UUID",
    );
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// template
// ---------------------------------------------------------------------------

fn template_data(tenant: Uuid, alias: &str) -> Template {
    Template {
        alias: alias.to_string(),
        tenant_id: tenant.to_string(),
        template: json!({"subject": format!("Subject of {alias}")}),
        created_by: "admin".to_string(),
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"note": "x"})),
        created_at: None,
        updated_at: None,
        communication_method: "EMAIL".to_string(),
        r#type: "CREDENTIALS".to_string(),
    }
}

#[tokio::test]
async fn insert_templates_writes_each_template() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let tenant = f.tenant().await;

    template::insert_templates(
        &tx,
        &vec![
            template_data(tenant, "welcome"),
            template_data(tenant, "reminder"),
        ],
    )
    .await
    .unwrap();

    let rows: Vec<Value> = tx
        .query(
            "SELECT to_jsonb(t) - 'id' - 'created_at' - 'updated_at'
             FROM sequent_backend.template t WHERE tenant_id = $1 ORDER BY alias",
            &[&tenant],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    let expected = |alias: &str| {
        json!({
            "alias": alias,
            "tenant_id": tenant.to_string(),
            "template": {"subject": format!("Subject of {alias}")},
            "created_by": "admin",
            "labels": {"label": 1},
            "annotations": {"note": "x"},
            "communication_method": "EMAIL",
            "type": "CREDENTIALS",
        })
    };
    assert_eq!(rows, vec![expected("reminder"), expected("welcome")]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_template_by_alias_returns_the_tenants_template() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let (tenant, other) = (f.tenant().await, f.tenant().await);
    template::insert_templates(&tx, &vec![template_data(tenant, "welcome")])
        .await
        .unwrap();
    f.execute(
        "UPDATE sequent_backend.template SET created_at = $2 WHERE tenant_id = $1",
        &[&tenant, &at(1)],
    )
    .await;

    let found = template::get_template_by_alias(&tx, &tenant.to_string(), "welcome")
        .await
        .unwrap();
    let foreign = template::get_template_by_alias(&tx, &other.to_string(), "welcome")
        .await
        .unwrap();

    let mut expected = template_data(tenant, "welcome");
    expected.created_at = Some(local(at(1)));
    // The updated_at trigger stamps the UPDATE above.
    expected.updated_at = Some(local(now(&tx).await));
    assert_eq!(found, Some(expected));
    assert_eq!(foreign, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_templates_by_tenant_id_lists_the_templates_of_the_tenant() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let (tenant, other) = (f.tenant().await, f.tenant().await);
    template::insert_templates(
        &tx,
        &vec![
            template_data(tenant, "welcome"),
            template_data(tenant, "reminder"),
            template_data(other, "other"),
        ],
    )
    .await
    .unwrap();

    let templates = template::get_templates_by_tenant_id(&tx, &tenant.to_string())
        .await
        .unwrap();

    assert_eq!(
        sorted(
            templates
                .into_iter()
                .map(|template| template.alias)
                .collect()
        ),
        vec!["reminder", "welcome"]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn template_adapters_reject_empty_and_invalid_tenant_ids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();

    let empty = template::get_templates_by_tenant_id(&tx, "")
        .await
        .unwrap_err();
    let invalid = template::get_templates_by_tenant_id(&tx, BAD_UUID)
        .await
        .unwrap_err();

    assert_eq!(empty.to_string(), "Tenant ID is empty");
    assert!(
        invalid
            .to_string()
            .starts_with("Error parsing tenant UUID: invalid UUID 'not-a-uuid'"),
        "{invalid}"
    );
    assert_invalid_uuid(template::get_template_by_alias(&tx, BAD_UUID, "welcome").await);
    let mut bad = template_data(Uuid::nil(), "welcome");
    bad.tenant_id = BAD_UUID.to_string();
    assert_invalid_uuid(template::insert_templates(&tx, &vec![bad]).await);
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// certificate_authority
// ---------------------------------------------------------------------------

fn authority(scope: Scope, id: Uuid, fingerprint: &str) -> CertificateAuthorityRecord {
    CertificateAuthorityRecord {
        id,
        tenant_id: scope.tenant,
        election_event_id: scope.event,
        common_name: format!("CA {fingerprint}"),
        subject: format!("CN=CA {fingerprint}"),
        issuer_common_name: "Root".to_string(),
        issuer: "CN=Root".to_string(),
        not_before: at(1),
        not_after: at(31),
        fingerprint_sha256: fingerprint.to_string(),
        serial_number: "01".to_string(),
        pem: format!("PEM {fingerprint}"),
    }
}

async fn authority_ids(tx: &Transaction<'_>, event: Uuid) -> Vec<Uuid> {
    tx.query(
        "SELECT id FROM sequent_backend.certificate_authority
         WHERE election_event_id = $1 ORDER BY id",
        &[&event],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| row.get(0))
    .collect()
}

#[tokio::test]
async fn insert_certificate_authority_writes_the_record() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL TimeZone = 'UTC'")
        .await
        .unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.id();

    let inserted = certificate_authority::insert_certificate_authority(&tx, authority(a, id, "aa"))
        .await
        .unwrap();

    assert!(inserted);
    let row: Value = scalar(
        &tx,
        "SELECT to_jsonb(t) - 'created_at' FROM sequent_backend.certificate_authority t
         WHERE id = $1",
        &[&id],
    )
    .await;
    assert_eq!(
        row,
        json!({
            "id": id.to_string(),
            "tenant_id": a.tenant_id(),
            "election_event_id": a.event_id(),
            "common_name": "CA aa",
            "subject": "CN=CA aa",
            "issuer_common_name": "Root",
            "issuer": "CN=Root",
            "not_before": "2026-01-01T12:00:00+00:00",
            "not_after": "2026-01-31T12:00:00+00:00",
            "fingerprint_sha256": "aa",
            "serial_number": "01",
            "pem": "PEM aa",
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_certificate_authority_skips_a_fingerprint_already_in_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, duplicate, elsewhere) = (f.id(), f.id(), f.id());

    let insert = |record: CertificateAuthorityRecord| {
        let tx = &tx;
        async move {
            certificate_authority::insert_certificate_authority(tx, record)
                .await
                .unwrap()
        }
    };

    assert!(insert(authority(a, first, "aa")).await);
    assert!(!insert(authority(a, duplicate, "aa")).await);
    assert!(insert(authority(sibling, elsewhere, "aa")).await);
    assert_eq!(authority_ids(&tx, a.event).await, vec![first]);
    assert_eq!(authority_ids(&tx, sibling.event).await, vec![elsewhere]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_certificate_authorities_deletes_the_given_ids_of_the_event_only() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second, kept, foreign) = (f.id(), f.id(), f.id(), f.id());
    for record in [
        authority(a, first, "aa"),
        authority(a, second, "bb"),
        authority(a, kept, "cc"),
        authority(sibling, foreign, "dd"),
    ] {
        certificate_authority::insert_certificate_authority(&tx, record)
            .await
            .unwrap();
    }

    let subjects = certificate_authority::delete_certificate_authorities(
        &tx,
        &[first, second, foreign],
        a.event,
        a.tenant,
    )
    .await
    .unwrap();

    assert_eq!(sorted(subjects), vec!["CN=CA aa", "CN=CA bb"]);
    assert_eq!(authority_ids(&tx, a.event).await, vec![kept]);
    assert_eq!(authority_ids(&tx, sibling.event).await, vec![foreign]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_certificate_authorities_ignores_a_mismatched_tenant() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let id = f.id();
    certificate_authority::insert_certificate_authority(&tx, authority(a, id, "aa"))
        .await
        .unwrap();

    let subjects =
        certificate_authority::delete_certificate_authorities(&tx, &[id], a.event, other.tenant)
            .await
            .unwrap();

    assert!(subjects.is_empty());
    assert_eq!(authority_ids(&tx, a.event).await, vec![id]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_certificate_authorities_pem_lists_the_event_pems_by_creation_time() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    for (scope, fingerprint, day) in [(a, "newer", 2), (a, "older", 1), (sibling, "elsewhere", 1)] {
        let id = f.id();
        certificate_authority::insert_certificate_authority(&tx, authority(scope, id, fingerprint))
            .await
            .unwrap();
        f.execute(
            "UPDATE sequent_backend.certificate_authority SET created_at = $2 WHERE id = $1",
            &[&id, &at(day)],
        )
        .await;
    }

    let pems = certificate_authority::get_certificate_authorities_pem(&tx, a.event)
        .await
        .unwrap();

    assert_eq!(pems, vec!["PEM older", "PEM newer"]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_certificate_authorities_pem_by_ids_filters_by_id_or_returns_all_when_empty() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second, elsewhere) = (f.id(), f.id(), f.id());
    for record in [
        authority(a, first, "first"),
        authority(a, second, "second"),
        authority(sibling, elsewhere, "elsewhere"),
    ] {
        certificate_authority::insert_certificate_authority(&tx, record)
            .await
            .unwrap();
    }

    let selected = certificate_authority::get_certificate_authorities_pem_by_ids(
        &tx,
        a.event,
        &[second, elsewhere],
    )
    .await
    .unwrap();
    let all = certificate_authority::get_certificate_authorities_pem_by_ids(&tx, a.event, &[])
        .await
        .unwrap();

    assert_eq!(selected, vec!["PEM second"]);
    assert_eq!(sorted(all), vec!["PEM first", "PEM second"]);
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// phone_blacklist
// ---------------------------------------------------------------------------

#[tokio::test]
async fn insert_phone_blacklist_entry_writes_and_returns_the_entry() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let admin = f.id();
    let reason = "reported".to_string();

    let with_reason = phone_blacklist::insert_phone_blacklist_entry(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        "+34600000001",
        Some(&reason),
        &admin.to_string(),
    )
    .await
    .unwrap();
    let without_reason = phone_blacklist::insert_phone_blacklist_entry(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        "+34600000002",
        None,
        &admin.to_string(),
    )
    .await
    .unwrap();

    let now = local(now(&tx).await);
    assert_eq!(
        with_reason,
        PhoneBlacklistEntry {
            id: with_reason.id.clone(),
            tenant_id: a.tenant_id(),
            election_event_id: a.event_id(),
            phone_e164: "+34600000001".to_string(),
            reason: Some("reported".to_string()),
            created_at: now,
            created_by: admin.to_string(),
            updated_at: now,
        }
    );
    assert_eq!(without_reason.reason, None);
    let stored: Vec<(String, Option<String>)> = tx
        .query(
            "SELECT phone_e164, reason FROM sequent_backend.phone_blacklist
             WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY phone_e164",
            &[&a.tenant, &a.event],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    assert_eq!(
        stored,
        vec![
            ("+34600000001".to_string(), Some("reported".to_string())),
            ("+34600000002".to_string(), None),
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_phone_blacklist_entry_rejects_a_number_already_blacklisted_in_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let admin = f.id().to_string();
    let insert = |scope: Scope| {
        let (tx, admin) = (&tx, admin.clone());
        async move {
            phone_blacklist::insert_phone_blacklist_entry(
                tx,
                &scope.tenant_id(),
                &scope.event_id(),
                "+34600000001",
                None,
                &admin,
            )
            .await
        }
    };

    insert(a).await.unwrap();
    insert(sibling).await.unwrap();
    tx.batch_execute("SAVEPOINT duplicate").await.unwrap();
    let error = insert(a).await.unwrap_err();
    tx.batch_execute("ROLLBACK TO SAVEPOINT duplicate")
        .await
        .unwrap();

    assert_eq!(error.to_string(), "db error");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_phone_blacklist_entry_deletes_and_returns_the_entry_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let admin = f.id().to_string();
    let target = phone_blacklist::insert_phone_blacklist_entry(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        "+34600000001",
        None,
        &admin,
    )
    .await
    .unwrap();
    let kept = phone_blacklist::insert_phone_blacklist_entry(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        "+34600000002",
        None,
        &admin,
    )
    .await
    .unwrap();

    let deleted = phone_blacklist::delete_phone_blacklist_entry(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &target.id,
    )
    .await
    .unwrap();

    assert_eq!(deleted, target);
    let remaining: Vec<Uuid> = tx
        .query(
            "SELECT id FROM sequent_backend.phone_blacklist WHERE tenant_id = $1",
            &[&a.tenant],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    assert_eq!(strings(&remaining), vec![kept.id]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_phone_blacklist_entry_fails_for_an_entry_outside_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let entry = phone_blacklist::insert_phone_blacklist_entry(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        "+34600000001",
        None,
        &f.id().to_string(),
    )
    .await
    .unwrap();

    let error = phone_blacklist::delete_phone_blacklist_entry(
        &tx,
        &a.tenant_id(),
        &sibling.event_id(),
        &entry.id,
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "query returned an unexpected number of rows"
    );
    let count: i64 = scalar(
        &tx,
        "SELECT count(*) FROM sequent_backend.phone_blacklist WHERE tenant_id = $1",
        &[&a.tenant],
    )
    .await;
    assert_eq!(count, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn phone_blacklist_adapters_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (tenant, event, id) = (a.tenant_id(), a.event_id(), f.id().to_string());

    for (tenant, event, other) in [
        (BAD_UUID, event.as_str(), id.as_str()),
        (tenant.as_str(), BAD_UUID, id.as_str()),
        (tenant.as_str(), event.as_str(), BAD_UUID),
    ] {
        assert_invalid_uuid(
            phone_blacklist::insert_phone_blacklist_entry(
                &tx,
                tenant,
                event,
                "+34600000001",
                None,
                other,
            )
            .await,
        );
        assert_invalid_uuid(
            phone_blacklist::delete_phone_blacklist_entry(&tx, tenant, event, other).await,
        );
    }
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// preview
// ---------------------------------------------------------------------------

#[tokio::test]
async fn insert_preview_writes_a_preview_for_the_document() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let tenant = f.tenant().await;
    // Unlike the other adapters, any UUID version is accepted.
    let document = Uuid::parse_str("00000000-0000-1000-8000-000000000001").unwrap();

    preview::insert_preview(
        &tx,
        &tenant.to_string(),
        &document.to_string(),
        "https://preview.example.test/1".to_string(),
        "admin",
    )
    .await
    .unwrap();

    let row = tx
        .query_one(
            "SELECT to_jsonb(t) - 'id' - 'created_at' - 'updated_at', created_at, updated_at
             FROM sequent_backend.preview t WHERE tenant_id = $1",
            &[&tenant],
        )
        .await
        .unwrap();
    assert_eq!(
        row.get::<_, Value>(0),
        json!({
            "tenant_id": tenant.to_string(),
            "document_id": document.to_string(),
            "url": "https://preview.example.test/1",
            "requested_by": "admin",
            "annotations": null,
        })
    );
    let now = now(&tx).await;
    assert_eq!(
        (
            row.get::<_, DateTime<Utc>>(1),
            row.get::<_, DateTime<Utc>>(2)
        ),
        (now, now)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn preview_wrapper_maps_every_column_of_a_preview_row() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let (tenant, document, id) = (f.tenant().await, f.id(), f.id());
    f.execute(
        "INSERT INTO sequent_backend.preview
             (id, tenant_id, document_id, url, requested_by, annotations, created_at, updated_at)
         VALUES ($1, $2, $3, 'https://preview.example.test/2', 'admin', '{\"pages\": 2}', $4, $5)",
        &[&id, &tenant, &document, &at(1), &at(2)],
    )
    .await;
    let row = tx
        .query_one(
            "SELECT * FROM sequent_backend.preview WHERE id = $1",
            &[&id],
        )
        .await
        .unwrap();

    let mapped = preview::PreviewWrapper::try_from(row).unwrap();

    assert_eq!(
        mapped.0,
        Preview {
            id: id.to_string(),
            tenant_id: tenant.to_string(),
            document_id: document.to_string(),
            url: "https://preview.example.test/2".to_string(),
            requested_by: "admin".to_string(),
            created_at: Some(local(at(1))),
            updated_at: Some(local(at(2))),
            annotations: Some(json!({"pages": 2})),
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_preview_names_the_invalid_id_argument() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let (tenant, document) = (f.tenant().await.to_string(), f.id().to_string());

    for (tenant, document, expected) in [
        (
            tenant.as_str(),
            BAD_UUID,
            "Error parsing document_id as UUID",
        ),
        (
            BAD_UUID,
            document.as_str(),
            "Error parsing tenant_id as UUID",
        ),
    ] {
        let error = preview::insert_preview(&tx, tenant, document, "url".to_string(), "admin")
            .await
            .unwrap_err();
        assert_eq!(error.to_string(), expected);
    }
    let count: i64 = scalar(
        &tx,
        "SELECT count(*) FROM sequent_backend.preview WHERE requested_by = 'admin' AND url = 'url'",
        &[],
    )
    .await;
    assert_eq!(count, 0);
    tx.rollback().await.unwrap();
}
