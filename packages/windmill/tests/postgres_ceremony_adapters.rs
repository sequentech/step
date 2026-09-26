// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The keys- and tally-ceremony PostgreSQL adapters against the migrated
//! schema: keys ceremonies, trustees, tally sessions and their contests and
//! executions. Each test writes its own rows in a transaction, checks what the
//! adapter returns and what it stored, and rolls back.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::ballot::ContestEncryptionPolicy;
use sequent_core::types::ceremonies::{
    Log, TallyCeremonyStatus, TallyElection, TallyElectionStatus, TallyExecutionStatus,
    TallyRunReason, TallySessionDocuments, TallyTrustee, TallyTrusteeStatus,
};
use sequent_core::types::hasura::core::{
    KeysCeremony, TallySession, TallySessionConfiguration, TallySessionContest,
    TallySessionExecution, Trustee,
};
use sequent_core::types::keycloak::VOTE_WEIGHT_BATCHES;
use serde_json::{json, Value};
use tokio_postgres::error::SqlState;
use uuid::Uuid;
use windmill::postgres::{
    keys_ceremony, tally_session, tally_session_contest, tally_session_execution, trustee,
};

/// Row identifiers of one test. The test's line keeps them apart from the rows
/// of tests running concurrently; `n` tells its own rows apart.
struct Ids(u32);

impl Ids {
    fn id(&self, n: u32) -> String {
        format!("{:08}-0000-4000-8000-{n:012}", self.0)
    }
}

macro_rules! ids {
    () => {
        Ids(line!())
    };
}

/// A tenant with two election events, and another tenant with one, so every
/// query can be checked against rows it must not return or change.
struct World {
    ids: Ids,
    tenant: String,
    event: String,
    other_event: String,
    other_tenant: String,
    other_tenant_event: String,
}

impl World {
    async fn new(tx: &Transaction<'_>, ids: Ids) -> World {
        let world = World {
            tenant: ids.id(1),
            event: ids.id(2),
            other_event: ids.id(3),
            other_tenant: ids.id(4),
            other_tenant_event: ids.id(5),
            ids,
        };
        tenant(tx, &world.tenant).await;
        tenant(tx, &world.other_tenant).await;
        event(tx, &world.tenant, &world.event).await;
        event(tx, &world.tenant, &world.other_event).await;
        event(tx, &world.other_tenant, &world.other_tenant_event).await;
        world
    }

    /// A row of this test; numbers below 10 name the world itself.
    fn id(&self, n: u32) -> String {
        self.ids.id(n)
    }
}

async fn tenant(tx: &Transaction<'_>, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1::text::uuid, $1)",
        &[&id],
    )
    .await
    .unwrap();
}

async fn event(tx: &Transaction<'_>, tenant: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
             VALUES ($1::text::uuid, $2::text::uuid, 'RSA256')",
        &[&id, &tenant],
    )
    .await
    .unwrap();
}

async fn election(tx: &Transaction<'_>, tenant: &str, event: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid)",
        &[&id, &tenant, &event],
    )
    .await
    .unwrap();
}

async fn area(tx: &Transaction<'_>, tenant: &str, event: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid)",
        &[&id, &tenant, &event],
    )
    .await
    .unwrap();
}

async fn contest(tx: &Transaction<'_>, tenant: &str, event: &str, election: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4::text::uuid)",
        &[&id, &tenant, &event, &election],
    )
    .await
    .unwrap();
}

/// A keys ceremony with no trustees, threshold 2 and every optional column
/// left to its default.
async fn keys_ceremony_row(tx: &Transaction<'_>, tenant: &str, event: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.keys_ceremony
                 (id, tenant_id, election_event_id, trustee_ids, threshold)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, '{}', 2)",
        &[&id, &tenant, &event],
    )
    .await
    .unwrap();
}

/// A tally session created at `created_at`, with threshold 2, on a keys
/// ceremony of the same id; every optional column is left NULL.
async fn tally_session_row(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    id: &str,
    created_at: &str,
) {
    keys_ceremony_row(tx, tenant, event, id).await;
    tx.execute(
        "INSERT INTO sequent_backend.tally_session
                 (id, tenant_id, election_event_id, keys_ceremony_id, threshold, created_at)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $1::text::uuid, 2,
                     $4::text::timestamptz)",
        &[&id, &tenant, &event, &created_at],
    )
    .await
    .unwrap();
}

async fn results_event_row(tx: &Transaction<'_>, tenant: &str, event: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.results_event (id, tenant_id, election_event_id)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid)",
        &[&id, &tenant, &event],
    )
    .await
    .unwrap();
}

/// An execution of message 1 of a tally session, created at `created_at`.
async fn execution_row(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    tally_session: &str,
    id: &str,
    created_at: Option<&str>,
) {
    tx.execute(
        "INSERT INTO sequent_backend.tally_session_execution
                 (id, tenant_id, election_event_id, tally_session_id, current_message_id,
                  created_at)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4::text::uuid, 1,
                     $5::text::timestamptz)",
        &[&id, &tenant, &event, &tally_session, &created_at],
    )
    .await
    .unwrap();
}

/// A contest-area batch `session_id` of a tally session.
async fn session_contest_row(
    tx: &Transaction<'_>,
    rows: &SessionContestRows<'_>,
    id: &str,
    session_id: i32,
) {
    tx.execute(
        "INSERT INTO sequent_backend.tally_session_contest
                 (id, tenant_id, election_event_id, area_id, contest_id, session_id,
                  tally_session_id, election_id)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4::text::uuid,
                     $5::text::uuid, $6, $7::text::uuid, $8::text::uuid)",
        &[
            &id,
            &rows.tenant,
            &rows.event,
            &rows.area,
            &rows.contest,
            &session_id,
            &rows.tally_session,
            &rows.election,
        ],
    )
    .await
    .unwrap();
}

/// The parents a tally-session contest row refers to.
struct SessionContestRows<'a> {
    tenant: &'a str,
    event: &'a str,
    election: &'a str,
    area: &'a str,
    contest: &'a str,
    tally_session: &'a str,
}

/// Inserts an election, area, contest and tally session in the event.
async fn session_contest_parents<'a>(
    tx: &Transaction<'_>,
    tenant: &'a str,
    event: &'a str,
    [election_id, area_id, contest_id, tally_session]: [&'a str; 4],
) -> SessionContestRows<'a> {
    election(tx, tenant, event, election_id).await;
    area(tx, tenant, event, area_id).await;
    contest(tx, tenant, event, election_id, contest_id).await;
    tally_session_row(tx, tenant, event, tally_session, H10).await;
    SessionContestRows {
        tenant,
        event,
        election: election_id,
        area: area_id,
        contest: contest_id,
        tally_session,
    }
}

/// The row of `table` with this id as `to_jsonb` renders it, without the
/// timestamps PostgreSQL fills in.
async fn stored(tx: &Transaction<'_>, table: &str, id: &str) -> Value {
    tx.query_one(
        &format!(
            "SELECT to_jsonb(r) - 'created_at' - 'last_updated_at'
                 FROM sequent_backend.{table} r WHERE r.id = $1::text::uuid"
        ),
        &[&id],
    )
    .await
    .unwrap()
    .get(0)
}

/// The same as [`stored`], for tables whose ids repeat across events.
async fn stored_in(tx: &Transaction<'_>, table: &str, event: &str, id: &str) -> Value {
    tx.query_one(
        &format!(
            "SELECT to_jsonb(r) - 'created_at' - 'last_updated_at'
                 FROM sequent_backend.{table} r
                 WHERE r.id = $1::text::uuid AND r.election_event_id = $2::text::uuid"
        ),
        &[&id, &event],
    )
    .await
    .unwrap()
    .get(0)
}

async fn count(tx: &Transaction<'_>, table: &str, tenant: &str) -> i64 {
    tx.query_one(
        &format!("SELECT count(*) FROM sequent_backend.{table} WHERE tenant_id = $1::text::uuid"),
        &[&tenant],
    )
    .await
    .unwrap()
    .get(0)
}

/// The start of the transaction, which `now()` and column defaults record.
async fn now(tx: &Transaction<'_>) -> DateTime<Local> {
    tx.query_one("SELECT now()", &[]).await.unwrap().get(0)
}

/// Fixed creation times: rows are ordered by these, never by the clock.
const H10: &str = "2026-01-01T10:00:00Z";
const H11: &str = "2026-01-01T11:00:00Z";
const H12: &str = "2026-01-01T12:00:00Z";
const H13: &str = "2026-01-01T13:00:00Z";
const H14: &str = "2026-01-01T14:00:00Z";
const H15: &str = "2026-01-01T15:00:00Z";
const NEXT_DAY: &str = "2026-01-02T10:00:00Z";

fn at(timestamp: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(timestamp)
        .unwrap()
        .with_timezone(&Local)
}

fn sql_state(error: &anyhow::Error) -> Option<SqlState> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<tokio_postgres::Error>())
        .and_then(|error| error.code().cloned())
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

// keys_ceremony

