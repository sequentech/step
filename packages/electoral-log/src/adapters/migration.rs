// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Moving election events out of a database that holds several, as the single
//! electoral-log database did, into a database per event.
//!
//! Each board's records are appended to the event's database in log order, so they
//! are committed again under a new log identity: checkpoints published of the old
//! logs do not verify against the new ones. The ballot box's rows are copied as they
//! are. An event is deleted from the source only once its copy has every record and
//! ballot and its audit is clean. A move that stops is run again: records and ballots
//! already copied are skipped.
//!
//! Run a move with Windmill and Harvest stopped: a record appended or a ballot accepted
//! in the source while its event is moved would be deleted with the source's copy.

use super::{
    events::{event_of_board, ActivityMark, EventDatabases},
    postgres::{decode, PostgresStore, COLUMNS},
};
use crate::{domain::LogEntry, ports::ElectoralLogStore};
use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use tokio_postgres::types::ToSql;

/// Records copied per append.
const MOVE_PAGE: i64 = 5_000;
/// Ballot-box rows copied per statement.
const ROW_PAGE: i64 = 1_000;
/// Connections to the source database.
const SOURCE_POOL_SIZE: usize = 2;
/// Text before the tenant in an event's board name.
const BOARD_TENANT_MARKER: &str = "tenant";
/// Text between the tenant and the event in an event's board name.
const BOARD_EVENT_MARKER: &str = "event";
/// Ballot-box tables, with the columns of their keys, in the order they are copied.
const BALLOT_BOX_TABLES: [(&str, &str); 4] = [
    ("ballot_box_ballot", "seq"),
    ("ballot_box_voter", "election_id, voter_id"),
    ("ballot_box_pending", "seq"),
    ("ballot_box_sequencer", "election_event_id"),
];

/// An event moved to its own database.
#[derive(Debug, Clone, Serialize)]
pub struct MovedEvent {
    pub election_event_id: String,
    pub tenant_id: String,
    /// Records moved, by board.
    pub records: BTreeMap<String, i64>,
    /// Rows moved, by ballot-box table.
    pub ballot_box: BTreeMap<String, i64>,
}

/// An event left in the source, and why.
#[derive(Debug, Clone, Serialize)]
pub struct SkippedEvent {
    pub election_event_id: String,
    pub reason: String,
}

/// What a move did.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MoveReport {
    pub moved: Vec<MovedEvent>,
    pub skipped: Vec<SkippedEvent>,
}

/// The tenant whose ID begins with the tenant part of a board name.
fn tenant_of_board<'a>(board: &str, tenants: &'a [String]) -> Result<&'a str> {
    let end = board
        .rfind(BOARD_EVENT_MARKER)
        .context("The board names no event")?;
    let start = board[..end]
        .rfind(BOARD_TENANT_MARKER)
        .context("The board names no tenant")?
        + BOARD_TENANT_MARKER.len();
    let prefix = &board[start..end];
    ensure!(!prefix.is_empty(), "The board names no tenant");
    let matching: Vec<&String> = tenants
        .iter()
        .filter(|tenant| tenant.replace('-', "").starts_with(prefix))
        .collect();
    match matching.as_slice() {
        [tenant] => Ok(tenant.as_str()),
        [] => bail!("No tenant ID begins with {prefix}"),
        _ => bail!("Several tenant IDs begin with {prefix}"),
    }
}

/// Count the rows of an event in a table.
async fn count_rows(store: &PostgresStore, table: &str, event: &str) -> Result<i64> {
    Ok(store
        .client()
        .await?
        .query_one(
            &format!("SELECT count(*) FROM {table} WHERE election_event_id = $1::text::uuid"),
            &[&event],
        )
        .await?
        .try_get(0)?)
}

