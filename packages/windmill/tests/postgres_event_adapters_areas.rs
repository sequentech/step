// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The area, area contest, contest and candidate adapters against the
//! migrated schema. Every test writes its own rows in a transaction, checks
//! them with independent SQL and rolls the transaction back.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local, TimeZone, Utc};
use deadpool_postgres::{Object, Transaction};
use futures::FutureExt;
use sequent_core::election_config::ImportElectionEventSchema;
use sequent_core::types::hasura::core::{Area, AreaContest, Candidate, Contest};
use sequent_core::types::keycloak::UserArea;
use serde_json::{json, Value};
use std::cell::Cell;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use tokio_postgres::types::{FromSql, ToSql};
use uuid::Uuid;
use windmill::postgres::{area, area_contest, candidate, contest};

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

/// Every column of a row but its timestamps, as PostgreSQL renders it to JSON.
async fn row_json(tx: &Transaction<'_>, table: &str, scope: Scope, id: Uuid) -> Value {
    scalar(
        tx,
        &format!(
            "SELECT to_jsonb(t) - 'created_at' - 'last_updated_at'
             FROM sequent_backend.{table} t
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3"
        ),
        &[&scope.tenant, &scope.event, &id],
    )
    .await
}

async fn ids_of(tx: &Transaction<'_>, table: &str, scope: Scope) -> Vec<Uuid> {
    tx.query(
        &format!(
            "SELECT id FROM sequent_backend.{table}
             WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY id"
        ),
        &[&scope.tenant, &scope.event],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| row.get(0))
    .collect()
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

    /// A new tenant with one election event.
    async fn scope(&self) -> Scope {
        let tenant = self.id();
        self.execute(
            "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
            &[&tenant, &format!("tenant-{tenant}")],
        )
        .await;
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
        self.contest_as(scope, id, election).await;
        id
    }

    async fn contest_as(&self, scope: Scope, id: Uuid, election: Uuid) {
        self.execute(
            "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
             VALUES ($1, $2, $3, $4)",
            &[&id, &scope.tenant, &scope.event, &election],
        )
        .await;
    }

    /// An area named `area-<id>`.
    async fn area(&self, scope: Scope) -> Uuid {
        let id = self.id();
        self.area_named(scope, id, Some(&format!("area-{id}")))
            .await;
        id
    }

    async fn area_named(&self, scope: Scope, id: Uuid, name: Option<&str>) {
        self.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, $4)",
            &[&id, &scope.tenant, &scope.event, &name],
        )
        .await;
    }

    async fn area_contest(&self, scope: Scope, area: Uuid, contest: Uuid) -> Uuid {
        let id = self.id();
        self.execute(
            "INSERT INTO sequent_backend.area_contest
                 (id, tenant_id, election_event_id, area_id, contest_id)
             VALUES ($1, $2, $3, $4, $5)",
            &[&id, &scope.tenant, &scope.event, &area, &contest],
        )
        .await;
        id
    }

    async fn candidate(&self, scope: Scope, contest: Uuid) -> Uuid {
        let id = self.id();
        self.execute(
            "INSERT INTO sequent_backend.candidate (id, tenant_id, election_event_id, contest_id)
             VALUES ($1, $2, $3, $4)",
            &[&id, &scope.tenant, &scope.event, &contest],
        )
        .await;
        id
    }
}

