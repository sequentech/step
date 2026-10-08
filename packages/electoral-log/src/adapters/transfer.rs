// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Copying an election event's electoral logs to another database with their roots.
//!
//! An export lists each log's records in log order, with their delivery IDs, and the
//! log's identity and checkpoints. Leaves commit to the log's identity and to the
//! records, not to where a database stores them, so an import that stores the same
//! records in the same order under the same identity reproduces every root. The
//! import checks this against the exported checkpoints before it commits.

use super::postgres::{insert_records, PostgresStore, COLUMNS};
use crate::{
    domain::{ElectoralLogMessage, LogEntry},
    messages::newtypes::ElectoralLogCheckpointReason,
    proofs::{verify_checkpoint_signature, Checkpoint, LogIdentity, LogState, LogSummary, Uuid},
};
use anyhow::{bail, ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use tokio_postgres::IsolationLevel;
use trellis::journal::{audit_tree, create_log, seal, INSERT_CHUNK};

/// Format tag of exported electoral logs.
pub const EXPORT_FORMAT_V1: &str = "sequent-electoral-log-export-v1";
/// Records read per query while exporting.
const EXPORT_PAGE: i64 = 1000;

/// A record of an exported log: one line of the records file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedRecord {
    pub log_name: String,
    pub log_uid: Uuid,
    pub delivery_id: String,
    pub created: i64,
    pub sender_pk: String,
    pub statement_timestamp: i64,
    pub statement_kind: String,
    /// Base64 with padding of the signed statement.
    pub message: String,
    pub version: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub ballot_id: Option<String>,
}

impl ExportedRecord {
    fn new(log: &LogIdentity, entry: LogEntry) -> Self {
        let record = entry.message;
        Self {
            log_name: log.name.clone(),
            log_uid: log.uid,
            delivery_id: entry.delivery_id,
            created: record.created,
            sender_pk: record.sender_pk,
            statement_timestamp: record.statement_timestamp,
            statement_kind: record.statement_kind,
            message: STANDARD.encode(record.message),
            version: record.version,
            user_id: record.user_id,
            username: record.username,
            election_id: record.election_id,
            area_id: record.area_id,
            ballot_id: record.ballot_id,
        }
    }

    /// The log the record belongs to.
    pub fn log(&self) -> LogIdentity {
        LogIdentity {
            name: self.log_name.clone(),
            uid: self.log_uid,
        }
    }

    /// The record as it is appended.
    pub fn entry(&self) -> Result<LogEntry> {
        Ok(LogEntry {
            delivery_id: self.delivery_id.clone(),
            message: ElectoralLogMessage {
                id: 0,
                created: self.created,
                sender_pk: self.sender_pk.clone(),
                statement_timestamp: self.statement_timestamp,
                statement_kind: self.statement_kind.clone(),
                message: STANDARD
                    .decode(&self.message)
                    .context("An exported record's message is not base64")?,
                version: self.version.clone(),
                user_id: self.user_id.clone(),
                username: self.username.clone(),
                election_id: self.election_id.clone(),
                area_id: self.area_id.clone(),
                ballot_id: self.ballot_id.clone(),
            },
        })
    }
}

/// A checkpoint published of a log, with its signature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedCheckpoint {
    pub checkpoint: Checkpoint,
    pub reason: ElectoralLogCheckpointReason,
    /// DER/base64 public key of the signer.
    pub signer_pk: String,
    /// Base64 signature over `checkpoint_signing_bytes`.
    pub signature: String,
}

impl SignedCheckpoint {
    /// Check the signature against the key it names. Whether that key is the one
    /// expected for the event is the caller's decision.
    pub fn verify_signature(&self) -> Result<()> {
        verify_checkpoint_signature(
            &self.checkpoint,
            self.reason,
            &self.signer_pk,
            &self.signature,
        )
    }
}

/// A log of an export: its state and size when exported and the checkpoints that
/// were published of it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedLog {
    /// Size and root of the log when it was exported.
    pub checkpoint: Checkpoint,
    pub state: LogState,
    pub published: Vec<SignedCheckpoint>,
}

/// What an export of an election event's electoral logs holds besides the records.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportManifest {
    /// `EXPORT_FORMAT_V1`.
    pub format: String,
    pub election_event_id: String,
    /// Logs in the order their records appear in the records file: every log that the
    /// event's board continues, then the board.
    pub logs: Vec<ExportedLog>,
}

impl ExportManifest {
    /// The event's board when exported: its last log.
    pub fn board(&self) -> Option<&ExportedLog> {
        self.logs.last()
    }
}

