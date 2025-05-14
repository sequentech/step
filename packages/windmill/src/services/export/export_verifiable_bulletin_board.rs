// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::keys_ceremony::get_keys_ceremony_by_id;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::postgres::tally_session_contest::get_tally_session_contests;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::protocol_manager::get_b3_pgsql_client;
use anyhow::{anyhow, Context, Result};
use csv::ReaderBuilder;
use deadpool_postgres::Transaction;
use futures::{pin_mut, StreamExt};
use rusqlite::{params, Connection};
use sequent_core::types::hasura::core::TallySessionContest;
use tempfile::{NamedTempFile, TempPath};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::instrument;

pub const VERIFIABLE_BULLETIN_BOARD_FILE: &str = "verifiable_bulletin_board.db";

pub async fn create_cast_vote_sqlite(
    sqlite_tx: &rusqlite::Transaction<'_>,
    hasura_transaction: &Transaction<'_>,
    election_event_id: &str,
    tally_session_contests: Vec<TallySessionContest>,
) -> Result<()> {
    let area_ids = tally_session_contests
        .iter()
        .map(|contest| contest.area_id.clone())
        .collect::<Vec<String>>();

    let area_ids_str = area_ids
        .iter()
        .map(|id| format!("\"{}\"", id)) // wrap each in double-quotes
        .collect::<Vec<_>>()
        .join(",");

    let mut tmp = NamedTempFile::new().context("creating temp CSV file")?;
    let mut file = File::from_std(tmp.reopen()?);

    let tmp_path: TempPath = tmp.into_temp_path();

    let copy_query = format!(
        r#"COPY (
        SELECT
          id::text,
          tenant_id,
          election_id::text,
          area_id::text,
          created_at::text,
          last_updated_at::text,
          content,
          voter_id_string,
          election_event_id,
          ballot_id,
          cast_ballot_signature
        FROM sequent_backend.cast_vote
        WHERE election_event_id = '{}'
            AND area_id = ANY('{{{}}}')
        ) TO STDOUT WITH CSV HEADER"#,
        election_event_id, area_ids_str
    );

    let mut stream = hasura_transaction
        .copy_out(&copy_query)
        .await
        .map_err(|err| anyhow!("Failed to create COPY OUT stream: {err}"))?;
    pin_mut!(stream);

    while let Some(chunk) = stream.next().await {
        let data = chunk.context("Error reading COPY OUT stream")?;
        file.write_all(&data)
            .await
            .context("Error writing CSV data to temp file")?;
    }
    file.flush().await?;
    drop(file);

    tokio::task::block_in_place(|| -> Result<()> {
        let mut insert_vote = sqlite_tx.prepare(
            "INSERT OR REPLACE INTO cast_vote
                 (id, tenant_id, election_id, area_id, created_at, last_updated_at,
                  content, voter_id_string, election_event_id, ballot_id,
                  cast_ballot_signature)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        )?;

        let mut rdr = ReaderBuilder::new()
            .has_headers(true)
            .from_path(&tmp_path)
            .context("opening temp CSV for parsing")?;

        for result in rdr.records() {
            let rec = result.context("CSV parse error")?;
            let id: &str = rec.get(0).unwrap();
            let tenant_id_s = rec.get(1).unwrap();
            let election_id_opt = match rec.get(2).unwrap() {
                "" => None,
                s => Some(s.to_string()),
            };
            let area_id_opt = match rec.get(3).unwrap() {
                "" => None,
                s => Some(s.to_string()),
            };
            let created_at_opt = match rec.get(4).unwrap() {
                "" => None,
                s => Some(s.to_string()),
            };
            let last_updated_opt = match rec.get(5).unwrap() {
                "" => None,
                s => Some(s.to_string()),
            };
            let content_opt = match rec.get(6).unwrap() {
                "" => None,
                s => Some(s.to_string()),
            };
            let voter_id_opt = match rec.get(7).unwrap() {
                "" => None,
                s => Some(s.to_string()),
            };
            let event_id_s = rec.get(8).unwrap();
            let ballot_id_opt = match rec.get(9).unwrap() {
                "" => None,
                s => Some(s.to_string()),
            };

            let cast_ballot_signature_opt = match rec.get(10).unwrap() {
                "" => None,
                sig => {
                    let hex_part = sig.strip_prefix(r"\x").ok_or_else(|| {
                        anyhow!("Invalid bytea format, expected leading `\\x`: {}", sig)
                    })?;

                    let bytes = hex::decode(hex_part).with_context(|| {
                        anyhow!("Failed to decode hex in bytea field: {}", hex_part)
                    })?;
                    Some(bytes)
                }
            };

            insert_vote.execute(params![
                id,
                tenant_id_s,
                election_id_opt,
                area_id_opt,
                created_at_opt,
                last_updated_opt,
                content_opt,
                voter_id_opt,
                event_id_s,
                ballot_id_opt,
                cast_ballot_signature_opt
            ])?;
        }
        Ok(())
    })?;

    Ok(())
}

