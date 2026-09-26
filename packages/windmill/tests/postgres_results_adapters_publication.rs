// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! `postgres::tally_results_publication` against the migrated schema: the
//! rules a publication source must meet, route versions, and the publication
//! lifecycle (Publishing, Published, Failed, Superseded, Revoked).

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ResultsWebsiteAccess, ResultsWebsiteVisibilityScope};
use serde_json::{json, Value};
use tokio_postgres::error::{DbError, SqlState};
use uuid::Uuid;
use windmill::postgres::tally_results_publication::{
    get_active_publication_for_route, get_publication_by_id, get_publication_source_facts,
    insert_publishing_publication, list_active_public_publications, list_superseded_publications,
    mark_publication_failed, mark_publication_published, mark_publication_superseded,
    next_publication_version, revoke_publication, set_publication_finalization_error,
    validate_new_publication_source, NewTallyResultsPublication, PublicationSource,
    PublicationSourceFacts, TallyResultsPublication,
};
use windmill::types::results_publication::ResultsRouteScope;

const TENANT: &str = "10000000-0000-4000-8000-000000000001";
const OTHER_TENANT: &str = "10000000-0000-4000-8000-000000000002";
const EVENT: &str = "20000000-0000-4000-8000-000000000001";
const OTHER_EVENT: &str = "20000000-0000-4000-8000-000000000002";
// ELECTION and OTHER_ELECTION are in SESSION; UNCOVERED_ELECTION is not.
const ELECTION: &str = "30000000-0000-4000-8000-000000000001";
const OTHER_ELECTION: &str = "30000000-0000-4000-8000-000000000002";
const UNCOVERED_ELECTION: &str = "30000000-0000-4000-8000-000000000003";
// In SESSION, but only an election of OTHER_EVENT.
const STRAY_ELECTION: &str = "30000000-0000-4000-8000-000000000004";
// CONTEST and OTHER_CONTEST have results in RESULTS, UNTALLIED_CONTEST only
// in OTHER_RESULTS.
const CONTEST: &str = "40000000-0000-4000-8000-000000000001";
const OTHER_CONTEST: &str = "40000000-0000-4000-8000-000000000002";
const UNCOVERED_CONTEST: &str = "40000000-0000-4000-8000-000000000003";
const UNTALLIED_CONTEST: &str = "40000000-0000-4000-8000-000000000004";
// A contest of ELECTION in OTHER_EVENT only.
const STRAY_CONTEST: &str = "40000000-0000-4000-8000-000000000005";
const KEYS_CEREMONY: &str = "50000000-0000-4000-8000-000000000001";
const SESSION: &str = "60000000-0000-4000-8000-000000000001";
const OTHER_SESSION: &str = "60000000-0000-4000-8000-000000000002";
const RESULTS: &str = "70000000-0000-4000-8000-000000000001";
const OTHER_RESULTS: &str = "70000000-0000-4000-8000-000000000002";
// EXECUTION runs SESSION into RESULTS, OTHER_EXECUTION runs OTHER_SESSION
// into OTHER_RESULTS, and UNLINKED_EXECUTION runs SESSION with no results yet.
const EXECUTION: &str = "80000000-0000-4000-8000-000000000001";
const OTHER_EXECUTION: &str = "80000000-0000-4000-8000-000000000002";
const UNLINKED_EXECUTION: &str = "80000000-0000-4000-8000-000000000003";
const TASK: &str = "90000000-0000-4000-8000-000000000001";
const USER: &str = "a0000000-0000-4000-8000-000000000001";
const P1: &str = "b0000000-0000-4000-8000-000000000001";
const P2: &str = "b0000000-0000-4000-8000-000000000002";
const P3: &str = "b0000000-0000-4000-8000-000000000003";
const P4: &str = "b0000000-0000-4000-8000-000000000004";
const P5: &str = "b0000000-0000-4000-8000-000000000005";
const P6: &str = "b0000000-0000-4000-8000-000000000006";

const NOT_TOGETHER: &str = "The tally session, execution, and results event do not belong together";
const ELECTIONS_OUTSIDE: &str = "One or more publication elections are outside the tally event";
const CONTESTS_OUTSIDE: &str =
    "One or more publication contests are outside the selected elections";
const NOT_TALLIED: &str =
    "Every selected contest must have results in the selected tally execution";

fn uuid(id: &str) -> Uuid {
    Uuid::parse_str(id).unwrap()
}

fn at(timestamp: &str) -> DateTime<Utc> {
    timestamp.parse().unwrap()
}

fn strings(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

fn db_error(error: &anyhow::Error) -> &DbError {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<tokio_postgres::Error>())
        .and_then(tokio_postgres::Error::as_db_error)
        .unwrap_or_else(|| panic!("not a database error: {error:?}"))
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

async fn election(tx: &Transaction<'_>, tenant: &str, event: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
         VALUES ($1, $2, $3)",
        &[&uuid(id), &uuid(tenant), &uuid(event)],
    )
    .await
    .unwrap();
}

async fn contest(tx: &Transaction<'_>, tenant: &str, event: &str, election: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
         VALUES ($1, $2, $3, $4)",
        &[&uuid(id), &uuid(tenant), &uuid(event), &uuid(election)],
    )
    .await
    .unwrap();
}

async fn contest_results(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    results: &str,
    election: &str,
    contest: &str,
) {
    tx.execute(
        "INSERT INTO sequent_backend.results_contest
             (tenant_id, election_event_id, results_event_id, election_id, contest_id)
         VALUES ($1, $2, $3, $4, $5)",
        &[
            &uuid(tenant),
            &uuid(event),
            &uuid(results),
            &uuid(election),
            &uuid(contest),
        ],
    )
    .await
    .unwrap();
}

