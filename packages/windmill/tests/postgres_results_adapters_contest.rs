// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The contest, area contest and candidate results adapters under
//! `postgres::results_*` against the migrated schema.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local, Utc};
use deadpool_postgres::Transaction;
use ordered_float::NotNan;
use sequent_core::types::results::{
    ResultDocuments, ResultsAreaContest, ResultsAreaContestCandidate, ResultsContest,
    ResultsContestCandidate,
};
use serde_json::{json, Value};
use tokio_postgres::error::SqlState;
use tokio_postgres::types::WasNull;
use uuid::Uuid;
use windmill::postgres::results_area_contest::{
    get_event_results_area_contest, get_results_area_contest, insert_many_results_area_contests,
    insert_results_area_contests, update_results_area_contest_documents,
};
use windmill::postgres::results_area_contest_candidate::{
    get_event_results_area_contest_candidates, get_results_area_contest_candidates,
    insert_many_results_area_contest_candidates, insert_results_area_contest_candidates,
};
use windmill::postgres::results_contest::{
    get_event_results_contest, get_results_contest, insert_many_results_contests,
    insert_results_contests, update_results_contest_documents,
};
use windmill::postgres::results_contest_candidate::{
    get_event_results_contest_candidates, insert_many_results_contest_candidates,
    insert_results_contest_candidates,
};

const TENANT: &str = "10000000-0000-4000-8000-000000000001";
const OTHER_TENANT: &str = "10000000-0000-4000-8000-000000000002";
const EVENT: &str = "20000000-0000-4000-8000-000000000001";
const OTHER_EVENT: &str = "20000000-0000-4000-8000-000000000002";
const ELECTION: &str = "30000000-0000-4000-8000-000000000001";
const OTHER_ELECTION: &str = "30000000-0000-4000-8000-000000000002";
// Both contests belong to ELECTION, both candidates to CONTEST.
const CONTEST: &str = "40000000-0000-4000-8000-000000000001";
const OTHER_CONTEST: &str = "40000000-0000-4000-8000-000000000002";
const AREA: &str = "50000000-0000-4000-8000-000000000001";
const OTHER_AREA: &str = "50000000-0000-4000-8000-000000000002";
const CANDIDATE: &str = "60000000-0000-4000-8000-000000000001";
const OTHER_CANDIDATE: &str = "60000000-0000-4000-8000-000000000002";
const RESULTS: &str = "70000000-0000-4000-8000-000000000001";
const OTHER_RESULTS: &str = "70000000-0000-4000-8000-000000000002";
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
        pdf: Some("results.pdf".to_string()),
        ..ResultDocuments::default()
    }
}

fn is_null_column_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| cause.is::<WasNull>())
}