#[instrument(err)]
pub async fn create_verifiable_bulletin_board_db_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    document_id: Option<String>,
    board_name: &str,
    tally_session_contests: Vec<TallySessionContest>,
) -> Result<TempPath> {
    let temp_file = NamedTempFile::new()
        .context("Failed to create temporary file for verifiable bulletin board")?;
    let temp_path: TempPath = temp_file.into_temp_path();

    // ── Step 1: Fetch B3 messages ────────────────────────────────────────────────
    let mut b3_client = get_b3_pgsql_client()
        .await
        .map_err(|err| anyhow!("Failed to get B3 client: {err}"))?;
    let raw_msgs = b3_client
        .get_messages(board_name, -1)
        .await
        .map_err(|err| anyhow!("Failed to fetch B3 messages: {err}"))?;

    // ── Step 3: write & populate db file ────────────────
    tokio::task::block_in_place(|| -> anyhow::Result<()> {
        let mut conn = Connection::open(&temp_path)?;
        let tx = conn.transaction()?;

        tx.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS b3_messages (
                id                   INTEGER PRIMARY KEY,
                created              TEXT NOT NULL,
                sender_pk            TEXT NOT NULL,
                statement_timestamp  TEXT NOT NULL,
                statement_kind       TEXT NOT NULL,
                batch                INTEGER NOT NULL,
                mix_number           INTEGER NOT NULL,
                message              BLOB    NOT NULL,
                version              TEXT    NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cast_vote (
                id                    TEXT    PRIMARY KEY,
                tenant_id             TEXT    NOT NULL,
                election_id           TEXT,
                area_id               TEXT,
                created_at            TEXT,
                last_updated_at       TEXT,
                content               TEXT,
                voter_id_string       TEXT,
                election_event_id     TEXT    NOT NULL,
                ballot_id             TEXT,
                cast_ballot_signature BLOB
            );
            ",
        )?;
        {
            let mut ins_b3 = tx.prepare(
                "INSERT OR REPLACE INTO b3_messages
                 (id, created, sender_pk, statement_timestamp, statement_kind,
                  batch, mix_number, message, version)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            )?;
            for m in raw_msgs {
                ins_b3.execute(params![
                    m.id,
                    m.created,
                    m.sender_pk,
                    m.statement_timestamp,
                    m.statement_kind,
                    m.batch,
                    m.mix_number,
                    m.message,
                    m.version
                ])?;
            }
        }
        {
            tokio::runtime::Handle::current().block_on(async {
                create_cast_vote_sqlite(
                    &tx,
                    &hasura_transaction,
                    election_event_id,
                    tally_session_contests,
                )
                .await
            })?;
        }
        tx.commit()?;
        Ok(())
    })?;

    Ok(temp_path)
}

#[instrument(err)]
pub async fn export_verifiable_bulletin_board_sqlite_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    document_id: String,
    tally_session_id: String,
    election_event_id: String,
) -> Result<TempPath> {
    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Failed to get Tally Session: {err}"))?;

    let tally_session_contest = get_tally_session_contests(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Failed to get Tally Session Contests: {err}"))?;

    let keys_ceremony = get_keys_ceremony_by_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session.keys_ceremony_id,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Failed to get Key Ceremony: {err}"))?;

    let (bulletin_board, election_id) = get_keys_ceremony_board(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &keys_ceremony,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Failed to get Key Ceremony Board: {err}"))?;

    let file_temp_path = create_verifiable_bulletin_board_db_file(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        election_id,
        Some(document_id.clone()),
        &bulletin_board,
        tally_session_contest,
    )
    .await
    .map_err(|err| anyhow::anyhow!("Error exporting verifiable bulletin board: {err}"))?;

    Ok(file_temp_path)
}
