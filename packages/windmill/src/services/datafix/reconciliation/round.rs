// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The record `generate_reconciliation_patches` leaves on its own task
//! execution once a round is uploaded: the diff envelope document and the
//! SHA-256 of the two documents apply reads back (the envelope and the
//! Sequent apply stream it references). Apply only accepts a diff document
//! named by such a record, and only with the content hashed when the round
//! was generated.

use crate::postgres::document::get_document;
use crate::postgres::tasks_execution::get_successful_task_by_document_id;
use crate::services::datafix::reconciliation::diff::ReconciliationApplyEnvelope;
use crate::services::datafix::reconciliation::patch::sha256_file_hex;
use crate::services::documents::get_document_as_temp_file;
use crate::types::tasks::ETasksExecution;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use tempfile::NamedTempFile;
use tracing::instrument;

/// The annotations of a successful `GENERATE_RECONCILIATION_PATCHES` task.
/// `document_id` keeps the key every task result uses, which the wizard
/// reads to find the envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedReconciliationRound {
    pub document_id: String,
    pub envelope_sha256: String,
    pub sequent_patch_sha256: String,
}

impl GeneratedReconciliationRound {
    /// A round generated before the document hashes were recorded carries
    /// only `document_id` and has to be generated again before it is applied.
    pub fn from_task(task: &TasksExecution) -> Result<Self> {
        let annotations = task
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Reconciliation round {} has no annotations", task.id))?;
        serde_json::from_value(annotations).with_context(|| {
            format!(
                "Reconciliation round {} has no recorded document hashes; generate it again before applying",
                task.id
            )
        })
    }
}

/// The successful generate round of this election event that produced
/// `diff_document_id`, or `None` when no generate task produced it.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_generated_round(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    diff_document_id: &str,
) -> Result<Option<GeneratedReconciliationRound>> {
    let task = get_successful_task_by_document_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &ETasksExecution::GENERATE_RECONCILIATION_PATCHES.to_string(),
        diff_document_id,
    )
    .await?;
    task.as_ref()
        .map(GeneratedReconciliationRound::from_task)
        .transpose()
}

/// Downloads one of a generated round's documents and checks it against the
/// hash the round recorded for it.
#[instrument(skip(hasura_transaction), err)]
pub async fn download_round_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    expected_sha256: &str,
) -> Result<NamedTempFile> {
    let document = get_document(
        hasura_transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Reconciliation document {document_id} not found"))?;
    let temp_file = get_document_as_temp_file(tenant_id, &document).await?;
    verify_sha256(temp_file.path(), expected_sha256)
        .with_context(|| format!("Reconciliation document {document_id}"))?;
    Ok(temp_file)
}

/// The apply metadata of a generated round's diff envelope.
#[instrument(skip(hasura_transaction), err)]
pub async fn load_round_envelope(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    round: &GeneratedReconciliationRound,
) -> Result<ReconciliationApplyEnvelope> {
    let temp_file = download_round_document(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &round.document_id,
        &round.envelope_sha256,
    )
    .await?;
    let file = File::open(temp_file.path())?;
    Ok(serde_json::from_reader(BufReader::new(file))?)
}

fn verify_sha256(path: &Path, expected_sha256: &str) -> Result<()> {
    let actual_sha256 = sha256_file_hex(path)?;
    if actual_sha256 != expected_sha256 {
        return Err(anyhow!(
            "content does not match the hash recorded when the round was generated"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::datafix::reconciliation::patch::sha256_hex;
    use chrono::Local;
    use sequent_core::types::hasura::extra::TasksExecutionStatus;
    use std::io::Write;

    fn generate_task(annotations: Option<serde_json::Value>) -> TasksExecution {
        TasksExecution {
            id: "task-id".to_string(),
            tenant_id: "tenant-id".to_string(),
            election_event_id: Some("event-id".to_string()),
            name: ETasksExecution::GENERATE_RECONCILIATION_PATCHES
                .to_name()
                .to_string(),
            task_type: ETasksExecution::GENERATE_RECONCILIATION_PATCHES.to_string(),
            execution_status: TasksExecutionStatus::SUCCESS.to_string(),
            created_at: Local::now(),
            start_at: None,
            end_at: None,
            annotations,
            labels: None,
            logs: None,
            executed_by_user: "user".to_string(),
        }
    }

    fn temp_file_with(content: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn round_annotations_keep_the_document_id_the_wizard_reads() {
        let round = GeneratedReconciliationRound {
            document_id: "envelope-id".to_string(),
            envelope_sha256: "envelope-sha256".to_string(),
            sequent_patch_sha256: "sequent-patch-sha256".to_string(),
        };
        let annotations = serde_json::to_value(&round).unwrap();
        assert_eq!(annotations["document_id"], "envelope-id");

        let task = generate_task(Some(annotations));
        assert_eq!(
            GeneratedReconciliationRound::from_task(&task).unwrap(),
            round
        );
    }

    #[test]
    fn round_without_recorded_hashes_is_not_applicable() {
        let task = generate_task(Some(serde_json::json!({ "document_id": "envelope-id" })));
        let error = GeneratedReconciliationRound::from_task(&task).unwrap_err();
        assert!(format!("{error:?}").contains("generate it again"));

        assert!(GeneratedReconciliationRound::from_task(&generate_task(None)).is_err());
    }

    #[test]
    fn round_document_must_match_its_recorded_hash() {
        let content = br#"{"sequence":7}"#;
        let file = temp_file_with(content);
        assert!(verify_sha256(file.path(), &sha256_hex(content)).is_ok());

        let changed = temp_file_with(br#"{"sequence":8}"#);
        assert!(verify_sha256(changed.path(), &sha256_hex(content)).is_err());
    }
}