/// The transaction's timestamp, which column defaults record.
async fn now(tx: &Transaction<'_>) -> DateTime<Local> {
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

/// Every row id of `table` with its documents, by id.
async fn documents_by_row(tx: &Transaction<'_>, table: &str) -> Vec<(String, Option<Value>)> {
    tx.query(
        &format!("SELECT id::text, documents FROM sequent_backend.{table} ORDER BY id"),
        &[],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

/// The stored rows of `table` in `results`, as JSON restricted to the columns
/// `expected` names, ordered by contest, area and candidate.
async fn assert_stored(tx: &Transaction<'_>, table: &str, results: &str, expected: Vec<Value>) {
    let rows: Vec<Value> = tx
        .query(
            &format!(
                "SELECT to_jsonb(r) FROM sequent_backend.{table} r
                 WHERE results_event_id = $1 ORDER BY to_jsonb(r) ->> 'contest_id',
                       to_jsonb(r) ->> 'area_id', to_jsonb(r) ->> 'candidate_id'"
            ),
            &[&uuid(results)],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    let restricted: Vec<Value> = rows
        .iter()
        .zip(&expected)
        .map(|(row, expected)| {
            expected
                .as_object()
                .unwrap()
                .keys()
                .map(|column| {
                    let value = row.get(column).cloned();
                    (column.clone(), value.unwrap_or(json!("<no such column>")))
                })
                .collect()
        })
        .collect();
    assert_eq!(rows.len(), expected.len(), "{rows:#?}");
    assert_eq!(restricted, expected);
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

/// The elections, contests, areas, candidates and results events the
/// constants name, in the given tenant and election event.
async fn world(tx: &Transaction<'_>, tenant: &str, event: &str) {
    let (tenant, event) = (uuid(tenant), uuid(event));
    for id in [ELECTION, OTHER_ELECTION] {
        tx.execute(
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&uuid(id), &tenant, &event],
        )
        .await
        .unwrap();
    }
    for id in [CONTEST, OTHER_CONTEST] {
        tx.execute(
            "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
             VALUES ($1, $2, $3, $4)",
            &[&uuid(id), &tenant, &event, &uuid(ELECTION)],
        )
        .await
        .unwrap();
    }
    for id in [AREA, OTHER_AREA] {
        tx.execute(
            "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&uuid(id), &tenant, &event],
        )
        .await
        .unwrap();
    }
    for id in [CANDIDATE, OTHER_CANDIDATE] {
        tx.execute(
            "INSERT INTO sequent_backend.candidate (id, tenant_id, election_event_id, contest_id)
             VALUES ($1, $2, $3, $4)",
            &[&uuid(id), &tenant, &event, &uuid(CONTEST)],
        )
        .await
        .unwrap();
    }
    for id in [RESULTS, OTHER_RESULTS] {
        tx.execute(
            "INSERT INTO sequent_backend.results_event (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&uuid(id), &tenant, &event],
        )
        .await
        .unwrap();
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

/// A stored result row; each table takes the columns it has, with every
/// percentage the adapters require set to zero.
#[derive(Clone, Copy)]
struct Stored {
    id: &'static str,
    tenant: &'static str,
    event: &'static str,
    results: &'static str,
    election: &'static str,
    contest: &'static str,
    area: &'static str,
    candidate: &'static str,
    created_at: &'static str,
}

const STORED: Stored = Stored {
    id: ROW_1,
    tenant: TENANT,
    event: EVENT,
    results: RESULTS,
    election: ELECTION,
    contest: CONTEST,
    area: AREA,
    candidate: CANDIDATE,
    created_at: "2026-01-01T00:00:00Z",
};

async fn store_contest_result(tx: &Transaction<'_>, row: Stored) {
    tx.execute(
        "INSERT INTO sequent_backend.results_contest (
             id, tenant_id, election_event_id, results_event_id, election_id,
             contest_id, created_at, total_votes_percent, total_auditable_votes_percent,
             total_valid_votes_percent, total_invalid_votes_percent,
             explicit_invalid_votes_percent, implicit_invalid_votes_percent
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, 0, 0, 0, 0, 0, 0)",
        &[
            &uuid(row.id),
            &uuid(row.tenant),
            &uuid(row.event),
            &uuid(row.results),
            &uuid(row.election),
            &uuid(row.contest),
            &at(row.created_at),
        ],
    )
    .await
    .unwrap();
}

async fn store_area_contest_result(tx: &Transaction<'_>, row: Stored) {
    tx.execute(
        "INSERT INTO sequent_backend.results_area_contest (
             id, tenant_id, election_event_id, results_event_id, election_id,
             contest_id, area_id, created_at, total_votes_percent,
             total_auditable_votes_percent, total_valid_votes_percent,
             total_invalid_votes_percent, explicit_invalid_votes_percent,
             implicit_invalid_votes_percent
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0, 0, 0, 0, 0, 0)",
        &[
            &uuid(row.id),
            &uuid(row.tenant),
            &uuid(row.event),
            &uuid(row.results),
            &uuid(row.election),
            &uuid(row.contest),
            &uuid(row.area),
            &at(row.created_at),
        ],
    )
    .await
    .unwrap();
}

async fn store_candidate_result(tx: &Transaction<'_>, row: Stored) {
    tx.execute(
        "INSERT INTO sequent_backend.results_contest_candidate (
             id, tenant_id, election_event_id, results_event_id, election_id,
             contest_id, candidate_id, created_at, cast_votes_percent
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 0)",
        &[
            &uuid(row.id),
            &uuid(row.tenant),
            &uuid(row.event),
            &uuid(row.results),
            &uuid(row.election),
            &uuid(row.contest),
            &uuid(row.candidate),
            &at(row.created_at),
        ],
    )
    .await
    .unwrap();
}

async fn store_area_candidate_result(tx: &Transaction<'_>, row: Stored) {
    tx.execute(
        "INSERT INTO sequent_backend.results_area_contest_candidate (
             id, tenant_id, election_event_id, results_event_id, election_id,
             contest_id, area_id, candidate_id, created_at, cast_votes_percent
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0)",
        &[
            &uuid(row.id),
            &uuid(row.tenant),
            &uuid(row.event),
            &uuid(row.results),
            &uuid(row.election),
            &uuid(row.contest),
            &uuid(row.area),
            &uuid(row.candidate),
            &at(row.created_at),
        ],
    )
    .await
    .unwrap();
}

/// A tally's contest result; every count and percentage differs so a
/// swapped column shows. The tenant, event, results event and row id are
/// placeholders the `insert_results_*` adapters replace.
fn contest_result(contest: &str) -> ResultsContest {
    ResultsContest {
        id: "ignored".to_string(),
        tenant_id: "ignored".to_string(),
        election_event_id: "ignored".to_string(),
        election_id: ELECTION.to_string(),
        contest_id: contest.to_string(),
        results_event_id: "ignored".to_string(),
        elegible_census: Some(200),
        total_valid_votes: Some(75),
        explicit_invalid_votes: Some(12),
        implicit_invalid_votes: Some(13),
        total_blank_votes: Some(19),
        explicit_blank_votes: Some(16),
        implicit_blank_votes: Some(3),
        voting_type: Some("online".to_string()),
        counting_algorithm: Some("plurality-at-large".to_string()),
        name: Some("Mayor".to_string()),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"origin": "tally"})),
        annotations: Some(json!({"extended_metrics": {"under_votes": 2}})),
        total_invalid_votes: Some(25),
        total_invalid_votes_percent: pct(0.25),
        total_valid_votes_percent: pct(0.75),
        explicit_invalid_votes_percent: pct(0.125),
        implicit_invalid_votes_percent: pct(0.0625),
        total_blank_votes_percent: pct(0.1875),
        explicit_blank_votes_percent: pct(0.15625),
        implicit_blank_votes_percent: pct(0.03125),
        total_votes: Some(100),
        total_votes_percent: pct(0.5),
        documents: Some(documents()),
        total_auditable_votes: Some(87),
        total_auditable_votes_percent: pct(0.4375),
    }
}

