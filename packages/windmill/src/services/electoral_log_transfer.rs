// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Export and import of an election event's electoral logs with their roots.
//!
//! An export holds the records of every log of the event's database in log order, in
//! `electoral_log_records-<event>.jsonl`, and the logs' identities, sizes, roots and
//! published checkpoints in `electoral_log_manifest-<event>.json`. An import stores the
//! logs under their identities in the new event's database, refuses them unless every
//! root matches its checkpoint, seals them, records the published checkpoints for
//! the new event and starts the new event's board with a record that continues the
//! exported board.
use crate::postgres::electoral_log_checkpoint::{
    get_checkpoints_of_logs, get_electoral_log_checkpoints, insert_electoral_log_checkpoint,
    PublishedCheckpoint,
};
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_event_store;
use crate::types::documents::EDocuments;
use anyhow::{anyhow, ensure, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::adapters::events::event_of_board;
use electoral_log::adapters::transfer::{
    ExportManifest, ExportedLog, ExportedRecord, ImportedLog, SignedCheckpoint, EXPORT_FORMAT_V1,
};
use electoral_log::proofs::Checkpoint;
use std::io::{BufRead, BufWriter, Write};
use tempfile::NamedTempFile;
use tracing::instrument;

/// Name of the records file of an event's export.
pub fn records_file_name(election_event_id: &str) -> String {
    format!(
        "{}-{election_event_id}.jsonl",
        EDocuments::ELECTORAL_LOG_RECORDS.to_file_name()
    )
}

/// Name of the manifest file of an event's export.
pub fn manifest_file_name(election_event_id: &str) -> String {
    format!(
        "{}-{election_event_id}.json",
        EDocuments::ELECTORAL_LOG_MANIFEST.to_file_name()
    )
}

/// A published checkpoint as an export carries it.
fn signed_checkpoint(published: &PublishedCheckpoint) -> Result<SignedCheckpoint> {
    Ok(SignedCheckpoint {
        checkpoint: Checkpoint {
            log_name: published.board_name.clone(),
            log_uid: published.log_uid,
            tree_size: u64::try_from(published.tree_size)?,
            root: hex::decode(&published.root)
                .with_context(|| format!("Checkpoint root {} is not hex", published.root))?,
        },
        reason: published
            .reason
            .parse()
            .map_err(|_| anyhow!("Unknown checkpoint reason {}", published.reason))?,
        signer_pk: published.signer_pk.clone(),
        signature: published.signature.clone(),
    })
}

/// A checkpoint of an export as the event's checkpoints table stores it.
fn published_checkpoint(signed: &SignedCheckpoint) -> Result<PublishedCheckpoint> {
    Ok(PublishedCheckpoint {
        board_name: signed.checkpoint.log_name.clone(),
        log_uid: signed.checkpoint.log_uid,
        tree_size: i64::try_from(signed.checkpoint.tree_size)?,
        root: hex::encode(&signed.checkpoint.root),
        reason: signed.reason.to_string(),
        signer_pk: signed.signer_pk.clone(),
        signature: signed.signature.clone(),
    })
}

/// The published checkpoints of an exported log: those of its identity up to its
/// exported size. Checkpoints are read after the records, so they may cover records
/// appended since, which the export leaves out.
fn published_of(
    log: &ExportedLog,
    published: &[PublishedCheckpoint],
) -> Result<Vec<SignedCheckpoint>> {
    published
        .iter()
        .filter(|row| {
            row.log_uid == log.checkpoint.log_uid
                && u64::try_from(row.tree_size).is_ok_and(|size| size <= log.checkpoint.tree_size)
        })
        .map(signed_checkpoint)
        .collect()
}

/// Checkpoints that this environment published of the exported logs, each by the
/// election event the log is the board of, which the export cannot have forged.
/// Copies that imports recorded for other events are left out.
async fn anchors_of(
    hasura_transaction: &Transaction<'_>,
    manifest: &ExportManifest,
) -> Result<Vec<Checkpoint>> {
    let log_uids: Vec<uuid::Uuid> = manifest
        .logs
        .iter()
        .map(|log| log.checkpoint.log_uid)
        .collect();
    get_checkpoints_of_logs(hasura_transaction, &log_uids)
        .await?
        .into_iter()
        .filter(|(election_event_id, published)| {
            event_of_board(&published.board_name)
                .ok()
                .and_then(|event| uuid::Uuid::parse_str(&event).ok())
                == Some(*election_event_id)
        })
        .map(|(_, published)| Ok(signed_checkpoint(&published)?.checkpoint))
        .collect()
}

/// Export an event's electoral logs: the records file, written to a temporary file,
/// and the manifest.
#[instrument(skip(hasura_transaction), err)]
pub async fn export_electoral_log(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(NamedTempFile, ExportManifest)> {
    let records = NamedTempFile::new().context("Error creating the electoral-log records file")?;
    let mut writer = BufWriter::new(records.reopen()?);
    let store = get_event_store(election_event_id).await?;
    let mut logs = store
        .export_logs(|record| {
            serde_json::to_writer(&mut writer, &record)?;
            writer.write_all(b"\n")?;
            Ok(())
        })
        .await?;
    writer.flush()?;
    drop(writer);
    let published =
        get_electoral_log_checkpoints(hasura_transaction, tenant_id, election_event_id).await?;
    for log in &mut logs {
        log.published = published_of(log, &published)?;
    }
    Ok((
        records,
        ExportManifest {
            format: EXPORT_FORMAT_V1.to_string(),
            election_event_id: election_event_id.to_string(),
            logs,
        },
    ))
}

/// What an import stored.
#[derive(Debug)]
pub struct ImportedElectoralLog {
    pub logs: Vec<ImportedLog>,
    /// The checkpoint the new event's board continues from.
    pub continues: Checkpoint,
}

/// Import an exported event's electoral logs into the new event's database, whose
/// board `board` was just created, and record their published checkpoints for the
/// new event in the Hasura transaction.
#[instrument(skip(hasura_transaction, manifest, records), err)]
pub async fn import_electoral_log(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    board: &str,
    manifest: &[u8],
    records: &[u8],
) -> Result<ImportedElectoralLog> {
    let manifest: ExportManifest =
        serde_json::from_slice(manifest).context("The electoral-log manifest is malformed")?;
    let previous = manifest
        .board()
        .context("The electoral-log manifest lists no log")?
        .checkpoint
        .clone();
    ensure!(
        manifest
            .logs
            .iter()
            .all(|log| log.checkpoint.log_name != board),
        "The exported logs include the new event's board {board}"
    );
    let mut lines = records.lines().enumerate().map(|(number, line)| {
        let line = line?;
        serde_json::from_str::<ExportedRecord>(&line).with_context(|| {
            format!(
                "Line {} of the electoral-log records is malformed",
                number + 1
            )
        })
    });
    let anchors = anchors_of(hasura_transaction, &manifest).await?;
    let store = get_event_store(election_event_id).await?;
    let logs = store.import_logs(&manifest, &mut lines, &anchors).await?;
    for log in &manifest.logs {
        for signed in &log.published {
            let published = published_checkpoint(signed)?;
            let stored = insert_electoral_log_checkpoint(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &published,
            )
            .await?;
            ensure!(
                stored.root == published.root,
                "A different checkpoint of log {} at size {} is already recorded",
                published.board_name,
                published.tree_size
            );
        }
    }
    ElectoralLog::new(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        board,
    )
    .await?
    .post_continuation(election_event_id, &previous)
    .await?;
    Ok(ImportedElectoralLog {
        logs,
        continues: previous,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use electoral_log::messages::newtypes::ElectoralLogCheckpointReason;
    use electoral_log::proofs::Uuid;

    #[test]
    fn export_files_are_told_apart_from_the_activity_logs() {
        let event = "0d1b2c3d-4e5f-4071-8293-a4b5c6d7e8f9";
        let records = records_file_name(event);
        let manifest = manifest_file_name(event);
        let activity = EDocuments::ACTIVITY_LOGS.to_file_name();
        for name in [&records, &manifest] {
            assert!(!name.contains(activity), "{name}");
        }
        assert!(!records.contains(EDocuments::ELECTORAL_LOG_MANIFEST.to_file_name()));
        assert!(!manifest.contains(EDocuments::ELECTORAL_LOG_RECORDS.to_file_name()));
    }

    #[test]
    fn exports_carry_the_checkpoints_of_each_log_up_to_its_exported_size() {
        let row = |log: u128, size: i64| PublishedCheckpoint {
            board_name: "board".into(),
            log_uid: Uuid::from_u128(log),
            tree_size: size,
            root: "ab".repeat(32),
            reason: ElectoralLogCheckpointReason::Periodic.to_string(),
            signer_pk: "pk".into(),
            signature: "sig".into(),
        };
        let log = ExportedLog {
            checkpoint: Checkpoint {
                log_name: "board".into(),
                log_uid: Uuid::from_u128(1),
                tree_size: 9,
                root: vec![0; 32],
            },
            state: electoral_log::proofs::LogState::Open,
            published: vec![],
        };
        let published = [row(1, 4), row(1, 9), row(1, 10), row(2, 3)];
        let sizes: Vec<u64> = published_of(&log, &published)
            .unwrap()
            .iter()
            .map(|signed| signed.checkpoint.tree_size)
            .collect();
        assert_eq!(sizes, [4, 9]);
    }

    #[test]
    fn published_checkpoints_round_trip_through_the_export() {
        let published = PublishedCheckpoint {
            board_name: "board".into(),
            log_uid: Uuid::from_u128(5),
            tree_size: 9,
            root: "ab".repeat(32),
            reason: ElectoralLogCheckpointReason::Periodic.to_string(),
            signer_pk: "pk".into(),
            signature: "sig".into(),
        };
        let signed = signed_checkpoint(&published).unwrap();
        assert_eq!(signed.checkpoint.root, vec![0xab; 32]);
        assert_eq!(published_checkpoint(&signed).unwrap(), published);
        let malformed = PublishedCheckpoint {
            root: "zz".into(),
            ..published
        };
        assert!(signed_checkpoint(&malformed).is_err());
    }
}
