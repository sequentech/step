// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Importing an exported electoral log CSV into a synthetic immudb board.
//!
//! Run with:
//! `IMMUDB_SERVER_URL=http://… IMMUDB_USER=… IMMUDB_PASSWORD=… \
//! cargo test -p windmill --test electoral_log_csv_import -- --include-ignored`

use anyhow::{Context, Result};
use electoral_log::client::board_client::BoardClient;
use std::io::Seek;
use strand::signature::StrandSignatureSk;
use tempfile::NamedTempFile;
use windmill::services::electoral_log::{ElectoralLog, ElectoralLogRow};

const EVENT_ID: &str = "00000000-0000-4000-8000-000000000000";
/// More rows than immudb accepts in one transaction (1,024 by default).
const LARGE_LOG_ROWS: usize = 2_500;

async fn board_client() -> Result<BoardClient> {
    BoardClient::new(
        &std::env::var("IMMUDB_SERVER_URL").context("IMMUDB_SERVER_URL")?,
        &std::env::var("IMMUDB_USER").context("IMMUDB_USER")?,
        &std::env::var("IMMUDB_PASSWORD").context("IMMUDB_PASSWORD")?,
    )
    .await
}

/// A fresh synthetic board and the electoral log that writes to it.
struct Board {
    name: String,
    log: ElectoralLog,
}

impl Board {
    async fn new() -> Result<Self> {
        let name = format!("synthetic{}", uuid::Uuid::new_v4().simple());
        board_client().await?.upsert_electoral_log_db(&name).await?;
        let log = ElectoralLog::for_system_with_signing_key(&name, &StrandSignatureSk::gen()?);
        Ok(Self { name, log })
    }

    /// A CSV in the export format holding `rows` signed log entries.
    fn exported_csv(&self, rows: usize) -> Result<NamedTempFile> {
        self.exported_csv_repeating(rows, 1)
    }

    /// A CSV in the export format holding the same `rows` signed log entries
    /// `times` times over.
    fn exported_csv_repeating(&self, rows: usize, times: usize) -> Result<NamedTempFile> {
        let block = (0..rows)
            .map(|index| {
                let message = self.log.build_keycloak_event_message(
                    EVENT_ID.to_string(),
                    "LOGIN".to_string(),
                    String::new(),
                    Some(format!("synthetic-voter-{index}")),
                    None,
                    None,
                )?;
                ElectoralLogRow::try_from(message)
            })
            .collect::<Result<Vec<_>>>()?;
        let file = NamedTempFile::new()?;
        let mut writer = csv::Writer::from_writer(file.reopen()?);
        for _ in 0..times {
            for row in &block {
                writer.serialize(row)?;
            }
        }
        writer.flush()?;
        Ok(file)
    }

    async fn stored(&self) -> Result<usize> {
        Ok(board_client()
            .await?
            .count_electoral_log_messages(&self.name, None)
            .await?
            .try_into()?)
    }

    /// The board's transaction entry limit, read as the import reads it.
    async fn max_tx_entries(&self) -> Result<usize> {
        self.max_tx_entries_of(&self.name).await
    }

    /// The limit of `database_name`, asked for in a session on this board.
    async fn max_tx_entries_of(&self, database_name: &str) -> Result<usize> {
        let mut client = board_client().await?;
        client.open_session(&self.name).await?;
        let max_tx_entries = client.max_tx_entries(database_name).await;
        client.close_session().await?;
        max_tx_entries
    }

    async fn tear_down(self) -> Result<()> {
        board_client().await?.delete_database(&self.name).await
    }
}

/// Runs `test` on a fresh board, and removes the board whether or not the
/// test fails.
async fn on_a_fresh_board<T>(test: impl AsyncFnOnce(&Board) -> Result<T>) -> Result<T> {
    let board = Board::new().await?;
    let outcome = test(&board).await;
    board.tear_down().await?;
    outcome
}

#[tokio::test]
#[ignore]
async fn a_log_larger_than_one_transaction_is_imported_whole() -> Result<()> {
    let stored = on_a_fresh_board(async |board| {
        let csv = board.exported_csv(LARGE_LOG_ROWS)?;
        board.log.import_from_csv(&csv).await?;
        board.stored().await
    })
    .await?;
    assert_eq!(stored, LARGE_LOG_ROWS);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn importing_the_same_log_again_does_not_duplicate_it() -> Result<()> {
    let stored = on_a_fresh_board(async |board| {
        let mut csv = board.exported_csv(LARGE_LOG_ROWS)?;
        board.log.import_from_csv(&csv).await?;
        csv.rewind()?;
        board.log.import_from_csv(&csv).await?;
        board.stored().await
    })
    .await?;
    assert_eq!(stored, LARGE_LOG_ROWS);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn an_empty_log_imports_nothing() -> Result<()> {
    let stored = on_a_fresh_board(async |board| {
        let csv = board.exported_csv(0)?;
        board.log.import_from_csv(&csv).await?;
        board.stored().await
    })
    .await?;
    assert_eq!(stored, 0);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn a_board_reports_its_transaction_entry_limit() -> Result<()> {
    let max_tx_entries = on_a_fresh_board(async |board| board.max_tx_entries().await).await?;
    assert_eq!(max_tx_entries, 1_024);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn the_limit_of_another_board_than_the_sessions_is_refused() -> Result<()> {
    let other = format!("synthetic{}", uuid::Uuid::new_v4().simple());
    let (name, refused) = on_a_fresh_board(async |board| {
        Ok((board.name.clone(), board.max_tx_entries_of(&other).await))
    })
    .await?;
    let error = refused.expect_err("the limit of another board was read");
    assert_eq!(
        error.to_string(),
        format!("The session is on database {name}, not {other}")
    );
    Ok(())
}

#[tokio::test]
#[ignore]
async fn identical_chunks_of_a_log_are_all_imported() -> Result<()> {
    let (rows_per_transaction, stored, stored_again) = on_a_fresh_board(async |board| {
        let rows_per_transaction = board.max_tx_entries().await? - 1;
        let mut csv = board.exported_csv_repeating(rows_per_transaction, 2)?;
        board.log.import_from_csv(&csv).await?;
        let stored = board.stored().await?;
        csv.rewind()?;
        board.log.import_from_csv(&csv).await?;
        Ok((rows_per_transaction, stored, board.stored().await?))
    })
    .await?;
    assert_eq!(stored, 2 * rows_per_transaction);
    assert_eq!(stored_again, stored);
    Ok(())
}
