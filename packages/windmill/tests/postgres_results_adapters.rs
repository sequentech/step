// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The results event, election and election area adapters and the tally
//! session resolution adapter under `postgres::*` against the migrated schema.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local, Utc};
use deadpool_postgres::Transaction;
use ordered_float::NotNan;
use sequent_core::types::ceremonies::{
    TallySessionResolutionData, TallySessionResolutionStatus, TallySessionResolutionType,
    TieBreakingMethod,
};
use sequent_core::types::results::{
    ResultDocuments, ResultsElection, ResultsElectionArea, ResultsEvent,
};
use serde_json::{json, Value};
use tokio_postgres::error::SqlState;
use tokio_postgres::types::WasNull;
use uuid::Uuid;
use windmill::postgres::results_election::{
    get_election_results, get_event_results_election, get_results_election_by_results_event_id,
    insert_many_results_elections, insert_results_elections, update_results_election_documents,
};
use windmill::postgres::results_election_area::{
    get_event_results_election_area, insert_many_results_elections_areas,
    insert_results_election_area_documents,
};
use windmill::postgres::results_event::{
    get_results_event_by_event_id, get_results_event_by_id, insert_many_results_events,
    insert_results_event, update_results_event_documents,
};
use windmill::postgres::tally_session_resolution::{
    create_tally_session_resolution, get_pending_resolutions, get_resolution_by_tally_session,
    submit_resolution, update_resolution,
};

const TENANT: &str = "10000000-0000-4000-8000-000000000001";
const OTHER_TENANT: &str = "10000000-0000-4000-8000-000000000002";
const EVENT: &str = "20000000-0000-4000-8000-000000000001";
const OTHER_EVENT: &str = "20000000-0000-4000-8000-000000000002";
const ELECTION: &str = "30000000-0000-4000-8000-000000000001";
const OTHER_ELECTION: &str = "30000000-0000-4000-8000-000000000002";
const CONTEST: &str = "40000000-0000-4000-8000-000000000001";
const AREA: &str = "50000000-0000-4000-8000-000000000001";
const OTHER_AREA: &str = "50000000-0000-4000-8000-000000000002";
const KEYS_CEREMONY: &str = "60000000-0000-4000-8000-000000000001";
const SESSION: &str = "60000000-0000-4000-8000-000000000011";
const OTHER_SESSION: &str = "60000000-0000-4000-8000-000000000012";
const RESULTS: &str = "70000000-0000-4000-8000-000000000001";
const OTHER_RESULTS: &str = "70000000-0000-4000-8000-000000000002";
// Not created by `world`.
const NEW_RESULTS: &str = "70000000-0000-4000-8000-000000000003";
const USER: &str = "80000000-0000-4000-8000-000000000001";
const OTHER_USER: &str = "80000000-0000-4000-8000-000000000002";
const CANDIDATE_A: &str = "a0000000-0000-4000-8000-000000000001";
const CANDIDATE_B: &str = "a0000000-0000-4000-8000-000000000002";
const ROW_1: &str = "90000000-0000-4000-8000-000000000001";
const ROW_2: &str = "90000000-0000-4000-8000-000000000002";
const ROW_3: &str = "90000000-0000-4000-8000-000000000003";
const ROW_4: &str = "90000000-0000-4000-8000-000000000004";
const ROW_5: &str = "90000000-0000-4000-8000-000000000005";

fn uuid(id: &str) -> Uuid {
    Uuid::parse_str(id).unwrap()
}

fn at(timestamp: &str) -> DateTime<Utc> {
    timestamp.parse().unwrap()
}

fn local(timestamp: &str) -> DateTime<Local> {
    at(timestamp).with_timezone(&Local)
}

fn pct(value: f64) -> Option<NotNan<f64>> {
    Some(NotNan::new(value).unwrap())
}

fn documents() -> ResultDocuments {
    ResultDocuments {
        json: Some("results.json".to_string()),
        html: Some("results.html".to_string()),
        ..ResultDocuments::default()
    }
}

fn documents_json() -> Value {
    serde_json::to_value(documents()).unwrap()
}

fn db_state(error: &anyhow::Error) -> Option<&SqlState> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<tokio_postgres::Error>())
        .and_then(tokio_postgres::Error::code)
}

async fn now<T: tokio_postgres::types::FromSqlOwned>(tx: &Transaction<'_>) -> T {
    tx.query_one("SELECT now()", &[]).await.unwrap().get(0)
}

async fn count(tx: &Transaction<'_>, table: &str) -> i64 {
    tx.query_one(
        &format!("SELECT count(*) FROM sequent_backend.{table}"),
        &[],
    )
    .await
    .unwrap()
    .get(0)
}

async fn tenant(tx: &Transaction<'_>, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&uuid(id), &format!("tenant-{id}")],
    )
    .await
    .unwrap();
}

async fn election_event(tx: &Transaction<'_>, tenant: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&uuid(id), &uuid(tenant)],
    )
    .await
    .unwrap();
}

async fn results_event(tx: &Transaction<'_>, tenant: &str, event: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.results_event (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&uuid(id), &uuid(tenant), &uuid(event)],
    )
    .await
    .unwrap();
}

