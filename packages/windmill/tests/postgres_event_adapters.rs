// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tenant, election event, election and Keycloak realm adapters against
//! the migrated schema. Every test writes its own rows in a transaction,
//! checks them with independent SQL and rolls the transaction back.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local, TimeZone, Utc};
use deadpool_postgres::{Object, Transaction};
use sequent_core::ballot::{ElectionPresentation, ElectionStatus};
use sequent_core::election_config::ImportElectionEventSchema;
use sequent_core::types::hasura::core::{Election, ElectionEvent, Tenant};
use serde_json::{json, Value};
use std::cell::Cell;
use tokio_postgres::error::SqlState;
use tokio_postgres::types::{FromSql, ToSql};
use uuid::Uuid;
use windmill::postgres::{election, election_event, keycloak_realm, tenant};

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

fn database_error(error: &anyhow::Error) -> &tokio_postgres::error::DbError {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<tokio_postgres::Error>())
        .and_then(tokio_postgres::Error::as_db_error)
        .expect("a database error")
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

/// Every column of a row but its timestamps, as PostgreSQL renders it to JSON.
async fn row_json(tx: &Transaction<'_>, table: &str, key: &str, id: Uuid) -> Value {
    scalar(
        tx,
        &format!(
            "SELECT to_jsonb(t) - 'created_at' - 'updated_at' - 'last_updated_at'
             FROM sequent_backend.{table} t WHERE {key} = $1"
        ),
        &[&id],
    )
    .await
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

    async fn election(&self, scope: Scope) -> Uuid {
        let id = self.id();
        self.election_as(scope, id).await;
        id
    }

    async fn election_as(&self, scope: Scope, id: Uuid) {
        self.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&id, &scope.tenant, &scope.event],
        )
        .await;
    }

    async fn contest(&self, scope: Scope, election: Uuid) -> Uuid {
        let id = self.id();
        self.execute(
            "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
             VALUES ($1, $2, $3, $4)",
            &[&id, &scope.tenant, &scope.event, &election],
        )
        .await;
        id
    }

    async fn area(&self, scope: Scope) -> Uuid {
        let id = self.id();
        self.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, $4)",
            &[&id, &scope.tenant, &scope.event, &format!("area-{id}")],
        )
        .await;
        id
    }

    async fn area_contest(&self, scope: Scope, area: Uuid, contest: Uuid) {
        self.execute(
            "INSERT INTO sequent_backend.area_contest
                 (id, tenant_id, election_event_id, area_id, contest_id)
             VALUES ($1, $2, $3, $4, $5)",
            &[&self.id(), &scope.tenant, &scope.event, &area, &contest],
        )
        .await;
    }

    /// One row in every event-scoped table this module's tests look at.
    async fn populate(&self, scope: Scope) {
        let election = self.election(scope).await;
        let contest = self.contest(scope, election).await;
        let area = self.area(scope).await;
        self.area_contest(scope, area, contest).await;
        let publication = self.id();
        let params: [&(dyn ToSql + Sync); 2] = [&scope.tenant, &scope.event];
        self.execute(
            "INSERT INTO sequent_backend.candidate (tenant_id, election_event_id, contest_id)
             VALUES ($1, $2, $3)",
            &[&scope.tenant, &scope.event, &contest],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.ballot_publication (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&publication, &scope.tenant, &scope.event],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.ballot_style
                 (tenant_id, election_event_id, election_id, area_id, ballot_publication_id)
             VALUES ($1, $2, $3, $4, $5)",
            &[&scope.tenant, &scope.event, &election, &area, &publication],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.cast_vote
                 (tenant_id, election_event_id, election_id, area_id, voter_id_string)
             VALUES ($1, $2, $3, $4, 'voter')",
            &[&scope.tenant, &scope.event, &election, &area],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.applications
                 (tenant_id, election_event_id, area_id, applicant_id, status,
                  verification_type, applicant_data)
             VALUES ($1, $2, $3, 'applicant', 'PENDING', 'MANUAL', '{}')",
            &[&scope.tenant, &scope.event, &area],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.secret (tenant_id, election_event_id, key, value)
             VALUES ($1, $2, $3, '\\x00')",
            &[
                &scope.tenant,
                &scope.event,
                &format!("secret-{}", self.id()),
            ],
        )
        .await;
        for sql in [
            "INSERT INTO sequent_backend.document (tenant_id, election_event_id, name)
             VALUES ($1, $2, 'document')",
            "INSERT INTO sequent_backend.scheduled_event
                 (tenant_id, election_event_id, event_processor)
             VALUES ($1, $2, 'processor')",
            "INSERT INTO sequent_backend.tasks_execution
                 (tenant_id, election_event_id, name, type, execution_status, executed_by_user)
             VALUES ($1, $2, 'task', 'type', 'SUCCESS', 'admin')",
        ] {
            self.execute(sql, &params).await;
        }
    }

    /// A blacklisted phone number, as the admin PhoneBlacklist page adds it.
    async fn phone_blacklist_entry(&self, scope: Scope) {
        self.execute(
            "INSERT INTO sequent_backend.phone_blacklist
                 (tenant_id, election_event_id, phone_e164, created_by)
             VALUES ($1, $2, '+34600000000', $3)",
            &[&scope.tenant, &scope.event, &self.id()],
        )
        .await;
    }

    /// A tally sheet import with its source document, one item and the tally
    /// sheet that item generated.
    async fn tally_sheet_import(&self, scope: Scope) {
        let election = self.election(scope).await;
        let contest = self.contest(scope, election).await;
        let area = self.area(scope).await;
        let (document, import, sheet) = (self.id(), self.id(), self.id());
        self.execute(
            "INSERT INTO sequent_backend.document (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, 'tally-sheets.csv')",
            &[&document, &scope.tenant, &scope.event],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.tally_sheet_import
                 (id, tenant_id, election_event_id, source_document_id, source_format,
                  selected_channel, created_by_user_id)
             VALUES ($1, $2, $3, $4, 'CSV', 'PAPER', 'admin')",
            &[&import, &scope.tenant, &scope.event, &document],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.tally_sheet
                 (id, tenant_id, election_event_id, election_id, contest_id, area_id,
                  created_by_user_id, version, import_id)
             VALUES ($1, $2, $3, $4, $5, $6, 'admin', 1, $7)",
            &[
                &sheet,
                &scope.tenant,
                &scope.event,
                &election,
                &contest,
                &area,
                &import,
            ],
        )
        .await;
        self.execute(
            "INSERT INTO sequent_backend.tally_sheet_import_item
                 (tenant_id, election_event_id, import_id, election_id, area_id, contest_id,
                  channel, generated_tally_sheet_id, baseline_approved_tally_sheet_id,
                  incoming_content_hash, change_type, incoming_csv)
             VALUES ($1, $2, $3, $4, $5, $6, 'PAPER', $7, $7, 'hash', 'NEW', 'csv')",
            &[
                &scope.tenant,
                &scope.event,
                &import,
                &election,
                &area,
                &contest,
                &sheet,
            ],
        )
        .await;
    }
}