fn import_schema(scope: Scope, contests: Vec<Contest>) -> ImportElectionEventSchema {
    serde_json::from_value(json!({
        "tenant_id": scope.tenant_id(),
        "keycloak_event_realm": null,
        "election_event": {
            "id": scope.event_id(),
            "tenant_id": scope.tenant_id(),
            "is_archived": false,
            "encryption_protocol": "RSA256",
        },
        "elections": [],
        "contests": contests,
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

// ---------------------------------------------------------------------------
// area
// ---------------------------------------------------------------------------

fn area_data(scope: Scope, id: Uuid, parent: Option<Uuid>) -> Area {
    Area {
        id: id.to_string(),
        tenant_id: scope.tenant_id(),
        election_event_id: scope.event_id(),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"note": "x"})),
        name: Some(format!("district-{id}")),
        description: Some("District".to_string()),
        r#type: Some("district".to_string()),
        parent_id: parent.map(|parent| parent.to_string()),
        presentation: Some(json!({"sort_order": 1})),
    }
}

fn area_json(scope: Scope, id: Uuid, parent: Option<Uuid>) -> Value {
    json!({
        "id": id.to_string(),
        "tenant_id": scope.tenant_id(),
        "election_event_id": scope.event_id(),
        "labels": {"label": 1},
        "annotations": {"note": "x"},
        "name": format!("district-{id}"),
        "description": "District",
        "type": "district",
        "parent_id": parent.map(|parent| parent.to_string()),
        "presentation": {"sort_order": 1},
    })
}

async fn parent_of(tx: &Transaction<'_>, scope: Scope, id: Uuid) -> Option<Uuid> {
    scalar(
        tx,
        "SELECT parent_id FROM sequent_backend.area
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
        &[&scope.tenant, &scope.event, &id],
    )
    .await
}

fn user_area(id: Uuid) -> UserArea {
    UserArea {
        id: Some(id.to_string()),
        name: Some(format!("area-{id}")),
    }
}

#[tokio::test]
async fn get_areas_returns_the_requested_areas_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, _, third) = (f.area(a).await, f.area(a).await, f.area(a).await);
    let elsewhere = f.area(sibling).await;

    let mut areas = area::get_areas(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &strings(&[third, first, elsewhere]),
    )
    .await
    .unwrap();

    areas.sort_by(|left, right| left.id.cmp(&right.id));
    assert_eq!(areas, vec![user_area(first), user_area(third)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn area_name_lookups_fail_for_an_area_without_a_name() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let unnamed = f.id();
    f.area_named(a, unnamed, None).await;

    let errors = [
        area::get_areas(&tx, &a.tenant_id(), &a.event_id(), &strings(&[unnamed]))
            .await
            .unwrap_err(),
        area::get_areas_by_name(&tx, &a.tenant_id(), &a.event_id())
            .await
            .unwrap_err(),
        area::get_areas_by_id(&tx, &a.tenant_id(), &a.event_id())
            .await
            .unwrap_err(),
    ];

    for error in errors {
        assert_eq!(error.to_string(), "Error getting name from row");
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_areas_by_name_and_by_id_map_the_areas_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (f.area(a).await, f.area(a).await);
    f.area(sibling).await;

    let by_name = area::get_areas_by_name(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();
    let by_id = area::get_areas_by_id(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    let names = |id: Uuid| (format!("area-{id}"), id.to_string());
    assert_eq!(by_name, HashMap::from([names(first), names(second)]));
    assert_eq!(
        by_id,
        HashMap::from([
            (first.to_string(), format!("area-{first}")),
            (second.to_string(), format!("area-{second}")),
        ])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_area_by_name_returns_the_unique_area_or_none() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let id = f.area(a).await;
    f.area_named(sibling, f.id(), Some(&format!("area-{id}")))
        .await;

    let found = area::get_area_by_name(&tx, &a.tenant_id(), &a.event_id(), &format!("area-{id}"))
        .await
        .unwrap();
    let missing = area::get_area_by_name(&tx, &a.tenant_id(), &a.event_id(), "missing")
        .await
        .unwrap();

    assert_eq!(found.map(|area| area.id), Some(id.to_string()));
    assert_eq!(missing, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_area_by_name_rejects_a_name_shared_by_several_areas() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    for _ in 0..2 {
        f.area_named(a, f.id(), Some("twin")).await;
    }

    let error = area::get_area_by_name(&tx, &a.tenant_id(), &a.event_id(), "twin")
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!(
            "Area name 'twin' matched 2 areas in election event {}",
            a.event
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_area_id_from_event_by_name_requires_exactly_one_match() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.area(a).await;
    for _ in 0..2 {
        f.area_named(a, f.id(), Some("twin")).await;
    }
    let lookup = |name: String| {
        let tx = &tx;
        async move {
            area::get_area_id_from_event_by_name(tx, &a.tenant_id(), &a.event_id(), &name).await
        }
    };

    assert_eq!(lookup(format!("area-{id}")).await.unwrap(), id.to_string());
    assert_eq!(
        lookup("missing".to_string()).await.unwrap_err().to_string(),
        format!(
            "No area found with name 'missing' in election event '{}'",
            a.event
        )
    );
    assert_eq!(
        lookup("twin".to_string()).await.unwrap_err().to_string(),
        format!(
            "Found 2 areas with name 'twin' in election event '{}'",
            a.event
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_elections_by_area_lists_an_election_once_per_linked_contest() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let (first_a, first_b, second_a) = (
        f.contest(a, first).await,
        f.contest(a, first).await,
        f.contest(a, second).await,
    );
    let (both, only_second, unlinked) = (f.area(a).await, f.area(a).await, f.area(a).await);
    for contest in [first_a, first_b, second_a] {
        f.area_contest(a, both, contest).await;
    }
    f.area_contest(a, only_second, second_a).await;

    let mut elections = area::get_elections_by_area(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    for list in elections.values_mut() {
        list.sort();
    }
    // Two contests of the first election list it twice.
    assert_eq!(
        elections,
        HashMap::from([
            (both.to_string(), strings(&[first, first, second])),
            (only_second.to_string(), strings(&[second])),
        ])
    );
    assert!(!elections.contains_key(&unlinked.to_string()));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_area_by_id_maps_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (parent, id) = (f.id(), f.id());
    area::insert_area(&tx, area_data(a, parent, None))
        .await
        .unwrap();
    let mut expected = area_data(a, id, Some(parent));
    area::insert_area(&tx, expected.clone()).await.unwrap();
    f.execute(
        "UPDATE sequent_backend.area SET created_at = $2, last_updated_at = $3 WHERE id = $1",
        &[&id, &at(1), &at(2)],
    )
    .await;

    let loaded = area::get_area_by_id(&tx, &a.tenant_id(), &id.to_string())
        .await
        .unwrap();

    expected.created_at = Some(local(at(1)));
    expected.last_updated_at = Some(local(at(2)));
    assert_eq!(loaded, Some(expected));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_area_by_id_is_scoped_to_the_tenant_but_not_to_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let other = f.scope().await;
    let id = f.area(sibling).await;

    let found = area::get_area_by_id(&tx, &a.tenant_id(), &id.to_string())
        .await
        .unwrap();
    let foreign = area::get_area_by_id(&tx, &other.tenant_id(), &id.to_string())
        .await
        .unwrap();

    assert_eq!(
        found.map(|area| area.election_event_id),
        Some(sibling.event_id())
    );
    assert_eq!(foreign, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_event_areas_and_get_areas_by_ids_return_areas_of_the_event_only() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (f.area(a).await, f.area(a).await);
    let elsewhere = f.area(sibling).await;

    let every = area::get_event_areas(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();
    let requested = area::get_areas_by_ids(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &strings(&[second, elsewhere]),
    )
    .await
    .unwrap();

    let ids = |areas: Vec<Area>| sorted(areas.into_iter().map(|area| area.id).collect());
    assert_eq!(ids(every), strings(&[first, second]));
    assert_eq!(ids(requested), strings(&[second]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_areas_by_election_id_returns_each_area_of_the_election_once() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (election, other_election) = (f.election(a).await, f.election(a).await);
    let (first, second) = (f.contest(a, election).await, f.contest(a, election).await);
    let other_contest = f.contest(a, other_election).await;
    let (shared, single, foreign) = (f.area(a).await, f.area(a).await, f.area(a).await);
    f.area_contest(a, shared, first).await;
    f.area_contest(a, shared, second).await;
    f.area_contest(a, single, second).await;
    f.area_contest(a, foreign, other_contest).await;

    let areas =
        area::get_areas_by_election_id(&tx, &a.tenant_id(), &a.event_id(), &election.to_string())
            .await
            .unwrap();

    assert_eq!(
        sorted(areas.into_iter().map(|area| area.id).collect()),
        strings(&[shared, single])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn upsert_area_parents_sets_the_parent_of_each_given_area() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let (root, child, orphan) = (f.area(a).await, f.area(a).await, f.area(a).await);
    f.area_named(other, child, Some("same id")).await;
    f.area_named(other, root, Some("same id as root")).await;

    area::upsert_area_parents(
        &tx,
        &vec![area_data(a, child, Some(root)), area_data(a, orphan, None)],
    )
    .await
    .unwrap();

    assert_eq!(parent_of(&tx, a, child).await, Some(root));
    assert_eq!(parent_of(&tx, a, orphan).await, None);
    assert_eq!(parent_of(&tx, other, child).await, None);
    // Only parent_id is written.
    assert_eq!(
        row_json(&tx, "area", a, child).await["name"],
        json!(format!("area-{child}"))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn upsert_area_parents_clears_the_parent_for_an_unparseable_parent_id() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (root, child) = (f.area(a).await, f.area(a).await);
    f.execute(
        "UPDATE sequent_backend.area SET parent_id = $2 WHERE id = $1",
        &[&child, &root],
    )
    .await;
    let mut update = area_data(a, child, None);
    update.parent_id = Some(BAD_UUID.to_string());

    area::upsert_area_parents(&tx, &vec![update]).await.unwrap();

    assert_eq!(parent_of(&tx, a, child).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_areas_writes_parents_before_their_children() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (root, middle, leaf) = (f.id(), f.id(), f.id());

    // Children first: the area foreign key only accepts existing parents.
    area::insert_areas(
        &tx,
        &vec![
            area_data(a, leaf, Some(middle)),
            area_data(a, middle, Some(root)),
            area_data(a, root, None),
        ],
    )
    .await
    .unwrap();

    for (id, parent) in [(root, None), (middle, Some(root)), (leaf, Some(middle))] {
        assert_eq!(row_json(&tx, "area", a, id).await, area_json(a, id, parent));
    }
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_areas_rejects_a_parent_missing_from_the_batch() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let existing = f.area(a).await;
    let child = f.id();

    let error = area::insert_areas(&tx, &vec![area_data(a, child, Some(existing))])
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!("Parent id {existing} not found in the tree structure")
    );
    assert_eq!(ids_of(&tx, "area", a).await, vec![existing]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_area_writes_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (parent, id) = (f.area(a).await, f.id());

    area::insert_area(&tx, area_data(a, id, Some(parent)))
        .await
        .unwrap();

    assert_eq!(
        row_json(&tx, "area", a, id).await,
        area_json(a, id, Some(parent))
    );
    let timestamps: (DateTime<Utc>, DateTime<Utc>) = {
        let row = tx
            .query_one(
                "SELECT created_at, last_updated_at FROM sequent_backend.area WHERE id = $1",
                &[&id],
            )
            .await
            .unwrap();
        (row.get(0), row.get(1))
    };
    let now = now(&tx).await;
    assert_eq!(timestamps, (now, now));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_area_rewrites_one_area_and_its_update_time() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let (parent, id) = (f.area(a).await, f.area(a).await);
    f.area_named(other, id, Some("untouched")).await;
    f.execute(
        "UPDATE sequent_backend.area SET last_updated_at = $2 WHERE id = $1",
        &[&id, &at(1)],
    )
    .await;

    area::update_area(&tx, area_data(a, id, Some(parent)))
        .await
        .unwrap();

    assert_eq!(
        row_json(&tx, "area", a, id).await,
        area_json(a, id, Some(parent))
    );
    let updated: DateTime<Utc> = scalar(
        &tx,
        "SELECT last_updated_at FROM sequent_backend.area
         WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
        &[&a.tenant, &a.event, &id],
    )
    .await;
    assert_eq!(updated, now(&tx).await);
    assert_eq!(
        row_json(&tx, "area", other, id).await["name"],
        json!("untouched")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn delete_area_contests_removes_only_the_links_of_the_area() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (first, second) = (f.contest(a, election).await, f.contest(a, election).await);
    let (target, neighbour) = (f.area(a).await, f.area(a).await);
    f.area_contest(a, target, first).await;
    f.area_contest(a, target, second).await;
    let kept = f.area_contest(a, neighbour, first).await;

    area::delete_area_contests(&tx, &a.tenant_id(), &a.event, &target.to_string())
        .await
        .unwrap();

    assert_eq!(ids_of(&tx, "area_contest", a).await, vec![kept]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn area_election_wrapper_maps_text_annotations_but_not_the_stored_jsonb() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.id();
    area::insert_area(&tx, area_data(a, id, None))
        .await
        .unwrap();
    let row = |annotations: &'static str| {
        let tx = &tx;
        async move {
            tx.query_one(
                &format!(
                    "SELECT id, name, description, {annotations} AS annotations
                     FROM sequent_backend.area WHERE id = $1"
                ),
                &[&id],
            )
            .await
            .unwrap()
        }
    };

    let mapped = area::AreaElectionWrapper::try_from(row("annotations::text").await).unwrap();
    let stored = area::AreaElectionWrapper::try_from(row("annotations").await);

    assert_eq!(
        mapped.0,
        area::AreaElection {
            id: id.to_string(),
            name: Some(format!("district-{id}")),
            description: Some("District".to_string()),
            annotations: Some("{\"note\": \"x\"}".to_string()),
        }
    );
    assert!(stored.is_err());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn area_adapters_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.area(a).await.to_string();
    let (tenant, event) = (a.tenant_id(), a.event_id());

    for (tenant, event) in [(BAD_UUID, event.as_str()), (tenant.as_str(), BAD_UUID)] {
        assert_invalid_uuid(area::get_areas_by_name(&tx, tenant, event).await);
        assert_invalid_uuid(area::get_area_by_name(&tx, tenant, event, "name").await);
        assert_invalid_uuid(area::get_areas_by_id(&tx, tenant, event).await);
        assert_invalid_uuid(area::get_elections_by_area(&tx, tenant, event).await);
        assert_invalid_uuid(area::get_event_areas(&tx, tenant, event).await);
        assert_invalid_uuid(area::get_area_id_from_event_by_name(&tx, tenant, event, "name").await);
        assert_invalid_uuid(area::get_areas_by_election_id(&tx, tenant, event, &id).await);
        assert_invalid_uuid(area::get_areas_by_ids(&tx, tenant, event, &vec![id.clone()]).await);
        assert_invalid_uuid(area::get_areas(&tx, tenant, event, &vec![id.clone()]).await);
    }
    assert_invalid_uuid(area::get_areas(&tx, &tenant, &event, &vec![BAD_UUID.to_string()]).await);
    assert_invalid_uuid(area::get_area_by_id(&tx, BAD_UUID, &id).await);
    assert_invalid_uuid(area::get_area_by_id(&tx, &tenant, BAD_UUID).await);
    assert_invalid_uuid(area::delete_area_contests(&tx, &tenant, &a.event, BAD_UUID).await);
    let mut invalid = area_data(a, f.id(), None);
    invalid.id = BAD_UUID.to_string();
    assert_invalid_uuid(area::insert_area(&tx, invalid.clone()).await);
    assert_invalid_uuid(area::update_area(&tx, invalid.clone()).await);
    assert_invalid_uuid(area::upsert_area_parents(&tx, &vec![invalid]).await);
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// area_contest
// ---------------------------------------------------------------------------

/// `(area_id, contest_id)` of the links of the event, sorted.
async fn links(tx: &Transaction<'_>, scope: Scope) -> Vec<(Uuid, Uuid)> {
    tx.query(
        "SELECT area_id, contest_id FROM sequent_backend.area_contest
         WHERE tenant_id = $1 AND election_event_id = $2 ORDER BY area_id, contest_id",
        &[&scope.tenant, &scope.event],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

#[tokio::test]
async fn insert_area_to_area_contests_links_the_area_to_each_contest() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (first, second) = (f.contest(a, election).await, f.contest(a, election).await);
    let area_id = f.area(a).await;

    area_contest::insert_area_to_area_contests(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &area_id.to_string(),
        &[first, second],
    )
    .await
    .unwrap();

    assert_eq!(
        links(&tx, a).await,
        vec![(area_id, first), (area_id, second)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_area_contests_writes_the_given_links_with_their_ids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let contest_id = f.contest(a, election).await;
    let (first, second) = (f.area(a).await, f.area(a).await);
    let (first_link, second_link) = (f.id(), f.id());

    area_contest::insert_area_contests(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &vec![
            AreaContest {
                id: first_link.to_string(),
                area_id: first.to_string(),
                contest_id: contest_id.to_string(),
            },
            AreaContest {
                id: second_link.to_string(),
                area_id: second.to_string(),
                contest_id: contest_id.to_string(),
            },
        ],
    )
    .await
    .unwrap();

    assert_eq!(
        ids_of(&tx, "area_contest", a).await,
        vec![first_link, second_link]
    );
    assert_eq!(
        links(&tx, a).await,
        vec![(first, contest_id), (second, contest_id)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_area_contests_rejects_a_contest_of_another_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(sibling).await;
    let foreign_contest = f.contest(sibling, election).await;
    let area_id = f.area(a).await;

    let error = area_contest::insert_area_to_area_contests(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &area_id.to_string(),
        &[foreign_contest],
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Error running the document query: db error"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn export_area_contests_returns_the_links_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let contest_id = f.contest(a, election).await;
    let area_id = f.area(a).await;
    let link = f.area_contest(a, area_id, contest_id).await;
    let sibling_election = f.election(sibling).await;
    let sibling_contest = f.contest(sibling, sibling_election).await;
    let sibling_area = f.area(sibling).await;
    f.area_contest(sibling, sibling_area, sibling_contest).await;

    let exported = area_contest::export_area_contests(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    assert_eq!(
        exported,
        vec![AreaContest {
            id: link.to_string(),
            area_id: area_id.to_string(),
            contest_id: contest_id.to_string(),
        }]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_areas_by_contest_id_panics_reading_its_uuid_column_as_text() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (linked, unlinked) = (f.contest(a, election).await, f.contest(a, election).await);
    let area_id = f.area(a).await;
    f.area_contest(a, area_id, linked).await;

    let empty = area_contest::get_areas_by_contest_id(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &unlinked.to_string(),
    )
    .await
    .unwrap();
    // `row.get::<_, String>` on the uuid area_id column panics.
    let panic = AssertUnwindSafe(area_contest::get_areas_by_contest_id(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &linked.to_string(),
    ))
    .catch_unwind()
    .await
    .unwrap_err();

    assert!(empty.is_empty());
    let message = panic.downcast_ref::<String>().cloned().unwrap_or_default();
    assert!(
        message.starts_with("error retrieving column area_id"),
        "{message}"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_contests_by_area_id_returns_the_contests_linked_to_the_area() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (first, second, unlinked) = (
        f.contest(a, election).await,
        f.contest(a, election).await,
        f.contest(a, election).await,
    );
    let (area_id, other_area) = (f.area(a).await, f.area(a).await);
    f.area_contest(a, area_id, first).await;
    f.area_contest(a, area_id, second).await;
    f.area_contest(a, other_area, unlinked).await;

    let contests = area_contest::get_contests_by_area_id(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &area_id.to_string(),
    )
    .await
    .unwrap();

    assert_eq!(sorted(contests), strings(&[first, second]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn area_contest_exists_checks_one_area_and_contest_pair_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let (linked, unlinked) = (f.contest(a, election).await, f.contest(a, election).await);
    let area_id = f.area(a).await;
    f.area_contest(a, area_id, linked).await;
    let exists = |scope: Scope, contest: Uuid| {
        let tx = &tx;
        async move {
            area_contest::area_contest_exists(
                tx,
                &scope.tenant_id(),
                &scope.event_id(),
                &area_id.to_string(),
                &contest.to_string(),
            )
            .await
            .unwrap()
        }
    };

    assert!(exists(a, linked).await);
    assert!(!exists(a, unlinked).await);
    assert!(!exists(sibling, linked).await);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_area_contests_by_area_contest_ids_matches_both_id_lists() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (first, second) = (f.contest(a, election).await, f.contest(a, election).await);
    let (north, south) = (f.area(a).await, f.area(a).await);
    let north_first = f.area_contest(a, north, first).await;
    f.area_contest(a, north, second).await;
    let south_first = f.area_contest(a, south, first).await;

    let found = area_contest::get_area_contests_by_area_contest_ids(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &strings(&[north, south]),
        &strings(&[first]),
    )
    .await
    .unwrap();

    assert_eq!(
        sorted(found.into_iter().map(|link| link.id).collect()),
        strings(&[north_first, south_first])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn area_contest_adapters_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (tenant, event) = (a.tenant_id(), a.event_id());
    let id = f.id().to_string();

    for (tenant, event) in [(BAD_UUID, event.as_str()), (tenant.as_str(), BAD_UUID)] {
        assert_invalid_uuid(area_contest::export_area_contests(&tx, tenant, event).await);
        assert_invalid_uuid(area_contest::get_areas_by_contest_id(&tx, tenant, event, &id).await);
        assert_invalid_uuid(area_contest::get_contests_by_area_id(&tx, tenant, event, &id).await);
        assert_invalid_uuid(area_contest::area_contest_exists(&tx, tenant, event, &id, &id).await);
        assert_invalid_uuid(
            area_contest::insert_area_to_area_contests(
                &tx,
                tenant,
                event,
                &id,
                &[Uuid::parse_str(&id).unwrap()],
            )
            .await,
        );
    }
    assert_invalid_uuid(
        area_contest::get_area_contests_by_area_contest_ids(
            &tx,
            &tenant,
            &event,
            &vec![BAD_UUID.to_string()],
            &vec![id.clone()],
        )
        .await,
    );
    assert_invalid_uuid(
        area_contest::get_area_contests_by_area_contest_ids(
            &tx,
            &tenant,
            &event,
            &vec![id.clone()],
            &vec![BAD_UUID.to_string()],
        )
        .await,
    );
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// contest
// ---------------------------------------------------------------------------

fn contest_data(scope: Scope, id: Uuid, election: Uuid) -> Contest {
    Contest {
        id: id.to_string(),
        tenant_id: scope.tenant_id(),
        election_event_id: scope.event_id(),
        election_id: election.to_string(),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"note": "x"})),
        is_acclaimed: Some(false),
        is_active: Some(true),
        description: Some("Council".to_string()),
        presentation: Some(json!({"allow_writeins": true})),
        min_votes: Some(0),
        max_votes: Some(3),
        winning_candidates_num: Some(2),
        voting_type: Some("non-preferential".to_string()),
        counting_algorithm: Some("plurality-at-large".to_string()),
        is_encrypted: Some(true),
        tally_configuration: Some(json!({"tally": 1})),
        image_document_id: Some("image".to_string()),
        conditions: Some(json!({"condition": 1})),
        external_id: Some(format!("contest-{id}")),
    }
}

fn contest_json(scope: Scope, id: Uuid, election: Uuid) -> Value {
    json!({
        "id": id.to_string(),
        "tenant_id": scope.tenant_id(),
        "election_event_id": scope.event_id(),
        "election_id": election.to_string(),
        "labels": {"label": 1},
        "annotations": {"note": "x"},
        "is_acclaimed": false,
        "is_active": true,
        "description": "Council",
        "presentation": {"allow_writeins": true},
        "min_votes": 0,
        "max_votes": 3,
        "winning_candidates_num": 2,
        "voting_type": "non-preferential",
        "counting_algorithm": "plurality-at-large",
        "is_encrypted": true,
        "tally_configuration": {"tally": 1},
        "image_document_id": "image",
        "conditions": {"condition": 1},
        "external_id": format!("contest-{id}"),
    })
}

fn contest_ids(contests: Vec<Contest>) -> Vec<String> {
    sorted(contests.into_iter().map(|contest| contest.id).collect())
}

#[tokio::test]
async fn insert_contest_copies_every_column_of_the_imported_contests() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (first, second) = (f.id(), f.id());
    let mut bare = contest_data(a, second, election);
    bare.min_votes = None;
    bare.presentation = None;

    contest::insert_contest(
        &tx,
        &import_schema(a, vec![contest_data(a, first, election), bare]),
    )
    .await
    .unwrap();

    assert_eq!(
        row_json(&tx, "contest", a, first).await,
        contest_json(a, first, election)
    );
    let mut expected_bare = contest_json(a, second, election);
    expected_bare["min_votes"] = Value::Null;
    expected_bare["presentation"] = Value::Null;
    assert_eq!(row_json(&tx, "contest", a, second).await, expected_bare);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_contest_writes_nothing_for_an_import_without_contests() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;

    contest::insert_contest(&tx, &import_schema(a, vec![]))
        .await
        .unwrap();

    assert!(ids_of(&tx, "contest", a).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_contest_rejects_invalid_presentation_json_and_writes_no_contest() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let mut invalid = contest_data(a, f.id(), election);
    invalid.presentation = Some(json!({"allow_writeins": "yes"}));

    tx.batch_execute("SAVEPOINT import").await.unwrap();
    let error = contest::insert_contest(
        &tx,
        &import_schema(a, vec![contest_data(a, f.id(), election), invalid]),
    )
    .await
    .unwrap_err();
    tx.batch_execute("ROLLBACK TO SAVEPOINT import")
        .await
        .unwrap();

    assert!(error.to_string().contains("invalid type"), "{error}");
    assert!(ids_of(&tx, "contest", a).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_contest_by_id_maps_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let id = f.id();
    let mut expected = contest_data(a, id, election);
    contest::insert_contest(&tx, &import_schema(a, vec![expected.clone()]))
        .await
        .unwrap();
    f.execute(
        "UPDATE sequent_backend.contest SET created_at = $2, last_updated_at = $3 WHERE id = $1",
        &[&id, &at(1), &at(2)],
    )
    .await;

    let loaded = contest::get_contest_by_id(&tx, &a.tenant_id(), &a.event_id(), &id.to_string())
        .await
        .unwrap();

    expected.created_at = Some(local(at(1)));
    expected.last_updated_at = Some(local(at(2)));
    assert_eq!(loaded, Some(expected));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_contest_by_id_only_finds_the_contest_of_the_given_tenant_and_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let other = f.scope().await;
    let election = f.id();
    f.election_as(a, election).await;
    f.election_as(other, election).await;
    let id = f.contest(a, election).await;
    f.contest_as(other, id, election).await;
    let get = |scope: Scope| {
        let tx = &tx;
        async move {
            contest::get_contest_by_id(tx, &scope.tenant_id(), &scope.event_id(), &id.to_string())
                .await
                .unwrap()
                .map(|contest| contest.tenant_id)
        }
    };

    assert_eq!(get(a).await, Some(a.tenant_id()));
    assert_eq!(get(other).await, Some(other.tenant_id()));
    assert_eq!(get(sibling).await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_contest_by_external_id_returns_the_unique_contest_or_none() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let sibling_election = f.election(sibling).await;
    let id = f.contest(a, election).await;
    let twin = f.contest(sibling, sibling_election).await;
    for (scope, contest) in [(a, id), (sibling, twin)] {
        f.execute(
            "UPDATE sequent_backend.contest SET external_id = 'council'
             WHERE election_event_id = $1 AND id = $2",
            &[&scope.event, &contest],
        )
        .await;
    }
    let lookup = |external_id: &'static str| {
        let tx = &tx;
        async move {
            contest::get_contest_by_external_id(tx, &a.tenant_id(), &a.event_id(), external_id)
                .await
                .unwrap()
                .map(|contest| contest.id)
        }
    };

    assert_eq!(lookup("council").await, Some(id.to_string()));
    assert_eq!(lookup("missing").await, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_contest_by_external_id_rejects_an_external_id_shared_by_several_contests() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    f.contest(a, election).await;
    f.contest(a, election).await;
    f.execute(
        "UPDATE sequent_backend.contest SET external_id = 'twin' WHERE election_event_id = $1",
        &[&a.event],
    )
    .await;

    let error = contest::get_contest_by_external_id(&tx, &a.tenant_id(), &a.event_id(), "twin")
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!(
            "Contest external id 'twin' matched 2 contests in election event {}. Contest \
             external_id must be unique within an election event for tally sheet import to \
             work — rename the duplicate(s) before importing.",
            a.event
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn export_contests_returns_every_contest_of_the_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let (first, second) = (f.election(a).await, f.election(a).await);
    let (one, two) = (f.contest(a, first).await, f.contest(a, second).await);
    let sibling_election = f.election(sibling).await;
    f.contest(sibling, sibling_election).await;

    let exported = contest::export_contests(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();

    assert_eq!(contest_ids(exported), strings(&[one, two]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_contest_by_election_id_and_ids_return_the_contests_of_those_elections() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (first, second, third) = (
        f.election(a).await,
        f.election(a).await,
        f.election(a).await,
    );
    let (first_one, first_two) = (f.contest(a, first).await, f.contest(a, first).await);
    let second_one = f.contest(a, second).await;
    f.contest(a, third).await;

    let of_first =
        contest::get_contest_by_election_id(&tx, &a.tenant_id(), &a.event_id(), &first.to_string())
            .await
            .unwrap();
    let of_both = contest::get_contest_by_election_ids(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &strings(&[first, second]),
    )
    .await
    .unwrap();

    assert_eq!(contest_ids(of_first), strings(&[first_one, first_two]));
    assert_eq!(
        contest_ids(of_both),
        strings(&[first_one, first_two, second_one])
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn contest_adapters_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.id().to_string();
    let (tenant, event) = (a.tenant_id(), a.event_id());

    for (tenant, event, target) in [
        (BAD_UUID, event.as_str(), id.as_str()),
        (tenant.as_str(), BAD_UUID, id.as_str()),
        (tenant.as_str(), event.as_str(), BAD_UUID),
    ] {
        assert_invalid_uuid(contest::get_contest_by_id(&tx, tenant, event, target).await);
        assert_invalid_uuid(contest::get_contest_by_election_id(&tx, tenant, event, target).await);
        assert_invalid_uuid(
            contest::get_contest_by_election_ids(&tx, tenant, event, &vec![target.to_string()])
                .await,
        );
    }
    for (tenant, event) in [(BAD_UUID, event.as_str()), (tenant.as_str(), BAD_UUID)] {
        assert_invalid_uuid(contest::export_contests(&tx, tenant, event).await);
        assert_invalid_uuid(contest::get_contest_by_external_id(&tx, tenant, event, "x").await);
    }
    tx.rollback().await.unwrap();
}

// ---------------------------------------------------------------------------
// candidate
// ---------------------------------------------------------------------------

fn candidate_data(scope: Scope, id: Uuid, contest: Option<String>) -> Candidate {
    Candidate {
        id: id.to_string(),
        tenant_id: scope.tenant_id(),
        election_event_id: scope.event_id(),
        contest_id: contest,
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"note": "x"})),
        description: Some("Alice".to_string()),
        r#type: Some("person".to_string()),
        presentation: Some(json!({"is_disabled": false})),
        is_public: Some(true),
        image_document_id: Some("image".to_string()),
        external_id: Some(format!("candidate-{id}")),
    }
}

fn candidate_json(scope: Scope, id: Uuid, contest: Option<Uuid>) -> Value {
    json!({
        "id": id.to_string(),
        "tenant_id": scope.tenant_id(),
        "election_event_id": scope.event_id(),
        "contest_id": contest.map(|contest| contest.to_string()),
        "labels": {"label": 1},
        "annotations": {"note": "x"},
        "description": "Alice",
        "type": "person",
        "presentation": {"is_disabled": false},
        "is_public": true,
        "image_document_id": "image",
        "external_id": format!("candidate-{id}"),
    })
}

fn candidate_ids(candidates: Vec<Candidate>) -> Vec<String> {
    sorted(
        candidates
            .into_iter()
            .map(|candidate| candidate.id)
            .collect(),
    )
}

#[tokio::test]
async fn insert_candidates_copies_every_column() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let contest_id = f.contest(a, election).await;
    let (first, second) = (f.id(), f.id());

    candidate::insert_candidates(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &vec![
            candidate_data(a, first, Some(contest_id.to_string())),
            candidate_data(a, second, None),
        ],
    )
    .await
    .unwrap();

    assert_eq!(
        row_json(&tx, "candidate", a, first).await,
        candidate_json(a, first, Some(contest_id))
    );
    assert_eq!(
        row_json(&tx, "candidate", a, second).await,
        candidate_json(a, second, None)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_candidates_writes_them_under_the_given_tenant_and_event() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let other = f.scope().await;
    let id = f.id();

    candidate::insert_candidates(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &vec![candidate_data(other, id, None)],
    )
    .await
    .unwrap();

    assert_eq!(ids_of(&tx, "candidate", a).await, vec![id]);
    assert!(ids_of(&tx, "candidate", other).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_candidates_stores_no_contest_for_an_unparseable_contest_id() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let id = f.id();

    candidate::insert_candidates(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &vec![candidate_data(a, id, Some(BAD_UUID.to_string()))],
    )
    .await
    .unwrap();

    assert_eq!(
        row_json(&tx, "candidate", a, id).await["contest_id"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_candidates_writes_nothing_without_candidates() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;

    candidate::insert_candidates(&tx, &a.tenant_id(), &a.event_id(), &vec![])
        .await
        .unwrap();
    // No statement runs, so not even the scope is parsed.
    candidate::insert_candidates(&tx, BAD_UUID, BAD_UUID, &vec![])
        .await
        .unwrap();

    assert!(ids_of(&tx, "candidate", a).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_candidates_rejects_invalid_presentation_json_and_writes_no_candidate() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let mut invalid = candidate_data(a, f.id(), None);
    invalid.presentation = Some(json!({"is_disabled": "yes"}));

    tx.batch_execute("SAVEPOINT import").await.unwrap();
    let error = candidate::insert_candidates(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &vec![candidate_data(a, f.id(), None), invalid],
    )
    .await
    .unwrap_err();
    tx.batch_execute("ROLLBACK TO SAVEPOINT import")
        .await
        .unwrap();

    assert!(error.to_string().contains("invalid type"), "{error}");
    assert!(ids_of(&tx, "candidate", a).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn export_candidates_and_get_candidates_by_contest_id_filter_by_event_and_contest() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let sibling = f.event_in(a.tenant).await;
    let election = f.election(a).await;
    let (first, second) = (f.contest(a, election).await, f.contest(a, election).await);
    let (first_one, first_two) = (f.candidate(a, first).await, f.candidate(a, first).await);
    let second_one = f.candidate(a, second).await;
    let sibling_election = f.election(sibling).await;
    let sibling_contest = f.contest(sibling, sibling_election).await;
    f.candidate(sibling, sibling_contest).await;

    let exported = candidate::export_candidates(&tx, &a.tenant_id(), &a.event_id())
        .await
        .unwrap();
    let of_first = candidate::get_candidates_by_contest_id(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &first.to_string(),
    )
    .await
    .unwrap();

    assert_eq!(
        candidate_ids(exported),
        strings(&[first_one, first_two, second_one])
    );
    assert_eq!(candidate_ids(of_first), strings(&[first_one, first_two]));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn candidate_adapters_reject_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let (tenant, event, id) = (a.tenant_id(), a.event_id(), f.id());
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("candidates.csv");

    for (tenant, event) in [(BAD_UUID, event.as_str()), (tenant.as_str(), BAD_UUID)] {
        assert_invalid_uuid(candidate::export_candidates(&tx, tenant, event).await);
        assert_invalid_uuid(
            candidate::get_candidates_by_contest_id(&tx, tenant, event, &id.to_string()).await,
        );
        assert_invalid_uuid(
            candidate::insert_candidates(&tx, tenant, event, &vec![candidate_data(a, id, None)])
                .await,
        );
        assert_invalid_uuid(
            candidate::export_candidate_csv(&tx, &path, &vec![], tenant, event).await,
        );
    }
    assert_invalid_uuid(
        candidate::get_candidates_by_contest_id(&tx, &tenant, &event, BAD_UUID).await,
    );
    let mut invalid = candidate_data(a, id, None);
    invalid.id = BAD_UUID.to_string();
    tx.batch_execute("SAVEPOINT import").await.unwrap();
    assert_invalid_uuid(candidate::insert_candidates(&tx, &tenant, &event, &vec![invalid]).await);
    tx.batch_execute("ROLLBACK TO SAVEPOINT import")
        .await
        .unwrap();
    assert!(ids_of(&tx, "candidate", a).await.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn export_candidate_csv_writes_the_candidates_of_the_requested_contests() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL TimeZone = 'UTC'; SET LOCAL DateStyle = 'ISO, MDY'")
        .await
        .unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let election = f.election(a).await;
    let (requested, unrequested) = (f.contest(a, election).await, f.contest(a, election).await);
    let id = f.id();
    candidate::insert_candidates(
        &tx,
        &a.tenant_id(),
        &a.event_id(),
        &vec![candidate_data(a, id, Some(requested.to_string()))],
    )
    .await
    .unwrap();
    f.execute(
        "UPDATE sequent_backend.candidate SET created_at = $2, last_updated_at = $3
         WHERE id = $1",
        &[&id, &at(1), &at(2)],
    )
    .await;
    f.candidate(a, unrequested).await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("candidates.csv");

    candidate::export_candidate_csv(
        &tx,
        &path,
        &strings(&[requested]),
        &a.tenant_id(),
        &a.event_id(),
    )
    .await
    .unwrap();

    let mut reader = csv::Reader::from_path(&path).unwrap();
    assert_eq!(
        reader.headers().unwrap().iter().collect::<Vec<_>>(),
        vec![
            "id",
            "tenant_id",
            "election_event_id",
            "contest_id",
            "created_at",
            "last_updated_at",
            "labels",
            "annotations",
            "description",
            "type",
            "presentation",
            "is_public",
            "image_document_id",
            "external_id",
        ]
    );
    let rows: Vec<Vec<String>> = reader
        .records()
        .map(|record| record.unwrap().iter().map(str::to_string).collect())
        .collect();
    assert_eq!(
        rows,
        vec![vec![
            id.to_string(),
            a.tenant_id(),
            a.event_id(),
            requested.to_string(),
            "2026-01-01 12:00:00+00".to_string(),
            "2026-01-02 12:00:00+00".to_string(),
            "{\"label\": 1}".to_string(),
            "{\"note\": \"x\"}".to_string(),
            "Alice".to_string(),
            "person".to_string(),
            "{\"is_disabled\": false}".to_string(),
            "true".to_string(),
            "image".to_string(),
            format!("candidate-{id}"),
        ]]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn export_candidate_csv_creates_the_file_before_rejecting_invalid_uuids() {
    let mut client = connect().await;
    let tx = client.transaction().await.unwrap();
    let f = Fixture::new(&tx, line!());
    let a = f.scope().await;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("candidates.csv");

    let error = candidate::export_candidate_csv(
        &tx,
        &path,
        &vec![BAD_UUID.to_string()],
        &a.tenant_id(),
        &a.event_id(),
    )
    .await;

    assert_invalid_uuid(error);
    assert_eq!(std::fs::read(&path).unwrap(), Vec::<u8>::new());
    tx.rollback().await.unwrap();
}