/// The elections, contest, areas, tally sessions and results events the
/// constants name, in the given tenant and election event.
async fn world(tx: &Transaction<'_>, tenant: &str, event: &str) {
    let (tenant_id, event_id) = (uuid(tenant), uuid(event));
    for id in [ELECTION, OTHER_ELECTION] {
        tx.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&uuid(id), &tenant_id, &event_id],
        )
        .await
        .unwrap();
    }
    tx.execute(
        "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
         VALUES ($1, $2, $3, $4)",
        &[&uuid(CONTEST), &tenant_id, &event_id, &uuid(ELECTION)],
    )
    .await
    .unwrap();
    for id in [AREA, OTHER_AREA] {
        tx.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&uuid(id), &tenant_id, &event_id],
        )
        .await
        .unwrap();
    }
    tx.execute(
        "INSERT INTO sequent_backend.keys_ceremony
             (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, '{}', 1)",
        &[&uuid(KEYS_CEREMONY), &tenant_id, &event_id],
    )
    .await
    .unwrap();
    for id in [SESSION, OTHER_SESSION] {
        tx.execute(
            "INSERT INTO sequent_backend.tally_session
                 (id, tenant_id, election_event_id, keys_ceremony_id, threshold, election_ids)
             VALUES ($1, $2, $3, $4, 1, $5)",
            &[
                &uuid(id),
                &tenant_id,
                &event_id,
                &uuid(KEYS_CEREMONY),
                &vec![uuid(ELECTION)],
            ],
        )
        .await
        .unwrap();
    }
    for id in [RESULTS, OTHER_RESULTS] {
        results_event(tx, tenant, event, id).await;
    }
}

/// TENANT's EVENT, which every test builds first.
async fn home(tx: &Transaction<'_>) {
    tenant(tx, TENANT).await;
    election_event(tx, TENANT, EVENT).await;
    world(tx, TENANT, EVENT).await;
}

/// The same identifiers in TENANT's OTHER_EVENT.
async fn away(tx: &Transaction<'_>) {
    election_event(tx, TENANT, OTHER_EVENT).await;
    world(tx, TENANT, OTHER_EVENT).await;
}

/// The same identifiers recorded under OTHER_TENANT for EVENT.
async fn stranger(tx: &Transaction<'_>) {
    tenant(tx, OTHER_TENANT).await;
    world(tx, OTHER_TENANT, EVENT).await;
}

/// A stored election or election area result row.
#[derive(Clone, Copy)]
struct Stored {
    id: &'static str,
    tenant: &'static str,
    event: &'static str,
    results: &'static str,
    election: &'static str,
    area: &'static str,
    created_at: &'static str,
}

const STORED: Stored = Stored {
    id: ROW_1,
    tenant: TENANT,
    event: EVENT,
    results: RESULTS,
    election: ELECTION,
    area: AREA,
    created_at: "2026-01-01T00:00:00Z",
};

async fn store_election_result(tx: &Transaction<'_>, row: Stored, annotations: Option<Value>) {
    tx.execute(
        "INSERT INTO sequent_backend.results_election (
             id, tenant_id, election_event_id, results_event_id, election_id,
             created_at, total_voters_percent, annotations
         )
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7)",
        &[
            &uuid(row.id),
            &uuid(row.tenant),
            &uuid(row.event),
            &uuid(row.results),
            &uuid(row.election),
            &at(row.created_at),
            &annotations,
        ],
    )
    .await
    .unwrap();
}

async fn store_election_area_result(tx: &Transaction<'_>, row: Stored) {
    tx.execute(
        "INSERT INTO sequent_backend.results_election_area (
             id, tenant_id, election_event_id, results_event_id, election_id,
             area_id, created_at, last_updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $7)",
        &[
            &uuid(row.id),
            &uuid(row.tenant),
            &uuid(row.event),
            &uuid(row.results),
            &uuid(row.election),
            &uuid(row.area),
            &at(row.created_at),
        ],
    )
    .await
    .unwrap();
}

/// Documents and annotations of every results_election row, by id.
async fn election_documents(tx: &Transaction<'_>) -> Vec<(String, Option<Value>, Option<Value>)> {
    tx.query(
        "SELECT id::text, documents, annotations FROM sequent_backend.results_election
         ORDER BY id",
        &[],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| (row.get(0), row.get(1), row.get(2)))
    .collect()
}

/// A tally's election result. The tenant, event, results event and row id
/// are placeholders `insert_results_elections` replaces.
fn election_result(election: &str) -> ResultsElection {
    ResultsElection {
        id: "ignored".to_string(),
        tenant_id: "ignored".to_string(),
        election_event_id: "ignored".to_string(),
        election_id: election.to_string(),
        results_event_id: "ignored".to_string(),
        name: Some("General".to_string()),
        elegible_census: Some(200),
        total_voters: Some(150),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"origin": "tally"})),
        annotations: Some(json!({"results_hash": "hash-0"})),
        total_voters_percent: pct(0.75),
        documents: Some(documents()),
        blank_ballots: Some(12),
        blank_ballots_percent: pct(0.0625),
    }
}

fn election_area_result(area: &str) -> ResultsElectionArea {
    ResultsElectionArea {
        id: ROW_1.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        election_id: ELECTION.to_string(),
        area_id: area.to_string(),
        results_event_id: RESULTS.to_string(),
        created_at: Some(local("2026-01-01T00:00:00Z")),
        last_updated_at: Some(local("2026-01-02T00:00:00Z")),
        documents: Some(documents()),
        name: Some("North".to_string()),
        blank_ballots: Some(12),
        blank_ballots_percent: pct(0.375),
    }
}