/// The elections, contests, tally sessions, executions and contest results
/// the constants describe, in the given tenant and election event.
async fn tally(tx: &Transaction<'_>, tenant: &str, event: &str) {
    for id in [ELECTION, OTHER_ELECTION, UNCOVERED_ELECTION] {
        election(tx, tenant, event, id).await;
    }
    for (election, id) in [
        (ELECTION, CONTEST),
        (OTHER_ELECTION, OTHER_CONTEST),
        (UNCOVERED_ELECTION, UNCOVERED_CONTEST),
        (OTHER_ELECTION, UNTALLIED_CONTEST),
    ] {
        contest(tx, tenant, event, election, id).await;
    }
    tx.execute(
        "INSERT INTO sequent_backend.keys_ceremony
             (id, tenant_id, election_event_id, trustee_ids, threshold)
         VALUES ($1, $2, $3, '{}', 1)",
        &[&uuid(KEYS_CEREMONY), &uuid(tenant), &uuid(event)],
    )
    .await
    .unwrap();
    for (id, elections) in [
        (SESSION, vec![ELECTION, OTHER_ELECTION, STRAY_ELECTION]),
        (OTHER_SESSION, vec![ELECTION, OTHER_ELECTION]),
    ] {
        let elections: Vec<Uuid> = elections.into_iter().map(uuid).collect();
        tx.execute(
            "INSERT INTO sequent_backend.tally_session
                 (id, tenant_id, election_event_id, keys_ceremony_id, threshold, election_ids)
             VALUES ($1, $2, $3, $4, 1, $5)",
            &[
                &uuid(id),
                &uuid(tenant),
                &uuid(event),
                &uuid(KEYS_CEREMONY),
                &elections,
            ],
        )
        .await
        .unwrap();
    }
    for id in [RESULTS, OTHER_RESULTS] {
        tx.execute(
            "INSERT INTO sequent_backend.results_event (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&uuid(id), &uuid(tenant), &uuid(event)],
        )
        .await
        .unwrap();
    }
    for (id, session, results) in [
        (EXECUTION, SESSION, Some(RESULTS)),
        (OTHER_EXECUTION, OTHER_SESSION, Some(OTHER_RESULTS)),
        (UNLINKED_EXECUTION, SESSION, None),
    ] {
        tx.execute(
            "INSERT INTO sequent_backend.tally_session_execution
                 (id, tenant_id, election_event_id, current_message_id,
                  tally_session_id, results_event_id)
             VALUES ($1, $2, $3, 0, $4, $5)",
            &[
                &uuid(id),
                &uuid(tenant),
                &uuid(event),
                &uuid(session),
                &results.map(uuid),
            ],
        )
        .await
        .unwrap();
    }
    for (results, election, contest) in [
        (RESULTS, ELECTION, CONTEST),
        (RESULTS, OTHER_ELECTION, OTHER_CONTEST),
        (RESULTS, UNCOVERED_ELECTION, UNCOVERED_CONTEST),
        (OTHER_RESULTS, OTHER_ELECTION, UNTALLIED_CONTEST),
    ] {
        contest_results(tx, tenant, event, results, election, contest).await;
    }
}

/// The tally of TENANT's EVENT, which every test builds first.
async fn home(tx: &Transaction<'_>) {
    tenant(tx, TENANT).await;
    election_event(tx, TENANT, EVENT).await;
    tally(tx, TENANT, EVENT).await;
}

/// The same tally in TENANT's OTHER_EVENT, plus the stray election and contest.
async fn away(tx: &Transaction<'_>) {
    election_event(tx, TENANT, OTHER_EVENT).await;
    tally(tx, TENANT, OTHER_EVENT).await;
    election(tx, TENANT, OTHER_EVENT, STRAY_ELECTION).await;
    contest(tx, TENANT, OTHER_EVENT, ELECTION, STRAY_CONTEST).await;
}

/// The same tally recorded under OTHER_TENANT for EVENT.
async fn stranger(tx: &Transaction<'_>) {
    tenant(tx, OTHER_TENANT).await;
    tally(tx, OTHER_TENANT, EVENT).await;
}

struct Source {
    tenant: &'static str,
    event: &'static str,
    session: &'static str,
    execution: &'static str,
    results: &'static str,
    elections: Vec<&'static str>,
    contests: Vec<&'static str>,
    route_election: Option<&'static str>,
}

/// A consistent source: EXECUTION of SESSION into RESULTS, with both tallied
/// contests of the two covered elections.
fn source() -> Source {
    Source {
        tenant: TENANT,
        event: EVENT,
        session: SESSION,
        execution: EXECUTION,
        results: RESULTS,
        elections: vec![ELECTION, OTHER_ELECTION],
        contests: vec![CONTEST, OTHER_CONTEST],
        route_election: None,
    }
}

async fn validate(tx: &Transaction<'_>, source: Source) -> anyhow::Result<()> {
    validate_new_publication_source(
        tx,
        source.tenant,
        source.event,
        source.session,
        source.execution,
        source.results,
        &strings(&source.elections),
        &strings(&source.contests),
        source.route_election,
    )
    .await
}

async fn validation_error(tx: &Transaction<'_>, source: Source) -> String {
    validate(tx, source).await.unwrap_err().to_string()
}

