// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The tally-sheet and tally-sheet-import PostgreSQL adapters against the
//! migrated schema: ballot-box versions, review, soft deletion and the import
//! records reviewed before sheets are approved. Each test writes its own rows
//! in a transaction, checks what the adapter returns and what it stored, and
//! rolls back.

#[path = "support/schema.rs"]
mod schema;

use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::TallySheet;
use sequent_core::types::tally_sheet_import::{
    TallySheetImport, TallySheetImportChangeType, TallySheetImportItem, TallySheetImportItemStatus,
    TallySheetImportSourceFormat, TallySheetImportStatus, TallySheetImportSummary,
};
use sequent_core::types::tally_sheets::{
    AreaContestResults, CandidateResults, InvalidVotes, TallySheetStatus, VotingChannel,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio_postgres::error::SqlState;
use uuid::Uuid;
use windmill::postgres::tally_sheet::{self, ReviewTallySheetOutcome};
use windmill::postgres::tally_sheet_import;

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

/// A tenant with two election events, and another tenant with one. The event
/// has an election with two areas and two contests, and a source document.
struct World {
    ids: Ids,
    tenant: String,
    event: String,
    other_event: String,
    other_tenant: String,
    election: String,
    area: String,
    second_area: String,
    contest: String,
    second_contest: String,
    document: String,
}

impl World {
    async fn new(tx: &Transaction<'_>, ids: Ids) -> World {
        let world = World {
            tenant: ids.id(1),
            event: ids.id(2),
            other_event: ids.id(3),
            other_tenant: ids.id(4),
            election: ids.id(5),
            area: ids.id(6),
            second_area: ids.id(7),
            contest: ids.id(8),
            second_contest: ids.id(9),
            document: ids.id(99),
            ids,
        };
        let (tenant, event) = (&world.tenant, &world.event);
        for id in [tenant, &world.other_tenant] {
            execute(
                tx,
                "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1::text::uuid, $1)",
                &[id],
            )
            .await;
        }
        for (tenant_id, id) in [
            (tenant, event),
            (tenant, &world.other_event),
            (&world.other_tenant, &world.ids.id(98)),
        ] {
            execute(
                tx,
                "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
                 VALUES ($1::text::uuid, $2::text::uuid, 'RSA256')",
                &[id, tenant_id],
            )
            .await;
        }
        execute(
            tx,
            "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid)",
            &[&world.election, tenant, event],
        )
        .await;
        for area in [&world.area, &world.second_area] {
            execute(
                tx,
                "INSERT INTO sequent_backend.area (id, tenant_id, election_event_id)
                 VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid)",
                &[area, tenant, event],
            )
            .await;
        }
        for contest in [&world.contest, &world.second_contest] {
            execute(
                tx,
                "INSERT INTO sequent_backend.contest
                     (id, tenant_id, election_event_id, election_id)
                 VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4::text::uuid)",
                &[contest, tenant, event, &world.election],
            )
            .await;
        }
        execute(
            tx,
            "INSERT INTO sequent_backend.document (id, tenant_id, election_event_id, name)
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, 'sheets.csv')",
            &[&world.document, tenant, event],
        )
        .await;
        world
    }

    /// A row of this test; numbers below 10 name the world itself.
    fn id(&self, n: u32) -> String {
        self.ids.id(n)
    }

    /// The paper ballot box of the first area and contest.
    fn ballot_box(&self) -> BallotBox<'_> {
        BallotBox {
            election: &self.election,
            area: &self.area,
            contest: &self.contest,
            channel: "PAPER",
        }
    }
}

/// One (election, area, contest, channel) of the world's event.
#[derive(Clone, Copy)]
struct BallotBox<'a> {
    election: &'a str,
    area: &'a str,
    contest: &'a str,
    channel: &'a str,
}

async fn execute(tx: &Transaction<'_>, sql: &str, params: &[&String]) {
    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
        .iter()
        .map(|param| *param as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();
    tx.execute(sql, &params).await.unwrap();
}

/// A pending sheet of version `version` in the ballot box, created at 10:00.
async fn sheet_row(
    tx: &Transaction<'_>,
    w: &World,
    ballot_box: BallotBox<'_>,
    id: &str,
    version: i32,
) {
    tx.execute(
        "INSERT INTO sequent_backend.tally_sheet
             (id, tenant_id, election_event_id, election_id, area_id, contest_id, channel,
              version, created_by_user_id, created_at, last_updated_at)
         VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4::text::uuid,
                 $5::text::uuid, $6::text::uuid, $7, $8, 'clerk-1', '2026-01-01T10:00:00Z',
                 '2026-01-01T10:00:00Z')",
        &[
            &id,
            &w.tenant,
            &w.event,
            &ballot_box.election,
            &ballot_box.area,
            &ballot_box.contest,
            &ballot_box.channel,
            &version,
        ],
    )
    .await
    .unwrap();
}

async fn set(tx: &Transaction<'_>, table: &str, id: &str, assignments: &str) {
    tx.batch_execute(&format!(
        "UPDATE sequent_backend.{table} SET {assignments} WHERE id = '{id}'"
    ))
    .await
    .unwrap();
}

/// Approved by `reviewer-1` at 11:00.
const APPROVED: &str =
    "status = 'APPROVED', reviewed_at = '2026-01-01T11:00:00Z', reviewed_by_user_id = 'reviewer-1'";
const DELETED: &str = "deleted_at = '2026-01-01T12:00:00Z'";

/// The row of `table` with this id as `to_jsonb` renders it, without the
/// timestamp columns listed.
async fn stored(tx: &Transaction<'_>, table: &str, id: &str, timestamps: &[&str]) -> Value {
    let row: Value = tx
        .query_one(
            &format!(
                "SELECT to_jsonb(r) FROM sequent_backend.{table} r WHERE r.id = $1::text::uuid"
            ),
            &[&id],
        )
        .await
        .unwrap()
        .get(0);
    let Value::Object(mut columns) = row else {
        panic!("{table} row is not an object");
    };
    for timestamp in timestamps {
        columns.remove(*timestamp);
    }
    Value::Object(columns)
}