/// An imported log and its root, as stored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportedLog {
    pub checkpoint: Checkpoint,
    /// Exported checkpoints the import checked its roots against.
    pub verified_checkpoints: usize,
}

impl PostgresStore {
    /// Every log of the database with its records in log order, from one snapshot.
    /// `write` receives each record; the returned logs carry no published checkpoints,
    /// which live outside this database.
    pub async fn export_logs<F>(&self, mut write: F) -> Result<Vec<ExportedLog>>
    where
        F: FnMut(ExportedRecord) -> Result<()> + Send,
    {
        let mut client = self.client().await?;
        let tx = client
            .build_transaction()
            .isolation_level(IsolationLevel::RepeatableRead)
            .read_only(true)
            .start()
            .await?;
        let rows = tx
            .query(
                "SELECT l.id, l.name, l.uid, l.size, l.root, l.sealed_at IS NOT NULL \
                 FROM trellis_logs l JOIN electoral_log_boards b ON b.board_name = l.name \
                 ORDER BY l.sealed_at IS NULL, l.id",
                &[],
            )
            .await?;
        let page = tx
            .prepare(&format!(
                "SELECT {COLUMNS}, delivery_id, e.leaf_index FROM trellis_leaves e \
                 JOIN electoral_log_messages m ON m.id = e.source_id \
                 WHERE e.log_id = $1 AND e.leaf_index >= $2 ORDER BY e.leaf_index LIMIT {EXPORT_PAGE}"
            ))
            .await?;
        let mut logs = Vec::with_capacity(rows.len());
        for row in rows {
            let log_id: i64 = row.try_get(0)?;
            let log = LogIdentity {
                name: row.try_get(1)?,
                uid: row.try_get(2)?,
            };
            let size: i64 = row.try_get(3)?;
            let mut next = 0_i64;
            while next < size {
                let records = tx.query(&page, &[&log_id, &next]).await?;
                ensure!(
                    !records.is_empty(),
                    "Log {} has no leaf at {next}",
                    log.name
                );
                for record in records {
                    let index: i64 = record.try_get("leaf_index")?;
                    ensure!(index == next, "Log {} has no leaf at {next}", log.name);
                    let entry = LogEntry {
                        delivery_id: record.try_get("delivery_id")?,
                        message: super::postgres::decode(record)?,
                    };
                    write(ExportedRecord::new(&log, entry))?;
                    next += 1;
                }
            }
            logs.push(ExportedLog {
                checkpoint: Checkpoint {
                    log_name: log.name,
                    log_uid: log.uid,
                    tree_size: u64::try_from(size)?,
                    root: row.try_get(4)?,
                },
                state: if row.try_get::<_, bool>(5)? {
                    LogState::Sealed
                } else {
                    LogState::Open
                },
                published: vec![],
            });
        }
        tx.commit().await?;
        Ok(logs)
    }

