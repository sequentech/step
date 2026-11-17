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
        let file = NamedTempFile::new()?;
        let mut writer = csv::Writer::from_writer(file.reopen()?);
        for index in 0..rows {
            let message = self.log.build_keycloak_event_message(
                EVENT_ID.to_string(),
                "LOGIN".to_string(),
                String::new(),
                Some(format!("synthetic-voter-{index}")),
                None,
                None,
            )?;
            writer.serialize(ElectoralLogRow::try_from(message)?)?;
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

    async fn tear_down(self) -> Result<()> {
        board_client().await?.delete_database(&self.name).await
    }
}

#[tokio::test]
#[ignore]
async fn a_log_larger_than_one_transaction_is_imported_whole() -> Result<()> {
    let board = Board::new().await?;
    let csv = board.exported_csv(LARGE_LOG_ROWS)?;
    let imported = board.log.import_from_csv(&csv).await;
    let stored = board.stored().await?;
    board.tear_down().await?;
    imported?;
    assert_eq!(stored, LARGE_LOG_ROWS);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn importing_the_same_log_again_does_not_duplicate_it() -> Result<()> {
    let board = Board::new().await?;
    let csv = board.exported_csv(LARGE_LOG_ROWS)?;
    let first = board.log.import_from_csv(&csv).await;
    let again = board.log.import_from_csv(&csv).await;
    let stored = board.stored().await?;
    board.tear_down().await?;
    first?;
    again?;
    assert_eq!(stored, LARGE_LOG_ROWS);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn an_empty_log_imports_nothing() -> Result<()> {
    let board = Board::new().await?;
    let csv = board.exported_csv(0)?;
    let imported = board.log.import_from_csv(&csv).await;
    let stored = board.stored().await?;
    board.tear_down().await?;
    imported?;
    assert_eq!(stored, 0);
    Ok(())
}
