// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::documents::upload_and_return_document;
use crate::services::protocol_manager::get_b3_pgsql_client;
use anyhow::{anyhow, Context, Result};
use csv::ReaderBuilder;
use deadpool_postgres::Transaction;
use futures::TryStreamExt;
use rusqlite::{params, Connection};
use tokio::task;
use tracing::instrument;

pub const VERIFIABLE_BULLETIN_BOARD_FILE: &str = "verifiable_bulletin_board.db";

#[instrument(err)]
pub async fn get_csv_bytes_cast_votes(
    hasura_transaction: &Transaction<'_>,
    election_event_id: &str,
    election_id: Option<String>,
) -> Result<Vec<u8>> {
    let filter_clause = if let Some(ref eid) = election_id {
        format!("AND election_id = '{}'", eid)
    } else {
        "".to_string()
    };
    let copy_sql = format!(
        "COPY (
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
               {}
         ) TO STDOUT WITH CSV HEADER",
        election_event_id, filter_clause
    );
    let copy_stream = hasura_transaction
        .copy_out(&copy_sql)
        .await
        .map_err(|err| anyhow!("Failed to start COPY OUT for cast_vote: {err}"))?;
    let csv_bytes: Vec<u8> = copy_stream
        .try_fold(Vec::new(), |mut buf, chunk| async move {
            buf.extend_from_slice(&chunk);
            Ok(buf)
        })
        .await
        .map_err(|err| anyhow!("Error while streaming cast_vote CSV: {err}"))?;

    Ok(csv_bytes)
}

#[instrument(err)]
pub async fn export_verifiable_bulletin_board_db_file(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    document_id: Option<String>,
    board_name: &str,
) -> Result<()> {
    // ── Step 1: Fetch B3 messages ────────────────────────────────────────────────
    let mut b3_client = get_b3_pgsql_client()
        .await
        .map_err(|err| anyhow!("Failed to get B3 client: {err}"))?;
    let raw_msgs = b3_client
        .get_messages(board_name, -1)
        .await
        .map_err(|err| anyhow!("Failed to fetch B3 messages: {err}"))?;

    // ── Step 2: Fetch cast vote as csv bytes ─────────────────────────────────────────
    let cast_votes_bytes =
        get_csv_bytes_cast_votes(hasura_transaction, election_event_id, election_id)
            .await
            .map_err(|err| anyhow!("Error while streaming cast_vote CSV"))?;

    // ── Step 3: write & populate db file ────────────────
    task::spawn_blocking(move || -> Result<()> {
        let mut conn = Connection::open(VERIFIABLE_BULLETIN_BOARD_FILE)?;
        let mut tx: rusqlite::Transaction<'_> = conn.transaction()?;

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

        // ── Insert B3 messages ─────────────────────────────────────────
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

        // ── Insert cast_vote rows ───────────────────────────────────────
        {
            let mut rdr = ReaderBuilder::new()
                .has_headers(true)
                .from_reader(cast_votes_bytes.as_slice());

            let mut ins_vote = tx.prepare(
                "INSERT OR REPLACE INTO cast_vote
                 (id, tenant_id, election_id, area_id, created_at, last_updated_at,
                  content, voter_id_string, election_event_id, ballot_id,
                  cast_ballot_signature)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            )?;

            for record in rdr.records() {
                let rec = record?;
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

                ins_vote.execute(params![
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
        }
        tx.commit()?;
        Ok(())
    })
    .await
    .context("Failed to write & populate dump.db")??;

    // ── Step 4: Upload to S3 ───────────────────────────────────────────────────────
    let meta = tokio::fs::metadata(VERIFIABLE_BULLETIN_BOARD_FILE)
        .await
        .context("Could not stat dump.db")?;
    let file_size = meta.len();
    let _document = upload_and_return_document(
        hasura_transaction,
        VERIFIABLE_BULLETIN_BOARD_FILE,
        file_size,
        "application/x-sqlite3",
        tenant_id,
        Some(election_event_id.to_string()),
        "export_verifiable_bulletin_board.db",
        document_id,
        false,
    )
    .await
    .context("Failed to upload dump.db to S3")?;

    Ok(())
}