fn area_contest_result(area: &str) -> ResultsAreaContest {
    ResultsAreaContest {
        id: "ignored".to_string(),
        tenant_id: "ignored".to_string(),
        election_event_id: "ignored".to_string(),
        election_id: ELECTION.to_string(),
        contest_id: CONTEST.to_string(),
        area_id: area.to_string(),
        results_event_id: "ignored".to_string(),
        elegible_census: Some(200),
        total_valid_votes: Some(75),
        explicit_invalid_votes: Some(12),
        implicit_invalid_votes: Some(13),
        total_blank_votes: Some(19),
        explicit_blank_votes: Some(16),
        implicit_blank_votes: Some(3),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"origin": "tally"})),
        annotations: Some(json!({"extended_metrics": {"under_votes": 2}})),
        total_valid_votes_percent: pct(0.75),
        total_invalid_votes: Some(25),
        total_invalid_votes_percent: pct(0.25),
        explicit_invalid_votes_percent: pct(0.125),
        implicit_invalid_votes_percent: pct(0.0625),
        total_blank_votes_percent: pct(0.1875),
        explicit_blank_votes_percent: pct(0.15625),
        implicit_blank_votes_percent: pct(0.03125),
        total_votes: Some(100),
        total_votes_percent: pct(0.5),
        documents: Some(documents()),
        total_auditable_votes: Some(87),
        total_auditable_votes_percent: pct(0.4375),
    }
}

fn candidate_result(candidate: &str) -> ResultsContestCandidate {
    ResultsContestCandidate {
        id: "ignored".to_string(),
        tenant_id: "ignored".to_string(),
        election_event_id: "ignored".to_string(),
        election_id: ELECTION.to_string(),
        contest_id: CONTEST.to_string(),
        candidate_id: candidate.to_string(),
        results_event_id: "ignored".to_string(),
        cast_votes: Some(42),
        winning_position: Some(1),
        points: Some(7),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"origin": "tally"})),
        annotations: Some(json!({"tie": false})),
        cast_votes_percent: pct(0.625),
        documents: Some(documents()),
    }
}

fn area_candidate_result(area: &str) -> ResultsAreaContestCandidate {
    ResultsAreaContestCandidate {
        id: "ignored".to_string(),
        tenant_id: "ignored".to_string(),
        election_event_id: "ignored".to_string(),
        election_id: ELECTION.to_string(),
        contest_id: CONTEST.to_string(),
        area_id: area.to_string(),
        candidate_id: CANDIDATE.to_string(),
        results_event_id: "ignored".to_string(),
        cast_votes: Some(42),
        winning_position: Some(1),
        points: Some(7),
        created_at: None,
        last_updated_at: None,
        labels: Some(json!({"origin": "tally"})),
        annotations: Some(json!({"tie": false})),
        cast_votes_percent: pct(0.625),
        documents: Some(documents()),
    }
}