/// Append a board's records to the target in the source's log order, skipping those
/// already copied. Returns how many the source has.
async fn copy_board(source: &PostgresStore, target: &PostgresStore, board: &str) -> Result<i64> {
    target.create_board(board).await?;
    let client = source.client().await?;
    let log_id: i64 = client
        .query_one("SELECT id FROM trellis_logs WHERE name = $1", &[&board])
        .await?
        .try_get(0)?;
    let page = client
        .prepare(&format!(
            "SELECT {COLUMNS}, delivery_id, e.leaf_index FROM trellis_leaves e \
             JOIN electoral_log_messages m ON m.id = e.source_id \
             WHERE e.log_id = $1 AND e.leaf_index >= $2 ORDER BY e.leaf_index LIMIT {MOVE_PAGE}"
        ))
        .await?;
    let mut next = 0_i64;
    loop {
        let rows = client.query(&page, &[&log_id, &next]).await?;
        if rows.is_empty() {
            break;
        }
        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let index: i64 = row.try_get("leaf_index")?;
            ensure!(index == next, "Board {board} has no leaf at {next}");
            next += 1;
            entries.push(LogEntry {
                delivery_id: row.try_get("delivery_id")?,
                message: decode(row)?,
            });
        }
        target
            .append(board, &mut entries.into_iter().map(anyhow::Ok))
            .await?;
    }
    Ok(next)
}

/// Copy an event's rows of a ballot-box table, skipping those already copied, in key
/// order. Returns how many the source has.
async fn copy_table(
    source: &PostgresStore,
    target: &PostgresStore,
    table: &str,
    key: &str,
    event: &str,
) -> Result<i64> {
    let source_client = source.client().await?;
    let target_client = target.client().await?;
    let mut copied = 0;
    let mut offset = 0_i64;
    loop {
        let params: [&(dyn ToSql + Sync); 2] = [&event, &offset];
        let rows: Option<String> = source_client
            .query_one(
                &format!(
                    "SELECT json_agg(t)::text FROM (SELECT * FROM {table} \
                     WHERE election_event_id = $1::text::uuid ORDER BY {key} \
                     OFFSET $2 LIMIT {ROW_PAGE}) t"
                ),
                &params,
            )
            .await?
            .try_get(0)?;
        let Some(rows) = rows else { break };
        let count: i64 = target_client
            .query_one(
                &format!(
                    "WITH added AS (INSERT INTO {table} \
                     SELECT * FROM json_populate_recordset(NULL::{table}, $1::text::json) \
                     ON CONFLICT DO NOTHING RETURNING 1) SELECT json_array_length($1::text::json)::bigint"
                ),
                &[&rows],
            )
            .await?
            .try_get(0)?;
        copied += count;
        offset += ROW_PAGE;
        if count < ROW_PAGE {
            break;
        }
    }
    Ok(copied)
}

/// Move an event's boards and ballot box from `source` into its own database.
async fn move_event(
    source: &PostgresStore,
    databases: &EventDatabases,
    tenant: &str,
    event: &str,
    boards: &[String],
) -> Result<MovedEvent> {
    let target = databases.create_event(tenant, event).await?;
    let mut moved = MovedEvent {
        election_event_id: event.to_string(),
        tenant_id: tenant.to_string(),
        records: BTreeMap::new(),
        ballot_box: BTreeMap::new(),
    };
    for board in boards {
        let records = copy_board(source, &target, board).await?;
        let stored = target.count(board, &Default::default()).await?;
        ensure!(
            stored == records,
            "Board {board} has {records} records in the source and {stored} in the copy"
        );
        let report = target.audit(board, &[]).await?;
        ensure!(
            report.is_clean(),
            "The copy of board {board} does not audit clean: {:?}",
            report.findings()
        );
        moved.records.insert(board.clone(), records);
    }
    if source.has_ballot_box(event).await? {
        target.create_ballot_box(event).await?;
        for (table, key) in BALLOT_BOX_TABLES {
            let rows = copy_table(source, &target, table, key, event).await?;
            let stored = count_rows(&target, table, event).await?;
            ensure!(
                stored == rows,
                "Table {table} has {rows} rows of the event in the source and {stored} in the copy"
            );
            moved.ballot_box.insert(table.to_string(), rows);
        }
        target
            .client()
            .await?
            .execute(
                "SELECT setval('ballot_box_seq', greatest((SELECT max(seq) FROM ballot_box_ballot), 1))",
                &[],
            )
            .await?;
        // Ballots waiting for the sequencer or for review are found by the mark.
        databases
            .mark_ballot_activity(event, ActivityMark::Immediate)
            .await?;
    }
    if source.has_ballot_box(event).await? {
        source.drop_ballot_box(event).await?;
    }
    for board in boards {
        source.delete_board(board).await?;
    }
    Ok(moved)
}

