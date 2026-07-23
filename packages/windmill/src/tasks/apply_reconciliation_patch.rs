// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Applies the `target = Sequent` diff of an already-computed reconciliation
//! round — the Sequent-patch document `generate_reconciliation_patches`
//! produced, not recomputed (spec: "calculated in the first diff, not
//! recalculated"). Per-voter atomic; failures are collected and reported at
//! the end rather than aborting (spec, "Implementation Requirements"). There
//! is no `datafix_reconciliation_import` row to mutate — the outcome is
//! reported via the row-failures document and a single electoral log entry,
//! not written back onto anything.

use crate::postgres::document::get_document;
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::external::reconciliation::apply::{apply_voter_changes, VoterApplyOutcome};
use crate::services::external::reconciliation::diff::{DiffItem, ReconciliationDiff};
use crate::services::external::reconciliation::patch::build_row_failures_csv;
use crate::services::external::types::{ReconciliationChangeCategory, ReconciliationPatchSource};
use crate::services::external::utils::bump_datafix_last_applied_sequence;
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_event_board;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use electoral_log::messages::newtypes::ExternalReconciliationKind;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use tracing::{info, instrument};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApplyReconciliationPatchBody {
    pub tenant_id: String,
    pub election_event_id: String,
    /// Which external system produced the round being applied — gates the
    /// source-specific bookkeeping below (e.g. Datafix's own `Sequence`
    /// tracking); the per-voter Sequent apply itself doesn't care.
    pub source: ReconciliationPatchSource,
    /// The `ReconciliationDiff` envelope document from the generate round
    /// being applied — re-fetched and re-parsed here, never trusted from the
    /// client, for the safety checks below (staleness, external side clean).
    pub diff_document_id: String,
    pub applied_by_user_id: String,
}

/// Applies every `target = Sequent` item of the round `diff_document_id`
/// points to, one voter at a time, per-row atomic. Always completes the
/// task_execution as `SUCCESS` even if some voters failed — row failures are
/// a normal, expected outcome reported in the result, not a task failure
/// (spec: "a row failure does not abort the process"); the task_execution
/// only fails on a genuine infrastructure error (e.g. cannot load the diff at
/// all).
#[instrument(
    skip_all,
    fields(
        tenant_id = %body.tenant_id,
        election_event_id = %body.election_event_id,
        diff_document_id = %body.diff_document_id
    ),
    err
)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn apply_reconciliation_patch(
    body: ApplyReconciliationPatchBody,
    task_execution: TasksExecution,
) -> Result<()> {
    match run_apply_reconciliation_patch(&body).await {
        Ok((failure_count, document_id)) => {
            info!("Reconciliation apply completed with {failure_count} row failure(s)");
            update_complete(&task_execution, document_id).await.ok();
            Ok(())
        }
        Err(message) => {
            update_fail(&task_execution, &message).await.ok();
            Err(Error::String(message))
        }
    }
}