#[tokio::test]
async fn inserted_contest_results_are_stored_under_the_given_results_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let mut inserted = insert_results_contests(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![contest_result(CONTEST), contest_result(OTHER_CONTEST)],
    )
    .await
    .unwrap();

    inserted.sort_by(|a, b| a.contest_id.cmp(&b.contest_id));
    let now = now(&transaction).await;
    let expected: Vec<ResultsContest> = inserted
        .iter()
        .zip([CONTEST, OTHER_CONTEST])
        .map(|(inserted, contest)| ResultsContest {
            id: inserted.id.clone(),
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            results_event_id: RESULTS.to_string(),
            created_at: Some(now),
            last_updated_at: Some(now),
            labels: None,
            documents: None,
            ..contest_result(contest)
        })
        .collect();
    assert_eq!(inserted, expected);
    let stored = |contest: &str| {
        json!({
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "results_event_id": RESULTS,
            "election_id": ELECTION,
            "contest_id": contest,
            "total_votes": 100,
            "total_votes_percent": 0.5,
            "total_auditable_votes": 87,
            "explicit_blank_votes_percent": 0.15625,
            "counting_algorithm": "plurality-at-large",
            "annotations": {"extended_metrics": {"under_votes": 2}},
            "labels": null,
            "documents": null,
        })
    };
    assert_stored(
        &transaction,
        "results_contest",
        RESULTS,
        vec![stored(CONTEST), stored(OTHER_CONTEST)],
    )
    .await;
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn empty_contest_result_batches_are_accepted_without_a_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let inserted =
        insert_results_contests(&transaction, "tenant-1", "event-1", "results-1", vec![])
            .await
            .unwrap();
    let copied = insert_many_results_contests(&transaction, vec![])
        .await
        .unwrap();
    assert!(inserted.is_empty() && copied.is_empty());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_contest_result_with_an_invalid_election_id_is_not_inserted() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = insert_results_contests(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![
            contest_result(CONTEST),
            ResultsContest {
                election_id: "election-1".to_string(),
                ..contest_result(OTHER_CONTEST)
            },
        ],
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(error.starts_with("invalid UUID 'election-1'"), "{error}");
    assert_eq!(count(&transaction, "results_contest").await, 0);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_contest_result_without_percentages_is_stored_but_cannot_be_read_back() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let without_percentages = ResultsContest {
        total_invalid_votes_percent: None,
        total_valid_votes_percent: None,
        explicit_invalid_votes_percent: None,
        implicit_invalid_votes_percent: None,
        total_blank_votes_percent: None,
        explicit_blank_votes_percent: None,
        implicit_blank_votes_percent: None,
        total_votes_percent: None,
        total_auditable_votes_percent: None,
        ..contest_result(CONTEST)
    };

    let error = insert_results_contests(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![without_percentages],
    )
    .await
    .unwrap_err();
    assert!(is_null_column_error(&error), "{error:?}");
    assert_eq!(count(&transaction, "results_contest").await, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn contest_result_documents_are_replaced_on_the_matching_row_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_contest_result(&transaction, STORED).await;
    for row in [
        Stored {
            id: ROW_2,
            results: OTHER_RESULTS,
            ..STORED
        },
        Stored {
            id: ROW_3,
            contest: OTHER_CONTEST,
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
        store_contest_result(&transaction, row).await;
    }

    update_results_contest_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        CONTEST,
        &documents(),
    )
    .await
    .unwrap();
    let replaced = serde_json::to_value(documents()).unwrap();
    assert_eq!(
        documents_by_row(&transaction, "results_contest").await,
        [
            (ROW_1.to_string(), Some(replaced)),
            (ROW_2.to_string(), None),
            (ROW_3.to_string(), None),
            (ROW_4.to_string(), None),
            (ROW_5.to_string(), None),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn replacing_the_documents_of_a_missing_contest_result_fails() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = update_results_contest_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        CONTEST,
        &documents(),
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "Rows not found in table results_contest");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn replacing_the_documents_of_a_repeated_contest_result_updates_every_copy_and_fails() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_contest_result(&transaction, STORED).await;
    store_contest_result(
        &transaction,
        Stored {
            id: ROW_2,
            ..STORED
        },
    )
    .await;

    let error = update_results_contest_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        CONTEST,
        &documents(),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Too many affected rows in table results_contest: 2"
    );
    let replaced = Some(serde_json::to_value(documents()).unwrap());
    assert_eq!(
        documents_by_row(&transaction, "results_contest").await,
        [
            (ROW_1.to_string(), replaced.clone()),
            (ROW_2.to_string(), replaced),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn reading_one_contest_result_always_fails_because_its_query_does_not_parse() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_contest_result(&transaction, STORED).await;

    let error = get_results_contest(&transaction, TENANT, EVENT, ELECTION, CONTEST)
        .await
        .unwrap_err();
    let code = error
        .downcast_ref::<tokio_postgres::Error>()
        .and_then(tokio_postgres::Error::code);
    assert_eq!(code, Some(&SqlState::SYNTAX_ERROR), "{error:?}");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn event_contest_results_are_those_of_the_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_contest_result(&transaction, STORED).await;
    for row in [
        Stored {
            id: ROW_2,
            results: OTHER_RESULTS,
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
        store_contest_result(&transaction, row).await;
    }

    let results = get_event_results_contest(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    let mut ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, [ROW_1, ROW_2]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn event_contest_results_need_a_v4_tenant_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let error = get_event_results_contest(&transaction, "tenant-1", EVENT)
        .await
        .unwrap_err()
        .to_string();
    assert!(
        error.starts_with("Error parsing tenant_id as UUID: invalid UUID 'tenant-1'"),
        "{error}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_contest_results_keep_every_given_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = ResultsContest {
        id: ROW_1.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        results_event_id: RESULTS.to_string(),
        created_at: Some(local("2026-01-01T00:00:00Z")),
        last_updated_at: Some(local("2026-01-02T00:00:00Z")),
        ..contest_result(CONTEST)
    };

    let copied = insert_many_results_contests(&transaction, vec![record.clone()])
        .await
        .unwrap();
    assert_eq!(copied, [record]);
    assert_stored(
        &transaction,
        "results_contest",
        RESULTS,
        vec![json!({
            "id": ROW_1,
            "tenant_id": TENANT,
            "total_invalid_votes_percent": 0.25,
            "implicit_blank_votes": 3,
            "name": "Mayor",
            "labels": {"origin": "tally"},
            "documents": serde_json::to_value(documents()).unwrap(),
        })],
    )
    .await;
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_contest_results_without_timestamps_store_null_timestamps() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = ResultsContest {
        id: ROW_1.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        results_event_id: RESULTS.to_string(),
        ..contest_result(CONTEST)
    };

    let copied = insert_many_results_contests(&transaction, vec![record])
        .await
        .unwrap();
    assert_eq!(copied[0].created_at, None);
    assert_eq!(copied[0].last_updated_at, None);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn inserted_area_contest_results_are_stored_under_the_given_results_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let mut inserted = insert_results_area_contests(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![area_contest_result(AREA), area_contest_result(OTHER_AREA)],
    )
    .await
    .unwrap();

    inserted.sort_by(|a, b| a.area_id.cmp(&b.area_id));
    let now = now(&transaction).await;
    let expected: Vec<ResultsAreaContest> = inserted
        .iter()
        .zip([AREA, OTHER_AREA])
        .map(|(inserted, area)| ResultsAreaContest {
            id: inserted.id.clone(),
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            results_event_id: RESULTS.to_string(),
            created_at: Some(now),
            last_updated_at: Some(now),
            labels: None,
            documents: None,
            ..area_contest_result(area)
        })
        .collect();
    assert_eq!(inserted, expected);
    let stored = |area: &str| {
        json!({
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "contest_id": CONTEST,
            "area_id": area,
            "total_valid_votes": 75,
            "implicit_invalid_votes_percent": 0.0625,
            "annotations": {"extended_metrics": {"under_votes": 2}},
            "labels": null,
            "documents": null,
        })
    };
    assert_stored(
        &transaction,
        "results_area_contest",
        RESULTS,
        vec![stored(AREA), stored(OTHER_AREA)],
    )
    .await;
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn empty_area_contest_result_batches_are_accepted_without_a_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let inserted =
        insert_results_area_contests(&transaction, "tenant-1", "event-1", "results-1", vec![])
            .await
            .unwrap();
    let copied = insert_many_results_area_contests(&transaction, vec![])
        .await
        .unwrap();
    assert!(inserted.is_empty() && copied.is_empty());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_area_contest_result_without_percentages_is_stored_but_cannot_be_read_back() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let without_percentages = ResultsAreaContest {
        total_valid_votes_percent: None,
        total_invalid_votes_percent: None,
        explicit_invalid_votes_percent: None,
        implicit_invalid_votes_percent: None,
        total_blank_votes_percent: None,
        explicit_blank_votes_percent: None,
        implicit_blank_votes_percent: None,
        total_votes_percent: None,
        total_auditable_votes_percent: None,
        ..area_contest_result(AREA)
    };

    let error = insert_results_area_contests(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![without_percentages],
    )
    .await
    .unwrap_err();
    assert!(is_null_column_error(&error), "{error:?}");
    assert_eq!(count(&transaction, "results_area_contest").await, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn area_contest_result_documents_are_replaced_on_the_matching_row_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_area_contest_result(&transaction, STORED).await;
    for row in [
        Stored {
            id: ROW_2,
            area: OTHER_AREA,
            ..STORED
        },
        Stored {
            id: ROW_3,
            results: OTHER_RESULTS,
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
        store_area_contest_result(&transaction, row).await;
    }

    update_results_area_contest_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        CONTEST,
        AREA,
        &documents(),
    )
    .await
    .unwrap();
    let replaced = serde_json::to_value(documents()).unwrap();
    assert_eq!(
        documents_by_row(&transaction, "results_area_contest").await,
        [
            (ROW_1.to_string(), Some(replaced)),
            (ROW_2.to_string(), None),
            (ROW_3.to_string(), None),
            (ROW_4.to_string(), None),
            (ROW_5.to_string(), None),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn replacing_the_documents_of_a_missing_area_contest_result_fails() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_area_contest_result(
        &transaction,
        Stored {
            area: OTHER_AREA,
            ..STORED
        },
    )
    .await;

    let error = update_results_area_contest_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        CONTEST,
        AREA,
        &documents(),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Rows not found in table results_area_contest"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn replacing_the_documents_of_a_repeated_area_contest_result_updates_every_copy_and_fails() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_area_contest_result(&transaction, STORED).await;
    store_area_contest_result(
        &transaction,
        Stored {
            id: ROW_2,
            ..STORED
        },
    )
    .await;

    let error = update_results_area_contest_documents(
        &transaction,
        TENANT,
        RESULTS,
        EVENT,
        ELECTION,
        CONTEST,
        AREA,
        &documents(),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Too many affected rows in table results_area_contest: 2"
    );
    let replaced = Some(serde_json::to_value(documents()).unwrap());
    assert_eq!(
        documents_by_row(&transaction, "results_area_contest").await,
        [
            (ROW_1.to_string(), replaced.clone()),
            (ROW_2.to_string(), replaced),
        ]
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_newest_area_contest_result_is_read_whatever_its_results_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_area_contest_result(&transaction, STORED).await;
    store_area_contest_result(
        &transaction,
        Stored {
            id: ROW_2,
            results: OTHER_RESULTS,
            created_at: "2026-02-01T00:00:00Z",
            ..STORED
        },
    )
    .await;

    let result =
        get_results_area_contest(&transaction, TENANT, EVENT, ELECTION, Some(CONTEST), AREA)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(result.id, ROW_2);
    assert_eq!(result.results_event_id, OTHER_RESULTS);
    assert_eq!(result.created_at, Some(local("2026-02-01T00:00:00Z")));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn without_a_contest_the_newest_result_of_the_election_area_is_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_area_contest_result(&transaction, STORED).await;
    for row in [
        Stored {
            id: ROW_2,
            contest: OTHER_CONTEST,
            created_at: "2026-02-01T00:00:00Z",
            ..STORED
        },
        Stored {
            id: ROW_3,
            area: OTHER_AREA,
            created_at: "2026-03-01T00:00:00Z",
            ..STORED
        },
        Stored {
            id: ROW_4,
            election: OTHER_ELECTION,
            created_at: "2026-04-01T00:00:00Z",
            ..STORED
        },
    ] {
        store_area_contest_result(&transaction, row).await;
    }

    let result = get_results_area_contest(&transaction, TENANT, EVENT, ELECTION, None, AREA)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.id, ROW_2);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn area_contest_results_of_other_events_and_tenants_are_not_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_area_contest_result(
        &transaction,
        Stored {
            event: OTHER_EVENT,
            ..STORED
        },
    )
    .await;
    store_area_contest_result(
        &transaction,
        Stored {
            id: ROW_2,
            tenant: OTHER_TENANT,
            ..STORED
        },
    )
    .await;

    let result =
        get_results_area_contest(&transaction, TENANT, EVENT, ELECTION, Some(CONTEST), AREA)
            .await
            .unwrap();
    assert!(result.is_none());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn reading_an_area_contest_result_needs_a_v4_contest_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let error = get_results_area_contest(
        &transaction,
        TENANT,
        EVENT,
        ELECTION,
        Some("contest-1"),
        AREA,
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(
        error.starts_with("Error parsing contest_id as UUID: invalid UUID 'contest-1'"),
        "{error}"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn event_area_contest_results_are_those_of_the_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_area_contest_result(&transaction, STORED).await;
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
        store_area_contest_result(&transaction, row).await;
    }

    let results = get_event_results_area_contest(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    let mut ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, [ROW_1, ROW_2]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_area_contest_results_keep_every_given_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = ResultsAreaContest {
        id: ROW_1.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        results_event_id: RESULTS.to_string(),
        created_at: Some(local("2026-01-01T00:00:00Z")),
        last_updated_at: Some(local("2026-01-02T00:00:00Z")),
        ..area_contest_result(AREA)
    };

    let copied = insert_many_results_area_contests(&transaction, vec![record.clone()])
        .await
        .unwrap();
    assert_eq!(copied, [record]);
    assert_stored(
        &transaction,
        "results_area_contest",
        RESULTS,
        vec![json!({
            "id": ROW_1,
            "area_id": AREA,
            "explicit_invalid_votes": 12,
            "total_blank_votes_percent": 0.1875,
            "labels": {"origin": "tally"},
            "documents": serde_json::to_value(documents()).unwrap(),
        })],
    )
    .await;
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn inserted_candidate_results_are_stored_under_the_given_results_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let mut inserted = insert_results_contest_candidates(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![
            candidate_result(CANDIDATE),
            candidate_result(OTHER_CANDIDATE),
        ],
    )
    .await
    .unwrap();

    inserted.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));
    let now = now(&transaction).await;
    let expected: Vec<ResultsContestCandidate> = inserted
        .iter()
        .zip([CANDIDATE, OTHER_CANDIDATE])
        .map(|(inserted, candidate)| ResultsContestCandidate {
            id: inserted.id.clone(),
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            results_event_id: RESULTS.to_string(),
            created_at: Some(now),
            last_updated_at: Some(now),
            labels: None,
            annotations: None,
            documents: None,
            ..candidate_result(candidate)
        })
        .collect();
    assert_eq!(inserted, expected);
    let stored = |candidate: &str| {
        json!({
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "contest_id": CONTEST,
            "candidate_id": candidate,
            "cast_votes": 42,
            "winning_position": 1,
            "points": 7,
            "cast_votes_percent": 0.625,
            "annotations": null,
            "documents": null,
        })
    };
    assert_stored(
        &transaction,
        "results_contest_candidate",
        RESULTS,
        vec![stored(CANDIDATE), stored(OTHER_CANDIDATE)],
    )
    .await;
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn empty_candidate_result_batches_are_accepted_without_a_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let inserted =
        insert_results_contest_candidates(&transaction, "tenant-1", "event-1", "results-1", vec![])
            .await
            .unwrap();
    let copied = insert_many_results_contest_candidates(&transaction, vec![])
        .await
        .unwrap();
    assert!(inserted.is_empty() && copied.is_empty());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_candidate_result_without_a_vote_percentage_is_stored_but_cannot_be_read_back() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = insert_results_contest_candidates(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![ResultsContestCandidate {
            cast_votes_percent: None,
            ..candidate_result(CANDIDATE)
        }],
    )
    .await
    .unwrap_err();
    assert!(is_null_column_error(&error), "{error:?}");
    assert_eq!(count(&transaction, "results_contest_candidate").await, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn event_candidate_results_are_those_of_the_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_candidate_result(&transaction, STORED).await;
    for row in [
        Stored {
            id: ROW_2,
            candidate: OTHER_CANDIDATE,
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
        store_candidate_result(&transaction, row).await;
    }

    let results = get_event_results_contest_candidates(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    let mut ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, [ROW_1, ROW_2]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_candidate_results_keep_every_given_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = ResultsContestCandidate {
        id: ROW_1.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        results_event_id: RESULTS.to_string(),
        created_at: Some(local("2026-01-01T00:00:00Z")),
        last_updated_at: Some(local("2026-01-02T00:00:00Z")),
        ..candidate_result(CANDIDATE)
    };

    let copied = insert_many_results_contest_candidates(&transaction, vec![record.clone()])
        .await
        .unwrap();
    assert_eq!(copied, [record]);
    assert_stored(
        &transaction,
        "results_contest_candidate",
        RESULTS,
        vec![json!({
            "id": ROW_1,
            "candidate_id": CANDIDATE,
            "cast_votes_percent": 0.625,
            "labels": {"origin": "tally"},
            "annotations": {"tie": false},
            "documents": serde_json::to_value(documents()).unwrap(),
        })],
    )
    .await;
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_area_result_of_a_candidate_is_read_by_election_contest_and_candidate() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_area_candidate_result(&transaction, STORED).await;
    store_area_candidate_result(
        &transaction,
        Stored {
            id: ROW_2,
            candidate: OTHER_CANDIDATE,
            ..STORED
        },
    )
    .await;

    let result = get_results_area_contest_candidates(
        &transaction,
        TENANT,
        EVENT,
        ELECTION,
        CONTEST,
        OTHER_CANDIDATE,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.id, ROW_2);
    assert_eq!(result.area_id, AREA);
    assert_eq!(result.cast_votes_percent, pct(0.0));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_candidate_with_results_in_several_areas_reads_as_one_of_them() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store_area_candidate_result(&transaction, STORED).await;
    store_area_candidate_result(
        &transaction,
        Stored {
            id: ROW_2,
            area: OTHER_AREA,
            ..STORED
        },
    )
    .await;

    let result = get_results_area_contest_candidates(
        &transaction,
        TENANT,
        EVENT,
        ELECTION,
        CONTEST,
        CANDIDATE,
    )
    .await
    .unwrap()
    .unwrap();
    assert!([ROW_1, ROW_2].contains(&result.id.as_str()), "{result:?}");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn area_candidate_results_of_other_events_and_tenants_are_not_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_area_candidate_result(
        &transaction,
        Stored {
            event: OTHER_EVENT,
            ..STORED
        },
    )
    .await;
    store_area_candidate_result(
        &transaction,
        Stored {
            id: ROW_2,
            tenant: OTHER_TENANT,
            ..STORED
        },
    )
    .await;

    let result = get_results_area_contest_candidates(
        &transaction,
        TENANT,
        EVENT,
        ELECTION,
        CONTEST,
        CANDIDATE,
    )
    .await
    .unwrap();
    assert!(result.is_none());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn inserted_area_candidate_results_are_stored_under_the_given_results_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let mut inserted = insert_results_area_contest_candidates(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![
            area_candidate_result(AREA),
            area_candidate_result(OTHER_AREA),
        ],
    )
    .await
    .unwrap();

    inserted.sort_by(|a, b| a.area_id.cmp(&b.area_id));
    let now = now(&transaction).await;
    let expected: Vec<ResultsAreaContestCandidate> = inserted
        .iter()
        .zip([AREA, OTHER_AREA])
        .map(|(inserted, area)| ResultsAreaContestCandidate {
            id: inserted.id.clone(),
            tenant_id: TENANT.to_string(),
            election_event_id: EVENT.to_string(),
            results_event_id: RESULTS.to_string(),
            created_at: Some(now),
            last_updated_at: Some(now),
            labels: None,
            annotations: None,
            documents: None,
            ..area_candidate_result(area)
        })
        .collect();
    assert_eq!(inserted, expected);
    let stored = |area: &str| {
        json!({
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "area_id": area,
            "candidate_id": CANDIDATE,
            "cast_votes": 42,
            "points": 7,
            "cast_votes_percent": 0.625,
            "annotations": null,
            "documents": null,
        })
    };
    assert_stored(
        &transaction,
        "results_area_contest_candidate",
        RESULTS,
        vec![stored(AREA), stored(OTHER_AREA)],
    )
    .await;
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn empty_area_candidate_result_batches_are_accepted_without_a_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let inserted = insert_results_area_contest_candidates(
        &transaction,
        "tenant-1",
        "event-1",
        "results-1",
        vec![],
    )
    .await
    .unwrap();
    let copied = insert_many_results_area_contest_candidates(&transaction, vec![])
        .await
        .unwrap();
    assert!(inserted.is_empty() && copied.is_empty());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_area_candidate_result_without_a_vote_percentage_is_stored_but_cannot_be_read_back() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = insert_results_area_contest_candidates(
        &transaction,
        TENANT,
        EVENT,
        RESULTS,
        vec![ResultsAreaContestCandidate {
            cast_votes_percent: None,
            ..area_candidate_result(AREA)
        }],
    )
    .await
    .unwrap_err();
    assert!(is_null_column_error(&error), "{error:?}");
    assert_eq!(
        count(&transaction, "results_area_contest_candidate").await,
        1
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn event_area_candidate_results_are_those_of_the_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store_area_candidate_result(&transaction, STORED).await;
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
        store_area_candidate_result(&transaction, row).await;
    }

    let results = get_event_results_area_contest_candidates(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    let mut ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, [ROW_1, ROW_2]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn copied_area_candidate_results_keep_every_given_column() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let record = ResultsAreaContestCandidate {
        id: ROW_1.to_string(),
        tenant_id: TENANT.to_string(),
        election_event_id: EVENT.to_string(),
        results_event_id: RESULTS.to_string(),
        created_at: Some(local("2026-01-01T00:00:00Z")),
        last_updated_at: Some(local("2026-01-02T00:00:00Z")),
        ..area_candidate_result(AREA)
    };

    let copied = insert_many_results_area_contest_candidates(&transaction, vec![record.clone()])
        .await
        .unwrap();
    assert_eq!(copied, [record]);
    assert_stored(
        &transaction,
        "results_area_contest_candidate",
        RESULTS,
        vec![json!({
            "id": ROW_1,
            "area_id": AREA,
            "winning_position": 1,
            "labels": {"origin": "tally"},
            "annotations": {"tie": false},
            "documents": serde_json::to_value(documents()).unwrap(),
        })],
    )
    .await;
    transaction.rollback().await.unwrap();
}