/// Move every event of `source_database`, a database of the server that holds
/// several, into a database of its own. `tenants` lists every tenant ID: a board's
/// name names its tenant by the start of its ID. An election event's database is
/// refused as the source.
pub async fn move_to_event_databases(
    databases: &EventDatabases,
    source_database: &str,
    tenants: &[String],
) -> Result<MoveReport> {
    ensure!(
        !databases.is_event_database(source_database),
        "{source_database} is an election event's database"
    );
    let source = databases
        .connection()
        .store_of(source_database, SOURCE_POOL_SIZE)?;
    let report = move_events(&source, databases, tenants).await;
    source.close();
    report
}

async fn move_events(
    source: &PostgresStore,
    databases: &EventDatabases,
    tenants: &[String],
) -> Result<MoveReport> {
    let names: Vec<String> = source
        .client()
        .await?
        .query(
            "SELECT b.board_name FROM electoral_log_boards b \
             JOIN trellis_logs l ON l.name = b.board_name ORDER BY l.id",
            &[],
        )
        .await?
        .iter()
        .map(|row| row.try_get(0))
        .collect::<Result<_, _>>()?;
    let mut events: BTreeMap<String, (Result<String, String>, Vec<String>)> = BTreeMap::new();
    let mut report = MoveReport::default();
    for name in names {
        let event = match event_of_board(&name) {
            Ok(event) => event,
            Err(error) => {
                report.skipped.push(SkippedEvent {
                    election_event_id: String::new(),
                    reason: format!("Board {name}: {error:#}"),
                });
                continue;
            }
        };
        let tenant = tenant_of_board(&name, tenants)
            .map(str::to_string)
            .map_err(|error| format!("Board {name}: {error:#}"));
        let entry = events
            .entry(event)
            .or_insert_with(|| (tenant.clone(), Vec::new()));
        if entry.0 != tenant {
            entry.0 = Err(format!(
                "Board {name} names another tenant than the event's other boards"
            ));
        }
        entry.1.push(name);
    }
    for (event, (tenant, boards)) in events {
        let result = match tenant {
            Ok(tenant) => move_event(source, databases, &tenant, &event, &boards).await,
            Err(reason) => Err(anyhow::anyhow!(reason)),
        };
        match result {
            Ok(moved) => report.moved.push(moved),
            Err(error) => report.skipped.push(SkippedEvent {
                election_event_id: event,
                reason: format!("{error:#}"),
            }),
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boards_name_their_tenant_by_the_start_of_its_id() {
        let tenants = vec![
            "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string(),
            "4e593654-9902-42de-ba97-fadd98e9a2c2".to_string(),
        ];
        let board = "devtenant90505c8a23a94cdfaevent0d1b2c3d4e5f40718293a4b5c6d7e8f9";
        assert_eq!(tenant_of_board(board, &tenants).unwrap(), tenants[0]);
        assert!(tenant_of_board(board, &tenants[1..]).is_err());
        assert!(tenant_of_board("devevent0d1b", &tenants).is_err());
        let ambiguous = vec![tenants[0].clone(), tenants[0].clone()];
        assert!(tenant_of_board(board, &ambiguous).is_err());
    }
}