#[instrument(skip(body), err)]
async fn run_apply_reconciliation_patch(
    body: &ApplyReconciliationPatchBody,
) -> std::result::Result<(usize, Option<String>), String> {
    let mut hasura_client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura client: {err}"))?;
    let hasura_transaction = hasura_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err}"))?;

    let envelope: ReconciliationDiff = fetch_json_document(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
        &body.diff_document_id,
    )
    .await
    .map_err(|err| format!("Error loading the reconciliation diff: {err:?}"))?;

    if envelope.external_patch_document_id.is_some() {
        return Err("The external-side diff is not empty — apply the external patch and re-import first".to_string());
    }

    // Source-specific bookkeeping: today only Datafix exists, tracking its
    // own last-applied `Sequence` per event annotation (D9) — a future
    // non-Datafix source would keep its own independent tracking here
    // instead, under its own arm, without touching the generic apply below.
    if let ReconciliationPatchSource::Datafix { .. } = &body.source {
        let election_event = get_election_event_by_id(&hasura_transaction, &body.tenant_id, &body.election_event_id)
            .await
            .map_err(|err| format!("Error loading election event: {err:?}"))?;
        let datafix_annotations = ElectionEventDatafix(election_event)
            .get_annotations()
            .map_err(|err| format!("Error reading Datafix configuration: {err}"))?;

        // Apply-time Sequence check (D9): reject only if this round is stale
        // relative to the current one. Unlike the earlier table-backed design,
        // there's no "already fully applied at this same Sequence" row to detect
        // a bare retry against — re-running apply against the same
        // diff_document_id simply re-applies the same Sequent-side items, which
        // is safe (the underlying per-voter operations are themselves
        // idempotent) but will post a second electoral log entry. Acceptable:
        // the frontend only calls apply when there's something outstanding to
        // apply in the first place (D8), so an accidental retry here is a rare
        // double-click, not a normal path.
        if envelope.sequence < datafix_annotations.last_applied_sequence {
            return Err(format!(
                "Reconciliation round Sequence {} is older than the current round ({})",
                envelope.sequence, datafix_annotations.last_applied_sequence
            ));
        }
    }

    let realm = get_event_realm(&body.tenant_id, &body.election_event_id);
    let sequent_items: Vec<DiffItem> = fetch_json_document(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
        &envelope.sequent_patch_document_id,
    )
    .await
    .map_err(|err| format!("Error loading the Sequent patch: {err:?}"))?;

    let mut indices_by_voter: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, item) in sequent_items.iter().enumerate() {
        indices_by_voter.entry(item.voter_username.clone()).or_default().push(index);
    }

    let mut keycloak_client = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Keycloak client: {err}"))?;
    let keycloak_transaction = keycloak_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Keycloak transaction: {err}"))?;

    let mut row_failures: Vec<(String, String)> = Vec::new();
    let mut applied_items: Vec<DiffItem> = Vec::new();

    for (voter_username, indices) in &indices_by_voter {
        let voter_items: Vec<DiffItem> = indices.iter().map(|&index| sequent_items[index].clone()).collect();
        if voter_items.iter().any(|item| item.category == ReconciliationChangeCategory::ROW_FAILURE) {
            continue; // never applied - excluded at generate time already, defensive only
        }

        let outcome = apply_voter_changes(
            &hasura_transaction,
            &keycloak_transaction,
            &body.tenant_id,
            &body.election_event_id,
            &realm,
            voter_username,
            &voter_items,
        )
        .await;

        match outcome {
            Ok(VoterApplyOutcome::Applied) => applied_items.extend(voter_items),
            Ok(VoterApplyOutcome::Failed { reason }) => row_failures.push((voter_username.clone(), reason)),
            Err(err) => row_failures.push((voter_username.clone(), format!("{err:?}"))),
        }
    }

    // Electoral log: one "changes applied" run-level entry, carrying every
    // applied voter's old/new values as the artifact (D10) - not one entry
    // per voter, since the spec asks for "a log" per run.
    if !applied_items.is_empty() {
        let slug = std::env::var("ENV_SLUG").map_err(|err| format!("Missing ENV_SLUG: {err}"))?;
        let board_name = get_event_board(&body.tenant_id, &body.election_event_id, &slug);
        if let Ok(electoral_log) =
            ElectoralLog::new(&hasura_transaction, &body.tenant_id, Some(&body.election_event_id), &board_name).await
        {
            let artifact = serde_json::to_vec(&applied_items).ok();
            electoral_log
                .post_external_reconciliation(
                    body.election_event_id.clone(),
                    ExternalReconciliationKind::ChangesApplied,
                    envelope.sequence,
                    envelope.generated_at,
                    envelope.source_sha256.clone(),
                    None,
                    artifact,
                    None,
                    None,
                )
                .await
                .ok();
        }
    }

    let mut row_failures_document_id = None;
    if !row_failures.is_empty() {
        let csv = build_row_failures_csv(&row_failures);
        let file_name = format!("apply_row_failures_seq{}.csv", envelope.sequence);
        let mut temp_file = tempfile::NamedTempFile::new().map_err(|err| format!("Error creating temp file: {err}"))?;
        temp_file
            .write_all(csv.as_bytes())
            .map_err(|err| format!("Error writing row failures CSV: {err}"))?;
        let uploaded = upload_and_return_document(
            &hasura_transaction,
            temp_file.path().to_str().unwrap_or_default(),
            csv.len() as u64,
            "text/csv",
            &body.tenant_id,
            Some(body.election_event_id.clone()),
            &file_name,
            None,
            false,
        )
        .await
        .map_err(|err| format!("Error uploading row failures report: {err:?}"))?;
        row_failures_document_id = Some(uploaded.id);
    }

    if let ReconciliationPatchSource::Datafix { .. } = &body.source {
        bump_datafix_last_applied_sequence(&hasura_transaction, &body.tenant_id, &body.election_event_id, envelope.sequence)
            .await
            .map_err(|err| format!("Error bumping datafix_last_applied_sequence: {err:?}"))?;
    }

    hasura_transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing transaction: {err}"))?;

    Ok((row_failures.len(), row_failures_document_id))
}

/// Downloads a `Document` and deserializes its content as JSON — shared by
/// the diff-envelope and Sequent-patch reads above.
#[instrument(skip(hasura_transaction), err)]
async fn fetch_json_document<T: serde::de::DeserializeOwned>(
    hasura_transaction: &deadpool_postgres::Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
) -> anyhow::Result<T> {
    let document = get_document(hasura_transaction, tenant_id, Some(election_event_id.to_string()), document_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document {document_id} not found"))?;
    let temp_file = get_document_as_temp_file(tenant_id, &document).await?;
    let bytes = std::fs::read(temp_file.path())?;
    Ok(serde_json::from_slice(&bytes)?)
}