#[tokio::test]
async fn a_new_results_event_is_stored_unnamed_and_without_documents() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let inserted = insert_results_event(&transaction, TENANT, EVENT, NEW_RESULTS)
        .await
        .unwrap();

    let now: DateTime<Local> = now(&transaction).await;
    assert_eq!(
        inserted,
        ResultsEvent {
            id: NEW_RESULTS.to_string(),
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            name: None,
            created_at: Some(now),
            last_updated_at: Some(now),
            labels: None,
            annotations: None,
            documents: None,
        }
    );
    let stored: i64 = transaction
        .query_one(
            "SELECT count(*) FROM sequent_backend.results_event
             WHERE id = $1 AND tenant_id = $2 AND election_event_id = $3",
            &[&uuid(NEW_RESULTS), &uuid(TENANT), &uuid(EVENT)],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(stored, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_results_event_already_in_the_event_fails_with_a_generic_insert_error() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = insert_results_event(&transaction, TENANT, EVENT, RESULTS)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Error inserting row: db error");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_new_results_event_needs_a_v4_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let error = insert_results_event(&transaction, TENANT, EVENT, "results-1")
        .await
        .unwrap_err()
        .to_string();
    assert!(error.starts_with("invalid UUID 'results-1'"), "{error}");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_results_event_is_read_by_id_with_its_documents() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    transaction
        .execute(
            "INSERT INTO sequent_backend.results_event (
                 id, tenant_id, election_event_id, name, created_at, last_updated_at,
                 labels, annotations, documents
             )
             VALUES ($1, $2, $3, 'First count', $4, $5, $6, $7, $8)",
            &[
                &uuid(NEW_RESULTS),
                &uuid(TENANT),
                &uuid(EVENT),
                &at("2026-01-01T00:00:00Z"),
                &at("2026-01-02T00:00:00Z"),
                &json!({"origin": "tally"}),
                &json!({"note": "recount"}),
                &documents_json(),
            ],
        )
        .await
        .unwrap();

    let results_event = get_results_event_by_id(&transaction, TENANT, EVENT, NEW_RESULTS)
        .await
        .unwrap();
    assert_eq!(
        results_event,
        ResultsEvent {
            id: NEW_RESULTS.to_string(),
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            name: Some("First count".to_string()),
            created_at: Some(local("2026-01-01T00:00:00Z")),
            last_updated_at: Some(local("2026-01-02T00:00:00Z")),
            labels: Some(json!({"origin": "tally"})),
            annotations: Some(json!({"note": "recount"})),
            documents: Some(documents()),
        }
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_results_event_of_another_event_or_tenant_is_not_found() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    results_event(&transaction, TENANT, OTHER_EVENT, NEW_RESULTS).await;
    results_event(&transaction, OTHER_TENANT, EVENT, NEW_RESULTS).await;

    let error = get_results_event_by_id(&transaction, TENANT, EVENT, NEW_RESULTS)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("Results event {NEW_RESULTS} not found")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn results_event_documents_that_are_not_result_documents_cannot_be_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    transaction
        .execute(
            "UPDATE sequent_backend.results_event SET documents = $1 WHERE id = $2",
            &[&json!({"json": 5}), &uuid(RESULTS)],
        )
        .await
        .unwrap();

    let error = get_results_event_by_id(&transaction, TENANT, EVENT, RESULTS)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        error.starts_with("json: invalid type: integer `5`"),
        "{error}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_results_events_of_an_event_are_those_of_the_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;

    let results_events = get_results_event_by_event_id(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    let mut listed: Vec<(&str, &str, &str)> = results_events
        .iter()
        .map(|r| {
            (
                r.id.as_str(),
                r.tenant_id.as_str(),
                r.election_event_id.as_str(),
            )
        })
        .collect();
    listed.sort();
    assert_eq!(
        listed,
        [(RESULTS, TENANT, EVENT), (OTHER_RESULTS, TENANT, EVENT)]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn results_event_documents_are_replaced_on_that_results_event_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;

    update_results_event_documents(&transaction, TENANT, RESULTS, EVENT, &documents())
        .await
        .unwrap();
    let stored: Vec<(String, String, String, Option<Value>)> = transaction
        .query(
            "SELECT tenant_id::text, election_event_id::text, id::text, documents
             FROM sequent_backend.results_event ORDER BY tenant_id, election_event_id, id",
            &[],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1), row.get(2), row.get(3)))
        .collect();
    let row = |tenant: &str, event: &str, id: &str, documents: Option<Value>| {
        (
            tenant.to_string(),
            event.to_string(),
            id.to_string(),
            documents,
        )
    };
    assert_eq!(
        stored,
        [
            row(TENANT, EVENT, RESULTS, Some(documents_json())),
            row(TENANT, EVENT, OTHER_RESULTS, None),
            row(TENANT, OTHER_EVENT, RESULTS, None),
            row(TENANT, OTHER_EVENT, OTHER_RESULTS, None),
            row(OTHER_TENANT, EVENT, RESULTS, None),
            row(OTHER_TENANT, EVENT, OTHER_RESULTS, None),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn replacing_the_documents_of_an_unknown_results_event_fails() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error =
        update_results_event_documents(&transaction, TENANT, NEW_RESULTS, EVENT, &documents())
            .await
            .unwrap_err();
    assert_eq!(error.to_string(), "Rows not found in table results_event");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_results_events_keep_every_given_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = ResultsEvent {
        id: NEW_RESULTS.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        name: Some("First count".to_string()),
        created_at: Some(local("2026-01-01T00:00:00Z")),
        last_updated_at: Some(local("2026-01-02T00:00:00Z")),
        labels: Some(json!({"origin": "import"})),
        annotations: Some(json!({"note": "recount"})),
        documents: Some(documents()),
    };

    let copied = insert_many_results_events(&transaction, vec![record.clone()])
        .await
        .unwrap();
    assert_eq!(copied, [record]);
    let row = transaction
        .query_one(
            "SELECT name, labels, documents, created_at FROM sequent_backend.results_event
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&uuid(TENANT), &uuid(EVENT), &uuid(NEW_RESULTS)],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "First count");
    assert_eq!(row.get::<_, Value>(1), json!({"origin": "import"}));
    assert_eq!(row.get::<_, Value>(2), documents_json());
    assert_eq!(row.get::<_, DateTime<Utc>>(3), at("2026-01-01T00:00:00Z"));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_empty_results_event_batch_is_accepted_without_a_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let copied = insert_many_results_events(&transaction, vec![])
        .await
        .unwrap();
    assert!(copied.is_empty());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn inserted_election_results_are_stored_under_the_given_results_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let undetermined_blank_ballots = ResultsElection {
        blank_ballots: None,
        blank_ballots_percent: None,
        ..election_result(OTHER_ELECTION)
    };

    let mut inserted = insert_results_elections(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![
            election_result(ELECTION),
            undetermined_blank_ballots.clone(),
        ],
    )
    .await
    .unwrap();

    inserted.sort_by(|a, b| a.election_id.cmp(&b.election_id));
    let now: DateTime<Local> = now(&transaction).await;
    let expected: Vec<ResultsElection> = inserted
        .iter()
        .zip([election_result(ELECTION), undetermined_blank_ballots])
        .map(|(inserted, input)| ResultsElection {
            id: inserted.id.clone(),
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            results_event_id: RESULTS.to_string(),
            created_at: Some(now),
            last_updated_at: Some(now),
            labels: None,
            annotations: None,
            documents: None,
            ..input
        })
        .collect();
    assert_eq!(inserted, expected);
    let stored: Vec<Value> = transaction
        .query(
            "SELECT jsonb_build_object(
                        'tenant_id', tenant_id, 'election_id', election_id,
                        'blank_ballots', blank_ballots,
                        'blank_ballots_percent', blank_ballots_percent,
                        'documents', documents, 'labels', labels, 'annotations', annotations)
             FROM sequent_backend.results_election WHERE results_event_id = $1
             ORDER BY election_id",
            &[&uuid(RESULTS)],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    let row = |election: &str, blank_ballots: Value, blank_ballots_percent: Value| {
        json!({
            "tenant_id": TENANT,
            "election_id": election,
            "blank_ballots": blank_ballots,
            "blank_ballots_percent": blank_ballots_percent,
            "documents": null,
            "labels": null,
            "annotations": null,
        })
    };
    assert_eq!(
        stored,
        [
            row(ELECTION, json!(12), json!(0.0625)),
            row(OTHER_ELECTION, Value::Null, Value::Null),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_election_result_without_a_voter_percentage_is_stored_but_cannot_be_read_back() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = insert_results_elections(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![ResultsElection {
            total_voters_percent: None,
            ..election_result(ELECTION)
        }],
    )
    .await
    .unwrap_err();
    assert!(
        error.chain().any(|cause| cause.is::<WasNull>()),
        "{error:?}"
    );
    assert_eq!(count(&transaction, "results_election").await, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn empty_election_result_batches_are_accepted_without_a_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let inserted =
        insert_results_elections(&transaction, "tenant-1", "event-1", "results-1", vec![])
            .await
            .unwrap();
    let copied = insert_many_results_elections(&transaction, vec![])
        .await
        .unwrap();
    assert!(inserted.is_empty() && copied.is_empty());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn election_result_documents_and_hash_are_recorded_keeping_other_annotations() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    let origin = Some(json!({"origin": "tally"}));
    store_election_result(&transaction, STORED, origin.clone()).await;
    for row in [
        Stored {
            id: ROW_2,
            results: OTHER_RESULTS,
            ..STORED
        },
        Stored {
            id: ROW_3,
            election: OTHER_ELECTION,
            ..STORED
        },
        Stored {
            id: ROW_4,
            event: OTHER_EVENT,
            ..STORED
        },
        Stored {
            id: ROW_5,
            tenant: OTHER_TENANT,
            ..STORED
        },
    ] {
        store_election_result(&transaction, row, origin.clone()).await;
    }

    update_results_election_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        &documents(),
        "hash-1",
    )
    .await
    .unwrap();
    let hashed = Some(json!({"origin": "tally", "results_hash": "hash-1"}));
    let untouched = |id: &str| (id.to_string(), None, origin.clone());
    assert_eq!(
        election_documents(&transaction).await,
        [
            (ROW_1.to_string(), Some(documents_json()), hashed),
            untouched(ROW_2),
            untouched(ROW_3),
            untouched(ROW_4),
            untouched(ROW_5),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_election_result_without_annotations_gets_just_the_results_hash() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_election_result(&transaction, STORED, None).await;

    update_results_election_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        &documents(),
        "hash-1",
    )
    .await
    .unwrap();
    assert_eq!(
        election_documents(&transaction).await,
        [(
            ROW_1.to_string(),
            Some(documents_json()),
            Some(json!({"results_hash": "hash-1"}))
        )]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn recording_documents_of_a_missing_election_result_fails() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = update_results_election_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        &documents(),
        "hash-1",
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Rows not found in table results_election"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn recording_documents_of_a_repeated_election_result_updates_every_copy_and_fails() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_election_result(&transaction, STORED, None).await;
    store_election_result(
        &transaction,
        Stored {
            id: ROW_2,
            ..STORED
        },
        None,
    )
    .await;

    let error = update_results_election_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        &documents(),
        "hash-1",
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Too many affected rows in table results_election: 2"
    );
    let hashed = |id: &str| {
        (
            id.to_string(),
            Some(documents_json()),
            Some(json!({"results_hash": "hash-1"})),
        )
    };
    assert_eq!(
        election_documents(&transaction).await,
        [hashed(ROW_1), hashed(ROW_2)]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn recording_election_result_documents_needs_a_v4_results_event_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let error = update_results_election_documents(
        &transaction,
        TENANT,
        "results-1",
        EVENT,
        ELECTION,
        &documents(),
        "hash-1",
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(
        error.starts_with("Error parsing results_event_id as UUID: invalid UUID 'results-1'"),
        "{error}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_results_of_an_election_are_listed_newest_first() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_election_result(&transaction, STORED, None).await;
    for row in [
        Stored {
            id: ROW_2,
            results: OTHER_RESULTS,
            created_at: "2026-02-01T00:00:00Z",
            ..STORED
        },
        Stored {
            id: ROW_3,
            election: OTHER_ELECTION,
            created_at: "2026-03-01T00:00:00Z",
            ..STORED
        },
        Stored {
            id: ROW_4,
            event: OTHER_EVENT,
            created_at: "2026-04-01T00:00:00Z",
            ..STORED
        },
        Stored {
            id: ROW_5,
            tenant: OTHER_TENANT,
            created_at: "2026-05-01T00:00:00Z",
            ..STORED
        },
    ] {
        store_election_result(&transaction, row, None).await;
    }

    let results = get_election_results(&transaction, TENANT, EVENT, ELECTION)
        .await
        .unwrap();
    let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, [ROW_2, ROW_1]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_newest_election_result_of_a_results_event_is_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_election_result(&transaction, STORED, None).await;
    store_election_result(
        &transaction,
        Stored {
            id: ROW_2,
            created_at: "2026-02-01T00:00:00Z",
            ..STORED
        },
        None,
    )
    .await;
    store_election_result(
        &transaction,
        Stored {
            id: ROW_3,
            results: OTHER_RESULTS,
            created_at: "2026-03-01T00:00:00Z",
            ..STORED
        },
        None,
    )
    .await;

    let result = get_results_election_by_results_event_id(&transaction, TENANT, ELECTION, RESULTS)
        .await
        .unwrap();
    assert_eq!(result.id, ROW_2);
    assert_eq!(result.total_voters_percent, pct(0.0));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_election_result_is_read_by_results_event_whatever_its_election_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    store_election_result(
        &transaction,
        Stored {
            event: OTHER_EVENT,
            ..STORED
        },
        None,
    )
    .await;

    let result = get_results_election_by_results_event_id(&transaction, TENANT, ELECTION, RESULTS)
        .await
        .unwrap();
    assert_eq!(result.id, ROW_1);
    assert_eq!(result.election_event_id, OTHER_EVENT);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_election_result_of_another_tenant_is_not_found() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    stranger(&transaction).await;
    store_election_result(
        &transaction,
        Stored {
            tenant: OTHER_TENANT,
            ..STORED
        },
        None,
    )
    .await;

    let error = get_results_election_by_results_event_id(&transaction, TENANT, ELECTION, RESULTS)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Results election not found");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn reading_an_election_result_needs_a_v4_results_event_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let error =
        get_results_election_by_results_event_id(&transaction, TENANT, ELECTION, "results-1")
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.starts_with("Error parsing results_event_id as UUID: invalid UUID 'results-1'"),
        "{error}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn event_election_results_are_those_of_the_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_election_result(&transaction, STORED, None).await;
    for row in [
        Stored {
            id: ROW_2,
            election: OTHER_ELECTION,
            ..STORED
        },
        Stored {
            id: ROW_3,
            event: OTHER_EVENT,
            ..STORED
        },
        Stored {
            id: ROW_4,
            tenant: OTHER_TENANT,
            ..STORED
        },
    ] {
        store_election_result(&transaction, row, None).await;
    }

    let results = get_event_results_election(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    let mut ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, [ROW_1, ROW_2]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_election_results_keep_every_given_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = ResultsElection {
        id: ROW_1.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        results_event_id: RESULTS.to_string(),
        created_at: Some(local("2026-01-01T00:00:00Z")),
        last_updated_at: Some(local("2026-01-02T00:00:00Z")),
        ..election_result(ELECTION)
    };

    let copied = insert_many_results_elections(&transaction, vec![record.clone()])
        .await
        .unwrap();
    assert_eq!(copied, [record]);
    let row = transaction
        .query_one(
            "SELECT total_voters, total_voters_percent::text, annotations, documents
             FROM sequent_backend.results_election WHERE id = $1",
            &[&uuid(ROW_1)],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, Option<i32>>(0), Some(150));
    assert_eq!(row.get::<_, String>(1), "0.75");
    assert_eq!(row.get::<_, Value>(2), json!({"results_hash": "hash-0"}));
    assert_eq!(row.get::<_, Value>(3), documents_json());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn election_area_documents_are_stored_as_a_new_named_area_result() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    insert_results_election_area_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        AREA,
        "North",
        &documents(),
        Some(12),
        Some(0.375),
    )
    .await
    .unwrap();
    let rows = transaction
        .query(
            "SELECT tenant_id, election_event_id, results_event_id, election_id, area_id,
                    name, documents, blank_ballots, blank_ballots_percent::text,
                    created_at = now()
             FROM sequent_backend.results_election_area",
            &[],
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    for (column, id) in [TENANT, EVENT, RESULTS, ELECTION, AREA].iter().enumerate() {
        assert_eq!(row.get::<_, Uuid>(column), uuid(id), "column {column}");
    }
    assert_eq!(row.get::<_, String>(5), "North");
    assert_eq!(row.get::<_, Value>(6), documents_json());
    assert_eq!(row.get::<_, Option<i32>>(7), Some(12));
    assert_eq!(row.get::<_, Option<String>>(8).as_deref(), Some("0.375"));
    assert!(row.get::<_, bool>(9));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_blank_ballots_percentage_is_stored_with_its_binary_expansion() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    insert_results_election_area_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        AREA,
        "North",
        &documents(),
        Some(1),
        Some(0.1),
    )
    .await
    .unwrap();
    let stored: String = transaction
        .query_one(
            "SELECT blank_ballots_percent::text FROM sequent_backend.results_election_area",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(stored, "0.1000000000000000055511151231");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_blank_ballots_percentage_that_is_not_a_number_is_stored_as_null() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    insert_results_election_area_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        AREA,
        "North",
        &documents(),
        Some(0),
        Some(f64::NAN),
    )
    .await
    .unwrap();
    let stored: Option<String> = transaction
        .query_one(
            "SELECT blank_ballots_percent::text FROM sequent_backend.results_election_area",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(stored, None);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn storing_election_area_documents_again_adds_another_area_result() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    for _ in 0..2 {
        insert_results_election_area_documents(
            &transaction,
            TENANT,
            RESULTS,
            EVENT,
            ELECTION,
            AREA,
            "North",
            &documents(),
            None,
            None,
        )
        .await
        .unwrap();
    }
    assert_eq!(count(&transaction, "results_election_area").await, 2);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn election_area_documents_for_an_area_outside_the_event_fail_with_a_generic_error() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = insert_results_election_area_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        "50000000-0000-4000-8000-000000000009",
        "North",
        &documents(),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Error at inser into results_election_area db error "
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn election_area_documents_need_a_v4_area_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let error = insert_results_election_area_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        "area-1",
        "North",
        &documents(),
        None,
        None,
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(
        error.starts_with("Error parsing area_id as UUID: invalid UUID 'area-1'"),
        "{error}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn event_election_area_results_are_those_of_the_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_election_area_result(&transaction, STORED).await;
    for row in [
        Stored {
            id: ROW_2,
            area: OTHER_AREA,
            ..STORED
        },
        Stored {
            id: ROW_3,
            event: OTHER_EVENT,
            ..STORED
        },
        Stored {
            id: ROW_4,
            tenant: OTHER_TENANT,
            ..STORED
        },
    ] {
        store_election_area_result(&transaction, row).await;
    }

    let results = get_event_results_election_area(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    let mut ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, [ROW_1, ROW_2]);
    let first = results.iter().find(|r| r.id == ROW_1).unwrap();
    assert_eq!(first.created_at, Some(local("2026-01-01T00:00:00Z")));
    assert_eq!(
        (first.blank_ballots, first.blank_ballots_percent),
        (None, None)
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_election_area_results_keep_every_given_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = election_area_result(AREA);

    let copied = insert_many_results_elections_areas(&transaction, vec![record.clone()])
        .await
        .unwrap();
    assert_eq!(copied, [record]);
    let row = transaction
        .query_one(
            "SELECT name, blank_ballots_percent::text, documents
             FROM sequent_backend.results_election_area WHERE id = $1",
            &[&uuid(ROW_1)],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, Option<String>>(0).as_deref(), Some("North"));
    assert_eq!(row.get::<_, String>(1), "0.375");
    assert_eq!(row.get::<_, Value>(2), documents_json());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_election_area_results_without_timestamps_are_refused() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = ResultsElectionArea {
        created_at: None,
        ..election_area_result(AREA)
    };

    let error = insert_many_results_elections_areas(&transaction, vec![record])
        .await
        .unwrap_err();
    assert_eq!(
        db_state(&error),
        Some(&SqlState::NOT_NULL_VIOLATION),
        "{error:?}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_empty_election_area_batch_is_accepted_without_a_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let copied = insert_many_results_elections_areas(&transaction, vec![])
        .await
        .unwrap();
    assert!(copied.is_empty());
    transaction.rollback().await.unwrap();
}

fn tie_break(resolved_by_candidate: Option<&str>) -> TallySessionResolutionData {
    TallySessionResolutionData {
        round_number: Some(2),
        tied_candidate_ids: vec![CANDIDATE_A.to_string(), CANDIDATE_B.to_string()],
        vote_count: 10,
        method_used: TieBreakingMethod::ExternalProcedure,
        resolved_by_candidate_id: resolved_by_candidate.map(str::to_string),
    }
}

/// A stored resolution of CONTEST.
#[derive(Clone, Copy)]
struct Resolution {
    id: &'static str,
    tenant: &'static str,
    event: &'static str,
    session: &'static str,
    resolution_type: &'static str,
    status: &'static str,
    created_at: &'static str,
}

const PENDING: Resolution = Resolution {
    id: ROW_1,
    tenant: TENANT,
    event: EVENT,
    session: SESSION,
    resolution_type: "irv_tie_break",
    status: "pending",
    created_at: "2026-01-01T00:00:00Z",
};

async fn store_resolution(tx: &Transaction<'_>, resolution: Resolution) {
    tx.execute(
        "INSERT INTO sequent_backend.tally_session_resolution (
             id, tenant_id, election_event_id, tally_session_id, contest_id,
             resolution_type, status, resolution_data, created_at, last_updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)",
        &[
            &uuid(resolution.id),
            &uuid(resolution.tenant),
            &uuid(resolution.event),
            &uuid(resolution.session),
            &uuid(CONTEST),
            &resolution.resolution_type,
            &resolution.status,
            &serde_json::to_value(tie_break(None)).unwrap(),
            &at(resolution.created_at),
        ],
    )
    .await
    .unwrap();
}

/// Status, decision, resolver, whether this transaction set the resolution
/// time, and the last update time of a stored resolution.
async fn decision(
    tx: &Transaction<'_>,
    id: &str,
) -> (String, Value, Option<Uuid>, Option<bool>, DateTime<Utc>) {
    let row = tx
        .query_one(
            "SELECT status, resolution_data, resolved_by_user, resolved_at = now(),
                    last_updated_at
             FROM sequent_backend.tally_session_resolution WHERE id = $1",
            &[&uuid(id)],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1), row.get(2), row.get(3), row.get(4))
}

fn undecided(status: &str) -> (String, Value, Option<Uuid>, Option<bool>, DateTime<Utc>) {
    (
        status.to_string(),
        serde_json::to_value(tie_break(None)).unwrap(),
        None,
        None,
        at(PENDING.created_at),
    )
}

#[tokio::test]
async fn a_created_resolution_is_pending_with_its_tie_break_data() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let id = create_tally_session_resolution(
        &transaction,
        TENANT,
        EVENT,
        SESSION,
        CONTEST,
        TallySessionResolutionType::IrvTieBreak,
        tie_break(None),
    )
    .await
    .unwrap();
    let row = transaction
        .query_one(
            "SELECT tenant_id, election_event_id, tally_session_id, contest_id,
                    resolution_type, status, resolution_data,
                    resolved_by_user IS NULL AND resolved_at IS NULL
             FROM sequent_backend.tally_session_resolution WHERE id = $1",
            &[&uuid(&id)],
        )
        .await
        .unwrap();
    for (column, expected) in [TENANT, EVENT, SESSION, CONTEST].iter().enumerate() {
        assert_eq!(
            row.get::<_, Uuid>(column),
            uuid(expected),
            "column {column}"
        );
    }
    assert_eq!(row.get::<_, String>(4), "irv_tie_break");
    assert_eq!(row.get::<_, String>(5), "pending");
    assert_eq!(
        row.get::<_, Value>(6),
        json!({
            "round_number": 2,
            "tied_candidate_ids": [CANDIDATE_A, CANDIDATE_B],
            "vote_count": 10,
            "method_used": "ExternalProcedure",
            "resolved_by_candidate_id": null,
        })
    );
    assert!(row.get::<_, bool>(7));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_resolution_for_a_contest_outside_the_event_is_refused() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = create_tally_session_resolution(
        &transaction,
        TENANT,
        EVENT,
        SESSION,
        "40000000-0000-4000-8000-000000000009",
        TallySessionResolutionType::IrvTieBreak,
        tie_break(None),
    )
    .await
    .unwrap_err();
    let violation = error
        .downcast_ref::<tokio_postgres::Error>()
        .and_then(tokio_postgres::Error::as_db_error)
        .unwrap();
    assert_eq!(violation.code(), &SqlState::FOREIGN_KEY_VIOLATION);
    assert_eq!(
        violation.constraint(),
        Some("fk_tally_session_resolution_contest")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_pending_resolutions_of_a_session_are_listed_oldest_first() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    for resolution in [
        Resolution {
            created_at: "2026-01-02T00:00:00Z",
            ..PENDING
        },
        Resolution {
            id: ROW_2,
            ..PENDING
        },
        Resolution {
            id: ROW_3,
            status: "resolved",
            ..PENDING
        },
        Resolution {
            id: ROW_4,
            session: OTHER_SESSION,
            ..PENDING
        },
        Resolution {
            id: ROW_5,
            event: OTHER_EVENT,
            ..PENDING
        },
        Resolution {
            id: "90000000-0000-4000-8000-000000000006",
            tenant: OTHER_TENANT,
            ..PENDING
        },
    ] {
        store_resolution(&transaction, resolution).await;
    }

    let pending = get_pending_resolutions(&transaction, TENANT, EVENT, SESSION)
        .await
        .unwrap();
    let ids: Vec<&str> = pending.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, [ROW_2, ROW_1]);
    assert!(pending
        .iter()
        .all(|r| r.status == TallySessionResolutionStatus::Pending));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn every_resolution_of_a_session_is_listed_oldest_first() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    for resolution in [
        Resolution {
            created_at: "2026-01-03T00:00:00Z",
            ..PENDING
        },
        Resolution {
            id: ROW_2,
            status: "resolved",
            ..PENDING
        },
        Resolution {
            id: ROW_3,
            created_at: "2026-01-02T00:00:00Z",
            ..PENDING
        },
        Resolution {
            id: ROW_4,
            session: OTHER_SESSION,
            ..PENDING
        },
    ] {
        store_resolution(&transaction, resolution).await;
    }

    let resolutions = get_resolution_by_tally_session(&transaction, TENANT, EVENT, SESSION)
        .await
        .unwrap();
    let listed: Vec<(&str, TallySessionResolutionStatus)> = resolutions
        .iter()
        .map(|r| (r.id.as_str(), r.status.clone()))
        .collect();
    assert_eq!(
        listed,
        [
            (ROW_2, TallySessionResolutionStatus::Resolved),
            (ROW_3, TallySessionResolutionStatus::Pending),
            (ROW_1, TallySessionResolutionStatus::Pending),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_listed_resolution_maps_every_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(
        &transaction,
        Resolution {
            status: "resolved",
            ..PENDING
        },
    )
    .await;
    transaction
        .execute(
            "UPDATE sequent_backend.tally_session_resolution
             SET resolution_data = $1, resolved_by_user = $2, resolved_at = $3,
                 last_updated_at = $3, labels = $4, annotations = $5
             WHERE id = $6",
            &[
                &serde_json::to_value(tie_break(Some(CANDIDATE_B))).unwrap(),
                &uuid(USER),
                &at("2026-01-05T00:00:00Z"),
                &json!({"origin": "tally"}),
                &json!({"note": "drawn by lot"}),
                &uuid(ROW_1),
            ],
        )
        .await
        .unwrap();

    let resolutions = get_resolution_by_tally_session(&transaction, TENANT, EVENT, SESSION)
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&resolutions).unwrap(),
        json!([{
            "id": ROW_1,
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "tally_session_id": SESSION,
            "contest_id": CONTEST,
            "created_at": "2026-01-01T00:00:00Z",
            "last_updated_at": "2026-01-05T00:00:00Z",
            "resolution_type": "irv_tie_break",
            "status": "resolved",
            "resolution_data": serde_json::to_value(tie_break(Some(CANDIDATE_B))).unwrap(),
            "resolved_by_user": USER,
            "resolved_at": "2026-01-05T00:00:00Z",
            "labels": {"origin": "tally"},
            "annotations": {"note": "drawn by lot"},
        }])
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_cancelled_resolution_makes_the_session_resolutions_unreadable() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(&transaction, PENDING).await;
    store_resolution(
        &transaction,
        Resolution {
            id: ROW_2,
            status: "cancelled",
            ..PENDING
        },
    )
    .await;

    let error = get_resolution_by_tally_session(&transaction, TENANT, EVENT, SESSION)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown variant `cancelled`"), "{error}");
    let pending = get_pending_resolutions(&transaction, TENANT, EVENT, SESSION)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_resolution_type_without_a_variant_makes_the_pending_resolutions_unreadable() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(
        &transaction,
        Resolution {
            resolution_type: "manual_recount",
            ..PENDING
        },
    )
    .await;

    let error = get_pending_resolutions(&transaction, TENANT, EVENT, SESSION)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unknown variant `manual_recount`"),
        "{error}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_resolution_with_the_default_empty_data_cannot_be_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(&transaction, PENDING).await;
    transaction
        .execute(
            "UPDATE sequent_backend.tally_session_resolution
             SET resolution_data = DEFAULT WHERE id = $1",
            &[&uuid(ROW_1)],
        )
        .await
        .unwrap();

    let error = get_pending_resolutions(&transaction, TENANT, EVENT, SESSION)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("missing field `tied_candidate_ids`"),
        "{error}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn submitting_a_pending_resolution_resolves_it_with_the_decision() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(&transaction, PENDING).await;

    submit_resolution(
        &transaction,
        TENANT,
        EVENT,
        ROW_1,
        tie_break(Some(CANDIDATE_A)),
        USER,
    )
    .await
    .unwrap();
    // last_updated_at keeps the value it had before the decision.
    assert_eq!(
        decision(&transaction, ROW_1).await,
        (
            "resolved".to_string(),
            serde_json::to_value(tie_break(Some(CANDIDATE_A))).unwrap(),
            Some(uuid(USER)),
            Some(true),
            at(PENDING.created_at),
        )
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_resolved_resolution_cannot_be_submitted_again() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(
        &transaction,
        Resolution {
            status: "resolved",
            ..PENDING
        },
    )
    .await;

    let error = submit_resolution(
        &transaction,
        TENANT,
        EVENT,
        ROW_1,
        tie_break(Some(CANDIDATE_A)),
        USER,
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("Resolution not found or already resolved: {ROW_1}")
    );
    assert_eq!(decision(&transaction, ROW_1).await, undecided("resolved"));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_resolution_of_another_event_cannot_be_submitted() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    store_resolution(
        &transaction,
        Resolution {
            event: OTHER_EVENT,
            ..PENDING
        },
    )
    .await;

    let error = submit_resolution(
        &transaction,
        TENANT,
        EVENT,
        ROW_1,
        tie_break(Some(CANDIDATE_A)),
        USER,
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("Resolution not found or already resolved: {ROW_1}")
    );
    assert_eq!(decision(&transaction, ROW_1).await, undecided("pending"));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_resolver_id_of_any_uuid_version_is_accepted() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(&transaction, PENDING).await;
    let version_one = "80000000-0000-1000-8000-000000000001";

    submit_resolution(
        &transaction,
        TENANT,
        EVENT,
        ROW_1,
        tie_break(Some(CANDIDATE_A)),
        version_one,
    )
    .await
    .unwrap();
    assert_eq!(
        decision(&transaction, ROW_1).await.2,
        Some(uuid(version_one))
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn updating_a_resolved_resolution_replaces_its_decision() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(
        &transaction,
        Resolution {
            status: "resolved",
            ..PENDING
        },
    )
    .await;
    transaction
        .execute(
            "UPDATE sequent_backend.tally_session_resolution
             SET resolved_by_user = $1, resolved_at = $2 WHERE id = $3",
            &[&uuid(OTHER_USER), &at("2026-01-05T00:00:00Z"), &uuid(ROW_1)],
        )
        .await
        .unwrap();

    update_resolution(
        &transaction,
        TENANT,
        EVENT,
        ROW_1,
        tie_break(Some(CANDIDATE_B)),
        USER,
    )
    .await
    .unwrap();
    assert_eq!(
        decision(&transaction, ROW_1).await,
        (
            "resolved".to_string(),
            serde_json::to_value(tie_break(Some(CANDIDATE_B))).unwrap(),
            Some(uuid(USER)),
            Some(true),
            at(PENDING.created_at),
        )
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn updating_a_pending_resolution_records_a_decision_but_leaves_it_pending() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_resolution(&transaction, PENDING).await;

    update_resolution(
        &transaction,
        TENANT,
        EVENT,
        ROW_1,
        tie_break(Some(CANDIDATE_B)),
        USER,
    )
    .await
    .unwrap();
    assert_eq!(
        decision(&transaction, ROW_1).await,
        (
            "pending".to_string(),
            serde_json::to_value(tie_break(Some(CANDIDATE_B))).unwrap(),
            Some(uuid(USER)),
            Some(true),
            at(PENDING.created_at),
        )
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn updating_an_unknown_resolution_fails() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = update_resolution(
        &transaction,
        TENANT,
        EVENT,
        ROW_1,
        tie_break(Some(CANDIDATE_B)),
        USER,
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), format!("Resolution not found: {ROW_1}"));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_resolution_of_another_tenant_cannot_be_updated() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    stranger(&transaction).await;
    store_resolution(
        &transaction,
        Resolution {
            tenant: OTHER_TENANT,
            ..PENDING
        },
    )
    .await;

    let error = update_resolution(
        &transaction,
        TENANT,
        EVENT,
        ROW_1,
        tie_break(Some(CANDIDATE_B)),
        USER,
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), format!("Resolution not found: {ROW_1}"));
    assert_eq!(decision(&transaction, ROW_1).await, undecided("pending"));
    transaction.rollback().await.unwrap();
}