/// A stored publication of SESSION, EXECUTION and RESULTS; no route election
/// is the event route.
#[derive(Clone, Copy)]
struct Stored {
    id: &'static str,
    tenant: &'static str,
    event: &'static str,
    route_election: Option<&'static str>,
    status: &'static str,
    version: i32,
    access: &'static str,
    revoked: bool,
    error_message: Option<&'static str>,
    updated_at: &'static str,
}

const PUBLISHED: Stored = Stored {
    id: P1,
    tenant: TENANT,
    event: EVENT,
    route_election: None,
    status: "Published",
    version: 1,
    access: "public",
    revoked: false,
    error_message: None,
    updated_at: "2026-01-01T00:00:00Z",
};

async fn store(tx: &Transaction<'_>, publication: Stored) {
    let route_scope = match publication.route_election {
        Some(_) => "election",
        None => "event",
    };
    let revoked_at = publication.revoked.then(|| at("2026-01-02T00:00:00Z"));
    tx.execute(
        "INSERT INTO sequent_backend.tally_results_publication (
             id, tenant_id, election_event_id, tally_session_id,
             tally_session_execution_id, results_event_id, route_scope,
             route_election_id, election_ids, access, visibility_scope,
             publication_status, version, error_message, revoked_at,
             created_at, updated_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'full_event',
                 $11, $12, $13, $14, $15, $15)",
        &[
            &uuid(publication.id),
            &uuid(publication.tenant),
            &uuid(publication.event),
            &uuid(SESSION),
            &uuid(EXECUTION),
            &uuid(RESULTS),
            &route_scope,
            &publication.route_election.map(uuid),
            &vec![uuid(ELECTION)],
            &publication.access,
            &publication.status,
            &publication.version,
            &publication.error_message,
            &revoked_at,
            &at(publication.updated_at),
        ],
    )
    .await
    .unwrap();
}

/// The stored status and error message, and whether this transaction
/// updated the row.
async fn state(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    id: &str,
) -> (String, Option<String>, bool) {
    let row = tx
        .query_one(
            "SELECT publication_status, error_message, updated_at = now()
             FROM sequent_backend.tally_results_publication
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&uuid(tenant), &uuid(event), &uuid(id)],
        )
        .await
        .unwrap();
    (row.get(0), row.get(1), row.get(2))
}

fn unchanged(status: &str) -> (String, Option<String>, bool) {
    (status.to_string(), None, false)
}

fn updated(status: &str) -> (String, Option<String>, bool) {
    (status.to_string(), None, true)
}

async fn publication(tx: &Transaction<'_>, id: &str) -> TallyResultsPublication {
    get_publication_by_id(tx, TENANT, EVENT, id).await.unwrap()
}

fn ids(publications: &[TallyResultsPublication]) -> Vec<&str> {
    publications.iter().map(|p| p.id.as_str()).collect()
}

fn new_publication<'a>(
    elections: &'a [String],
    contests: &'a [String],
) -> NewTallyResultsPublication<'a> {
    NewTallyResultsPublication {
        tenant_id: TENANT,
        election_event_id: EVENT,
        tally_session_id: SESSION,
        tally_session_execution_id: EXECUTION,
        results_event_id: RESULTS,
        task_execution_id: TASK,
        route_scope: ResultsRouteScope::Event,
        route_election_id: None,
        election_ids: elections,
        access: ResultsWebsiteAccess::Public,
        visibility_scope: ResultsWebsiteVisibilityScope::FullEvent,
        contest_ids: contests,
        published_by_user_id: None,
    }
}

