// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The console's pages and queries against a real server, in the database of
//! `ELECTORAL_LOG_TEST_DATABASE_URL`.

use anyhow::Result;
use electoral_log::adapters::ballot_box::{AcceptBallot, AcceptOutcome, BallotStatus};
use electoral_log::adapters::console::{
    run_read_only_query, ConsoleFilters, ConsoleTable, PageOrder, PageRequest,
};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::ports::ElectoralLogStore;
use electoral_log::{ElectoralLogMessage, LogEntry};
use serde_json::{json, Value};
use std::time::Duration;
use uuid::Uuid;

/// Applying the schema takes locks that deadlock with the appends of tests running
/// at the same time, so each test binary applies it once.
static SCHEMA: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

async fn initialized(store: &PostgresStore) -> Result<()> {
    SCHEMA.get_or_try_init(|| store.initialize()).await?;
    Ok(())
}

async fn store() -> Result<PostgresStore> {
    let config = std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?;
    let store = PostgresStore::new(config)?;
    initialized(&store).await?;
    Ok(store)
}

fn record(kind: &str, index: i64) -> LogEntry {
    LogEntry {
        delivery_id: format!("delivery-{index}"),
        message: ElectoralLogMessage {
            id: 0,
            created: 1_000 + index,
            sender_pk: "sender".into(),
            statement_timestamp: 1_000 + index,
            statement_kind: kind.into(),
            message: vec![1, 2, 3],
            version: "2".into(),
            user_id: Some(format!("user-{index}")),
            username: Some("someone".into()),
            election_id: None,
            area_id: None,
            ballot_id: None,
        },
    }
}

fn request<'a>(
    table: ConsoleTable,
    board: &'a str,
    event: &'a str,
    filters: &'a ConsoleFilters,
    after: Option<&'a str>,
) -> PageRequest<'a> {
    PageRequest {
        table,
        board,
        election_event_id: event,
        filters,
        order: PageOrder::NewestFirst,
        after,
        limit: 2,
    }
}

fn column(page_columns: &[String], name: &str) -> usize {
    page_columns
        .iter()
        .position(|column| column == name)
        .unwrap()
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn records_are_paged_newest_first_and_filtered() -> Result<()> {
    let store = store().await?;
    let board = format!("consoletest{}", Uuid::new_v4().simple());
    store.create_board(&board).await?;
    let kinds = [
        "CastVote",
        "KeycloakUserEvent",
        "CastVote",
        "CastVote",
        "Admin",
    ];
    store
        .append(
            &board,
            &mut kinds
                .iter()
                .enumerate()
                .map(|(index, kind)| anyhow::Ok(record(kind, index as i64))),
        )
        .await?;

    let none = ConsoleFilters::default();
    let first = store
        .console_page(&request(ConsoleTable::Records, &board, "", &none, None))
        .await?;
    assert_eq!(first.estimated_rows, 5);
    let kind = column(&first.rows.columns, "kind");
    let position = column(&first.rows.columns, "position");
    assert_eq!(
        first
            .rows
            .rows
            .iter()
            .map(|row| row[kind].clone())
            .collect::<Vec<_>>(),
        [json!("Admin"), json!("CastVote")]
    );
    assert!(first.rows.rows[0][position].as_i64() > first.rows.rows[1][position].as_i64());
    assert!(first.personal_columns.contains(&"username".to_string()));
    let second = store
        .console_page(&request(
            ConsoleTable::Records,
            &board,
            "",
            &none,
            first.next.as_deref(),
        ))
        .await?;
    assert_eq!(second.rows.rows.len(), 2);
    assert!(second.rows.rows[0][position].as_i64() < first.rows.rows[1][position].as_i64());

    let cast_votes = ConsoleFilters {
        statement_kind: Some("CastVote".into()),
        created_after: Some(1_002),
        ..Default::default()
    };
    let filtered = store
        .console_page(&PageRequest {
            limit: 10,
            ..request(ConsoleTable::Records, &board, "", &cast_votes, None)
        })
        .await?;
    let user = column(&filtered.rows.columns, "user_id");
    assert_eq!(
        filtered
            .rows
            .rows
            .iter()
            .map(|row| row[user].clone())
            .collect::<Vec<_>>(),
        [json!("user-3"), json!("user-2")]
    );
    assert!(filtered.next.is_none());

    let found = store
        .console_record(&board, first.rows.rows[0][position].as_i64().unwrap())
        .await?
        .unwrap();
    assert_eq!(found.statement_kind, "Admin");
    assert_eq!(found.delivery_id, "delivery-4");
    assert!(store.console_record(&board, -1).await?.is_none());
    store.delete_board(&board).await
}

fn vote<'a>(
    event: &'a str,
    election: &'a str,
    area: &'a str,
    voter: &'a str,
    ballot: &'a str,
) -> AcceptBallot<'a> {
    AcceptBallot {
        election_event_id: event,
        election_id: election,
        area_id: area,
        voter_id: voter,
        ballot_id: ballot,
        format: "test",
        content: "ciphertext",
        voter_signature: None,
        pseudonym_hash: &[1; 64],
        ballot_hash: &[2; 64],
        voting_channel: "ONLINE",
        status: BallotStatus::Valid,
        voter_ip: Some("192.0.2.1"),
        voter_country: Some("ES"),
        username: Some("voter"),
        allowed_votes: 0,
    }
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn the_ballot_box_is_paged_by_its_keys() -> Result<()> {
    let store = store().await?;
    let event = Uuid::new_v4().to_string();
    let election = Uuid::new_v4().to_string();
    let (area, other_area) = (Uuid::new_v4().to_string(), Uuid::new_v4().to_string());
    store.create_ballot_box(&event).await?;
    for (voter, area, ballot) in [
        ("voter-a", &area, "b1"),
        ("voter-b", &area, "b2"),
        ("voter-c", &other_area, "b3"),
    ] {
        assert!(matches!(
            store
                .accept_ballot(&vote(&event, &election, area, voter, ballot))
                .await?,
            AcceptOutcome::Accepted { .. }
        ));
    }

    let in_area = ConsoleFilters {
        area_id: Some(area.clone()),
        ..Default::default()
    };
    let ballots = store
        .console_page(&request(ConsoleTable::Ballots, "", &event, &in_area, None))
        .await?;
    let ballot_id = column(&ballots.rows.columns, "ballot_id");
    assert_eq!(
        ballots
            .rows
            .rows
            .iter()
            .map(|row| row[ballot_id].clone())
            .collect::<Vec<_>>(),
        [json!("b2"), json!("b1")]
    );
    assert_eq!(ballots.estimated_rows, 3);
    for personal in ["username", "voter_ip", "voter_country"] {
        assert!(ballots.personal_columns.contains(&personal.to_string()));
    }
    assert!(!ballots.rows.columns.contains(&"content".to_string()));

    let none = ConsoleFilters::default();
    let first = store
        .console_page(&PageRequest {
            limit: 1,
            order: PageOrder::OldestFirst,
            ..request(ConsoleTable::Voters, "", &event, &none, None)
        })
        .await?;
    let voter = column(&first.rows.columns, "voter_id");
    assert_eq!(first.rows.rows[0][voter], json!("voter-a"));
    let next = first.next.clone().unwrap();
    assert_eq!(next, format!("{election}/voter-a"));
    let second = store
        .console_page(&PageRequest {
            limit: 1,
            order: PageOrder::OldestFirst,
            ..request(ConsoleTable::Voters, "", &event, &none, Some(&next))
        })
        .await?;
    assert_eq!(second.rows.rows[0][voter], json!("voter-b"));

    let queue = store
        .console_page(&PageRequest {
            limit: 10,
            ..request(ConsoleTable::Queue, "", &event, &none, None)
        })
        .await?;
    assert_eq!(queue.rows.rows.len(), 3);
    assert_eq!(queue.estimated_rows, 3);
    assert!(store
        .console_page(&request(
            ConsoleTable::Ballots,
            "",
            "not-an-event",
            &none,
            None
        ))
        .await
        .is_err());

    let suffix = event.replace('-', "");
    store
        .client()
        .await?
        .batch_execute(&format!(
            "DELETE FROM ballot_box_pending WHERE election_event_id = '{event}';
             DROP TABLE ballot_box_ballot_{suffix}, ballot_box_voter_{suffix};"
        ))
        .await?;
    Ok(())
}