const SHEET_TIMESTAMPS: [&str; 4] = ["created_at", "last_updated_at", "reviewed_at", "deleted_at"];

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

// tally_sheet

fn results(w: &World) -> AreaContestResults {
    AreaContestResults {
        area_id: w.area.clone(),
        contest_id: w.contest.clone(),
        total_votes: Some(10),
        total_valid_votes: Some(8),
        invalid_votes: Some(InvalidVotes {
            total_invalid: Some(2),
            implicit_invalid: Some(1),
            explicit_invalid: Some(1),
        }),
        total_blank_votes: Some(1),
        blank_ballots: Some(1),
        census: Some(100),
        candidate_results: HashMap::from([(
            "candidate-1".into(),
            CandidateResults {
                candidate_id: "candidate-1".into(),
                total_votes: Some(7),
            },
        )]),
        annotations: None,
    }
}

/// `results(w)` as the content column stores it.
fn results_json(w: &World) -> Value {
    json!({
        "area_id": w.area,
        "contest_id": w.contest,
        "total_votes": 10,
        "total_valid_votes": 8,
        "invalid_votes": {"total_invalid": 2, "implicit_invalid": 1, "explicit_invalid": 1},
        "total_blank_votes": 1,
        "blank_ballots": 1,
        "census": 100,
        "candidate_results": {"candidate-1": {"candidate_id": "candidate-1", "total_votes": 7}},
        "annotations": null,
    })
}

fn sheet_ids(sheets: Vec<TallySheet>) -> Vec<String> {
    sorted(sheets.into_iter().map(|sheet| sheet.id).collect())
}

async fn insert_sheet(
    tx: &Transaction<'_>,
    w: &World,
    version: i32,
    import_id: Option<&str>,
) -> anyhow::Result<TallySheet> {
    tally_sheet::insert_tally_sheet(
        tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.contest,
        &w.area,
        &results(w),
        &VotingChannel::PAPER,
        "clerk-1",
        TallySheetStatus::PENDING,
        version,
        import_id,
    )
    .await
}