#[tokio::test]
async fn insert_keys_ceremony_returns_and_stores_every_supplied_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (ceremony, trustee_a, trustee_b) = (w.id(10), w.id(11), w.id(12));
    let status = json!({"stop_date": null, "public_key": null, "logs": [], "trustees": []});

    let inserted = keys_ceremony::insert_keys_ceremony(
        &tx,
        ceremony.clone(),
        w.tenant.clone(),
        w.event.clone(),
        vec![trustee_a.clone(), trustee_b.clone()],
        2,
        Some(status.clone()),
        Some("NOT_STARTED".into()),
        Some("Main ceremony".into()),
        Some(json!({"policy": "automated-ceremonies"})),
        false,
        vec!["north".into(), "south".into()],
    )
    .await
    .unwrap();

    let started = now(&tx).await;
    assert_eq!(
        inserted,
        KeysCeremony {
            id: ceremony.clone(),
            created_at: Some(started),
            last_updated_at: Some(started),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            trustee_ids: vec![trustee_a.clone(), trustee_b.clone()],
            status: Some(status.clone()),
            execution_status: Some("NOT_STARTED".into()),
            labels: None,
            annotations: None,
            threshold: 2,
            name: Some("Main ceremony".into()),
            settings: Some(json!({"policy": "automated-ceremonies"})),
            is_default: Some(false),
            permission_label: Some(vec!["north".into(), "south".into()]),
        }
    );
    assert_eq!(
        stored(&tx, "keys_ceremony", &ceremony).await,
        json!({
            "id": ceremony,
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "trustee_ids": [trustee_a, trustee_b],
            "status": status,
            "execution_status": "NOT_STARTED",
            "labels": null,
            "annotations": null,
            "threshold": 2,
            "name": "Main ceremony",
            "settings": {"policy": "automated-ceremonies"},
            "is_default": false,
            "permission_label": ["north", "south"],
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_keys_ceremony_rejects_a_trustee_id_that_is_not_a_v4_uuid_before_writing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = keys_ceremony::insert_keys_ceremony(
        &tx,
        w.id(10),
        w.tenant.clone(),
        w.event.clone(),
        vec![w.id(11), "trustee-b".into()],
        2,
        None,
        None,
        None,
        None,
        true,
        vec![],
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "Error parsing trustee_ids as UUIDs");
    assert!(error
        .root_cause()
        .to_string()
        .starts_with("invalid UUID 'trustee-b'"));
    assert_eq!(count(&tx, "keys_ceremony", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_keys_ceremonies_returns_the_ceremonies_of_the_event_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(10)).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(11)).await;
    keys_ceremony_row(&tx, &w.tenant, &w.other_event, &w.id(12)).await;
    keys_ceremony_row(&tx, &w.other_tenant, &w.other_tenant_event, &w.id(13)).await;

    let ceremonies = keys_ceremony::get_keys_ceremonies(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    assert_eq!(
        sorted(ceremonies.into_iter().map(|ceremony| ceremony.id).collect()),
        [w.id(10), w.id(11)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_keys_ceremony_by_id_maps_null_optional_columns_to_none() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(10)).await;

    let ceremony = keys_ceremony::get_keys_ceremony_by_id(&tx, &w.tenant, &w.event, &w.id(10))
        .await
        .unwrap();

    let started = now(&tx).await;
    assert_eq!(
        ceremony,
        KeysCeremony {
            id: w.id(10),
            created_at: Some(started),
            last_updated_at: Some(started),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            trustee_ids: vec![],
            status: None,
            execution_status: None,
            labels: None,
            annotations: None,
            threshold: 2,
            name: None,
            settings: None,
            // The column defaults to true.
            is_default: Some(true),
            permission_label: None,
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_keys_ceremony_by_id_reports_a_ceremony_of_another_event_as_not_found() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.other_event, &w.id(10)).await;

    let error = keys_ceremony::get_keys_ceremony_by_id(&tx, &w.tenant, &w.event, &w.id(10))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!("Keys ceremony {} not found", w.id(10))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_keys_ceremony_by_id_rejects_a_uuid_that_is_not_version_4() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let version_1 = "a0e9c2c4-1dd1-11b2-8000-000000000010";

    let error = keys_ceremony::get_keys_ceremony_by_id(&tx, &w.tenant, &w.event, version_1)
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!("UUID '{version_1}' is not v4 (version: Some(Mac))")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn lock_keys_ceremony_by_id_returns_the_ceremony_it_locks() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(10)).await;
    keys_ceremony_row(&tx, &w.tenant, &w.other_event, &w.id(10)).await;

    let locked = keys_ceremony::lock_keys_ceremony_by_id(&tx, &w.tenant, &w.event, &w.id(10))
        .await
        .unwrap();

    assert_eq!(
        (locked.id.as_str(), locked.election_event_id.as_str()),
        (w.id(10).as_str(), w.event.as_str())
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn lock_keys_ceremony_by_id_makes_another_transaction_wait_until_it_ends() {
    // Another transaction only sees committed rows, so this setup commits and
    // deletes its rows at the end.
    let pool = schema::pool().await;
    let ids = ids!();
    let (tenant_id, event_id, ceremony) = (ids.id(1), ids.id(2), ids.id(10));
    let mut setup = pool.get().await.unwrap();
    let rows = setup.transaction().await.unwrap();
    tenant(&rows, &tenant_id).await;
    event(&rows, &tenant_id, &event_id).await;
    keys_ceremony_row(&rows, &tenant_id, &event_id, &ceremony).await;
    rows.commit().await.unwrap();

    let mut holder = pool.get().await.unwrap();
    let held = holder.transaction().await.unwrap();
    keys_ceremony::lock_keys_ceremony_by_id(&held, &tenant_id, &event_id, &ceremony)
        .await
        .unwrap();
    let mut contender = pool.get().await.unwrap();
    let waiting = contender.transaction().await.unwrap();
    waiting
        .batch_execute("SET LOCAL lock_timeout = '10ms'")
        .await
        .unwrap();
    let error = keys_ceremony::lock_keys_ceremony_by_id(&waiting, &tenant_id, &event_id, &ceremony)
        .await
        .unwrap_err();
    assert_eq!(sql_state(&error), Some(SqlState::LOCK_NOT_AVAILABLE));
    waiting.rollback().await.unwrap();
    held.rollback().await.unwrap();

    let retry = contender.transaction().await.unwrap();
    retry
        .batch_execute("SET LOCAL lock_timeout = '10ms'")
        .await
        .unwrap();
    keys_ceremony::lock_keys_ceremony_by_id(&retry, &tenant_id, &event_id, &ceremony)
        .await
        .unwrap();
    retry.rollback().await.unwrap();

    let cleanup = setup.transaction().await.unwrap();
    cleanup
        .batch_execute(&format!(
            "DELETE FROM sequent_backend.keys_ceremony WHERE tenant_id = '{tenant_id}';
             DELETE FROM sequent_backend.election_event WHERE tenant_id = '{tenant_id}';
             DELETE FROM sequent_backend.tenant WHERE id = '{tenant_id}';"
        ))
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
}

#[tokio::test]
async fn update_keys_ceremony_status_changes_only_the_ceremony_of_the_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    // Keys ceremonies are keyed by (id, tenant, event): the same id may repeat.
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(10)).await;
    keys_ceremony_row(&tx, &w.tenant, &w.other_event, &w.id(10)).await;
    let status = json!({"stop_date": "2026-01-01", "public_key": "pk", "logs": [], "trustees": []});

    keys_ceremony::update_keys_ceremony_status(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        &status,
        "SUCCESS",
    )
    .await
    .unwrap();

    let updated = stored_in(&tx, "keys_ceremony", &w.event, &w.id(10)).await;
    assert_eq!(
        (&updated["status"], &updated["execution_status"]),
        (&status, &json!("SUCCESS"))
    );
    let untouched = stored_in(&tx, "keys_ceremony", &w.other_event, &w.id(10)).await;
    assert_eq!(
        (&untouched["status"], &untouched["execution_status"]),
        (&Value::Null, &Value::Null)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_keys_ceremony_status_fails_for_a_ceremony_of_another_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.other_tenant, &w.other_tenant_event, &w.id(10)).await;

    let error = keys_ceremony::update_keys_ceremony_status(
        &tx,
        &w.tenant,
        &w.other_tenant_event,
        &w.id(10),
        &json!({}),
        "SUCCESS",
    )
    .await
    .unwrap_err();

    assert_eq!(error.to_string(), "No keys ceremony found");
    let untouched = stored(&tx, "keys_ceremony", &w.id(10)).await;
    assert_eq!(untouched["execution_status"], Value::Null);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn list_keys_ceremony_without_permission_labels_returns_every_ceremony_of_the_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(10)).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(11)).await;
    keys_ceremony_row(&tx, &w.tenant, &w.other_event, &w.id(12)).await;
    tx.batch_execute(&format!(
        "UPDATE sequent_backend.keys_ceremony SET permission_label = '{{north}}'
             WHERE id = '{}'",
        w.id(10)
    ))
    .await
    .unwrap();

    let ceremonies = keys_ceremony::list_keys_ceremony(&tx, &w.tenant, &w.event, &vec![])
        .await
        .unwrap();

    assert_eq!(
        sorted(ceremonies.into_iter().map(|ceremony| ceremony.id).collect()),
        [w.id(10), w.id(11)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn list_keys_ceremony_returns_the_ceremonies_sharing_a_permission_label() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    for (n, labels) in [
        (10, "'{north,south}'"),
        (11, "'{east}'"),
        (12, "NULL"),
        (13, "'{}'"),
    ] {
        keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(n)).await;
        tx.batch_execute(&format!(
            "UPDATE sequent_backend.keys_ceremony SET permission_label = {labels}
                 WHERE id = '{}'",
            w.id(n)
        ))
        .await
        .unwrap();
    }

    let ceremonies = keys_ceremony::list_keys_ceremony(
        &tx,
        &w.tenant,
        &w.event,
        &vec!["south".into(), "west".into()],
    )
    .await
    .unwrap();

    assert_eq!(
        ceremonies
            .into_iter()
            .map(|ceremony| ceremony.id)
            .collect::<Vec<_>>(),
        [w.id(10)]
    );
    tx.rollback().await.unwrap();
}

// trustee

async fn trustee_row(tx: &Transaction<'_>, tenant: &str, id: &str, name: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.trustee (id, tenant_id, name)
             VALUES ($1::text::uuid, $2::text::uuid, $3)",
        &[&id, &tenant, &name],
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn get_trustees_by_id_returns_the_requested_trustees_of_the_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    trustee_row(&tx, &w.tenant, &w.id(10), "trustee-1").await;
    trustee_row(&tx, &w.tenant, &w.id(11), "trustee-2").await;
    trustee_row(&tx, &w.tenant, &w.id(12), "trustee-3").await;
    trustee_row(&tx, &w.other_tenant, &w.id(13), "trustee-4").await;

    let trustees = trustee::get_trustees_by_id(&tx, &w.tenant, &vec![w.id(10), w.id(12), w.id(13)])
        .await
        .unwrap();

    assert_eq!(
        sorted(trustees.into_iter().map(|trustee| trustee.id).collect()),
        [w.id(10), w.id(12)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_trustees_by_id_rejects_an_invalid_trustee_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = trustee::get_trustees_by_id(&tx, &w.tenant, &vec!["trustee-1".into()])
        .await
        .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'trustee-1'"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_trustees_by_name_returns_the_named_trustees_of_the_tenant() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    trustee_row(&tx, &w.tenant, &w.id(10), "trustee-1").await;
    trustee_row(&tx, &w.tenant, &w.id(11), "trustee-2").await;
    trustee_row(&tx, &w.tenant, &w.id(12), "trustee-3").await;
    trustee_row(&tx, &w.other_tenant, &w.id(13), "trustee-1").await;

    let trustees = trustee::get_trustees_by_name(
        &tx,
        &w.tenant,
        &vec!["trustee-1".into(), "trustee-3".into()],
    )
    .await
    .unwrap();

    assert_eq!(
        sorted(trustees.into_iter().map(|trustee| trustee.id).collect()),
        [w.id(10), w.id(12)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_trustee_by_name_returns_the_trustee_with_its_columns() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tx.execute(
        "INSERT INTO sequent_backend.trustee
                 (id, tenant_id, name, public_key, labels, annotations, created_at,
                  last_updated_at)
             VALUES ($1::text::uuid, $2::text::uuid, 'trustee-1', 'public-key',
                     '{\"team\": \"a\"}', '{\"order\": 1}', '2026-01-01T10:00:00Z',
                     '2026-01-02T10:00:00Z')",
        &[&w.id(10), &w.tenant],
    )
    .await
    .unwrap();

    let found = trustee::get_trustee_by_name(&tx, &w.tenant, "trustee-1")
        .await
        .unwrap();

    assert_eq!(
        found,
        Trustee {
            id: w.id(10),
            public_key: Some("public-key".into()),
            name: Some("trustee-1".into()),
            created_at: Some(at(H10)),
            last_updated_at: Some(at(NEXT_DAY)),
            labels: Some(json!({"team": "a"})),
            annotations: Some(json!({"order": 1})),
            tenant_id: w.tenant.clone(),
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_trustee_by_name_reports_a_trustee_of_another_tenant_as_not_found() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    trustee_row(&tx, &w.other_tenant, &w.id(10), "trustee-1").await;

    let error = trustee::get_trustee_by_name(&tx, &w.tenant, "trustee-1")
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Trustee trustee-1 not found");
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_all_trustees_returns_every_trustee_of_the_tenant_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    trustee_row(&tx, &w.tenant, &w.id(10), "trustee-1").await;
    trustee_row(&tx, &w.tenant, &w.id(11), "trustee-2").await;
    trustee_row(&tx, &w.other_tenant, &w.id(12), "trustee-3").await;

    let trustees = trustee::get_all_trustees(&tx, &w.tenant).await.unwrap();

    assert_eq!(
        sorted(trustees.into_iter().map(|trustee| trustee.id).collect()),
        [w.id(10), w.id(11)]
    );
    tx.rollback().await.unwrap();
}

// tally_session

fn configuration() -> TallySessionConfiguration {
    TallySessionConfiguration {
        report_content_template_id: Some("template-1".into()),
        contest_encryption_policy: Some(ContestEncryptionPolicy::MULTIPLE_CONTESTS),
        ..TallySessionConfiguration::default()
    }
}

/// `configuration()` as the column stores it.
fn configuration_json() -> Value {
    json!({
        "report_content_template_id": "template-1",
        "contest_encryption_policy": "multiple-contests",
        "decoded_ballots_inclusion_policy": null,
        "delegated_voting_policy": null,
        "consolidated_report_policy": null,
        "weighted_voting_policy": null,
    })
}

async fn set(tx: &Transaction<'_>, table: &str, id: &str, assignments: &str) {
    tx.batch_execute(&format!(
        "UPDATE sequent_backend.{table} SET {assignments} WHERE id = '{id}'"
    ))
    .await
    .unwrap();
}

fn session_ids(sessions: Vec<TallySession>) -> Vec<String> {
    sessions.into_iter().map(|session| session.id).collect()
}

#[tokio::test]
async fn insert_tally_session_returns_and_stores_the_session() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (session, ceremony, election_id, area_id) = (w.id(10), w.id(11), w.id(12), w.id(13));
    keys_ceremony_row(&tx, &w.tenant, &w.event, &ceremony).await;

    let inserted = tally_session::insert_tally_session(
        &tx,
        &w.tenant,
        &w.event,
        vec![election_id.clone()],
        vec![area_id.clone()],
        &session,
        &ceremony,
        TallyExecutionStatus::STARTED,
        3,
        Some(configuration()),
        "ELECTORAL_RESULTS",
        json!({"is_post_task_completed": false}),
        vec!["north".into()],
    )
    .await
    .unwrap();

    let started = now(&tx).await;
    assert_eq!(
        inserted,
        TallySession {
            id: session.clone(),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            created_at: Some(started),
            last_updated_at: Some(started),
            labels: None,
            annotations: Some(json!({"is_post_task_completed": false})),
            election_ids: Some(vec![election_id.clone()]),
            area_ids: Some(vec![area_id.clone()]),
            is_execution_completed: false,
            keys_ceremony_id: ceremony.clone(),
            execution_status: Some("STARTED".into()),
            threshold: 3,
            configuration: Some(configuration()),
            tally_type: Some("ELECTORAL_RESULTS".into()),
            permission_label: Some(vec!["north".into()]),
        }
    );
    assert_eq!(
        stored(&tx, "tally_session", &session).await,
        json!({
            "id": session,
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "labels": null,
            "annotations": {"is_post_task_completed": false},
            "election_ids": [election_id],
            "area_ids": [area_id],
            "is_execution_completed": false,
            "keys_ceremony_id": ceremony,
            "execution_status": "STARTED",
            "threshold": 3,
            "configuration": configuration_json(),
            "tally_type": "ELECTORAL_RESULTS",
            "permission_label": ["north"],
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_session_without_configuration_stores_null() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(11)).await;

    let inserted = tally_session::insert_tally_session(
        &tx,
        &w.tenant,
        &w.event,
        vec![],
        vec![],
        &w.id(10),
        &w.id(11),
        TallyExecutionStatus::STARTED,
        2,
        None,
        "INITIALIZATION_REPORT",
        json!({}),
        vec![],
    )
    .await
    .unwrap();

    assert_eq!(inserted.configuration, None);
    let row = stored(&tx, "tally_session", &w.id(10)).await;
    assert_eq!(
        (
            &row["configuration"],
            &row["election_ids"],
            &row["permission_label"]
        ),
        (&Value::Null, &json!([]), &json!([]))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_session_rejects_an_invalid_election_id_before_writing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(11)).await;

    let error = tally_session::insert_tally_session(
        &tx,
        &w.tenant,
        &w.event,
        vec!["election-1".into()],
        vec![],
        &w.id(10),
        &w.id(11),
        TallyExecutionStatus::STARTED,
        2,
        None,
        "ELECTORAL_RESULTS",
        json!({}),
        vec![],
    )
    .await
    .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'election-1'"));
    assert_eq!(count(&tx, "tally_session", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn pending_post_tally_task_sessions_are_those_marked_incomplete_newest_first() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    for (n, event_id, created_at, annotations) in [
        (10, &w.event, H10, "'{\"is_post_task_completed\": false}'"),
        (
            11,
            &w.event,
            H11,
            "'{\"is_post_task_completed\": false, \"note\": 1}'",
        ),
        (12, &w.event, H12, "'{\"is_post_task_completed\": true}'"),
        (13, &w.event, H13, "NULL"),
        (14, &w.event, H14, "'{}'"),
        (
            15,
            &w.other_event,
            H15,
            "'{\"is_post_task_completed\": false}'",
        ),
    ] {
        tally_session_row(&tx, &w.tenant, event_id, &w.id(n), created_at).await;
        set(
            &tx,
            "tally_session",
            &w.id(n),
            &format!("annotations = {annotations}"),
        )
        .await;
    }

    let pending = tally_session::get_tally_session_by_election_event_id_pending_post_tally_task(
        &tx, &w.tenant, &w.event,
    )
    .await
    .unwrap();

    assert_eq!(session_ids(pending), [w.id(11), w.id(10)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sessions_by_election_event_id_returns_every_session_newest_first() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(11), H12).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(12), H11).await;
    tally_session_row(&tx, &w.tenant, &w.other_event, &w.id(13), H13).await;
    set(
        &tx,
        "tally_session",
        &w.id(12),
        "is_execution_completed = true",
    )
    .await;

    let sessions =
        tally_session::get_tally_sessions_by_election_event_id(&tx, &w.tenant, &w.event, false)
            .await
            .unwrap();

    assert_eq!(session_ids(sessions), [w.id(11), w.id(12), w.id(10)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sessions_by_election_event_id_only_active_keeps_unfinished_sessions_in_progress()
{
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    for (n, event_id, created_at, assignments) in [
        (10, &w.event, H10, "execution_status = 'IN_PROGRESS'"),
        (11, &w.event, H11, "execution_status = 'IN_PROGRESS'"),
        (
            12,
            &w.event,
            H12,
            "execution_status = 'IN_PROGRESS', is_execution_completed = true",
        ),
        (13, &w.event, H13, "execution_status = 'STARTED'"),
        (14, &w.event, H14, "execution_status = NULL"),
        (15, &w.other_event, H15, "execution_status = 'IN_PROGRESS'"),
    ] {
        tally_session_row(&tx, &w.tenant, event_id, &w.id(n), created_at).await;
        set(&tx, "tally_session", &w.id(n), assignments).await;
    }

    let active =
        tally_session::get_tally_sessions_by_election_event_id(&tx, &w.tenant, &w.event, true)
            .await
            .unwrap();

    assert_eq!(session_ids(active), [w.id(11), w.id(10)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_by_id_maps_null_optional_columns_to_none() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;

    let session = tally_session::get_tally_session_by_id(&tx, &w.tenant, &w.event, &w.id(10))
        .await
        .unwrap();

    assert_eq!(
        session,
        TallySession {
            id: w.id(10),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            created_at: Some(at(H10)),
            last_updated_at: Some(now(&tx).await),
            labels: None,
            annotations: None,
            election_ids: None,
            area_ids: None,
            is_execution_completed: false,
            keys_ceremony_id: w.id(10),
            execution_status: None,
            threshold: 2,
            configuration: None,
            tally_type: None,
            permission_label: None,
        }
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_by_id_reports_a_session_of_another_event_as_not_found() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.other_event, &w.id(10), H10).await;

    let error = tally_session::get_tally_session_by_id(&tx, &w.tenant, &w.event, &w.id(10))
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!("Tally Session {} not found", w.id(10))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_by_id_fails_on_a_configuration_it_cannot_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    set(
        &tx,
        "tally_session",
        &w.id(10),
        "configuration = '{\"contest_encryption_policy\": \"every-contest\"}'",
    )
    .await;

    let error = tally_session::get_tally_session_by_id(&tx, &w.tenant, &w.event, &w.id(10))
        .await
        .unwrap_err();

    assert!(error
        .to_string()
        .starts_with("contest_encryption_policy: unknown variant `every-contest`"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_by_id_rejects_an_invalid_uuid() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = tally_session::get_tally_session_by_id(&tx, &w.tenant, &w.event, "session")
        .await
        .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'session'"));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn lock_tally_session_for_update_reports_a_missing_session() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.other_tenant, &w.other_tenant_event, &w.id(10), H10).await;

    let error = tally_session::lock_tally_session_for_update(
        &tx,
        &w.tenant,
        &w.other_tenant_event,
        &w.id(10),
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        format!("Tally Session {} not found", w.id(10))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn lock_tally_session_for_update_makes_another_transaction_wait_until_it_ends() {
    // Another transaction only sees committed rows, so this setup commits and
    // deletes its rows at the end.
    let pool = schema::pool().await;
    let ids = ids!();
    let (tenant_id, event_id, session) = (ids.id(1), ids.id(2), ids.id(10));
    let mut setup = pool.get().await.unwrap();
    let rows = setup.transaction().await.unwrap();
    tenant(&rows, &tenant_id).await;
    event(&rows, &tenant_id, &event_id).await;
    tally_session_row(&rows, &tenant_id, &event_id, &session, H10).await;
    rows.commit().await.unwrap();

    let mut holder = pool.get().await.unwrap();
    let held = holder.transaction().await.unwrap();
    tally_session::lock_tally_session_for_update(&held, &tenant_id, &event_id, &session)
        .await
        .unwrap();
    let mut contender = pool.get().await.unwrap();
    let waiting = contender.transaction().await.unwrap();
    waiting
        .batch_execute("SET LOCAL lock_timeout = '10ms'")
        .await
        .unwrap();
    let error =
        tally_session::lock_tally_session_for_update(&waiting, &tenant_id, &event_id, &session)
            .await
            .unwrap_err();
    assert_eq!(sql_state(&error), Some(SqlState::LOCK_NOT_AVAILABLE));
    waiting.rollback().await.unwrap();
    held.rollback().await.unwrap();

    let retry = contender.transaction().await.unwrap();
    retry
        .batch_execute("SET LOCAL lock_timeout = '10ms'")
        .await
        .unwrap();
    tally_session::lock_tally_session_for_update(&retry, &tenant_id, &event_id, &session)
        .await
        .unwrap();
    retry.rollback().await.unwrap();

    let cleanup = setup.transaction().await.unwrap();
    cleanup
        .batch_execute(&format!(
            "DELETE FROM sequent_backend.tally_session WHERE tenant_id = '{tenant_id}';
             DELETE FROM sequent_backend.keys_ceremony WHERE tenant_id = '{tenant_id}';
             DELETE FROM sequent_backend.election_event WHERE tenant_id = '{tenant_id}';
             DELETE FROM sequent_backend.tenant WHERE id = '{tenant_id}';"
        ))
        .await
        .unwrap();
    cleanup.commit().await.unwrap();
}

#[tokio::test]
async fn update_tally_session_annotation_replaces_the_annotations_of_the_event_session_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    // Tally sessions are keyed by (id, tenant, event): the same id may repeat.
    for event_id in [&w.event, &w.other_event] {
        tally_session_row(&tx, &w.tenant, event_id, &w.id(10), H10).await;
    }
    set(
        &tx,
        "tally_session",
        &w.id(10),
        "annotations = '{\"kept\": 1}'",
    )
    .await;

    tally_session::update_tally_session_annotation(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        json!({"replaced": true}),
    )
    .await
    .unwrap();

    assert_eq!(
        stored_in(&tx, "tally_session", &w.event, &w.id(10)).await["annotations"],
        json!({"replaced": true})
    );
    assert_eq!(
        stored_in(&tx, "tally_session", &w.other_event, &w.id(10)).await["annotations"],
        json!({"kept": 1})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sessions_by_election_id_returns_the_sessions_tallying_the_election() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (first, second) = (w.id(20), w.id(21));
    for (n, event_id, created_at, elections) in [
        (10, &w.event, H10, format!("'{{{first}}}'")),
        (11, &w.event, H11, format!("'{{{second},{first}}}'")),
        (12, &w.event, H12, format!("'{{{second}}}'")),
        (13, &w.event, H13, "NULL".to_string()),
        (14, &w.other_event, H14, format!("'{{{first}}}'")),
    ] {
        tally_session_row(&tx, &w.tenant, event_id, &w.id(n), created_at).await;
        set(
            &tx,
            "tally_session",
            &w.id(n),
            &format!("election_ids = {elections}"),
        )
        .await;
    }

    let sessions =
        tally_session::get_tally_sessions_by_election_id(&tx, &w.tenant, &w.event, &first)
            .await
            .unwrap();

    assert_eq!(session_ids(sessions), [w.id(11), w.id(10)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_session_status_sets_status_and_completion_of_the_event_session_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    for event_id in [&w.event, &w.other_event] {
        tally_session_row(&tx, &w.tenant, event_id, &w.id(10), H10).await;
    }

    tally_session::update_tally_session_status(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        TallyExecutionStatus::SUCCESS,
        true,
    )
    .await
    .unwrap();

    let updated = stored_in(&tx, "tally_session", &w.event, &w.id(10)).await;
    assert_eq!(
        (
            &updated["execution_status"],
            &updated["is_execution_completed"]
        ),
        (&json!("SUCCESS"), &json!(true))
    );
    let untouched = stored_in(&tx, "tally_session", &w.other_event, &w.id(10)).await;
    assert_eq!(
        (
            &untouched["execution_status"],
            &untouched["is_execution_completed"]
        ),
        (&Value::Null, &json!(false))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn set_post_tally_task_completed_adds_the_flag_to_existing_annotations() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    for event_id in [&w.event, &w.other_event] {
        tally_session_row(&tx, &w.tenant, event_id, &w.id(10), H10).await;
    }
    set(
        &tx,
        "tally_session",
        &w.id(10),
        "annotations = '{\"note\": 1, \"is_post_task_completed\": false}'",
    )
    .await;

    tally_session::set_post_tally_task_completed(&tx, &w.tenant, &w.event, &w.id(10))
        .await
        .unwrap();

    assert_eq!(
        stored_in(&tx, "tally_session", &w.event, &w.id(10)).await["annotations"],
        json!({"note": 1, "is_post_task_completed": true})
    );
    assert_eq!(
        stored_in(&tx, "tally_session", &w.other_event, &w.id(10)).await["annotations"],
        json!({"note": 1, "is_post_task_completed": false})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn set_post_tally_task_completed_initializes_missing_annotations() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;

    tally_session::set_post_tally_task_completed(&tx, &w.tenant, &w.event, &w.id(10))
        .await
        .unwrap();

    assert_eq!(
        stored(&tx, "tally_session", &w.id(10)).await["annotations"],
        json!({"is_post_task_completed": true})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn set_tally_session_completed_records_completion_and_a_pending_post_task() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    for event_id in [&w.event, &w.other_event] {
        tally_session_row(&tx, &w.tenant, event_id, &w.id(10), H10).await;
    }
    set(
        &tx,
        "tally_session",
        &w.id(10),
        "annotations = '{\"note\": 1}'",
    )
    .await;

    tally_session::set_tally_session_completed(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        TallyExecutionStatus::SUCCESS,
    )
    .await
    .unwrap();

    let updated = stored_in(&tx, "tally_session", &w.event, &w.id(10)).await;
    assert_eq!(
        (
            &updated["execution_status"],
            &updated["is_execution_completed"],
            &updated["annotations"]
        ),
        (
            &json!("SUCCESS"),
            &json!(true),
            &json!({"note": 1, "is_post_task_completed": false})
        )
    );
    let untouched = stored_in(&tx, "tally_session", &w.other_event, &w.id(10)).await;
    assert_eq!(
        (
            &untouched["is_execution_completed"],
            &untouched["annotations"]
        ),
        (&json!(false), &json!({"note": 1}))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn set_tally_session_completed_without_annotations_queues_the_post_task() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;

    tally_session::set_tally_session_completed(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        TallyExecutionStatus::SUCCESS,
    )
    .await
    .unwrap();

    let row = stored(&tx, "tally_session", &w.id(10)).await;
    assert_eq!(
        (&row["is_execution_completed"], &row["annotations"]),
        (&json!(true), &json!({"is_post_task_completed": false}))
    );
    let pending = tally_session::get_tally_session_by_election_event_id_pending_post_tally_task(
        &tx, &w.tenant, &w.event,
    )
    .await
    .unwrap();
    assert_eq!(
        pending
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>(),
        [w.id(10)]
    );
    tx.rollback().await.unwrap();
}

fn session(w: &World, n: u32, ceremony: &str) -> TallySession {
    TallySession {
        id: w.id(n),
        tenant_id: w.tenant.clone(),
        election_event_id: w.event.clone(),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"ignored": true})),
        annotations: Some(json!({"is_post_task_completed": true})),
        election_ids: Some(vec![w.id(20)]),
        area_ids: Some(vec![w.id(21), w.id(22)]),
        is_execution_completed: true,
        keys_ceremony_id: ceremony.into(),
        execution_status: Some("SUCCESS".into()),
        threshold: 3,
        configuration: Some(configuration()),
        tally_type: Some("ELECTORAL_RESULTS".into()),
        permission_label: Some(vec!["north".into()]),
    }
}

#[tokio::test]
async fn insert_many_tally_sessions_stores_every_session_with_its_columns() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(30)).await;

    let inserted = tally_session::insert_many_tally_sessions(
        &tx,
        vec![session(&w, 10, &w.id(30)), session(&w, 11, &w.id(30))],
    )
    .await
    .unwrap();

    assert_eq!(sorted(session_ids(inserted.clone())), [w.id(10), w.id(11)]);
    let started = now(&tx).await;
    let first = inserted
        .iter()
        .find(|session| session.id == w.id(10))
        .unwrap();
    assert_eq!(
        first,
        &TallySession {
            created_at: Some(started),
            last_updated_at: Some(started),
            // Labels and completion are not part of the bulk insert.
            labels: None,
            is_execution_completed: false,
            ..session(&w, 10, &w.id(30))
        }
    );
    assert_eq!(
        stored(&tx, "tally_session", &w.id(11)).await,
        json!({
            "id": w.id(11),
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "labels": null,
            "annotations": {"is_post_task_completed": true},
            "election_ids": [w.id(20)],
            "area_ids": [w.id(21), w.id(22)],
            "is_execution_completed": false,
            "keys_ceremony_id": w.id(30),
            "execution_status": "SUCCESS",
            "threshold": 3,
            "configuration": configuration_json(),
            "tally_type": "ELECTORAL_RESULTS",
            "permission_label": ["north"],
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_sessions_stores_missing_election_and_area_ids_as_empty_arrays() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(30)).await;

    let inserted = tally_session::insert_many_tally_sessions(
        &tx,
        vec![TallySession {
            election_ids: None,
            area_ids: None,
            configuration: None,
            annotations: None,
            permission_label: None,
            ..session(&w, 10, &w.id(30))
        }],
    )
    .await
    .unwrap();

    assert_eq!(
        (&inserted[0].election_ids, &inserted[0].area_ids),
        (&Some(vec![]), &Some(vec![]))
    );
    let row = stored(&tx, "tally_session", &w.id(10)).await;
    assert_eq!(
        (
            &row["election_ids"],
            &row["area_ids"],
            &row["configuration"],
            &row["annotations"],
            &row["permission_label"]
        ),
        (
            &json!([]),
            &json!([]),
            &Value::Null,
            &Value::Null,
            &Value::Null
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_sessions_with_no_sessions_returns_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    let inserted = tally_session::insert_many_tally_sessions(&tx, vec![])
        .await
        .unwrap();

    assert!(inserted.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_sessions_rejects_an_invalid_area_id_before_writing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    keys_ceremony_row(&tx, &w.tenant, &w.event, &w.id(30)).await;

    let error = tally_session::insert_many_tally_sessions(
        &tx,
        vec![
            session(&w, 10, &w.id(30)),
            TallySession {
                area_ids: Some(vec!["area-1".into()]),
                ..session(&w, 11, &w.id(30))
            },
        ],
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .starts_with("Invalid area_id: area-1 - invalid UUID 'area-1'"));
    assert_eq!(count(&tx, "tally_session", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

// tally_session_contest

fn contest_ids(contests: Vec<TallySessionContest>) -> Vec<String> {
    sorted(contests.into_iter().map(|contest| contest.id).collect())
}

#[tokio::test]
async fn insert_tally_session_contest_returns_and_stores_the_contest_batch() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (election_id, area_id, contest_id, session) = (w.id(10), w.id(11), w.id(12), w.id(13));
    session_contest_parents(
        &tx,
        &w.tenant,
        &w.event,
        [&election_id, &area_id, &contest_id, &session],
    )
    .await;

    let inserted = tally_session_contest::insert_tally_session_contest(
        &tx,
        &w.tenant,
        &w.event,
        &area_id,
        Some(contest_id.clone()),
        7,
        &session,
        &election_id,
    )
    .await
    .unwrap();

    let started = now(&tx).await;
    assert_eq!(
        Uuid::parse_str(&inserted.id).unwrap().get_version_num(),
        4,
        "PostgreSQL generates the id"
    );
    assert_eq!(
        inserted,
        TallySessionContest {
            id: inserted.id.clone(),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            area_id: area_id.clone(),
            contest_id: Some(contest_id.clone()),
            session_id: 7,
            created_at: Some(started),
            last_updated_at: Some(started),
            labels: None,
            annotations: None,
            tally_session_id: session.clone(),
            election_id: election_id.clone(),
        }
    );
    assert_eq!(
        stored(&tx, "tally_session_contest", &inserted.id).await,
        json!({
            "id": inserted.id,
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "area_id": area_id,
            "contest_id": contest_id,
            "session_id": 7,
            "labels": null,
            "annotations": null,
            "tally_session_id": session,
            "election_id": election_id,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_session_contest_without_a_contest_stores_a_null_contest() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (election_id, area_id, session) = (w.id(10), w.id(11), w.id(13));
    session_contest_parents(
        &tx,
        &w.tenant,
        &w.event,
        [&election_id, &area_id, &w.id(12), &session],
    )
    .await;

    let inserted = tally_session_contest::insert_tally_session_contest(
        &tx,
        &w.tenant,
        &w.event,
        &area_id,
        None,
        1,
        &session,
        &election_id,
    )
    .await
    .unwrap();

    assert_eq!(inserted.contest_id, None);
    assert_eq!(
        stored(&tx, "tally_session_contest", &inserted.id).await["contest_id"],
        Value::Null
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_session_contest_rejects_a_batch_number_beyond_the_int4_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (election_id, area_id, session) = (w.id(10), w.id(11), w.id(13));
    session_contest_parents(
        &tx,
        &w.tenant,
        &w.event,
        [&election_id, &area_id, &w.id(12), &session],
    )
    .await;

    let valid = tally_session_contest::insert_tally_session_contest(
        &tx,
        &w.tenant,
        &w.event,
        &area_id,
        None,
        i32::MAX as u64,
        &session,
        &election_id,
    )
    .await
    .unwrap();
    assert_eq!(valid.session_id, i32::MAX);
    let error = tally_session_contest::insert_tally_session_contest(
        &tx,
        &w.tenant,
        &w.event,
        &area_id,
        None,
        1 << 31,
        &session,
        &election_id,
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Tally session batch number exceeds the database integer range"
    );
    assert_eq!(count(&tx, "tally_session_contest", &w.tenant).await, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_session_contest_rejects_an_invalid_contest_id_before_writing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (election_id, area_id, session) = (w.id(10), w.id(11), w.id(13));
    session_contest_parents(
        &tx,
        &w.tenant,
        &w.event,
        [&election_id, &area_id, &w.id(12), &session],
    )
    .await;

    let error = tally_session_contest::insert_tally_session_contest(
        &tx,
        &w.tenant,
        &w.event,
        &area_id,
        Some("contest-1".into()),
        1,
        &session,
        &election_id,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'contest-1'"));
    assert_eq!(count(&tx, "tally_session_contest", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_highest_batch_is_zero_for_an_event_without_contests() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let batch = tally_session_contest::get_tally_session_highest_batch(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    assert_eq!(batch, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_highest_batch_skips_the_weight_batches_of_the_last_contest_area() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (a, b, c, d) = (w.id(10), w.id(11), w.id(12), w.id(13));
    let rows = session_contest_parents(&tx, &w.tenant, &w.event, [&a, &b, &c, &d]).await;
    session_contest_row(&tx, &rows, &w.id(20), 1).await;
    session_contest_row(&tx, &rows, &w.id(21), 40).await;
    let (e, f, g, h) = (w.id(14), w.id(15), w.id(16), w.id(17));
    let other = session_contest_parents(&tx, &w.tenant, &w.other_event, [&e, &f, &g, &h]).await;
    session_contest_row(&tx, &other, &w.id(22), 90).await;

    let batch = tally_session_contest::get_tally_session_highest_batch(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    // Session 40 owns batches 40..40 + VOTE_WEIGHT_BATCHES under weighted voting.
    assert_eq!(batch, 40 + u64::from(VOTE_WEIGHT_BATCHES));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_contests_returns_the_contests_of_the_session_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (a, b, c, d) = (w.id(10), w.id(11), w.id(12), w.id(13));
    let rows = session_contest_parents(&tx, &w.tenant, &w.event, [&a, &b, &c, &d]).await;
    let other_session = w.id(14);
    tally_session_row(&tx, &w.tenant, &w.event, &other_session, H11).await;
    session_contest_row(&tx, &rows, &w.id(20), 1).await;
    session_contest_row(&tx, &rows, &w.id(21), 2).await;
    let other_rows = SessionContestRows {
        tally_session: &other_session,
        ..rows
    };
    session_contest_row(&tx, &other_rows, &w.id(22), 3).await;

    let contests = tally_session_contest::get_tally_session_contests(&tx, &w.tenant, &w.event, &d)
        .await
        .unwrap();

    assert_eq!(contest_ids(contests), [w.id(20), w.id(21)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_event_tally_session_contest_returns_every_contest_of_the_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (a, b, c, d) = (w.id(10), w.id(11), w.id(12), w.id(13));
    let rows = session_contest_parents(&tx, &w.tenant, &w.event, [&a, &b, &c, &d]).await;
    let other_session = w.id(14);
    tally_session_row(&tx, &w.tenant, &w.event, &other_session, H11).await;
    session_contest_row(&tx, &rows, &w.id(20), 1).await;
    let second_session = SessionContestRows {
        tally_session: &other_session,
        ..rows
    };
    session_contest_row(&tx, &second_session, &w.id(21), 2).await;
    let (e, f, g, h) = (w.id(15), w.id(16), w.id(17), w.id(18));
    let other = session_contest_parents(&tx, &w.tenant, &w.other_event, [&e, &f, &g, &h]).await;
    session_contest_row(&tx, &other, &w.id(22), 3).await;

    let contests = tally_session_contest::get_event_tally_session_contest(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    assert_eq!(contest_ids(contests), [w.id(20), w.id(21)]);
    tx.rollback().await.unwrap();
}

fn session_contest(
    rows: &SessionContestRows<'_>,
    id: &str,
    annotations: Value,
) -> TallySessionContest {
    TallySessionContest {
        id: id.into(),
        tenant_id: rows.tenant.into(),
        election_event_id: rows.event.into(),
        area_id: rows.area.into(),
        contest_id: Some(rows.contest.into()),
        session_id: 1,
        created_at: Some(at(H10)),
        last_updated_at: Some(at(NEXT_DAY)),
        labels: Some(json!({"label": 1})),
        annotations: Some(annotations),
        tally_session_id: rows.tally_session.into(),
        election_id: rows.election.into(),
    }
}

#[tokio::test]
async fn update_tally_session_contests_annotations_replaces_the_annotations_of_listed_contests() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (a, b, c, d) = (w.id(10), w.id(11), w.id(12), w.id(13));
    let rows = session_contest_parents(&tx, &w.tenant, &w.event, [&a, &b, &c, &d]).await;
    for n in [20, 21] {
        session_contest_row(&tx, &rows, &w.id(n), 1).await;
        set(
            &tx,
            "tally_session_contest",
            &w.id(n),
            "annotations = '{\"old\": 1}'",
        )
        .await;
    }

    tally_session_contest::update_tally_session_contests_annotations(
        &tx,
        &[session_contest(
            &rows,
            &w.id(20),
            json!({"casted_ballots": 5}),
        )],
    )
    .await
    .unwrap();

    assert_eq!(
        stored(&tx, "tally_session_contest", &w.id(20)).await["annotations"],
        json!({"casted_ballots": 5})
    );
    assert_eq!(
        stored(&tx, "tally_session_contest", &w.id(21)).await["annotations"],
        json!({"old": 1})
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_session_contests_annotations_skips_contests_without_a_matching_row() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (a, b, c, d) = (w.id(10), w.id(11), w.id(12), w.id(13));
    let rows = session_contest_parents(&tx, &w.tenant, &w.event, [&a, &b, &c, &d]).await;
    session_contest_row(&tx, &rows, &w.id(20), 1).await;
    let other_tenant_copy = TallySessionContest {
        tenant_id: w.other_tenant.clone(),
        ..session_contest(&rows, &w.id(20), json!({"changed": true}))
    };

    tally_session_contest::update_tally_session_contests_annotations(
        &tx,
        &[
            other_tenant_copy,
            session_contest(&rows, &w.id(29), json!({"changed": true})),
        ],
    )
    .await
    .unwrap();

    assert_eq!(
        stored(&tx, "tally_session_contest", &w.id(20)).await["annotations"],
        Value::Null
    );
    assert_eq!(count(&tx, "tally_session_contest", &w.tenant).await, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_session_contests_keeps_the_supplied_ids_and_timestamps() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (a, b, c, d) = (w.id(10), w.id(11), w.id(12), w.id(13));
    let rows = session_contest_parents(&tx, &w.tenant, &w.event, [&a, &b, &c, &d]).await;
    let contests = vec![
        session_contest(&rows, &w.id(20), json!({"casted_ballots": 1})),
        TallySessionContest {
            contest_id: None,
            session_id: 18,
            ..session_contest(&rows, &w.id(21), json!({"casted_ballots": 2}))
        },
    ];

    let inserted = tally_session_contest::insert_many_tally_session_contests(&tx, contests.clone())
        .await
        .unwrap();

    let mut inserted = inserted;
    inserted.sort_by(|left, right| left.id.cmp(&right.id));
    assert_eq!(inserted, contests);
    assert_eq!(
        stored(&tx, "tally_session_contest", &w.id(21)).await,
        json!({
            "id": w.id(21),
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "area_id": b,
            "contest_id": null,
            "session_id": 18,
            "labels": {"label": 1},
            "annotations": {"casted_ballots": 2},
            "tally_session_id": d,
            "election_id": a,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_session_contests_stores_missing_timestamps_as_null() {
    // The bulk insert names the timestamp columns, so their defaults never apply.
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (a, b, c, d) = (w.id(10), w.id(11), w.id(12), w.id(13));
    let rows = session_contest_parents(&tx, &w.tenant, &w.event, [&a, &b, &c, &d]).await;

    let inserted = tally_session_contest::insert_many_tally_session_contests(
        &tx,
        vec![TallySessionContest {
            created_at: None,
            last_updated_at: None,
            ..session_contest(&rows, &w.id(20), json!({}))
        }],
    )
    .await
    .unwrap();

    assert_eq!(
        (inserted[0].created_at, inserted[0].last_updated_at),
        (None, None)
    );
    let row = tx
        .query_one(
            "SELECT created_at IS NULL AND last_updated_at IS NULL
             FROM sequent_backend.tally_session_contest WHERE id = $1::text::uuid",
            &[&w.id(20)],
        )
        .await
        .unwrap();
    assert!(row.get::<_, bool>(0));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_session_contests_with_no_contests_returns_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    let inserted = tally_session_contest::insert_many_tally_session_contests(&tx, vec![])
        .await
        .unwrap();

    assert!(inserted.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_session_contests_rejects_an_invalid_area_id_before_writing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (a, b, c, d) = (w.id(10), w.id(11), w.id(12), w.id(13));
    let rows = session_contest_parents(&tx, &w.tenant, &w.event, [&a, &b, &c, &d]).await;

    let error = tally_session_contest::insert_many_tally_session_contests(
        &tx,
        vec![
            session_contest(&rows, &w.id(20), json!({})),
            TallySessionContest {
                area_id: "area-1".into(),
                ..session_contest(&rows, &w.id(21), json!({}))
            },
        ],
    )
    .await
    .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'area-1'"));
    assert_eq!(count(&tx, "tally_session_contest", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

// tally_session_execution

fn ceremony_status(election_id: &str) -> TallyCeremonyStatus {
    TallyCeremonyStatus {
        stop_date: None,
        logs: vec![Log {
            created_date: H10.into(),
            log_text: "Mixing".into(),
        }],
        trustees: vec![TallyTrustee {
            name: "trustee-1".into(),
            status: TallyTrusteeStatus::KEY_RESTORED,
        }],
        elections_status: vec![TallyElection {
            election_id: election_id.into(),
            status: TallyElectionStatus::MIXING,
            progress: 0.5,
        }],
    }
}

/// `ceremony_status(election_id)` as the column stores it.
fn ceremony_status_json(election_id: &str) -> Value {
    json!({
        "stop_date": null,
        "logs": [{"created_date": H10, "log_text": "Mixing"}],
        "trustees": [{"name": "trustee-1", "status": "KEY_RESTORED"}],
        "elections_status": [
            {"election_id": election_id, "status": "MIXING", "progress": 0.5}
        ],
    })
}

fn execution_ids(executions: Vec<TallySessionExecution>) -> Vec<String> {
    executions
        .into_iter()
        .map(|execution| execution.id)
        .collect()
}

#[tokio::test]
async fn insert_tally_session_execution_returns_and_stores_status_documents_and_reason() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let (session, results, election_id) = (w.id(10), w.id(11), w.id(12));
    tally_session_row(&tx, &w.tenant, &w.event, &session, H10).await;
    results_event_row(&tx, &w.tenant, &w.event, &results).await;

    let inserted = tally_session_execution::insert_tally_session_execution(
        &tx,
        &w.tenant,
        &w.event,
        3,
        &session,
        Some(ceremony_status(&election_id)),
        Some(results.clone()),
        Some(vec![1, 18]),
        Some(TallySessionDocuments {
            sqlite: Some(w.id(13)),
            xlsx: None,
        }),
        TallyRunReason::RECOUNT,
    )
    .await
    .unwrap();

    assert_eq!(
        inserted,
        TallySessionExecution {
            id: inserted.id.clone(),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            created_at: inserted.created_at,
            last_updated_at: Some(now(&tx).await),
            labels: None,
            annotations: None,
            current_message_id: 3,
            tally_session_id: session.clone(),
            session_ids: Some(vec![1, 18]),
            status: Some(ceremony_status_json(&election_id)),
            results_event_id: Some(results.clone()),
            documents: Some(json!({"sqlite": w.id(13), "xlsx": null})),
            run_reason: Some("RECOUNT".into()),
        }
    );
    assert_eq!(
        stored(&tx, "tally_session_execution", &inserted.id).await,
        json!({
            "id": inserted.id,
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "labels": null,
            "annotations": null,
            "current_message_id": 3,
            "tally_session_id": session,
            "session_ids": [1, 18],
            "status": ceremony_status_json(&election_id),
            "results_event_id": results,
            "documents": {"sqlite": w.id(13), "xlsx": null},
            "run_reason": "RECOUNT",
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_session_execution_dates_each_execution_when_it_is_written() {
    // Readers order executions by `created_at`, so two executions of one
    // transaction must not share the transaction's start time.
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    let mut executions = vec![];
    for message in [1, 2] {
        executions.push(
            tally_session_execution::insert_tally_session_execution(
                &tx,
                &w.tenant,
                &w.event,
                message,
                &w.id(10),
                None,
                None,
                None,
                None,
                TallyRunReason::NORMAL,
            )
            .await
            .unwrap(),
        );
    }

    let started = now(&tx).await;
    let (first, second) = (
        executions[0].created_at.unwrap(),
        executions[1].created_at.unwrap(),
    );
    assert!(started < first && first < second);
    let last = tally_session_execution::get_last_tally_session_execution(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(last.id, executions[1].id);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_session_execution_without_optional_values_stores_nulls() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;

    let inserted = tally_session_execution::insert_tally_session_execution(
        &tx,
        &w.tenant,
        &w.event,
        1,
        &w.id(10),
        None,
        None,
        None,
        None,
        TallyRunReason::NORMAL,
    )
    .await
    .unwrap();

    let row = stored(&tx, "tally_session_execution", &inserted.id).await;
    assert_eq!(
        (
            &row["status"],
            &row["results_event_id"],
            &row["session_ids"],
            &row["documents"],
            &row["run_reason"]
        ),
        (
            &Value::Null,
            &Value::Null,
            &Value::Null,
            &Value::Null,
            &json!("NORMAL")
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_session_execution_rejects_an_invalid_results_event_id_before_writing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;

    let error = tally_session_execution::insert_tally_session_execution(
        &tx,
        &w.tenant,
        &w.event,
        1,
        &w.id(10),
        None,
        Some("results-1".into()),
        None,
        None,
        TallyRunReason::NORMAL,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'results-1'"));
    assert_eq!(count(&tx, "tally_session_execution", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_executions_returns_the_executions_of_the_session_newest_first() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let session = w.id(10);
    tally_session_row(&tx, &w.tenant, &w.event, &session, H10).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(11), H10).await;
    tally_session_row(&tx, &w.tenant, &w.other_event, &session, H10).await;
    for (n, event_id, tally_session, created_at) in [
        (20, &w.event, &session, H10),
        (21, &w.event, &session, H12),
        (22, &w.event, &session, H11),
        (23, &w.event, &w.id(11), H13),
        (24, &w.other_event, &session, H14),
    ] {
        execution_row(
            &tx,
            &w.tenant,
            event_id,
            tally_session,
            &w.id(n),
            Some(created_at),
        )
        .await;
    }

    let executions =
        tally_session_execution::get_tally_session_executions(&tx, &w.tenant, &w.event, &session)
            .await
            .unwrap();

    assert_eq!(execution_ids(executions), [w.id(21), w.id(22), w.id(20)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_last_tally_session_execution_returns_the_newest_execution_of_the_session() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let session = w.id(10);
    tally_session_row(&tx, &w.tenant, &w.event, &session, H10).await;
    tally_session_row(&tx, &w.tenant, &w.other_event, &session, H10).await;
    execution_row(&tx, &w.tenant, &w.event, &session, &w.id(20), Some(H12)).await;
    execution_row(&tx, &w.tenant, &w.event, &session, &w.id(21), Some(H11)).await;
    execution_row(
        &tx,
        &w.tenant,
        &w.other_event,
        &session,
        &w.id(22),
        Some(H13),
    )
    .await;

    let last = tally_session_execution::get_last_tally_session_execution(
        &tx, &w.tenant, &w.event, &session,
    )
    .await
    .unwrap();

    assert_eq!(
        last,
        Some(TallySessionExecution {
            id: w.id(20),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            created_at: Some(at(H12)),
            last_updated_at: Some(now(&tx).await),
            labels: None,
            annotations: None,
            current_message_id: 1,
            tally_session_id: session.clone(),
            session_ids: None,
            status: None,
            results_event_id: None,
            documents: None,
            run_reason: None,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_last_tally_session_execution_is_none_without_executions() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;

    let last = tally_session_execution::get_last_tally_session_execution(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
    )
    .await
    .unwrap();

    assert_eq!(last, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_last_tally_session_execution_prefers_dated_history_and_lists_undated_history_last() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    execution_row(&tx, &w.tenant, &w.event, &w.id(10), &w.id(20), None).await;
    execution_row(&tx, &w.tenant, &w.event, &w.id(10), &w.id(21), Some(H12)).await;

    let last = tally_session_execution::get_last_tally_session_execution(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(last.id, w.id(21));
    let executions =
        tally_session_execution::get_tally_session_executions(&tx, &w.tenant, &w.event, &w.id(10))
            .await
            .unwrap();
    assert_eq!(
        executions
            .into_iter()
            .map(|execution| execution.id)
            .collect::<Vec<_>>(),
        [w.id(21), w.id(20)]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_event_tally_session_executions_returns_every_execution_of_the_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(11), H10).await;
    tally_session_row(&tx, &w.tenant, &w.other_event, &w.id(12), H10).await;
    execution_row(&tx, &w.tenant, &w.event, &w.id(10), &w.id(20), None).await;
    execution_row(&tx, &w.tenant, &w.event, &w.id(11), &w.id(21), None).await;
    execution_row(&tx, &w.tenant, &w.other_event, &w.id(12), &w.id(22), None).await;

    let executions =
        tally_session_execution::get_event_tally_session_executions(&tx, &w.tenant, &w.event)
            .await
            .unwrap();

    assert_eq!(sorted(execution_ids(executions)), [w.id(20), w.id(21)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_execution_documents_returns_the_stored_documents() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    execution_row(&tx, &w.tenant, &w.event, &w.id(10), &w.id(20), None).await;
    set(
        &tx,
        "tally_session_execution",
        &w.id(20),
        "documents = '{\"sqlite\": \"sqlite-doc\", \"xlsx\": \"xlsx-doc\"}'",
    )
    .await;

    let documents = tally_session_execution::get_tally_session_execution_documents(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(20),
    )
    .await
    .unwrap();

    assert_eq!(
        documents,
        Some(TallySessionDocuments {
            sqlite: Some("sqlite-doc".into()),
            xlsx: Some("xlsx-doc".into()),
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_execution_documents_is_none_when_no_documents_were_stored() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    execution_row(&tx, &w.tenant, &w.event, &w.id(10), &w.id(20), None).await;

    let documents = tally_session_execution::get_tally_session_execution_documents(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(20),
    )
    .await
    .unwrap();

    assert_eq!(documents, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_execution_documents_is_none_for_an_execution_of_another_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.other_event, &w.id(10), H10).await;
    execution_row(&tx, &w.tenant, &w.other_event, &w.id(10), &w.id(20), None).await;
    set(
        &tx,
        "tally_session_execution",
        &w.id(20),
        "documents = '{\"sqlite\": \"sqlite-doc\", \"xlsx\": null}'",
    )
    .await;

    let documents = tally_session_execution::get_tally_session_execution_documents(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(20),
    )
    .await
    .unwrap();

    assert_eq!(documents, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_session_execution_documents_fails_on_documents_it_cannot_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    execution_row(&tx, &w.tenant, &w.event, &w.id(10), &w.id(20), None).await;
    set(
        &tx,
        "tally_session_execution",
        &w.id(20),
        "documents = '\"sqlite-doc\"'",
    )
    .await;

    let error = tally_session_execution::get_tally_session_execution_documents(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(20),
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "invalid type: string \"sqlite-doc\", expected struct TallySessionDocuments"
    );
    tx.rollback().await.unwrap();
}

fn execution(w: &World, n: u32, tally_session: &str) -> TallySessionExecution {
    TallySessionExecution {
        id: w.id(n),
        tenant_id: w.tenant.clone(),
        election_event_id: w.event.clone(),
        created_at: Some(at(H10)),
        last_updated_at: Some(at(NEXT_DAY)),
        labels: Some(json!({"label": 1})),
        annotations: Some(json!({"note": n})),
        current_message_id: 4,
        tally_session_id: tally_session.into(),
        session_ids: Some(vec![1, 2]),
        status: Some(
            json!({"stop_date": null, "logs": [], "trustees": [], "elections_status": []}),
        ),
        results_event_id: None,
        documents: None,
        run_reason: None,
    }
}

#[tokio::test]
async fn insert_many_tally_session_executions_keeps_the_supplied_ids_timestamps_and_status() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    results_event_row(&tx, &w.tenant, &w.event, &w.id(11)).await;
    let executions = vec![
        execution(&w, 20, &w.id(10)),
        TallySessionExecution {
            results_event_id: Some(w.id(11)),
            session_ids: None,
            ..execution(&w, 21, &w.id(10))
        },
    ];

    let mut inserted =
        tally_session_execution::insert_many_tally_session_executions(&tx, executions.clone())
            .await
            .unwrap();

    inserted.sort_by(|left, right| left.id.cmp(&right.id));
    assert_eq!(inserted, executions);
    assert_eq!(
        stored(&tx, "tally_session_execution", &w.id(21)).await,
        json!({
            "id": w.id(21),
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "labels": {"label": 1},
            "annotations": {"note": 21},
            "current_message_id": 4,
            "tally_session_id": w.id(10),
            "session_ids": null,
            "status": {"stop_date": null, "logs": [], "trustees": [], "elections_status": []},
            "results_event_id": w.id(11),
            "documents": null,
            "run_reason": null,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_session_executions_does_not_store_documents_or_run_reason() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;

    let inserted = tally_session_execution::insert_many_tally_session_executions(
        &tx,
        vec![TallySessionExecution {
            documents: Some(json!({"sqlite": "sqlite-doc", "xlsx": null})),
            run_reason: Some("RECOUNT".into()),
            ..execution(&w, 20, &w.id(10))
        }],
    )
    .await
    .unwrap();

    assert_eq!(
        (&inserted[0].documents, &inserted[0].run_reason),
        (&None, &None)
    );
    let row = stored(&tx, "tally_session_execution", &w.id(20)).await;
    assert_eq!(
        (&row["documents"], &row["run_reason"]),
        (&Value::Null, &Value::Null)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_session_executions_stores_a_missing_created_at_as_null() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;

    let inserted = tally_session_execution::insert_many_tally_session_executions(
        &tx,
        vec![TallySessionExecution {
            created_at: None,
            ..execution(&w, 20, &w.id(10))
        }],
    )
    .await
    .unwrap();

    assert_eq!(inserted[0].created_at, None);
    let row = tx
        .query_one(
            "SELECT created_at IS NULL FROM sequent_backend.tally_session_execution
             WHERE id = $1::text::uuid",
            &[&w.id(20)],
        )
        .await
        .unwrap();
    assert!(row.get::<_, bool>(0));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_many_tally_session_executions_with_no_executions_returns_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();

    let inserted = tally_session_execution::insert_many_tally_session_executions(&tx, vec![])
        .await
        .unwrap();

    assert!(inserted.is_empty());
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_session_execution_documents_replaces_the_documents_of_the_target_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    tally_session_row(&tx, &w.tenant, &w.event, &w.id(10), H10).await;
    tally_session_row(&tx, &w.tenant, &w.other_event, &w.id(10), H10).await;
    // Executions are keyed by (id, tenant, event): the same id may repeat.
    for event_id in [&w.event, &w.other_event] {
        execution_row(&tx, &w.tenant, event_id, &w.id(10), &w.id(20), None).await;
    }
    execution_row(&tx, &w.tenant, &w.event, &w.id(10), &w.id(21), None).await;
    for n in [20, 21] {
        set(
            &tx,
            "tally_session_execution",
            &w.id(n),
            "documents = '{\"sqlite\": \"old\", \"xlsx\": \"old\"}'",
        )
        .await;
    }

    tally_session_execution::update_tally_session_execution_documents(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(20),
        TallySessionDocuments {
            sqlite: Some("sqlite-doc".into()),
            xlsx: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(
        stored_in(&tx, "tally_session_execution", &w.event, &w.id(20)).await["documents"],
        json!({"sqlite": "sqlite-doc", "xlsx": null})
    );
    for (event_id, id) in [(&w.other_event, w.id(20)), (&w.event, w.id(21))] {
        assert_eq!(
            stored_in(&tx, "tally_session_execution", event_id, &id).await["documents"],
            json!({"sqlite": "old", "xlsx": "old"})
        );
    }
    tx.rollback().await.unwrap();
}