#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn queries_only_read_within_their_limits() -> Result<()> {
    let store = store().await?;
    let mut client = store.client().await?;
    let second = Duration::from_secs(1);

    let result = run_read_only_query(&mut client, "SELECT 1 AS a, 'x' AS b;\n", 10, second).await?;
    assert_eq!(result.rows.columns, ["a", "b"]);
    assert_eq!(result.rows.rows, [vec![json!(1), json!("x")]]);
    assert!(!result.truncated);

    let many = run_read_only_query(
        &mut client,
        "SELECT g FROM generate_series(1, 20) g",
        5,
        second,
    )
    .await?;
    assert_eq!(many.rows.rows.len(), 5);
    assert!(many.truncated);

    let empty = run_read_only_query(&mut client, "SELECT 1 AS a WHERE false", 5, second).await?;
    assert_eq!(empty.rows.columns, ["a"]);
    assert!(empty.rows.rows.is_empty());

    for refused in [
        "",
        "SELECT 1; SELECT 2",
        "DELETE FROM ballot_box_pending",
        "WITH d AS (DELETE FROM ballot_box_pending RETURNING *) SELECT * FROM d",
        "SELECT nextval('ballot_box_seq')",
        "SELECT * FROM ballot_box_pending FOR UPDATE",
        "SELECT pg_sleep(5)",
    ] {
        assert!(
            run_read_only_query(&mut client, refused, 5, Duration::from_millis(300))
                .await
                .is_err(),
            "{refused:?} was not refused"
        );
    }
    let delete = run_read_only_query(&mut client, "DELETE FROM ballot_box_pending", 5, second)
        .await
        .unwrap_err()
        .to_string();
    assert!(delete.contains("Only queries that return rows"), "{delete}");
    let timeout = run_read_only_query(
        &mut client,
        "SELECT pg_sleep(5)",
        5,
        Duration::from_millis(300),
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(timeout.contains("statement timeout"), "{timeout}");

    // The connection still works and is back to read-write after a refused query.
    let row = client
        .query_one("SELECT current_setting('transaction_read_only')", &[])
        .await?;
    assert_eq!(row.get::<_, String>(0), "off");
    assert_eq!(
        run_read_only_query(&mut client, "SELECT 2 AS n", 1, second)
            .await?
            .rows
            .rows,
        [vec![Value::from(2)]]
    );
    Ok(())
}