#[tokio::test]
async fn insert_tally_sheet_returns_and_stores_the_sheet() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let inserted = insert_sheet(&tx, &w, 3, None).await.unwrap();

    let started = now(&tx).await;
    assert_eq!(
        inserted,
        TallySheet {
            id: inserted.id.clone(),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            election_id: w.election.clone(),
            contest_id: w.contest.clone(),
            area_id: w.area.clone(),
            created_at: Some(started),
            last_updated_at: Some(started),
            labels: None,
            annotations: None,
            reviewed_at: None,
            reviewed_by_user_id: None,
            content: Some(results(&w)),
            channel: Some("PAPER".into()),
            deleted_at: None,
            created_by_user_id: "clerk-1".into(),
            status: TallySheetStatus::PENDING,
            version: 3,
            import_id: None,
        }
    );
    assert_eq!(
        stored(&tx, "tally_sheet", &inserted.id, &SHEET_TIMESTAMPS).await,
        json!({
            "id": inserted.id,
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "election_id": w.election,
            "contest_id": w.contest,
            "area_id": w.area,
            "labels": null,
            "annotations": null,
            "content": results_json(&w),
            "channel": "PAPER",
            "created_by_user_id": "clerk-1",
            "reviewed_by_user_id": null,
            "status": "PENDING",
            "version": 3,
            "import_id": null,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_sheet_links_the_sheet_to_its_import() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;

    let inserted = insert_sheet(&tx, &w, 1, Some(&w.id(10))).await.unwrap();

    assert_eq!(inserted.import_id, Some(w.id(10)));
    assert_eq!(
        stored(&tx, "tally_sheet", &inserted.id, &SHEET_TIMESTAMPS).await["import_id"],
        json!(w.id(10))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_sheet_rejects_a_second_live_sheet_of_the_same_version() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    insert_sheet(&tx, &w, 1, None).await.unwrap();
    tx.batch_execute("SAVEPOINT duplicate").await.unwrap();

    let error = insert_sheet(&tx, &w, 1, None).await.unwrap_err();

    // The database error is flattened into text: its SQLSTATE is lost.
    assert_eq!(error.to_string(), "Error inserting tally sheet: db error");
    assert_eq!(sql_state(&error), None);
    tx.batch_execute("ROLLBACK TO SAVEPOINT duplicate")
        .await
        .unwrap();
    assert_eq!(count(&tx, "tally_sheet", &w.tenant).await, 1);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_sheet_rejects_an_invalid_area_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = tally_sheet::insert_tally_sheet(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.contest,
        "area-1",
        &results(&w),
        &VotingChannel::PAPER,
        "clerk-1",
        TallySheetStatus::PENDING,
        1,
        None,
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .starts_with("Error parsing area_id as UUID: invalid UUID 'area-1'"));
    assert_eq!(count(&tx, "tally_sheet", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_approved_tally_sheets_by_event_needs_approval_review_and_no_deletion() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let paper = w.ballot_box();
    let postal = BallotBox {
        channel: "POSTAL",
        ..paper
    };
    for (n, ballot_box, version, assignments) in [
        (10, paper, 1, APPROVED.to_string()),
        (11, postal, 1, APPROVED.to_string()),
        (12, paper, 2, "status = 'PENDING'".to_string()),
        (
            13,
            paper,
            3,
            "status = 'APPROVED', reviewed_at = NULL".to_string(),
        ),
        (
            14,
            paper,
            4,
            "status = 'APPROVED', reviewed_at = '2026-01-01T11:00:00Z'".to_string(),
        ),
        (15, paper, 5, format!("{APPROVED}, {DELETED}")),
        (
            16,
            paper,
            6,
            "status = 'DISAPPROVED', reviewed_at = '2026-01-01T11:00:00Z', \
             reviewed_by_user_id = 'reviewer-1'"
                .to_string(),
        ),
    ] {
        sheet_row(&tx, &w, ballot_box, &w.id(n), version).await;
        set(&tx, "tally_sheet", &w.id(n), &assignments).await;
    }

    let approved = tally_sheet::get_approved_tally_sheets_by_event(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    assert_eq!(sheet_ids(approved), [w.id(10), w.id(11)]);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_approved_tally_sheets_by_event_maps_the_reviewed_sheet() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;
    set(&tx, "tally_sheet", &w.id(10), APPROVED).await;
    tx.execute(
        "UPDATE sequent_backend.tally_sheet SET content = $2, labels = '{\"l\": 1}',
             annotations = '{\"a\": 1}'
         WHERE id = $1::text::uuid",
        &[&w.id(10), &results_json(&w)],
    )
    .await
    .unwrap();

    let approved = tally_sheet::get_approved_tally_sheets_by_event(&tx, &w.tenant, &w.event)
        .await
        .unwrap();

    assert_eq!(
        approved,
        [TallySheet {
            id: w.id(10),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            election_id: w.election.clone(),
            contest_id: w.contest.clone(),
            area_id: w.area.clone(),
            created_at: Some(at("2026-01-01T10:00:00Z")),
            last_updated_at: Some(at("2026-01-01T10:00:00Z")),
            labels: Some(json!({"l": 1})),
            annotations: Some(json!({"a": 1})),
            reviewed_at: Some(at("2026-01-01T11:00:00Z")),
            reviewed_by_user_id: Some("reviewer-1".into()),
            content: Some(results(&w)),
            channel: Some("PAPER".into()),
            deleted_at: None,
            created_by_user_id: "clerk-1".into(),
            status: TallySheetStatus::APPROVED,
            version: 1,
            import_id: None,
        }]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_approved_tally_sheet_returns_the_highest_live_approved_version() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let paper = w.ballot_box();
    for (n, version, assignments) in [
        (10, 1, APPROVED.to_string()),
        (11, 2, APPROVED.to_string()),
        (12, 3, "status = 'PENDING'".to_string()),
        (13, 4, format!("{APPROVED}, {DELETED}")),
    ] {
        sheet_row(&tx, &w, paper, &w.id(n), version).await;
        set(&tx, "tally_sheet", &w.id(n), &assignments).await;
    }

    let latest = tally_sheet::get_latest_approved_tally_sheet(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.area,
        &w.contest,
        &VotingChannel::PAPER,
    )
    .await
    .unwrap();

    assert_eq!(latest.map(|sheet| sheet.id), Some(w.id(11)));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_approved_tally_sheet_is_none_without_an_approved_sheet_of_the_ballot_box() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let paper = w.ballot_box();
    for (n, ballot_box) in [
        (
            10,
            BallotBox {
                channel: "POSTAL",
                ..paper
            },
        ),
        (
            11,
            BallotBox {
                area: &w.second_area,
                ..paper
            },
        ),
        (
            12,
            BallotBox {
                contest: &w.second_contest,
                ..paper
            },
        ),
    ] {
        sheet_row(&tx, &w, ballot_box, &w.id(n), 1).await;
        set(&tx, "tally_sheet", &w.id(n), APPROVED).await;
    }
    sheet_row(&tx, &w, paper, &w.id(13), 1).await;

    let latest = tally_sheet::get_latest_approved_tally_sheet(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.area,
        &w.contest,
        &VotingChannel::PAPER,
    )
    .await
    .unwrap();

    assert_eq!(latest, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_ballot_box_version_is_zero_for_an_empty_ballot_box() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let postal = BallotBox {
        channel: "POSTAL",
        ..w.ballot_box()
    };
    sheet_row(&tx, &w, postal, &w.id(10), 5).await;

    let version = tally_sheet::get_latest_ballot_box_version(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.area,
        &w.contest,
        &VotingChannel::PAPER,
    )
    .await
    .unwrap();

    assert_eq!(version, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_ballot_box_version_counts_deleted_versions() {
    // A new version must not reuse the number of a soft-deleted one.
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(11), 2).await;
    set(&tx, "tally_sheet", &w.id(11), DELETED).await;

    let version = tally_sheet::get_latest_ballot_box_version(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.area,
        &w.contest,
        &VotingChannel::PAPER,
    )
    .await
    .unwrap();

    assert_eq!(version, 2);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_ballot_box_tally_sheet_ignores_deleted_versions() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(11), 2).await;
    set(&tx, "tally_sheet", &w.id(11), DELETED).await;
    let postal = BallotBox {
        channel: "POSTAL",
        ..w.ballot_box()
    };
    sheet_row(&tx, &w, postal, &w.id(12), 3).await;

    let latest = tally_sheet::get_latest_ballot_box_tally_sheet(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.area,
        &w.contest,
        &VotingChannel::PAPER,
    )
    .await
    .unwrap();

    assert_eq!(
        latest.map(|sheet| (sheet.id, sheet.version, sheet.status)),
        Some((w.id(10), 1, TallySheetStatus::PENDING))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_ballot_box_tally_sheet_fails_on_an_unknown_status() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;
    set(&tx, "tally_sheet", &w.id(10), "status = 'ARCHIVED'").await;

    let error = tally_sheet::get_latest_ballot_box_tally_sheet(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.area,
        &w.contest,
        &VotingChannel::PAPER,
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Invalid tally sheet status: Matching variant not found"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_latest_ballot_box_tally_sheet_fails_on_content_it_cannot_read() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;
    set(
        &tx,
        "tally_sheet",
        &w.id(10),
        "content = '{\"area_id\": \"a\", \"contest_id\": \"c\", \"total_votes\": -1}'",
    )
    .await;

    let error = tally_sheet::get_latest_ballot_box_tally_sheet(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.area,
        &w.contest,
        &VotingChannel::PAPER,
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .starts_with("total_votes: invalid value: integer `-1`, expected u64"));
    tx.rollback().await.unwrap();
}

/// Takes the version-assignment lock of the ballot box of `area` and `channel`
/// in election 5, contest 8 of tenant 1 and event 2 of `ids`.
async fn lock_ballot_box(
    tx: &Transaction<'_>,
    ids: &Ids,
    area: &str,
    channel: &VotingChannel,
) -> anyhow::Result<()> {
    tally_sheet::lock_ballot_box_version_assignment(
        tx,
        &ids.id(1),
        &ids.id(2),
        &ids.id(5),
        area,
        &ids.id(8),
        channel,
    )
    .await
}

#[tokio::test]
async fn lock_ballot_box_version_assignment_makes_another_assignment_to_the_ballot_box_wait() {
    // Advisory locks are shared across sessions, so no row has to be committed.
    let pool = schema::pool().await;
    let ids = ids!();
    let area = ids.id(6);
    let mut holder = pool.get().await.unwrap();
    let held = holder.transaction().await.unwrap();
    lock_ballot_box(&held, &ids, &area, &VotingChannel::PAPER)
        .await
        .unwrap();

    let mut contender = pool.get().await.unwrap();
    let waiting = contender.transaction().await.unwrap();
    waiting
        .batch_execute("SET LOCAL lock_timeout = '10ms'")
        .await
        .unwrap();
    lock_ballot_box(&waiting, &ids, &area, &VotingChannel::POSTAL)
        .await
        .unwrap();
    let error = lock_ballot_box(&waiting, &ids, &area, &VotingChannel::PAPER)
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
    lock_ballot_box(&retry, &ids, &area, &VotingChannel::PAPER)
        .await
        .unwrap();
    retry.rollback().await.unwrap();
}

#[tokio::test]
async fn lock_ballot_box_version_assignment_keys_the_lock_on_the_ids_as_written() {
    // The same ballot box written with an upper-case id takes a different lock.
    let pool = schema::pool().await;
    let ids = ids!();
    let area = ids.id(6).replace("-0000-", "-abcd-");
    let upper_case_area = area.to_uppercase();
    assert_eq!(
        Uuid::parse_str(&area).unwrap(),
        Uuid::parse_str(&upper_case_area).unwrap()
    );
    let mut holder = pool.get().await.unwrap();
    let held = holder.transaction().await.unwrap();
    lock_ballot_box(&held, &ids, &area, &VotingChannel::PAPER)
        .await
        .unwrap();

    let mut contender = pool.get().await.unwrap();
    let waiting = contender.transaction().await.unwrap();
    waiting
        .batch_execute("SET LOCAL lock_timeout = '10ms'")
        .await
        .unwrap();
    let result = lock_ballot_box(&waiting, &ids, &upper_case_area, &VotingChannel::PAPER).await;

    assert!(result.is_ok());
    waiting.rollback().await.unwrap();
    held.rollback().await.unwrap();
}

#[tokio::test]
async fn soft_delete_tally_sheet_leftover_versions_deletes_the_other_live_versions_of_the_box() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let paper = w.ballot_box();
    for (n, ballot_box, version) in [
        (10, paper, 1),
        (11, paper, 2),
        (12, paper, 3),
        (13, paper, 4),
        (
            14,
            BallotBox {
                channel: "POSTAL",
                ..paper
            },
            1,
        ),
        (
            15,
            BallotBox {
                area: &w.second_area,
                ..paper
            },
            1,
        ),
    ] {
        sheet_row(&tx, &w, ballot_box, &w.id(n), version).await;
    }
    set(&tx, "tally_sheet", &w.id(13), DELETED).await;
    let kept = tally_sheet::get_latest_ballot_box_tally_sheet(
        &tx,
        &w.tenant,
        &w.event,
        &w.election,
        &w.area,
        &w.contest,
        &VotingChannel::PAPER,
    )
    .await
    .unwrap()
    .unwrap();

    tally_sheet::soft_delete_tally_sheet_leftover_versions(&tx, &kept)
        .await
        .unwrap();

    let deletions: Vec<(String, Option<DateTime<Local>>)> = tx
        .query(
            "SELECT id::text, deleted_at FROM sequent_backend.tally_sheet
             WHERE tenant_id = $1::text::uuid ORDER BY id",
            &[&w.tenant],
        )
        .await
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    let started = now(&tx).await;
    assert_eq!(
        deletions,
        [
            (w.id(10), Some(started)),
            (w.id(11), Some(started)),
            (w.id(12), None),
            (w.id(13), Some(at("2026-01-01T12:00:00Z"))),
            (w.id(14), None),
            (w.id(15), None),
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn soft_delete_tally_sheet_leftover_versions_needs_the_sheet_channel() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(11), 2).await;
    let mut sheet = insert_sheet(&tx, &w, 3, None).await.unwrap();
    sheet.channel = None;

    let error = tally_sheet::soft_delete_tally_sheet_leftover_versions(&tx, &sheet)
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Channel is None");
    let live: i64 = tx
        .query_one(
            "SELECT count(*) FROM sequent_backend.tally_sheet
             WHERE tenant_id = $1::text::uuid AND deleted_at IS NULL",
            &[&w.tenant],
        )
        .await
        .unwrap()
        .get(0);
    assert_eq!(live, 3);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn review_tally_sheet_status_records_the_review_of_a_pending_sheet() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;

    let outcome = tally_sheet::review_tally_sheet_status(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        "reviewer-1",
        TallySheetStatus::APPROVED,
    )
    .await
    .unwrap();

    let ReviewTallySheetOutcome::Reviewed(reviewed) = outcome else {
        panic!("a pending sheet is reviewed");
    };
    let started = now(&tx).await;
    assert_eq!(
        (
            reviewed.id.as_str(),
            &reviewed.status,
            reviewed.reviewed_at,
            reviewed.reviewed_by_user_id.as_deref(),
            reviewed.last_updated_at
        ),
        (
            w.id(10).as_str(),
            &TallySheetStatus::APPROVED,
            Some(started),
            Some("reviewer-1"),
            Some(started)
        )
    );
    let row = stored(&tx, "tally_sheet", &w.id(10), &SHEET_TIMESTAMPS).await;
    assert_eq!(
        (&row["status"], &row["reviewed_by_user_id"]),
        (&json!("APPROVED"), &json!("reviewer-1"))
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn review_tally_sheet_status_leaves_an_already_reviewed_sheet_unchanged() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;
    set(&tx, "tally_sheet", &w.id(10), APPROVED).await;

    let outcome = tally_sheet::review_tally_sheet_status(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        "reviewer-2",
        TallySheetStatus::DISAPPROVED,
    )
    .await
    .unwrap();

    let ReviewTallySheetOutcome::NotPending(existing) = outcome else {
        panic!("an approved sheet is not pending");
    };
    assert_eq!(
        (
            &existing.status,
            existing.reviewed_by_user_id.as_deref(),
            existing.reviewed_at
        ),
        (
            &TallySheetStatus::APPROVED,
            Some("reviewer-1"),
            Some(at("2026-01-01T11:00:00Z"))
        )
    );
    assert_eq!(
        stored(&tx, "tally_sheet", &w.id(10), &SHEET_TIMESTAMPS).await["reviewed_by_user_id"],
        json!("reviewer-1")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn review_tally_sheet_status_reports_a_sheet_of_another_event_as_not_found() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;

    let outcome = tally_sheet::review_tally_sheet_status(
        &tx,
        &w.tenant,
        &w.other_event,
        &w.id(10),
        "reviewer-1",
        TallySheetStatus::APPROVED,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, ReviewTallySheetOutcome::NotFound));
    assert_eq!(
        stored(&tx, "tally_sheet", &w.id(10), &SHEET_TIMESTAMPS).await["status"],
        json!("PENDING")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn review_tally_sheet_status_reports_a_deleted_sheet_as_not_found() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(10), 1).await;
    set(&tx, "tally_sheet", &w.id(10), DELETED).await;

    let outcome = tally_sheet::review_tally_sheet_status(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        "reviewer-1",
        TallySheetStatus::APPROVED,
    )
    .await
    .unwrap();

    assert!(matches!(outcome, ReviewTallySheetOutcome::NotFound));
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn review_tally_sheet_status_rejects_an_invalid_tally_sheet_id() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let error = tally_sheet::review_tally_sheet_status(
        &tx,
        &w.tenant,
        &w.event,
        "sheet-1",
        "reviewer-1",
        TallySheetStatus::APPROVED,
    )
    .await
    .err()
    .unwrap();

    assert!(error
        .to_string()
        .starts_with("Error parsing tally_sheet_id as UUID: invalid UUID 'sheet-1'"));
    tx.rollback().await.unwrap();
}

// tally_sheet_import

fn summary() -> TallySheetImportSummary {
    TallySheetImportSummary {
        imported_ballot_box_count: 3,
        changed_ballot_box_count: 1,
        new_ballot_box_count: 1,
        unchanged_ballot_box_count: 1,
        conflicted_ballot_box_count: 0,
        validation_error_count: 0,
    }
}

fn summary_json() -> Value {
    json!({
        "imported_ballot_box_count": 3,
        "changed_ballot_box_count": 1,
        "new_ballot_box_count": 1,
        "unchanged_ballot_box_count": 1,
        "conflicted_ballot_box_count": 0,
        "validation_error_count": 0,
    })
}

/// A pending canonical-CSV import of paper sheets, with `summary()`.
async fn import_row(tx: &Transaction<'_>, w: &World, event: &str, id: &str) {
    tx.execute(
        "INSERT INTO sequent_backend.tally_sheet_import
             (id, tenant_id, election_event_id, source_document_id, source_format,
              selected_channel, created_by_user_id, summary)
         VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4::text::uuid,
                 'CANONICAL_CSV', 'PAPER', 'clerk-1', $5)",
        &[&id, &w.tenant, &event, &w.document, &summary_json()],
    )
    .await
    .unwrap();
}

fn item(w: &World, n: u32, import_id: &str, ballot_box: BallotBox<'_>) -> TallySheetImportItem {
    TallySheetImportItem {
        id: w.id(n),
        tenant_id: w.tenant.clone(),
        election_event_id: w.event.clone(),
        import_id: import_id.into(),
        election_id: ballot_box.election.into(),
        area_id: ballot_box.area.into(),
        contest_id: ballot_box.contest.into(),
        channel: ballot_box.channel.parse().unwrap(),
        generated_tally_sheet_id: None,
        baseline_approved_tally_sheet_id: None,
        baseline_approved_version: None,
        baseline_content_hash: None,
        incoming_content_hash: "incoming-hash".into(),
        change_type: TallySheetImportChangeType::NEW,
        status: TallySheetImportItemStatus::PENDING_REVIEW,
        previous_csv: None,
        incoming_csv: "field,value\ntotal_votes,10\n".into(),
        source_refs: None,
        validation_warnings: None,
        labels: None,
        annotations: None,
    }
}

/// The status of every import item of the tenant, by item id.
async fn item_statuses(tx: &Transaction<'_>, tenant: &str) -> Vec<(String, String)> {
    tx.query(
        "SELECT id::text, status FROM sequent_backend.tally_sheet_import_item
         WHERE tenant_id = $1::text::uuid ORDER BY id",
        &[&tenant],
    )
    .await
    .unwrap()
    .iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

#[tokio::test]
async fn insert_tally_sheet_import_returns_and_stores_the_import() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    let report = json!({"errors": []});
    let annotations = json!({"source": "ess"});

    let inserted = tally_sheet_import::insert_tally_sheet_import(
        &tx,
        &w.tenant,
        &w.event,
        &w.document,
        Some("sheets.xml"),
        Some("source-sha"),
        &TallySheetImportSourceFormat::ESS_ENHANCED_XML,
        &VotingChannel::POSTAL,
        &TallySheetImportStatus::PENDING_REVIEW,
        "clerk-1",
        &summary(),
        Some(&report),
        Some("canonical-sha"),
        Some(&annotations),
    )
    .await
    .unwrap();

    assert_eq!(Uuid::parse_str(&inserted.id).unwrap().get_version_num(), 4);
    assert_eq!(
        inserted,
        TallySheetImport {
            id: inserted.id.clone(),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            source_document_id: w.document.clone(),
            source_file_name: Some("sheets.xml".into()),
            source_sha256: Some("source-sha".into()),
            source_format: TallySheetImportSourceFormat::ESS_ENHANCED_XML,
            selected_channel: VotingChannel::POSTAL,
            status: TallySheetImportStatus::PENDING_REVIEW,
            created_by_user_id: "clerk-1".into(),
            labels: None,
            annotations: Some(annotations.clone()),
            summary: summary(),
            validation_report: Some(report.clone()),
            canonical_csv_sha256: Some("canonical-sha".into()),
        }
    );
    assert_eq!(
        stored(
            &tx,
            "tally_sheet_import",
            &inserted.id,
            &["created_at", "last_updated_at"]
        )
        .await,
        json!({
            "id": inserted.id,
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "source_document_id": w.document,
            "source_file_name": "sheets.xml",
            "source_sha256": "source-sha",
            "source_format": "ESS_ENHANCED_XML",
            "selected_channel": "POSTAL",
            "status": "PENDING_REVIEW",
            "created_by_user_id": "clerk-1",
            "annotations": annotations,
            "labels": null,
            "summary": summary_json(),
            "validation_report": report,
            "canonical_csv_sha256": "canonical-sha",
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_sheet_import_without_optional_values_stores_nulls() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    let inserted = tally_sheet_import::insert_tally_sheet_import(
        &tx,
        &w.tenant,
        &w.event,
        &w.document,
        None,
        None,
        &TallySheetImportSourceFormat::CANONICAL_CSV,
        &VotingChannel::PAPER,
        &TallySheetImportStatus::FAILED_VALIDATION,
        "clerk-1",
        &TallySheetImportSummary::default(),
        None,
        None,
        None,
    )
    .await
    .unwrap();

    let row = stored(
        &tx,
        "tally_sheet_import",
        &inserted.id,
        &["created_at", "last_updated_at"],
    )
    .await;
    assert_eq!(
        (
            &row["source_file_name"],
            &row["source_sha256"],
            &row["annotations"],
            &row["validation_report"],
            &row["canonical_csv_sha256"],
            &row["status"]
        ),
        (
            &Value::Null,
            &Value::Null,
            &Value::Null,
            &Value::Null,
            &Value::Null,
            &json!("FAILED_VALIDATION")
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sheet_import_by_id_returns_the_import_of_the_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;

    let found =
        tally_sheet_import::get_tally_sheet_import_by_id(&tx, &w.tenant, &w.event, &w.id(10))
            .await
            .unwrap();

    assert_eq!(
        found,
        Some(TallySheetImport {
            id: w.id(10),
            tenant_id: w.tenant.clone(),
            election_event_id: w.event.clone(),
            source_document_id: w.document.clone(),
            source_file_name: None,
            source_sha256: None,
            source_format: TallySheetImportSourceFormat::CANONICAL_CSV,
            selected_channel: VotingChannel::PAPER,
            // The column default.
            status: TallySheetImportStatus::PENDING_REVIEW,
            created_by_user_id: "clerk-1".into(),
            labels: None,
            annotations: None,
            summary: summary(),
            validation_report: None,
            canonical_csv_sha256: None,
        })
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sheet_import_by_id_is_none_for_an_import_of_another_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.other_event, &w.id(10)).await;

    let found =
        tally_sheet_import::get_tally_sheet_import_by_id(&tx, &w.tenant, &w.event, &w.id(10))
            .await
            .unwrap();

    assert_eq!(found, None);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sheet_import_by_id_fails_on_an_unknown_status() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    set(&tx, "tally_sheet_import", &w.id(10), "status = 'ABANDONED'").await;

    let error =
        tally_sheet_import::get_tally_sheet_import_by_id(&tx, &w.tenant, &w.event, &w.id(10))
            .await
            .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Invalid import status: Matching variant not found"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sheet_import_by_id_cannot_read_the_default_empty_summary() {
    // The column defaults to '{}', but every summary counter is required.
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    set(&tx, "tally_sheet_import", &w.id(10), "summary = DEFAULT").await;

    let error =
        tally_sheet_import::get_tally_sheet_import_by_id(&tx, &w.tenant, &w.event, &w.id(10))
            .await
            .unwrap_err();

    assert_eq!(
        error.to_string(),
        "missing field `imported_ballot_box_count`"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_sheet_import_items_stores_every_item_with_its_columns() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(11), 1).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(12), 2).await;
    let changed = TallySheetImportItem {
        generated_tally_sheet_id: Some(w.id(12)),
        baseline_approved_tally_sheet_id: Some(w.id(11)),
        baseline_approved_version: Some(1),
        baseline_content_hash: Some("baseline-hash".into()),
        change_type: TallySheetImportChangeType::CHANGED,
        previous_csv: Some("field,value\ntotal_votes,9\n".into()),
        source_refs: Some(json!({"row": 3})),
        validation_warnings: Some(json!(["census missing"])),
        labels: Some(json!({"l": 1})),
        annotations: Some(json!({"a": 1})),
        ..item(&w, 20, &w.id(10), w.ballot_box())
    };
    let new = item(
        &w,
        21,
        &w.id(10),
        BallotBox {
            channel: "POSTAL",
            ..w.ballot_box()
        },
    );

    tally_sheet_import::insert_tally_sheet_import_items(&tx, &[changed, new])
        .await
        .unwrap();

    let timestamps = ["created_at", "last_updated_at"];
    assert_eq!(
        stored(&tx, "tally_sheet_import_item", &w.id(20), &timestamps).await,
        json!({
            "id": w.id(20),
            "tenant_id": w.tenant,
            "election_event_id": w.event,
            "import_id": w.id(10),
            "election_id": w.election,
            "area_id": w.area,
            "contest_id": w.contest,
            "channel": "PAPER",
            "generated_tally_sheet_id": w.id(12),
            "baseline_approved_tally_sheet_id": w.id(11),
            "baseline_approved_version": 1,
            "baseline_content_hash": "baseline-hash",
            "incoming_content_hash": "incoming-hash",
            "change_type": "CHANGED",
            "status": "PENDING_REVIEW",
            "previous_csv": "field,value\ntotal_votes,9\n",
            "incoming_csv": "field,value\ntotal_votes,10\n",
            "source_refs": {"row": 3},
            "validation_warnings": ["census missing"],
            "annotations": {"a": 1},
            "labels": {"l": 1},
        })
    );
    let second = stored(&tx, "tally_sheet_import_item", &w.id(21), &timestamps).await;
    assert_eq!(
        (
            &second["channel"],
            &second["generated_tally_sheet_id"],
            &second["change_type"],
            &second["previous_csv"]
        ),
        (&json!("POSTAL"), &Value::Null, &json!("NEW"), &Value::Null)
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_sheet_import_items_with_no_items_writes_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;

    tally_sheet_import::insert_tally_sheet_import_items(&tx, &[])
        .await
        .unwrap();

    assert_eq!(count(&tx, "tally_sheet_import_item", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn insert_tally_sheet_import_items_rejects_an_invalid_generated_sheet_id_before_writing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    let invalid = TallySheetImportItem {
        generated_tally_sheet_id: Some("sheet-1".into()),
        ..item(
            &w,
            21,
            &w.id(10),
            BallotBox {
                channel: "POSTAL",
                ..w.ballot_box()
            },
        )
    };

    let error = tally_sheet_import::insert_tally_sheet_import_items(
        &tx,
        &[item(&w, 20, &w.id(10), w.ballot_box()), invalid],
    )
    .await
    .unwrap_err();

    assert!(error.to_string().starts_with("invalid UUID 'sheet-1'"));
    assert_eq!(count(&tx, "tally_sheet_import_item", &w.tenant).await, 0);
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sheet_import_items_for_review_orders_the_import_items_by_ballot_box() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    import_row(&tx, &w, &w.event, &w.id(11)).await;
    let paper = w.ballot_box();
    tally_sheet_import::insert_tally_sheet_import_items(
        &tx,
        &[
            item(
                &w,
                20,
                &w.id(10),
                BallotBox {
                    area: &w.second_area,
                    ..paper
                },
            ),
            item(
                &w,
                21,
                &w.id(10),
                BallotBox {
                    contest: &w.second_contest,
                    ..paper
                },
            ),
            item(
                &w,
                22,
                &w.id(10),
                BallotBox {
                    channel: "POSTAL",
                    ..paper
                },
            ),
            item(
                &w,
                23,
                &w.id(10),
                BallotBox {
                    channel: "IN_PERSON",
                    ..paper
                },
            ),
            item(&w, 24, &w.id(11), paper),
        ],
    )
    .await
    .unwrap();

    let items = tally_sheet_import::get_tally_sheet_import_items_for_review(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
    )
    .await
    .unwrap();

    // Area, then contest, then channel name.
    assert_eq!(
        items
            .into_iter()
            .map(|item| (item.id, item.channel))
            .collect::<Vec<_>>(),
        [
            (w.id(23), VotingChannel::IN_PERSON),
            (w.id(22), VotingChannel::POSTAL),
            (w.id(21), VotingChannel::PAPER),
            (w.id(20), VotingChannel::PAPER),
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sheet_import_items_for_review_maps_the_baseline_of_an_item() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(11), 1).await;
    sheet_row(&tx, &w, w.ballot_box(), &w.id(12), 2).await;
    tally_sheet_import::insert_tally_sheet_import_items(
        &tx,
        &[TallySheetImportItem {
            generated_tally_sheet_id: Some(w.id(12)),
            baseline_approved_tally_sheet_id: Some(w.id(11)),
            baseline_approved_version: Some(1),
            baseline_content_hash: Some("baseline-hash".into()),
            status: TallySheetImportItemStatus::CONFLICTED,
            ..item(&w, 20, &w.id(10), w.ballot_box())
        }],
    )
    .await
    .unwrap();

    let items = tally_sheet_import::get_tally_sheet_import_items_for_review(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
    )
    .await
    .unwrap();

    let [item] = items.as_slice() else {
        panic!("one item");
    };
    assert_eq!(
        (
            item.id.as_str(),
            item.election_id.as_str(),
            item.area_id.as_str(),
            item.contest_id.as_str(),
            &item.channel,
            item.generated_tally_sheet_id.as_deref(),
            item.baseline_approved_tally_sheet_id.as_deref(),
            item.baseline_approved_version,
            item.baseline_content_hash.as_deref(),
            &item.status,
        ),
        (
            w.id(20).as_str(),
            w.election.as_str(),
            w.area.as_str(),
            w.contest.as_str(),
            &VotingChannel::PAPER,
            Some(w.id(12).as_str()),
            Some(w.id(11).as_str()),
            Some(1),
            Some("baseline-hash"),
            &TallySheetImportItemStatus::CONFLICTED,
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn get_tally_sheet_import_items_for_review_fails_on_an_unknown_channel() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    tally_sheet_import::insert_tally_sheet_import_items(
        &tx,
        &[item(&w, 20, &w.id(10), w.ballot_box())],
    )
    .await
    .unwrap();
    set(
        &tx,
        "tally_sheet_import_item",
        &w.id(20),
        "channel = 'EMAIL'",
    )
    .await;

    let error = tally_sheet_import::get_tally_sheet_import_items_for_review(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Invalid import item channel: Matching variant not found"
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_sheet_import_status_returns_the_updated_import() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    import_row(&tx, &w, &w.event, &w.id(11)).await;
    set(
        &tx,
        "tally_sheet_import",
        &w.id(10),
        "last_updated_at = '2026-01-01T10:00:00Z'",
    )
    .await;

    let updated = tally_sheet_import::update_tally_sheet_import_status(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        &TallySheetImportStatus::APPROVED,
    )
    .await
    .unwrap();

    assert_eq!(
        (updated.id.as_str(), &updated.status, &updated.summary),
        (
            w.id(10).as_str(),
            &TallySheetImportStatus::APPROVED,
            &summary()
        )
    );
    let row = tx
        .query_one(
            "SELECT status, last_updated_at FROM sequent_backend.tally_sheet_import
             WHERE id = $1::text::uuid",
            &[&w.id(10)],
        )
        .await
        .unwrap();
    assert_eq!(
        (row.get::<_, String>(0), row.get::<_, DateTime<Local>>(1)),
        ("APPROVED".into(), now(&tx).await)
    );
    assert_eq!(
        stored(
            &tx,
            "tally_sheet_import",
            &w.id(11),
            &["created_at", "last_updated_at"]
        )
        .await["status"],
        json!("PENDING_REVIEW")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_sheet_import_status_fails_for_an_import_of_another_event() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.other_event, &w.id(10)).await;

    let error = tally_sheet_import::update_tally_sheet_import_status(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        &TallySheetImportStatus::APPROVED,
    )
    .await
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Unexpected tally_sheet_import rows affected 0"
    );
    assert_eq!(
        stored(
            &tx,
            "tally_sheet_import",
            &w.id(10),
            &["created_at", "last_updated_at"]
        )
        .await["status"],
        json!("PENDING_REVIEW")
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_sheet_import_items_status_updates_every_item_of_the_import_only() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    import_row(&tx, &w, &w.event, &w.id(11)).await;
    let paper = w.ballot_box();
    tally_sheet_import::insert_tally_sheet_import_items(
        &tx,
        &[
            item(&w, 20, &w.id(10), paper),
            item(
                &w,
                21,
                &w.id(10),
                BallotBox {
                    channel: "POSTAL",
                    ..paper
                },
            ),
            item(&w, 22, &w.id(11), paper),
        ],
    )
    .await
    .unwrap();

    tally_sheet_import::update_tally_sheet_import_items_status(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        &TallySheetImportItemStatus::APPROVED,
    )
    .await
    .unwrap();

    assert_eq!(
        item_statuses(&tx, &w.tenant).await,
        [
            (w.id(20), "APPROVED".into()),
            (w.id(21), "APPROVED".into()),
            (w.id(22), "PENDING_REVIEW".into()),
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_sheet_import_status_with_conflict_count_records_the_count_in_the_summary() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;

    let updated = tally_sheet_import::update_tally_sheet_import_status_with_conflict_count(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        &TallySheetImportStatus::CONFLICTED,
        2,
    )
    .await
    .unwrap();

    let expected = TallySheetImportSummary {
        conflicted_ballot_box_count: 2,
        ..summary()
    };
    assert_eq!(
        (&updated.status, &updated.summary),
        (&TallySheetImportStatus::CONFLICTED, &expected)
    );
    let row = stored(
        &tx,
        "tally_sheet_import",
        &w.id(10),
        &["created_at", "last_updated_at"],
    )
    .await;
    assert_eq!(
        (&row["status"], &row["summary"]),
        (
            &json!("CONFLICTED"),
            &json!({
                "imported_ballot_box_count": 3,
                "changed_ballot_box_count": 1,
                "new_ballot_box_count": 1,
                "unchanged_ballot_box_count": 1,
                "conflicted_ballot_box_count": 2,
                "validation_error_count": 0,
            })
        )
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_sheet_import_items_status_by_ids_updates_only_the_listed_items_of_the_import()
{
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    import_row(&tx, &w, &w.event, &w.id(11)).await;
    let paper = w.ballot_box();
    tally_sheet_import::insert_tally_sheet_import_items(
        &tx,
        &[
            item(&w, 20, &w.id(10), paper),
            item(
                &w,
                21,
                &w.id(10),
                BallotBox {
                    channel: "POSTAL",
                    ..paper
                },
            ),
            item(&w, 22, &w.id(11), paper),
        ],
    )
    .await
    .unwrap();

    tally_sheet_import::update_tally_sheet_import_items_status_by_ids(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        &[w.id(20), w.id(22)],
        &TallySheetImportItemStatus::DISAPPROVED,
    )
    .await
    .unwrap();

    assert_eq!(
        item_statuses(&tx, &w.tenant).await,
        [
            (w.id(20), "DISAPPROVED".into()),
            (w.id(21), "PENDING_REVIEW".into()),
            (w.id(22), "PENDING_REVIEW".into()),
        ]
    );
    tx.rollback().await.unwrap();
}

#[tokio::test]
async fn update_tally_sheet_import_items_status_by_ids_without_ids_changes_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let w = World::new(&tx, ids!()).await;
    import_row(&tx, &w, &w.event, &w.id(10)).await;
    tally_sheet_import::insert_tally_sheet_import_items(
        &tx,
        &[item(&w, 20, &w.id(10), w.ballot_box())],
    )
    .await
    .unwrap();

    tally_sheet_import::update_tally_sheet_import_items_status_by_ids(
        &tx,
        &w.tenant,
        &w.event,
        &w.id(10),
        &[],
        &TallySheetImportItemStatus::APPROVED,
    )
    .await
    .unwrap();

    assert_eq!(
        item_statuses(&tx, &w.tenant).await,
        [(w.id(20), "PENDING_REVIEW".into())]
    );
    tx.rollback().await.unwrap();
}