#[tokio::test]
async fn a_source_whose_execution_session_results_and_contests_match_is_valid() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    validate(&transaction, source()).await.unwrap();
    validate(
        &transaction,
        Source {
            route_election: Some(OTHER_ELECTION),
            ..source()
        },
    )
    .await
    .unwrap();
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_execution_of_another_tally_session_does_not_belong_together() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        session: OTHER_SESSION,
        ..source()
    };
    assert_eq!(validation_error(&transaction, source).await, NOT_TOGETHER);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_execution_into_another_results_event_does_not_belong_together() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        results: OTHER_RESULTS,
        ..source()
    };
    assert_eq!(validation_error(&transaction, source).await, NOT_TOGETHER);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_execution_without_a_results_event_does_not_belong_together() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        execution: UNLINKED_EXECUTION,
        ..source()
    };
    assert_eq!(validation_error(&transaction, source).await, NOT_TOGETHER);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_unknown_execution_does_not_belong_together() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        execution: "80000000-0000-4000-8000-000000000009",
        ..source()
    };
    assert_eq!(validation_error(&transaction, source).await, NOT_TOGETHER);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_session_that_does_not_cover_every_source_election_does_not_belong_together() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        elections: vec![ELECTION, UNCOVERED_ELECTION],
        contests: vec![CONTEST, UNCOVERED_CONTEST],
        ..source()
    };
    assert_eq!(validation_error(&transaction, source).await, NOT_TOGETHER);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_execution_only_belongs_to_the_tenant_and_event_that_recorded_it() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let other_event = Source {
        event: OTHER_EVENT,
        ..source()
    };
    let other_tenant = Source {
        tenant: OTHER_TENANT,
        ..source()
    };
    assert_eq!(
        validation_error(&transaction, other_event).await,
        NOT_TOGETHER
    );
    assert_eq!(
        validation_error(&transaction, other_tenant).await,
        NOT_TOGETHER
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_session_election_of_another_election_event_is_outside_the_tally_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;

    let source = Source {
        elections: vec![ELECTION, STRAY_ELECTION],
        contests: vec![CONTEST],
        ..source()
    };
    assert_eq!(
        validation_error(&transaction, source).await,
        ELECTIONS_OUTSIDE
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_repeated_source_election_is_outside_the_tally_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        elections: vec![ELECTION, ELECTION],
        contests: vec![CONTEST],
        ..source()
    };
    assert_eq!(
        validation_error(&transaction, source).await,
        ELECTIONS_OUTSIDE
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_contest_of_an_unselected_election_is_outside_the_selected_elections() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        contests: vec![CONTEST, UNCOVERED_CONTEST],
        ..source()
    };
    assert_eq!(
        validation_error(&transaction, source).await,
        CONTESTS_OUTSIDE
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_contest_of_another_election_event_is_outside_the_selected_elections() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;

    let source = Source {
        contests: vec![CONTEST, STRAY_CONTEST],
        ..source()
    };
    assert_eq!(
        validation_error(&transaction, source).await,
        CONTESTS_OUTSIDE
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_repeated_source_contest_is_outside_the_selected_elections() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        contests: vec![CONTEST, CONTEST],
        ..source()
    };
    assert_eq!(
        validation_error(&transaction, source).await,
        CONTESTS_OUTSIDE
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_contest_with_results_only_in_another_results_event_is_not_tallied() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let source = Source {
        contests: vec![CONTEST, UNTALLIED_CONTEST],
        ..source()
    };
    assert_eq!(validation_error(&transaction, source).await, NOT_TALLIED);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn results_recorded_in_another_election_event_do_not_tally_a_contest() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    contest_results(
        &transaction,
        TENANT,
        OTHER_EVENT,
        RESULTS,
        OTHER_ELECTION,
        UNTALLIED_CONTEST,
    )
    .await;

    let source = Source {
        contests: vec![CONTEST, UNTALLIED_CONTEST],
        ..source()
    };
    assert_eq!(validation_error(&transaction, source).await, NOT_TALLIED);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_route_election_outside_the_source_elections_is_rejected_before_any_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let source = Source {
        route_election: Some(UNCOVERED_ELECTION),
        ..source()
    };
    assert_eq!(
        validation_error(&transaction, source).await,
        "The route election must be included in the publication elections"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_source_without_contests_is_rejected_before_any_query() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let source = Source {
        contests: vec![],
        ..source()
    };
    assert_eq!(
        validation_error(&transaction, source).await,
        "A publication requires at least one election and one contest"
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn source_identifiers_must_be_v4_uuids() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let source = Source {
        session: "session-1",
        ..source()
    };
    let error = validation_error(&transaction, source).await;
    assert!(error.starts_with("invalid UUID 'session-1'"), "{error}");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn source_facts_count_the_distinct_source_records_of_the_election_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;

    let source = PublicationSource {
        tenant_id: uuid(TENANT),
        election_event_id: uuid(EVENT),
        tally_session_id: uuid(SESSION),
        tally_session_execution_id: uuid(EXECUTION),
        results_event_id: uuid(RESULTS),
        election_ids: [ELECTION, OTHER_ELECTION, STRAY_ELECTION, ELECTION]
            .map(uuid)
            .to_vec(),
        contest_ids: [
            CONTEST,
            OTHER_CONTEST,
            UNCOVERED_CONTEST,
            UNTALLIED_CONTEST,
            STRAY_CONTEST,
            CONTEST,
        ]
        .map(uuid)
        .to_vec(),
    };
    assert_eq!(
        get_publication_source_facts(&transaction, &source)
            .await
            .unwrap(),
        PublicationSourceFacts {
            valid_execution: true,
            election_count: 2,
            contest_count: 3,
            tallied_contest_count: 2,
        }
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_first_publication_of_a_route_is_version_one() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let version =
        next_publication_version(&transaction, TENANT, EVENT, ResultsRouteScope::Event, None)
            .await
            .unwrap();
    assert_eq!(version, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_next_version_follows_the_highest_version_of_the_route_whatever_its_status() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Superseded",
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P2,
            status: "Failed",
            version: 3,
            ..PUBLISHED
        },
    )
    .await;

    let version =
        next_publication_version(&transaction, TENANT, EVENT, ResultsRouteScope::Event, None)
            .await
            .unwrap();
    assert_eq!(version, 4);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn each_election_route_is_versioned_apart_from_the_event_route() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            version: 5,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P2,
            route_election: Some(ELECTION),
            version: 2,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P3,
            route_election: Some(OTHER_ELECTION),
            version: 7,
            ..PUBLISHED
        },
    )
    .await;

    let mut versions = vec![];
    for (scope, route_election) in [
        (ResultsRouteScope::Event, None),
        (ResultsRouteScope::Election, Some(ELECTION)),
        (ResultsRouteScope::Election, Some(OTHER_ELECTION)),
        (ResultsRouteScope::Election, Some(UNCOVERED_ELECTION)),
    ] {
        versions.push(
            next_publication_version(&transaction, TENANT, EVENT, scope, route_election)
                .await
                .unwrap(),
        );
    }
    assert_eq!(versions, [6, 3, 8, 1]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn versions_of_other_events_and_tenants_do_not_count() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store(
        &transaction,
        Stored {
            event: OTHER_EVENT,
            version: 4,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            tenant: OTHER_TENANT,
            version: 6,
            ..PUBLISHED
        },
    )
    .await;

    let version =
        next_publication_version(&transaction, TENANT, EVENT, ResultsRouteScope::Event, None)
            .await
            .unwrap();
    assert_eq!(version, 1);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_route_election_must_be_a_v4_uuid_to_version_its_route() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let error = next_publication_version(
        &transaction,
        TENANT,
        EVENT,
        ResultsRouteScope::Election,
        Some("election-1"),
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(error.starts_with("invalid UUID 'election-1'"), "{error}");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_new_publication_is_stored_publishing_with_every_contest_published() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let elections = strings(&[ELECTION, OTHER_ELECTION]);
    let contests = strings(&[CONTEST, OTHER_CONTEST]);

    let publication =
        insert_publishing_publication(&transaction, new_publication(&elections, &contests))
            .await
            .unwrap();

    assert_eq!(
        serde_json::to_value(&publication).unwrap(),
        json!({
            "id": publication.id,
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "tally_session_id": SESSION,
            "tally_session_execution_id": EXECUTION,
            "results_event_id": RESULTS,
            "task_execution_id": TASK,
            "route_scope": "event",
            "route_election_id": null,
            "election_ids": [ELECTION, OTHER_ELECTION],
            "access": "public",
            "visibility_scope": "full_event",
            "published_contest_ids": [CONTEST, OTHER_CONTEST],
            "contest_publication_state": {CONTEST: "published", OTHER_CONTEST: "published"},
            "documents": {},
            "manifest": null,
            "publication_status": "Publishing",
            "version": 1,
            "error_message": null,
            "published_by_user_id": null,
        })
    );
    let row = transaction
        .query_one(
            "SELECT publication_status, version, documents, published_contest_ids,
                    contest_publication_state, election_ids, manifest IS NULL,
                    published_at IS NULL, revoked_at IS NULL, created_at = now()
             FROM sequent_backend.tally_results_publication
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&uuid(TENANT), &uuid(EVENT), &uuid(&publication.id)],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "Publishing");
    assert_eq!(row.get::<_, i32>(1), 1);
    assert_eq!(row.get::<_, Value>(2), json!({}));
    assert_eq!(row.get::<_, Value>(3), json!([CONTEST, OTHER_CONTEST]));
    assert_eq!(
        row.get::<_, Value>(4),
        json!({CONTEST: "published", OTHER_CONTEST: "published"})
    );
    assert_eq!(
        row.get::<_, Vec<Uuid>>(5),
        [uuid(ELECTION), uuid(OTHER_ELECTION)]
    );
    for column in 6..10 {
        assert!(row.get::<_, bool>(column), "column {column}");
    }
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_new_publication_takes_the_next_version_of_its_route() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            version: 2,
            ..PUBLISHED
        },
    )
    .await;
    let elections = strings(&[ELECTION]);
    let contests = strings(&[CONTEST]);

    let publication =
        insert_publishing_publication(&transaction, new_publication(&elections, &contests))
            .await
            .unwrap();
    assert_eq!(publication.version, 3);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_new_election_route_publication_records_its_route_election_and_publisher() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let elections = strings(&[ELECTION]);
    let contests = strings(&[CONTEST]);

    let publication = insert_publishing_publication(
        &transaction,
        NewTallyResultsPublication {
            route_scope: ResultsRouteScope::Election,
            route_election_id: Some(ELECTION),
            access: ResultsWebsiteAccess::Authenticated,
            visibility_scope: ResultsWebsiteVisibilityScope::AreaBased,
            published_by_user_id: Some(USER),
            ..new_publication(&elections, &contests)
        },
    )
    .await
    .unwrap();

    assert_eq!(publication.route_scope, ResultsRouteScope::Election);
    assert_eq!(publication.route_election_id.as_deref(), Some(ELECTION));
    assert_eq!(publication.access, ResultsWebsiteAccess::Authenticated);
    assert_eq!(
        publication.visibility_scope,
        ResultsWebsiteVisibilityScope::AreaBased
    );
    assert_eq!(publication.published_by_user_id.as_deref(), Some(USER));
    let row = transaction
        .query_one(
            "SELECT route_scope, route_election_id, access, visibility_scope,
                    published_by_user_id
             FROM sequent_backend.tally_results_publication
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&uuid(TENANT), &uuid(EVENT), &uuid(&publication.id)],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "election");
    assert_eq!(row.get::<_, Option<Uuid>>(1), Some(uuid(ELECTION)));
    assert_eq!(row.get::<_, String>(2), "authenticated");
    assert_eq!(row.get::<_, String>(3), "area_based");
    assert_eq!(row.get::<_, Option<Uuid>>(4), Some(uuid(USER)));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_public_area_based_publication_is_refused_by_the_access_policy_check() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let elections = strings(&[ELECTION]);
    let contests = strings(&[CONTEST]);

    let error = insert_publishing_publication(
        &transaction,
        NewTallyResultsPublication {
            visibility_scope: ResultsWebsiteVisibilityScope::AreaBased,
            ..new_publication(&elections, &contests)
        },
    )
    .await
    .unwrap_err();
    let db_error = db_error(&error);
    assert_eq!(db_error.code(), &SqlState::CHECK_VIOLATION);
    assert_eq!(
        db_error.constraint(),
        Some("tally_results_publication_check1")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_election_route_without_its_election_is_refused_by_the_route_check() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let elections = strings(&[ELECTION]);
    let contests = strings(&[CONTEST]);

    let error = insert_publishing_publication(
        &transaction,
        NewTallyResultsPublication {
            route_scope: ResultsRouteScope::Election,
            ..new_publication(&elections, &contests)
        },
    )
    .await
    .unwrap_err();
    let db_error = db_error(&error);
    assert_eq!(db_error.code(), &SqlState::CHECK_VIOLATION);
    assert_eq!(
        db_error.constraint(),
        Some("tally_results_publication_check")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_new_publication_needs_a_v4_task_execution_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let elections = strings(&[ELECTION]);
    let contests = strings(&[CONTEST]);

    let error = insert_publishing_publication(
        &transaction,
        NewTallyResultsPublication {
            task_execution_id: "task-1",
            ..new_publication(&elections, &contests)
        },
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(error.starts_with("invalid UUID 'task-1'"), "{error}");
    let stored: i64 = transaction
        .query_one(
            "SELECT count(*) FROM sequent_backend.tally_results_publication",
            &[],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(stored, 0);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_stored_publication_is_read_with_its_json_and_enum_columns() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    transaction
        .execute(
            "INSERT INTO sequent_backend.tally_results_publication (
                 id, tenant_id, election_event_id, tally_session_id,
                 tally_session_execution_id, results_event_id, task_execution_id,
                 route_scope, route_election_id, election_ids, access,
                 visibility_scope, published_contest_ids, contest_publication_state,
                 documents, manifest, publication_status, version, error_message,
                 published_by_user_id
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'election', $8, $9, 'authenticated',
                     'area_based', $10, $11, $12, $13, 'Published', 4, 'late upload', $14)",
            &[
                &uuid(P1),
                &uuid(TENANT),
                &uuid(EVENT),
                &uuid(SESSION),
                &uuid(EXECUTION),
                &uuid(RESULTS),
                &uuid(TASK),
                &uuid(ELECTION),
                &vec![uuid(ELECTION), uuid(OTHER_ELECTION)],
                &json!([CONTEST]),
                &json!({CONTEST: "published", OTHER_CONTEST: "not_published"}),
                &json!({"json": "results.json"}),
                &json!({"version": 4}),
                &uuid(USER),
            ],
        )
        .await
        .unwrap();

    let publication = publication(&transaction, P1).await;
    assert_eq!(
        serde_json::to_value(&publication).unwrap(),
        json!({
            "id": P1,
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "tally_session_id": SESSION,
            "tally_session_execution_id": EXECUTION,
            "results_event_id": RESULTS,
            "task_execution_id": TASK,
            "route_scope": "election",
            "route_election_id": ELECTION,
            "election_ids": [ELECTION, OTHER_ELECTION],
            "access": "authenticated",
            "visibility_scope": "area_based",
            "published_contest_ids": [CONTEST],
            "contest_publication_state": {CONTEST: "published", OTHER_CONTEST: "not_published"},
            "documents": {"json": "results.json"},
            "manifest": {"version": 4},
            "publication_status": "Published",
            "version": 4,
            "error_message": "late upload",
            "published_by_user_id": USER,
        })
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_unknown_publication_is_not_found() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;

    let error = get_publication_by_id(&transaction, TENANT, EVENT, P1)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Publication not found");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_publication_is_only_found_in_its_tenant_and_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store(
        &transaction,
        Stored {
            event: OTHER_EVENT,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            tenant: OTHER_TENANT,
            ..PUBLISHED
        },
    )
    .await;

    let error = get_publication_by_id(&transaction, TENANT, EVENT, P1)
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Publication not found");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_publication_with_an_unknown_contest_state_cannot_be_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(&transaction, PUBLISHED).await;
    transaction
        .execute(
            "UPDATE sequent_backend.tally_results_publication
             SET contest_publication_state = $1 WHERE id = $2",
            &[&json!({CONTEST: "withdrawn"}), &uuid(P1)],
        )
        .await
        .unwrap();

    let error = get_publication_by_id(&transaction, TENANT, EVENT, P1)
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown variant `withdrawn`"), "{error}");
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn the_active_event_publication_is_the_published_unrevoked_one() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    for (id, status, version) in [
        (P1, "Superseded", 1),
        (P2, "Published", 2),
        (P3, "Failed", 3),
        (P4, "Publishing", 4),
    ] {
        store(
            &transaction,
            Stored {
                id,
                status,
                version,
                ..PUBLISHED
            },
        )
        .await;
    }
    store(
        &transaction,
        Stored {
            id: P5,
            status: "Revoked",
            version: 5,
            revoked: true,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P6,
            route_election: Some(ELECTION),
            version: 6,
            ..PUBLISHED
        },
    )
    .await;

    let active = get_active_publication_for_route(
        &transaction,
        TENANT,
        EVENT,
        ResultsRouteScope::Event,
        None,
    )
    .await
    .unwrap();
    assert_eq!(active.map(|p| p.id).as_deref(), Some(P2));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_published_publication_with_a_revocation_time_is_not_active() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            revoked: true,
            ..PUBLISHED
        },
    )
    .await;

    let active = get_active_publication_for_route(
        &transaction,
        TENANT,
        EVENT,
        ResultsRouteScope::Event,
        None,
    )
    .await
    .unwrap();
    assert!(active.is_none());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn an_election_route_resolves_to_the_publication_of_its_election() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(&transaction, PUBLISHED).await;
    store(
        &transaction,
        Stored {
            id: P2,
            route_election: Some(ELECTION),
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P3,
            route_election: Some(OTHER_ELECTION),
            ..PUBLISHED
        },
    )
    .await;

    let active = get_active_publication_for_route(
        &transaction,
        TENANT,
        EVENT,
        ResultsRouteScope::Election,
        Some(OTHER_ELECTION),
    )
    .await
    .unwrap();
    assert_eq!(active.map(|p| p.id).as_deref(), Some(P3));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_route_publication_of_another_event_or_tenant_is_not_active() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store(
        &transaction,
        Stored {
            event: OTHER_EVENT,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            tenant: OTHER_TENANT,
            ..PUBLISHED
        },
    )
    .await;

    let active = get_active_publication_for_route(
        &transaction,
        TENANT,
        EVENT,
        ResultsRouteScope::Event,
        None,
    )
    .await
    .unwrap();
    assert!(active.is_none());
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn active_publications_are_listed_by_route_whatever_their_access() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            version: 3,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P2,
            route_election: Some(OTHER_ELECTION),
            access: "authenticated",
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P3,
            route_election: Some(ELECTION),
            version: 2,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P4,
            status: "Superseded",
            version: 2,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P5,
            route_election: Some(ELECTION),
            revoked: true,
            ..PUBLISHED
        },
    )
    .await;

    let active = list_active_public_publications(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    assert_eq!(ids(&active), [P3, P2, P1]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn active_publications_of_other_events_and_tenants_are_not_listed() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store(&transaction, PUBLISHED).await;
    store(
        &transaction,
        Stored {
            id: P2,
            event: OTHER_EVENT,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P3,
            tenant: OTHER_TENANT,
            ..PUBLISHED
        },
    )
    .await;

    let active = list_active_public_publications(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    assert_eq!(ids(&active), [P1]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn superseded_publications_are_listed_least_recently_updated_first() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Superseded",
            updated_at: "2026-01-03T00:00:00Z",
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P2,
            status: "Superseded",
            version: 2,
            updated_at: "2026-01-01T00:00:00Z",
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P3,
            version: 3,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P4,
            status: "Revoked",
            version: 4,
            revoked: true,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P5,
            status: "Failed",
            version: 5,
            ..PUBLISHED
        },
    )
    .await;

    let superseded = list_superseded_publications(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    assert_eq!(ids(&superseded), [P2, P1]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn superseded_publications_of_other_events_and_tenants_are_not_listed() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    let superseded = Stored {
        status: "Superseded",
        ..PUBLISHED
    };
    store(&transaction, superseded).await;
    store(
        &transaction,
        Stored {
            id: P2,
            event: OTHER_EVENT,
            ..superseded
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P3,
            tenant: OTHER_TENANT,
            ..superseded
        },
    )
    .await;

    let listed = list_superseded_publications(&transaction, TENANT, EVENT)
        .await
        .unwrap();
    assert_eq!(ids(&listed), [P1]);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn activating_a_publishing_publication_publishes_its_documents_and_manifest() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Publishing",
            ..PUBLISHED
        },
    )
    .await;
    let publishing = publication(&transaction, P1).await;

    mark_publication_published(
        &transaction,
        &publishing,
        json!({"json": "results.json"}),
        json!({"version": 1}),
    )
    .await
    .unwrap();

    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        updated("Published")
    );
    let row = transaction
        .query_one(
            "SELECT documents, manifest, published_at = now()
             FROM sequent_backend.tally_results_publication WHERE id = $1",
            &[&uuid(P1)],
        )
        .await
        .unwrap();
    assert_eq!(row.get::<_, Value>(0), json!({"json": "results.json"}));
    assert_eq!(row.get::<_, Value>(1), json!({"version": 1}));
    assert!(row.get::<_, bool>(2));
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn activating_a_failed_publication_clears_its_error() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Failed",
            error_message: Some("upload failed"),
            ..PUBLISHED
        },
    )
    .await;
    let failed = publication(&transaction, P1).await;

    mark_publication_published(&transaction, &failed, json!({}), json!({}))
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        updated("Published")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn activating_supersedes_the_active_publication_of_its_route() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(&transaction, PUBLISHED).await;
    store(
        &transaction,
        Stored {
            id: P2,
            status: "Publishing",
            version: 2,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P3,
            status: "Failed",
            version: 3,
            ..PUBLISHED
        },
    )
    .await;
    let publishing = publication(&transaction, P2).await;

    mark_publication_published(&transaction, &publishing, json!({}), json!({}))
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        updated("Superseded")
    );
    assert_eq!(
        state(&transaction, TENANT, EVENT, P2).await,
        updated("Published")
    );
    assert_eq!(
        state(&transaction, TENANT, EVENT, P3).await,
        unchanged("Failed")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn activating_the_event_route_leaves_election_routes_active() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            route_election: Some(ELECTION),
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P2,
            status: "Publishing",
            ..PUBLISHED
        },
    )
    .await;
    let publishing = publication(&transaction, P2).await;

    mark_publication_published(&transaction, &publishing, json!({}), json!({}))
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        unchanged("Published")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn activating_an_election_route_supersedes_only_that_election() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    let election_route = Stored {
        route_election: Some(ELECTION),
        ..PUBLISHED
    };
    store(&transaction, election_route).await;
    store(
        &transaction,
        Stored {
            id: P2,
            route_election: Some(OTHER_ELECTION),
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P3,
            status: "Publishing",
            version: 2,
            ..election_route
        },
    )
    .await;
    let publishing = publication(&transaction, P3).await;

    mark_publication_published(&transaction, &publishing, json!({}), json!({}))
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        updated("Superseded")
    );
    assert_eq!(
        state(&transaction, TENANT, EVENT, P2).await,
        unchanged("Published")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn activating_leaves_the_routes_of_other_events_and_tenants_active() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    stranger(&transaction).await;
    store(
        &transaction,
        Stored {
            event: OTHER_EVENT,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            tenant: OTHER_TENANT,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P2,
            status: "Publishing",
            version: 2,
            ..PUBLISHED
        },
    )
    .await;
    let publishing = publication(&transaction, P2).await;

    mark_publication_published(&transaction, &publishing, json!({}), json!({}))
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, OTHER_EVENT, P1).await,
        unchanged("Published")
    );
    assert_eq!(
        state(&transaction, OTHER_TENANT, EVENT, P1).await,
        unchanged("Published")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_published_publication_cannot_be_activated_again() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(&transaction, PUBLISHED).await;
    let published = publication(&transaction, P1).await;

    let error = mark_publication_published(&transaction, &published, json!({}), json!({}))
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Publication is not in a state that can be activated"
    );
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        unchanged("Published")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_refused_activation_has_already_superseded_the_active_publication_of_its_route() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            version: 2,
            ..PUBLISHED
        },
    )
    .await;
    store(
        &transaction,
        Stored {
            id: P2,
            status: "Superseded",
            ..PUBLISHED
        },
    )
    .await;
    let superseded = publication(&transaction, P2).await;

    let error = mark_publication_published(&transaction, &superseded, json!({}), json!({}))
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Publication is not in a state that can be activated"
    );
    // The caller must roll back: the route has no active publication left.
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        updated("Superseded")
    );
    assert_eq!(
        state(&transaction, TENANT, EVENT, P2).await,
        unchanged("Superseded")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_publishing_publication_is_marked_failed_with_its_error() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Publishing",
            ..PUBLISHED
        },
    )
    .await;

    mark_publication_failed(&transaction, TENANT, EVENT, P1, "render failed")
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        (
            "Failed".to_string(),
            Some("render failed".to_string()),
            true
        )
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_failed_publication_is_marked_failed_again_with_the_new_error() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Failed",
            error_message: Some("render failed"),
            ..PUBLISHED
        },
    )
    .await;

    mark_publication_failed(&transaction, TENANT, EVENT, P1, "upload failed")
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        (
            "Failed".to_string(),
            Some("upload failed".to_string()),
            true
        )
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_published_publication_cannot_be_marked_failed() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(&transaction, PUBLISHED).await;

    let error = mark_publication_failed(&transaction, TENANT, EVENT, P1, "render failed")
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Publication is not in a state that can be marked failed"
    );
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        unchanged("Published")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_publication_of_another_event_cannot_be_marked_failed() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    away(&transaction).await;
    store(
        &transaction,
        Stored {
            event: OTHER_EVENT,
            status: "Publishing",
            ..PUBLISHED
        },
    )
    .await;

    let error = mark_publication_failed(&transaction, TENANT, EVENT, P1, "render failed")
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Publication is not in a state that can be marked failed"
    );
    assert_eq!(
        state(&transaction, TENANT, OTHER_EVENT, P1).await,
        unchanged("Publishing")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_published_publication_records_a_finalization_error() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(&transaction, PUBLISHED).await;

    set_publication_finalization_error(&transaction, TENANT, EVENT, P1, Some("cleanup failed"))
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        (
            "Published".to_string(),
            Some("cleanup failed".to_string()),
            true
        )
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn clearing_the_finalization_error_of_a_published_publication_sets_it_to_null() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            error_message: Some("cleanup failed"),
            ..PUBLISHED
        },
    )
    .await;

    set_publication_finalization_error(&transaction, TENANT, EVENT, P1, None)
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        updated("Published")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn only_a_published_publication_has_a_finalization_error() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Publishing",
            ..PUBLISHED
        },
    )
    .await;

    let error =
        set_publication_finalization_error(&transaction, TENANT, EVENT, P1, Some("cleanup failed"))
            .await
            .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Published publication was not available to update its finalization error"
    );
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        unchanged("Publishing")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_published_publication_is_marked_superseded() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(&transaction, PUBLISHED).await;
    let published = publication(&transaction, P1).await;

    mark_publication_superseded(&transaction, &published)
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        updated("Superseded")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn superseding_a_publication_that_is_not_published_silently_changes_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Failed",
            ..PUBLISHED
        },
    )
    .await;
    let failed = publication(&transaction, P1).await;

    mark_publication_superseded(&transaction, &failed)
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        unchanged("Failed")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn revoking_a_published_publication_records_its_revocation() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(&transaction, PUBLISHED).await;

    revoke_publication(&transaction, TENANT, EVENT, P1)
        .await
        .unwrap();
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        updated("Revoked")
    );
    let revoked_now: bool = transaction
        .query_one(
            "SELECT revoked_at = now() FROM sequent_backend.tally_results_publication
             WHERE id = $1",
            &[&uuid(P1)],
        )
        .await
        .unwrap()
        .get(0);
    assert!(revoked_now);
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_publication_that_is_not_published_cannot_be_revoked() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    store(
        &transaction,
        Stored {
            status: "Superseded",
            ..PUBLISHED
        },
    )
    .await;

    let error = revoke_publication(&transaction, TENANT, EVENT, P1)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Publication not found or not currently published"
    );
    assert_eq!(
        state(&transaction, TENANT, EVENT, P1).await,
        unchanged("Superseded")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn a_publication_of_another_tenant_cannot_be_revoked() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();
    home(&transaction).await;
    stranger(&transaction).await;
    store(
        &transaction,
        Stored {
            tenant: OTHER_TENANT,
            ..PUBLISHED
        },
    )
    .await;

    let error = revoke_publication(&transaction, TENANT, EVENT, P1)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Publication not found or not currently published"
    );
    assert_eq!(
        state(&transaction, OTHER_TENANT, EVENT, P1).await,
        unchanged("Published")
    );
    transaction.rollback().await.unwrap();
}

#[tokio::test]
async fn revoking_needs_a_v4_publication_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let transaction = client.transaction().await.unwrap();

    let error = revoke_publication(&transaction, TENANT, EVENT, "publication-1")
        .await
        .unwrap_err()
        .to_string();
    assert!(error.starts_with("invalid UUID 'publication-1'"), "{error}");
    transaction.rollback().await.unwrap();
}