    /// Store exported logs under their identities and seal them, in one transaction.
    ///
    /// `records` must list each log's records in log order, logs in the manifest's
    /// order. Each log's recomputed root must equal its exported checkpoint, and its
    /// roots at the sizes of its published checkpoints must equal theirs, whose
    /// signatures must verify; otherwise nothing is stored. A log that already exists
    /// in the database is refused.
    pub async fn import_logs(
        &self,
        manifest: &ExportManifest,
        records: &mut (dyn Iterator<Item = Result<ExportedRecord>> + Send),
    ) -> Result<Vec<ImportedLog>> {
        ensure!(
            manifest.format == EXPORT_FORMAT_V1,
            "Unknown electoral-log export format {:?}",
            manifest.format
        );
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        let mut records = records.peekable();
        let mut imported = Vec::with_capacity(manifest.logs.len());
        for exported in &manifest.logs {
            let log = LogIdentity::of(&exported.checkpoint);
            for published in &exported.published {
                ensure!(
                    LogIdentity::of(&published.checkpoint) == log,
                    "A published checkpoint of log {} names another log",
                    log.name
                );
                published.verify_signature().with_context(|| {
                    format!(
                        "The checkpoint of log {} at size {} has an invalid signature",
                        log.name, published.checkpoint.tree_size
                    )
                })?;
            }
            let existing: Option<i64> = tx
                .query_opt(
                    "SELECT size FROM trellis_logs WHERE name = $1 OR uid = $2",
                    &[&log.name, &log.uid],
                )
                .await?
                .map(|row| row.try_get(0))
                .transpose()?;
            ensure!(
                existing.is_none(),
                "Log {} ({}) is already in this database",
                log.name,
                log.uid
            );
            create_log(&*tx, &log.name, Some(log.uid)).await?;
            tx.execute(
                "INSERT INTO electoral_log_boards (board_name) VALUES ($1)",
                &[&log.name],
            )
            .await?;
            let mut pending = Vec::with_capacity(INSERT_CHUNK);
            let mut count = 0_u64;
            while let Some(record) = records.next_if(|record| match record {
                Ok(record) => record.log_name == log.name,
                Err(_) => true,
            }) {
                let record = record?;
                ensure!(
                    record.log_uid == log.uid,
                    "A record of log {} names another log identity",
                    log.name
                );
                pending.push(record.entry()?);
                count += 1;
                if pending.len() >= INSERT_CHUNK {
                    insert_records(&tx, &log, &mut pending).await?;
                }
            }
            insert_records(&tx, &log, &mut pending).await?;
            let tx_ref: &tokio_postgres::Transaction<'_> = &tx;
            let sizes: Vec<u64> = exported
                .published
                .iter()
                .map(|published| published.checkpoint.tree_size)
                .chain([exported.checkpoint.tree_size])
                .collect();
            let tree = audit_tree(tx_ref, &log.name, &sizes).await?;
            ensure!(
                tree.is_clean(),
                "The tree of imported log {} is inconsistent",
                log.name
            );
            ensure!(
                count == exported.checkpoint.tree_size && tree.leaves == count,
                "Log {} has {} records in the export and {} stored, but its checkpoint \
                 has {}: records are missing or repeated",
                log.name,
                count,
                tree.leaves,
                exported.checkpoint.tree_size
            );
            for checkpoint in exported
                .published
                .iter()
                .map(|published| &published.checkpoint)
                .chain([&exported.checkpoint])
            {
                match tree.roots.get(&checkpoint.tree_size) {
                    Some(root) if *root == checkpoint.root => {}
                    Some(_) => bail!(
                        "The records of log {} do not have the root of its checkpoint at \
                         size {}",
                        log.name,
                        checkpoint.tree_size
                    ),
                    None => bail!(
                        "Log {} has no checkpoint size {} among its {} records",
                        log.name,
                        checkpoint.tree_size,
                        tree.leaves
                    ),
                }
            }
            let checkpoint = seal(tx_ref, &log.name).await?;
            imported.push(ImportedLog {
                checkpoint,
                verified_checkpoints: exported.published.len(),
            });
        }
        if let Some(record) = records.next() {
            let record = record?;
            bail!(
                "The export holds records of log {}, which its manifest does not list in \
                 that position",
                record.log_name
            );
        }
        tx.commit().await?;
        Ok(imported)
    }

    /// Every log of the database: the logs an imported board continues, sealed, and
    /// boards.
    pub async fn logs(&self) -> Result<Vec<LogSummary>> {
        Ok(self.journal().logs().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_records_keep_every_field() {
        let log = LogIdentity {
            name: "board".into(),
            uid: Uuid::from_u128(9),
        };
        let entry = LogEntry {
            delivery_id: "d".into(),
            message: ElectoralLogMessage {
                id: 0,
                created: 1,
                sender_pk: "pk".into(),
                statement_timestamp: 2,
                statement_kind: "CastVote".into(),
                message: vec![0, 1, 255],
                version: "1".into(),
                user_id: Some("u".into()),
                username: None,
                election_id: Some("e".into()),
                area_id: None,
                ballot_id: Some("b".into()),
            },
        };
        let exported = ExportedRecord::new(&log, entry.clone());
        let line = serde_json::to_string(&exported).unwrap();
        let read: ExportedRecord = serde_json::from_str(&line).unwrap();
        assert_eq!(read.log(), log);
        assert_eq!(read.entry().unwrap(), entry);
    }

    #[test]
    fn the_board_is_the_last_log() {
        let log = |n: u128, state| ExportedLog {
            checkpoint: Checkpoint {
                log_name: format!("log{n}"),
                log_uid: Uuid::from_u128(n),
                tree_size: 0,
                root: vec![],
            },
            state,
            published: vec![],
        };
        let manifest = ExportManifest {
            format: EXPORT_FORMAT_V1.into(),
            election_event_id: "event".into(),
            logs: vec![log(1, LogState::Sealed), log(2, LogState::Open)],
        };
        assert_eq!(manifest.board().unwrap().checkpoint.log_name, "log2");
    }
}