const POPULATED_TABLES: [&str; 13] = [
    "election",
    "contest",
    "candidate",
    "area",
    "area_contest",
    "ballot_publication",
    "ballot_style",
    "cast_vote",
    "applications",
    "secret",
    "document",
    "scheduled_event",
    "tasks_execution",
];

/// The tables delete_election_event must clear for events with blacklisted
/// phones or tally sheet imports.
const BLOCKING_TABLES: [&str; 5] = [
    "phone_blacklist",
    "tally_sheet_import",
    "tally_sheet_import_item",
    "tally_sheet",
    "document",
];

async fn event_row_counts(tx: &Transaction<'_>, scope: Scope) -> Vec<(&'static str, i64)> {
    row_counts(tx, scope, &POPULATED_TABLES).await
}

async fn row_counts(
    tx: &Transaction<'_>,
    scope: Scope,
    tables: &[&'static str],
) -> Vec<(&'static str, i64)> {
    let mut counts = Vec::new();
    for &table in tables {
        let count: i64 = scalar(
            tx,
            &format!(
                "SELECT count(*) FROM sequent_backend.{table}
                 WHERE tenant_id = $1 AND election_event_id = $2"
            ),
            &[&scope.tenant, &scope.event],
        )
        .await;
        counts.push((table, count));
    }
    counts
}

async fn event_exists(tx: &Transaction<'_>, event: Uuid) -> bool {
    scalar(
        tx,
        "SELECT EXISTS (SELECT 1 FROM sequent_backend.election_event WHERE id = $1)",
        &[&event],
    )
    .await
}

// ---------------------------------------------------------------------------
// tenant
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_tenant_by_id_maps_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let id = f.id();
    f.execute(
        "INSERT INTO sequent_backend.tenant
             (id, slug, created_at, updated_at, labels, annotations, is_active,
              voting_channels, settings, test)
         VALUES ($1, 'mapped', $2, $3, $4, $5, true, $6, $7, 1)",
        &[
            &id,
            &at(1),
            &at(2),
            &json!({"label": 1}),
            &json!({"note": "x"}),
            &json!({"online": false}),
            &json!({"theme": "dark"}),
        ],
    )
    .await;

    let loaded = tenant::get_tenant_by_id(&tx, &id.to_string())
        .await
        .unwrap();

    assert_eq!(
        loaded,
        Tenant {
            id: id.to_string(),
            slug: "mapped".to_string(),
            created_at: Some(local(at(1))),
            updated_at: Some(local(at(2))),
            labels: Some(json!({"label": 1})),
            annotations: Some(json!({"note": "x"})),
            is_active: true,
            voting_channels: Some(json!({"online": false})),
            settings: Some(json!({"theme": "dark"})),
            test: Some(1),
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tenant_by_id_reports_a_missing_tenant() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    f.tenant().await;

    let error = tenant::get_tenant_by_id(&tx, &f.id().to_string())
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Error obtaining Tenant");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn tenant_lookups_reject_empty_and_invalid_ids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();

    for error in [
        tenant::get_tenant_by_id(&tx, "").await.unwrap_err(),
        tenant::get_tenant_by_id_if_exist(&tx, "")
            .await
            .unwrap_err(),
    ] {
        assert_eq!(error.to_string(), "Tenant ID is empty");
    }
    for error in [
        tenant::get_tenant_by_id(&tx, BAD_UUID).await.unwrap_err(),
        tenant::get_tenant_by_id_if_exist(&tx, BAD_UUID)
            .await
            .unwrap_err(),
    ] {
        assert!(
            error
                .to_string()
                .starts_with("Error parsing tenant UUID: invalid UUID 'not-a-uuid'"),
            "{error}"
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tenant_by_id_if_exist_returns_the_tenant_or_none() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let id = f.tenant().await;

    let found = tenant::get_tenant_by_id_if_exist(&tx, &id.to_string())
        .await
        .unwrap();
    let missing = tenant::get_tenant_by_id_if_exist(&tx, &f.id().to_string())
        .await
        .unwrap();

    assert_eq!(
        found.map(|tenant| tenant.slug),
        Some(format!("tenant-{id}"))
    );
    assert_eq!(missing, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tenant_by_slug_if_exist_finds_the_tenant_with_that_slug() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let id = f.tenant().await;
    f.tenant().await;

    let found = tenant::get_tenant_by_slug_if_exist(&tx, &format!("tenant-{id}"))
        .await
        .unwrap();
    let missing = tenant::get_tenant_by_slug_if_exist(&tx, &format!("missing-{id}"))
        .await
        .unwrap();

    assert_eq!(found.map(|tenant| tenant.id), Some(id.to_string()));
    assert_eq!(missing, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tenant_writes_an_active_tenant_with_the_schema_defaults() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let id = f.id();

    tenant::insert_tenant(&tx, &id.to_string(), &format!("slug-{id}"))
        .await
        .unwrap();

    assert_eq!(
        row_json(&tx, "tenant", "id", id).await,
        json!({
            "id": id.to_string(),
            "slug": format!("slug-{id}"),
            "labels": null,
            "annotations": null,
            "is_active": true,
            "voting_channels": {"kiosk": true, "online": true},
            "settings": null,
            "test": 0,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tenant_rejects_a_taken_slug_with_the_update_error_message() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let existing = f.tenant().await;

    let error = tenant::insert_tenant(&tx, &f.id().to_string(), &format!("tenant-{existing}"))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Failed to execute update tenant");
    assert_eq!(database_error(&error).code(), &SqlState::UNIQUE_VIOLATION);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tenant_overwrites_the_mutable_columns_of_the_target_tenant_only() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let target = f.tenant().await;
    let other = f.tenant().await;

    tenant::update_tenant(
        &tx,
        Tenant {
            id: other.to_string(),
            slug: "ignored".to_string(),
            created_at: Some(local(at(1))),
            updated_at: Some(local(at(2))),
            labels: Some(json!({"label": 2})),
            annotations: Some(json!({"note": "y"})),
            is_active: true,
            voting_channels: Some(json!({"online": true})),
            settings: Some(json!({"theme": "light"})),
            test: Some(1),
        },
        &target.to_string(),
    )
    .await
    .unwrap();

    // Neither the id nor the slug of the argument is written.
    assert_eq!(
        row_json(&tx, "tenant", "id", target).await,
        json!({
            "id": target.to_string(),
            "slug": format!("tenant-{target}"),
            "labels": {"label": 2},
            "annotations": {"note": "y"},
            "is_active": true,
            "voting_channels": {"online": true},
            "settings": {"theme": "light"},
            "test": 1,
        })
    );
    let created_at: DateTime<Utc> = scalar(
        &tx,
        "SELECT created_at FROM sequent_backend.tenant WHERE id = $1",
        &[&target],
    )
    .await;
    assert_eq!(created_at, at(1));
    assert_eq!(
        row_json(&tx, "tenant", "id", other).await["is_active"],
        json!(false)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tenant_updated_at_is_always_the_transaction_time() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let id = f.tenant().await;
    let mut loaded = tenant::get_tenant_by_id(&tx, &id.to_string())
        .await
        .unwrap();
    loaded.updated_at = Some(local(at(2)));

    tenant::update_tenant(&tx, loaded, &id.to_string())
        .await
        .unwrap();

    // The set_current_timestamp_updated_at trigger overrides the argument.
    let updated_at: DateTime<Utc> = scalar(
        &tx,
        "SELECT updated_at FROM sequent_backend.tenant WHERE id = $1",
        &[&id],
    )
    .await;
    assert_eq!(updated_at, now(&tx).await);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tenant_fails_for_a_missing_tenant_or_an_invalid_id() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let id = f.tenant().await;
    let loaded = tenant::get_tenant_by_id(&tx, &id.to_string())
        .await
        .unwrap();

    let missing = tenant::update_tenant(&tx, loaded.clone(), &f.id().to_string())
        .await
        .unwrap_err();
    let invalid = tenant::update_tenant(&tx, loaded, BAD_UUID)
        .await
        .unwrap_err();

    assert_eq!(
        missing.to_string(),
        "No tenant found with the given tenant_id and id"
    );
    assert_eq!(invalid.to_string(), "Failed to parse old_tenant_id as UUID");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_tenant_removes_the_tenant_and_its_tenant_level_rows() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let target = f.tenant().await;
    let other = f.tenant().await;
    let tables = [
        "trustee",
        "template",
        "election_type",
        "document",
        "tasks_execution",
        "secret",
    ];
    for tenant in [target, other] {
        f.execute(
            "INSERT INTO sequent_backend.trustee (tenant_id, name) VALUES ($1, 'trustee')",
            &[&tenant],
        )
        .await;
        f.execute(
            "INSERT INTO sequent_backend.template
                 (tenant_id, template, created_by, communication_method, type)
             VALUES ($1, '{}', 'admin', 'EMAIL', 'CREDENTIALS')",
            &[&tenant],
        )
        .await;
        f.execute(
            "INSERT INTO sequent_backend.election_type (tenant_id, name) VALUES ($1, 'type')",
            &[&tenant],
        )
        .await;
        f.execute(
            "INSERT INTO sequent_backend.document (tenant_id, name) VALUES ($1, 'export')",
            &[&tenant],
        )
        .await;
        f.execute(
            "INSERT INTO sequent_backend.tasks_execution
                 (tenant_id, name, type, execution_status, executed_by_user)
             VALUES ($1, 'delete-tenant', 'type', 'FAILED', 'admin')",
            &[&tenant],
        )
        .await;
        f.execute(
            "INSERT INTO sequent_backend.secret (tenant_id, key, value)
             VALUES ($1, $2, '\\x00')",
            &[&tenant, &format!("secret-{tenant}")],
        )
        .await;
    }

    tenant::delete_tenant(&tx, &target.to_string())
        .await
        .unwrap();

    for (tenant, expected) in [(target, 0), (other, 1)] {
        let exists: bool = scalar(
            &tx,
            "SELECT EXISTS (SELECT 1 FROM sequent_backend.tenant WHERE id = $1)",
            &[&tenant],
        )
        .await;
        assert_eq!(exists, expected == 1);
        for table in tables {
            let count: i64 = scalar(
                &tx,
                &format!("SELECT count(*) FROM sequent_backend.{table} WHERE tenant_id = $1"),
                &[&tenant],
            )
            .await;
            assert_eq!(count, expected, "{table} of {tenant}");
        }
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_tenant_fails_while_the_tenant_still_has_election_events() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;

    let error = tenant::delete_tenant(&tx, &a.tenant_id())
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Error executing the delete query: db error"
    );
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// election_event
// ---------------------------------------------------------------------------

fn event_data(scope: Scope) -> ElectionEvent {
    ElectionEvent {
        id: scope.event_id(),
        created_at: None,
        updated_at: None,
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"note": "x"})),
        tenant_id: scope.tenant_id(),
        description: Some("Board election".to_string()),
        presentation: Some(json!({"logo_url": "https://example.test/logo.png"})),
        bulletin_board_reference: Some(
            json!({"id": 7, "database_name": "board", "is_archived": false}),
        ),
        is_archived: false,
        voting_channels: Some(json!({"online": true, "kiosk": false})),
        status: Some(json!({"voting_status": "OPEN"})),
        user_boards: Some("boards".to_string()),
        encryption_protocol: "RSA256".to_string(),
        is_audit: Some(false),
        audit_election_event_id: None,
        public_key: Some("public-key".to_string()),
        statistics: Some(json!({"num_emails_sent": 3})),
        external_id: Some("external".to_string()),
    }
}

#[tokio::test]
async fn insert_election_event_writes_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let audited = f.scope().await;
    let scope = Scope {
        tenant: audited.tenant,
        event: f.id(),
    };
    let mut event = event_data(scope);
    event.audit_election_event_id = Some(audited.event_id());

    election_event::insert_election_event(&tx, &event)
        .await
        .unwrap();

    assert_eq!(
        row_json(&tx, "election_event", "id", scope.event).await,
        json!({
            "id": scope.event_id(),
            "tenant_id": scope.tenant_id(),
            "labels": {"label": 1},
            "annotations": {"note": "x"},
            "description": "Board election",
            "presentation": {"logo_url": "https://example.test/logo.png"},
            "bulletin_board_reference": {"id": 7, "database_name": "board", "is_archived": false},
            "is_archived": false,
            "voting_channels": {"online": true, "kiosk": false},
            "status": {"voting_status": "OPEN"},
            "user_boards": "boards",
            "encryption_protocol": "RSA256",
            "is_audit": false,
            "audit_election_event_id": audited.event_id(),
            "public_key": "public-key",
            "statistics": {"num_emails_sent": 3},
            "external_id": "external",
        })
    );
    let created_at: DateTime<Utc> = scalar(
        &tx,
        "SELECT created_at FROM sequent_backend.election_event WHERE id = $1",
        &[&scope.event],
    )
    .await;
    assert_eq!(created_at, now(&tx).await);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_election_event_silently_drops_an_invalid_audit_event_id() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let tenant = f.tenant().await;
    let scope = Scope {
        tenant,
        event: f.id(),
    };
    let mut event = event_data(scope);
    event.audit_election_event_id = Some(BAD_UUID.to_string());

    election_event::insert_election_event(&tx, &event)
        .await
        .unwrap();

    assert_eq!(
        row_json(&tx, "election_event", "id", scope.event).await["audit_election_event_id"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_election_event_rejects_invalid_policy_json_before_writing() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let tenant = f.tenant().await;
    let scope = Scope {
        tenant,
        event: f.id(),
    };
    let mut event = event_data(scope);
    event.voting_channels = Some(json!({"online": "yes"}));

    let error = election_event::insert_election_event(&tx, &event)
        .await
        .unwrap_err();

    assert!(error.to_string().contains("invalid type"), "{error}");
    assert!(!event_exists(&tx, scope.event).await);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_event_by_id_maps_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let audited = f.scope().await;
    let scope = Scope {
        tenant: audited.tenant,
        event: f.id(),
    };
    let mut expected = event_data(scope);
    expected.audit_election_event_id = Some(audited.event_id());
    expected.is_audit = Some(true);
    election_event::insert_election_event(&tx, &expected)
        .await
        .unwrap();
    f.execute(
        "UPDATE sequent_backend.election_event SET created_at = $2, updated_at = $3
         WHERE id = $1",
        &[&scope.event, &at(1), &at(2)],
    )
    .await;

    let loaded =
        election_event::get_election_event_by_id(&tx, &scope.tenant_id(), &scope.event_id())
            .await
            .unwrap();

    expected.created_at = Some(local(at(1)));
    // The updated_at trigger stamps every UPDATE with the transaction time.
    expected.updated_at = Some(local(now(&tx).await));
    assert_eq!(loaded, expected);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_event_by_id_reports_events_outside_the_tenant() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;

    let error = election_event::get_election_event_by_id(&tx, &other.tenant_id(), &a.event_id())
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!("Election event {} not found", a.event)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_event_by_id_if_exist_returns_none_outside_the_tenant() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;

    let found =
        election_event::get_election_event_by_id_if_exist(&tx, &a.tenant_id(), &a.event_id())
            .await
            .unwrap();
    let foreign =
        election_event::get_election_event_by_id_if_exist(&tx, &other.tenant_id(), &a.event_id())
            .await
            .unwrap();

    assert_eq!(found.map(|event| event.id), Some(a.event_id()));
    assert_eq!(foreign, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_all_tenant_election_events_returns_the_unarchived_events_of_the_tenant() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let archived = f.event_in(a.tenant).await;
    f.scope().await;
    f.execute(
        "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1",
        &[&archived.event],
    )
    .await;

    let events = election_event::get_all_tenant_election_events(&tx, &a.tenant_id())
        .await
        .unwrap();

    assert_eq!(
        sorted(events.into_iter().map(|event| event.0.id).collect()),
        strings(&[a.event, sibling.event])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn count_tenant_election_events_counts_archived_events_too() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let archived = f.event_in(a.tenant).await;
    let empty = f.tenant().await;
    f.scope().await;
    f.execute(
        "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1",
        &[&archived.event],
    )
    .await;

    assert_eq!(
        election_event::count_tenant_election_events(&tx, &a.tenant_id())
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        election_event::count_tenant_election_events(&tx, &empty.to_string())
            .await
            .unwrap(),
        0
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_election_event_annotations_replaces_the_annotations_of_one_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let other = f.scope().await;
    f.execute(
        "UPDATE sequent_backend.election_event SET annotations = '{\"old\": true}'
         WHERE tenant_id = ANY($1)",
        &[&vec![a.tenant, other.tenant]],
    )
    .await;

    election_event::update_election_event_annotations(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        json!({"new": 1}),
    )
    .await
    .unwrap();
    // A mismatched tenant and event pair changes nothing.
    election_event::update_election_event_annotations(
        &tx,
        &other.tenant_id(),
        &sibling.event_id(),
        json!({"new": 2}),
    )
    .await
    .unwrap();

    for (scope, expected) in [
        (a, json!({"new": 1})),
        (sibling, json!({"old": true})),
        (other, json!({"old": true})),
    ] {
        assert_eq!(
            row_json(&tx, "election_event", "id", scope.event).await["annotations"],
            expected
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_election_event_presentation_can_change_the_results_website_outside_hasura() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let presentation = json!({"results_website": "https://results.example.test"});

    // The results website guard only applies to Hasura sessions.
    election_event::update_election_event_presentation(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        presentation.clone(),
    )
    .await
    .unwrap();

    assert_eq!(
        row_json(&tx, "election_event", "id", a.event).await["presentation"],
        presentation
    );
    assert_eq!(
        row_json(&tx, "election_event", "id", sibling.event).await["presentation"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_elections_status_by_election_event_sets_the_status_of_every_election_of_the_event()
{
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let untouched = f.election(sibling).await;

    let ids = election_event::update_elections_status_by_election_event(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        json!({"voting_status": "CLOSED"}),
    )
    .await
    .unwrap();

    assert_eq!(sorted(ids), strings(&[first, second]));
    for (id, expected) in [
        (first, json!({"voting_status": "CLOSED"})),
        (second, json!({"voting_status": "CLOSED"})),
        (untouched, Value::Null),
    ] {
        assert_eq!(
            row_json(&tx, "election", "id", id).await["status"],
            expected
        );
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_election_event_status_sets_the_status_of_one_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;

    election_event::update_election_event_status(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        json!({"voting_status": "PAUSED"}),
    )
    .await
    .unwrap();

    assert_eq!(
        row_json(&tx, "election_event", "id", a.event).await["status"],
        json!({"voting_status": "PAUSED"})
    );
    assert_eq!(
        row_json(&tx, "election_event", "id", sibling.event).await["status"],
        Value::Null
    );
    assert_eq!(
        row_json(&tx, "election", "id", election).await["status"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_bulletin_board_sets_the_board_reference_of_one_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let board = json!({"id": 3, "database_name": "board-3", "is_archived": false});

    election_event::update_bulletin_board(&tx, &a.tenant_id(), &a.event_id(), &board)
        .await
        .unwrap();

    assert_eq!(
        row_json(&tx, "election_event", "id", a.event).await["bulletin_board_reference"],
        board
    );
    assert_eq!(
        row_json(&tx, "election_event", "id", sibling.event).await["bulletin_board_reference"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_event_by_election_area_finds_the_event_through_an_area_contest() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let contest = f.contest(a, election).await;
    let (linked, unlinked) = (f.area(a).await, f.area(a).await);
    f.area_contest(a, linked, contest).await;

    let event = election_event::get_election_event_by_election_area(
        &tx,
        &a.tenant_id(),
        &election.to_string(),
        &linked.to_string(),
    )
    .await
    .unwrap();
    let error = election_event::get_election_event_by_election_area(
        &tx,
        &a.tenant_id(),
        &election.to_string(),
        &unlinked.to_string(),
    )
    .await
    .unwrap_err();

    assert_eq!(event.id, a.event_id());
    assert_eq!(error.to_string(), "Election event not found");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_election_event_removes_the_event_and_its_rows_only() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    f.populate(a).await;
    f.populate(sibling).await;

    election_event::delete_election_event(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    assert!(!event_exists(&tx, a.event).await);
    assert!(event_exists(&tx, sibling.event).await);
    assert!(event_row_counts(&tx, a)
        .await
        .iter()
        .all(|(_, count)| *count == 0));
    assert!(event_row_counts(&tx, sibling)
        .await
        .iter()
        .all(|(_, count)| *count == 1));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_election_event_removes_the_events_phone_blacklist_entries() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    f.phone_blacklist_entry(a).await;
    f.phone_blacklist_entry(sibling).await;

    election_event::delete_election_event(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    assert!(!event_exists(&tx, a.event).await);
    assert_eq!(
        row_counts(&tx, a, &["phone_blacklist"]).await,
        vec![("phone_blacklist", 0)]
    );
    assert_eq!(
        row_counts(&tx, sibling, &["phone_blacklist"]).await,
        vec![("phone_blacklist", 1)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_election_event_removes_the_events_tally_sheet_imports() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    f.tally_sheet_import(a).await;
    f.tally_sheet_import(sibling).await;

    election_event::delete_election_event(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    assert!(!event_exists(&tx, a.event).await);
    assert!(row_counts(&tx, a, &BLOCKING_TABLES)
        .await
        .iter()
        .all(|(_, count)| *count == 0));
    let sibling_counts = row_counts(&tx, sibling, &BLOCKING_TABLES).await;
    assert_eq!(
        sibling_counts,
        vec![
            ("phone_blacklist", 0),
            ("tally_sheet_import", 1),
            ("tally_sheet_import_item", 1),
            ("tally_sheet", 1),
            ("document", 1),
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_tenant_succeeds_once_events_with_blacklist_and_import_rows_are_deleted() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let first = f.scope().await;
    let second = f.event_in(first.tenant).await;
    let other = f.scope().await;
    for scope in [first, second, other] {
        f.phone_blacklist_entry(scope).await;
        f.tally_sheet_import(scope).await;
    }

    // The delete-tenant task requires the tenant to have no events left.
    for scope in [first, second] {
        election_event::delete_election_event(&tx, &scope.tenant_id(), &scope.event_id())
            .await
            .unwrap();
    }
    assert_eq!(
        election_event::count_tenant_election_events(&tx, &first.tenant_id())
            .await
            .unwrap(),
        0
    );
    tenant::delete_tenant(&tx, &first.tenant_id())
        .await
        .unwrap();

    let tenant_exists = |tenant: Uuid| {
        let tx = &tx;
        async move {
            scalar::<bool>(
                tx,
                "SELECT EXISTS (SELECT 1 FROM sequent_backend.tenant WHERE id = $1)",
                &[&tenant],
            )
            .await
        }
    };
    assert!(!tenant_exists(first.tenant).await);
    assert!(tenant_exists(other.tenant).await);
    assert!(event_exists(&tx, other.event).await);
    assert!(row_counts(&tx, other, &BLOCKING_TABLES)
        .await
        .iter()
        .all(|(_, count)| *count == 1));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_batch_election_events_pages_unarchived_events_of_every_tenant_by_creation_time() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let first = f.scope().await;
    let second = f.scope().await;
    let third = f.event_in(first.tenant).await;
    let archived = f.event_in(first.tenant).await;
    for (scope, day) in [(third, 4), (first, 2), (archived, 1), (second, 3)] {
        f.execute(
            "UPDATE sequent_backend.election_event SET created_at = $2 WHERE id = $1",
            &[&scope.event, &at(day)],
        )
        .await;
    }
    f.execute(
        "UPDATE sequent_backend.election_event SET is_archived = true WHERE id = $1",
        &[&archived.event],
    )
    .await;
    let page = |limit: i64, offset: i64| {
        let tx = &tx;
        async move {
            election_event::get_batch_election_events(tx, limit, offset)
                .await
                .unwrap()
                .into_iter()
                .map(|event| event.id)
                .collect::<Vec<_>>()
        }
    };

    assert_eq!(page(2, 0).await, strings(&[first.event, second.event]));
    assert_eq!(page(2, 2).await, strings(&[third.event]));
    assert!(page(2, 3).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn election_event_adapters_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (tenant, event) = (a.tenant_id(), a.event_id());

    for (tenant, event) in [(BAD_UUID, event.as_str()), (tenant.as_str(), BAD_UUID)] {
        assert_invalid_uuid(election_event::get_election_event_by_id(&tx, tenant, event).await);
        assert_invalid_uuid(
            election_event::get_election_event_by_id_if_exist(&tx, tenant, event).await,
        );
        assert_invalid_uuid(
            election_event::update_election_event_annotations(&tx, tenant, event, json!({})).await,
        );
        assert_invalid_uuid(
            election_event::update_election_event_presentation(&tx, tenant, event, json!({})).await,
        );
        assert_invalid_uuid(
            election_event::update_elections_status_by_election_event(
                &tx,
                tenant,
                event,
                json!({}),
            )
            .await,
        );
        assert_invalid_uuid(
            election_event::update_election_event_status(&tx, tenant, event, json!({})).await,
        );
        assert_invalid_uuid(
            election_event::update_bulletin_board(&tx, tenant, event, &json!({})).await,
        );
        assert_invalid_uuid(election_event::delete_election_event(&tx, tenant, event).await);
    }
    assert_invalid_uuid(
        election_event::get_all_tenant_election_events(&tx, BAD_UUID)
            .await
            .map(|events| events.len()),
    );
    assert_invalid_uuid(election_event::count_tenant_election_events(&tx, BAD_UUID).await);
    assert!(event_exists(&tx, a.event).await);
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// election
// ---------------------------------------------------------------------------

fn election_data(scope: Scope, id: Uuid, keys_ceremony: Uuid) -> Election {
    Election {
        id: id.to_string(),
        tenant_id: scope.tenant_id(),
        election_event_id: scope.event_id(),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"note": "x"})),
        description: Some("President".to_string()),
        presentation: Some(json!({"sort_order": 2})),
        status: Some(json!({"voting_status": "OPEN"})),
        eml: Some("<eml/>".to_string()),
        external_id: Some("external".to_string()),
        num_allowed_revotes: Some(3),
        is_consolidated_ballot_encoding: Some(true),
        spoil_ballot_option: Some(false),
        is_kiosk: Some(true),
        voting_channels: Some(json!({"online": true})),
        image_document_id: Some("image".to_string()),
        statistics: Some(json!({"num_emails_sent": 1})),
        receipts: Some(json!({"receipt": true})),
        permission_label: Some("label-a".to_string()),
        initialization_report_generated: Some(true),
        keys_ceremony_id: Some(keys_ceremony.to_string()),
    }
}

/// The JSON PostgreSQL renders for the row `election_data` describes.
fn election_json(scope: Scope, id: Uuid, keys_ceremony: Uuid) -> Value {
    json!({
        "id": id.to_string(),
        "tenant_id": scope.tenant_id(),
        "election_event_id": scope.event_id(),
        "labels": {"label": 1},
        "annotations": {"note": "x"},
        "description": "President",
        "presentation": {"sort_order": 2},
        "status": {"voting_status": "OPEN"},
        "eml": "<eml/>",
        "external_id": "external",
        "num_allowed_revotes": 3,
        "is_consolidated_ballot_encoding": true,
        "spoil_ballot_option": false,
        "is_kiosk": true,
        "voting_channels": {"online": true},
        "image_document_id": "image",
        "statistics": {"num_emails_sent": 1},
        "receipts": {"receipt": true},
        "permission_label": "label-a",
        "initialization_report_generated": true,
        "keys_ceremony_id": keys_ceremony.to_string(),
    })
}

fn import_schema(scope: Scope, elections: Vec<Election>) -> ImportElectionEventSchema {
    serde_json::from_value(json!({
        "tenant_id": scope.tenant_id(),
        "keycloak_event_realm": null,
        "election_event": {
            "id": scope.event_id(),
            "tenant_id": scope.tenant_id(),
            "is_archived": false,
            "encryption_protocol": "RSA256",
        },
        "elections": elections,
        "contests": [],
        "candidates": [],
        "areas": [],
        "area_contests": [],
        "scheduled_events": null,
        "reports": [],
        "keys_ceremonies": null,
        "applications": null,
    }))
    .unwrap()
}

async fn election_ids(tx: &Transaction<'_>, scope: Scope) -> Vec<Uuid> {
    tx.query(
        "SELECT id FROM sequent_backend.election
         WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY id",
        &[&scope.tenant, &scope.event],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| row.get(0))
    .collect()
}

#[tokio::test]
async fn get_election_max_revotes_returns_the_configured_limit() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.election(a).await;
    f.execute(
        "UPDATE sequent_backend.election SET num_allowed_revotes = 3 WHERE id = $1",
        &[&id],
    )
    .await;

    let revotes =
        election::get_election_max_revotes(&tx, &a.tenant_id(), &a.event_id(), &id.to_string())
            .await
            .unwrap();

    assert_eq!(revotes, 3);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_max_revotes_defaults_to_one_for_an_unset_limit_or_a_missing_election() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let unset = f.election(a).await;
    let elsewhere = f.election(sibling).await;
    f.execute(
        "UPDATE sequent_backend.election SET num_allowed_revotes = 5 WHERE id = $1",
        &[&elsewhere],
    )
    .await;

    for id in [unset, elsewhere, f.id()] {
        let revotes =
            election::get_election_max_revotes(&tx, &a.tenant_id(), &a.event_id(), &id.to_string())
                .await
                .unwrap();
        assert_eq!(revotes, 1);
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_by_id_maps_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (id, ceremony) = (f.id(), f.id());
    let mut expected = election_data(a, id, ceremony);
    election::insert_elections(&tx, &import_schema(a, vec![expected.clone()]))
        .await
        .unwrap();
    f.execute(
        "UPDATE sequent_backend.election SET created_at = $2, last_updated_at = $3
         WHERE id = $1",
        &[&id, &at(1), &at(2)],
    )
    .await;

    let loaded = election::get_election_by_id(&tx, &a.tenant_id(), &a.event_id(), &id.to_string())
        .await
        .unwrap();

    expected.created_at = Some(local(at(1)));
    expected.last_updated_at = Some(local(at(2)));
    assert_eq!(loaded, Some(expected));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_by_id_only_finds_the_election_of_the_given_tenant_and_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let other = f.scope().await;
    let id = f.election(a).await;
    f.election_as(other, id).await;
    let get = |scope: Scope| {
        let tx = &tx;
        async move {
            election::get_election_by_id(tx, &scope.tenant_id(), &scope.event_id(), &id.to_string())
                .await
                .unwrap()
                .map(|election| election.tenant_id)
        }
    };

    assert_eq!(get(a).await, Some(a.tenant_id()));
    assert_eq!(get(other).await, Some(other.tenant_id()));
    assert_eq!(get(sibling).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_elections_and_export_elections_return_every_election_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    f.election(sibling).await;

    let listed = election::get_elections(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();
    let exported = election::export_elections(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();
    let ids = election::get_elections_ids(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    let expected = strings(&[first, second]);
    assert_eq!(
        sorted(listed.iter().map(|e| e.id.clone()).collect()),
        expected
    );
    assert_eq!(
        sorted(exported.iter().map(|e| e.id.clone()).collect()),
        expected
    );
    assert_eq!(sorted(ids), expected);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_elections_by_ids_returns_only_the_requested_elections_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (wanted, unwanted) = (f.election(a).await, f.election(a).await);
    let elsewhere = f.election(sibling).await;

    let elections = election::get_elections_by_ids(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &strings(&[wanted, elsewhere]),
    )
    .await
    .unwrap();

    assert_eq!(
        elections.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        strings(&[wanted])
    );
    assert_invalid_uuid(
        election::get_elections_by_ids(
            &tx,
            &a.tenant_id(),
            &a.event_id(),
            &vec![unwanted.to_string(), BAD_UUID.to_string()],
        )
        .await,
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_elections_by_keys_ceremony_id_returns_the_elections_of_the_ceremony() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let ceremony = f.id();
    let member = f.election(a).await;
    f.election(a).await; // outside the ceremony
    let elsewhere = f.election(sibling).await;
    for id in [member, elsewhere] {
        f.execute(
            "UPDATE sequent_backend.election SET keys_ceremony_id = $2 WHERE id = $1",
            &[&id, &ceremony],
        )
        .await;
    }

    let elections = election::get_elections_by_keys_ceremony_id(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &ceremony.to_string(),
    )
    .await
    .unwrap();

    assert_eq!(
        elections.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        strings(&[member])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_election_presentation_and_voting_status_rewrite_one_election() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let id = f.election(a).await;
    f.election_as(other, id).await;

    election::update_election_presentation(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &id.to_string(),
        json!({"sort_order": 9}),
    )
    .await
    .unwrap();
    election::update_election_voting_status(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &id.to_string(),
        json!({"voting_status": "PAUSED"}),
    )
    .await
    .unwrap();

    let row = |scope: Scope| {
        let tx = &tx;
        async move {
            scalar::<Value>(
                tx,
                "SELECT jsonb_build_object('presentation', presentation, 'status', status)
                 FROM sequent_backend.election
                 WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
                &[&scope.tenant, &scope.event, &id],
            )
            .await
        }
    };
    assert_eq!(
        row(a).await,
        json!({"presentation": {"sort_order": 9}, "status": {"voting_status": "PAUSED"}})
    );
    assert_eq!(
        row(other).await,
        json!({"presentation": null, "status": null})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn create_election_writes_the_default_status_and_voting_channels() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let presentation: ElectionPresentation =
        serde_json::from_value(json!({"sort_order": 4})).unwrap();

    let created = election::create_election(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &presentation,
        Some("Mayor".to_string()),
        "mayor-2026",
    )
    .await
    .unwrap();

    let id = Uuid::parse_str(&created.id).unwrap();
    let default_status = serde_json::to_value(ElectionStatus::default()).unwrap();
    let voting_channels = json!({
        "online": true,
        "kiosk": null,
        "telephone": null,
        "paper": null,
        "early_voting": null,
    });
    let now = Some(local(now(&tx).await));
    assert_eq!(created.tenant_id, a.tenant_id());
    assert_eq!(created.election_event_id, a.event_id());
    assert_eq!(created.description.as_deref(), Some("Mayor"));
    assert_eq!(created.external_id.as_deref(), Some("mayor-2026"));
    assert_eq!(created.status, Some(default_status.clone()));
    assert_eq!(created.voting_channels, Some(voting_channels.clone()));
    assert_eq!((created.created_at, created.last_updated_at), (now, now));
    let row = row_json(&tx, "election", "id", id).await;
    assert_eq!(
        row["presentation"],
        serde_json::to_value(&presentation).unwrap()
    );
    assert_eq!(row["status"], default_status);
    assert_eq!(row["voting_channels"], voting_channels);
    assert_eq!(row["is_kiosk"], json!(false));
    assert_eq!(row["initialization_report_generated"], json!(false));
    assert_eq!(row["statistics"], json!({}));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_elections_copies_every_column_of_the_imported_elections() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second, ceremony) = (f.id(), f.id(), f.id());
    let mut bare = election_data(a, second, ceremony);
    bare.keys_ceremony_id = None;
    bare.num_allowed_revotes = None;
    bare.presentation = None;

    election::insert_elections(
        &tx,
        &import_schema(a, vec![election_data(a, first, ceremony), bare]),
    )
    .await
    .unwrap();

    assert_eq!(
        row_json(&tx, "election", "id", first).await,
        election_json(a, first, ceremony)
    );
    let mut expected_bare = election_json(a, second, ceremony);
    expected_bare["keys_ceremony_id"] = Value::Null;
    expected_bare["num_allowed_revotes"] = Value::Null;
    expected_bare["presentation"] = Value::Null;
    assert_eq!(row_json(&tx, "election", "id", second).await, expected_bare);
    let same_time: bool = scalar(
        &tx,
        "SELECT bool_and(created_at = last_updated_at AND created_at IS NOT NULL)
         FROM sequent_backend.election WHERE tenant_id = $1",
        &[&a.tenant],
    )
    .await;
    assert!(same_time);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_elections_writes_nothing_for_an_import_without_elections() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;

    election::insert_elections(&tx, &import_schema(a, vec![]))
        .await
        .unwrap();

    assert!(election_ids(&tx, a).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_elections_rejects_invalid_policy_json_and_writes_no_election() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let ceremony = f.id();
    let valid = election_data(a, f.id(), ceremony);
    let mut invalid = election_data(a, f.id(), ceremony);
    invalid.presentation = Some(json!({"sort_order": "first"}));

    tx.batch_execute("SAVEPOINT import").await.unwrap();
    let error = election::insert_elections(&tx, &import_schema(a, vec![valid, invalid]))
        .await
        .unwrap_err();
    tx.batch_execute("ROLLBACK TO SAVEPOINT import")
        .await
        .unwrap();

    assert!(error.to_string().contains("invalid type"), "{error}");
    assert!(election_ids(&tx, a).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn set_election_keys_ceremony_assigns_one_election_or_every_election_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let elsewhere = f.election(sibling).await;
    let (one, all) = (f.id(), f.id());
    let ceremony_of = |id: Uuid| {
        let tx = &tx;
        async move { row_json(tx, "election", "id", id).await["keys_ceremony_id"].clone() }
    };

    let single = election::set_election_keys_ceremony(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        Some(first.to_string()),
        &one.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(
        single.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
        strings(&[first])
    );
    assert_eq!(single[0].keys_ceremony_id, Some(one.to_string()));
    assert_eq!(ceremony_of(second).await, Value::Null);

    let every = election::set_election_keys_ceremony(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        None,
        &all.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(
        sorted(every.iter().map(|e| e.id.clone()).collect()),
        strings(&[first, second])
    );
    assert_eq!(ceremony_of(first).await, json!(all.to_string()));
    assert_eq!(ceremony_of(second).await, json!(all.to_string()));
    assert_eq!(ceremony_of(elsewhere).await, Value::Null);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn set_election_keys_ceremony_fails_when_no_election_matches() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let elsewhere = f.election(sibling).await;

    let error = election::set_election_keys_ceremony(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        Some(elsewhere.to_string()),
        &f.id().to_string(),
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "No election found");
    assert_eq!(
        row_json(&tx, "election", "id", elsewhere).await["keys_ceremony_id"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn set_election_initialization_report_generated_flags_one_election() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (target, neighbour) = (f.election(a).await, f.election(a).await);

    election::set_election_initialization_report_generated(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &target.to_string(),
        &true,
    )
    .await
    .unwrap();

    assert_eq!(
        row_json(&tx, "election", "id", target).await["initialization_report_generated"],
        json!(true)
    );
    assert_eq!(
        row_json(&tx, "election", "id", neighbour).await["initialization_report_generated"],
        json!(false)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_election_status_sets_is_published_and_keeps_the_rest_of_the_status() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (with_status, without_status) = (f.election(a).await, f.election(a).await);
    f.execute(
        "UPDATE sequent_backend.election
         SET status = '{\"voting_status\": \"OPEN\", \"is_published\": false}',
             last_updated_at = $2
         WHERE id = $1",
        &[&with_status, &at(1)],
    )
    .await;
    let update = |id: Uuid, published: bool| {
        let tx = &tx;
        async move {
            election::update_election_status(
                tx,
                &id.to_string(),
                &a.tenant_id(),
                &a.event_id(),
                published,
            )
            .await
            .unwrap()
        }
    };

    let updated = update(with_status, true).await;
    update(without_status, false).await;
    let unmatched = update(f.id(), true).await;

    assert_eq!(updated.len(), 1);
    assert_eq!(
        updated[0].status,
        Some(json!({"voting_status": "OPEN", "is_published": true}))
    );
    assert_eq!(updated[0].last_updated_at, Some(local(now(&tx).await)));
    assert_eq!(
        row_json(&tx, "election", "id", without_status).await["status"],
        json!({"is_published": false})
    );
    assert!(unmatched.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_permission_label_returns_the_labels_that_are_set() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, unlabelled, third) = (
        f.election(a).await,
        f.election(a).await,
        f.election(a).await,
    );
    let elsewhere = f.election(sibling).await;
    for (id, label) in [(first, "label-1"), (third, "label-3"), (elsewhere, "other")] {
        f.execute(
            "UPDATE sequent_backend.election SET permission_label = $2 WHERE id = $1",
            &[&id, &label],
        )
        .await;
    }
    let labels = |election: Option<Uuid>| {
        let tx = &tx;
        async move {
            election::get_election_permission_label(
                tx,
                &a.tenant_id(),
                &a.event_id(),
                election.map(|id| id.to_string()),
            )
            .await
            .unwrap()
        }
    };

    assert_eq!(sorted(labels(None).await), vec!["label-1", "label-3"]);
    assert_eq!(labels(Some(third)).await, vec!["label-3"]);
    // An election without a label is found, but contributes no label.
    assert!(labels(Some(unlabelled)).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_election_permission_label_fails_when_no_election_matches() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let elsewhere = f.election(sibling).await;

    for election in [None, Some(elsewhere.to_string())] {
        let error =
            election::get_election_permission_label(&tx, &a.tenant_id(), &a.event_id(), election)
                .await
                .unwrap_err();
        assert_eq!(error.to_string(), "No election found");
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_cast_vote_configuration_reads_the_election_policy_and_its_voting_window() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let id = f.election(a).await;
    f.execute(
        "UPDATE sequent_backend.election
         SET presentation = '{\"sort_order\": 1}', status = '{\"voting_status\": \"OPEN\"}',
             voting_channels = '{\"online\": true}'
         WHERE id = $1",
        &[&id],
    )
    .await;
    // The projection the scheduled event triggers maintain.
    for (scope, start) in [(a, "2026-01-01T09:00:00Z"), (other, "2030-01-01T00:00:00Z")] {
        f.execute(
            "INSERT INTO sequent_backend.election_voting_window
                 (tenant_id, election_event_id, election_id, start_date, end_date)
             VALUES ($1, $2, $3, $4, '2026-01-02T18:00:00Z')",
            &[&scope.tenant, &scope.event, &id, &start],
        )
        .await;
    }

    let configuration =
        election::get_cast_vote_configuration(&tx, &a.tenant_id(), &a.event_id(), &id.to_string())
            .await
            .unwrap();

    assert_eq!(configuration.presentation, Some(json!({"sort_order": 1})));
    assert_eq!(configuration.status, Some(json!({"voting_status": "OPEN"})));
    assert_eq!(configuration.voting_channels, Some(json!({"online": true})));
    assert_eq!(
        configuration.dates.start_date.as_deref(),
        Some("2026-01-01T09:00:00Z")
    );
    assert_eq!(
        configuration.dates.end_date.as_deref(),
        Some("2026-01-02T18:00:00Z")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_cast_vote_configuration_reports_absent_dates_without_a_voting_window() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.election(a).await;

    let configuration =
        election::get_cast_vote_configuration(&tx, &a.tenant_id(), &a.event_id(), &id.to_string())
            .await
            .unwrap();

    assert_eq!(
        (
            configuration.presentation,
            configuration.status,
            configuration.voting_channels
        ),
        (None, None, None)
    );
    assert_eq!(
        (configuration.dates.start_date, configuration.dates.end_date),
        (None, None)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_cast_vote_configuration_fails_for_an_election_outside_the_scope() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let elsewhere = f.election(sibling).await;

    let error = election::get_cast_vote_configuration(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &elsewhere.to_string(),
    )
    .await
    .err()
    .unwrap();

    assert_eq!(
        error.to_string(),
        "query returned an unexpected number of rows"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn election_adapters_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.election(a).await.to_string();
    let (tenant, event) = (a.tenant_id(), a.event_id());

    for (tenant, event, election) in [
        (BAD_UUID, event.as_str(), id.as_str()),
        (tenant.as_str(), BAD_UUID, id.as_str()),
        (tenant.as_str(), event.as_str(), BAD_UUID),
    ] {
        assert_invalid_uuid(election::get_election_max_revotes(&tx, tenant, event, election).await);
        assert_invalid_uuid(election::get_election_by_id(&tx, tenant, event, election).await);
        assert_invalid_uuid(
            election::update_election_presentation(&tx, tenant, event, election, json!({})).await,
        );
        assert_invalid_uuid(
            election::update_election_voting_status(&tx, tenant, event, election, json!({})).await,
        );
        assert_invalid_uuid(
            election::set_election_initialization_report_generated(
                &tx, tenant, event, election, &true,
            )
            .await,
        );
        assert_invalid_uuid(
            election::update_election_status(&tx, election, tenant, event, true).await,
        );
        assert_invalid_uuid(
            election::get_cast_vote_configuration(&tx, tenant, event, election)
                .await
                .map(|configuration| configuration.dates),
        );
    }
    let context = election::update_election_presentation(&tx, BAD_UUID, &event, &id, json!({}))
        .await
        .unwrap_err();
    assert_eq!(context.to_string(), "Error parsing tenant_id as UUID");
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// keycloak_realm
// ---------------------------------------------------------------------------

/// A stand-in for Keycloak's `realm` table, visible only to this transaction.
/// It lacks Keycloak's unique realm name so duplicates can be written.
async fn realm_table(tx: &Transaction<'_>) {
    tx.batch_execute(
        "CREATE TEMPORARY TABLE realm (id varchar(36) PRIMARY KEY, name varchar(255))
         ON COMMIT DROP",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn get_realm_id_returns_the_id_of_the_named_realm() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    realm_table(&tx).await;
    tx.batch_execute(
        "INSERT INTO realm (id, name) VALUES ('realm-1', 'tenant-a'), ('realm-2', 'tenant-b')",
    )
    .await
    .unwrap();

    let id = keycloak_realm::get_realm_id(&tx, "tenant-b".to_string())
        .await
        .unwrap();

    assert_eq!(id, "realm-2");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_realm_id_reports_a_missing_realm() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    realm_table(&tx).await;

    let error = keycloak_realm::get_realm_id(&tx, "missing".to_string())
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "realm not found: missing");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_realm_id_rejects_ambiguous_realm_names() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    realm_table(&tx).await;
    tx.batch_execute(
        "INSERT INTO realm (id, name) VALUES ('realm-1', 'twin'), ('realm-2', 'twin')",
    )
    .await
    .unwrap();

    let error = keycloak_realm::get_realm_id(&tx, "twin".to_string())
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "found too many realms with same name: 2");
    tx.rollback().await.unwrap();
}
