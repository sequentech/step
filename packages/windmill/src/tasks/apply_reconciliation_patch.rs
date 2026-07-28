// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Applies the `target = Sequent` diff of an already-computed reconciliation
//! round — the Sequent-patch document `generate_reconciliation_patches`
//! produced, not recomputed (spec: "calculated in the first diff, not
//! recalculated"). Per-voter atomic; failures are collected and reported at
//! the end rather than aborting (spec, "Implementation Requirements"). There
//! is no `datafix_reconciliation_import` row to mutate, and no downloadable
//! row-failures document either — the outcome is reported straight into the
//! task_execution's own logs (see `apply_reconciliation_patch` below) and a
//! single electoral log entry, not written back onto anything else.

use crate::postgres::document::get_document;
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::documents::get_document_as_temp_file;
use crate::services::electoral_log::ElectoralLog;
use crate::services::external::reconciliation::apply::{apply_voter_changes, VoterApplyOutcome};
use crate::services::external::reconciliation::bulk_create::apply_voters_added_bulk;
use crate::services::external::reconciliation::diff::{DiffItem, ReconciliationDiff};
use crate::services::external::types::{ReconciliationChangeCategory, ReconciliationPatchSource};
use crate::services::external::utils::set_datafix_reconciliation_state;
use crate::services::protocol_manager::get_event_board;
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::tasks_execution::{update, update_fail};
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use electoral_log::messages::newtypes::ExternalReconciliationKind;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub applied_by_username: Option<String>,
}

/// Applies every `target = Sequent` item of the round `diff_document_id`
/// points to, one voter at a time, per-row atomic (spec: "a row failure does
/// not abort the process" — every voter is still attempted). Row failures are
/// reported directly in the task_execution's own logs rather than a
/// downloadable document: a line stating how many rows applied, one line per
/// failed voter naming it and the reason, and a closing summary line. Any row
/// failure marks the task_execution `FAILED` (not `SUCCESS`) so it's visible
/// wherever task executions are surfaced, same as a genuine infrastructure
/// error (e.g. cannot load the diff at all) — the two are distinguished by
/// the logged detail, not by status.
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
        Ok((applied_count, row_failures)) => {
            info!(
                "Reconciliation apply completed: {applied_count} row(s) applied, {} row failure(s)",
                row_failures.len()
            );

            let mut logs = task_execution.logs.clone();
            logs = append_task_log(&logs, &format!("Applied {applied_count} row(s)."));
            for (voter_username, reason) in &row_failures {
                logs = append_task_log(
                    &logs,
                    &format!("Row failed for voter {voter_username}: {reason}"),
                );
            }

            let (status, closing_message) = if row_failures.is_empty() {
                (
                    TasksExecutionStatus::SUCCESS,
                    "Task completed successfully".to_string(),
                )
            } else {
                (
                    TasksExecutionStatus::FAILED,
                    format!("Error: {} row(s) failed to apply.", row_failures.len()),
                )
            };
            logs = append_task_log(&logs, &closing_message);

            update(
                &task_execution.tenant_id,
                &task_execution.id,
                status,
                logs.unwrap_or_else(|| serde_json::Value::Array(vec![])),
                None,
            )
            .await
            .ok();
            Ok(())
        }
        Err(message) => {
            update_fail(&task_execution, &message).await.ok();
            Err(Error::String(message))
        }
    }
}

/// Appends one log line to `current_logs`, same shape `update_complete`/
/// `update_fail` write — but callable multiple times before a single terminal
/// `update`, since this task has more than one line to add (the applied
/// count, then one per row failure) instead of the single summary message
/// those two helpers are built for.
fn append_task_log(
    current_logs: &Option<serde_json::Value>,
    message: &str,
) -> Option<serde_json::Value> {
    serde_json::to_value(append_general_log(current_logs, message)).ok()
}

#[instrument(skip(body), err)]
async fn run_apply_reconciliation_patch(
    body: &ApplyReconciliationPatchBody,
) -> std::result::Result<(usize, Vec<(String, String)>), String> {
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
        return Err(
            "The external-side diff is not empty — apply the external patch and re-import first"
                .to_string(),
        );
    }

    // Source-specific bookkeeping: today only Datafix exists, tracking its
    // own last-applied `Sequence` per event annotation — a future
    // non-Datafix source would keep its own independent tracking here
    // instead, under its own arm, without touching the generic apply below.
    if let ReconciliationPatchSource::Datafix { .. } = &body.source {
        let election_event = get_election_event_by_id(
            &hasura_transaction,
            &body.tenant_id,
            &body.election_event_id,
        )
        .await
        .map_err(|err| format!("Error loading election event: {err:?}"))?;
        let datafix_annotations = ElectionEventDatafix(election_event)
            .get_annotations()
            .map_err(|err| format!("Error reading Datafix configuration: {err}"))?;

        // Apply-time Sequence check: reject only if this round is stale
        // relative to the current one. Unlike the earlier table-backed design,
        // there's no "already fully applied at this same Sequence" row to detect
        // a bare retry against — re-running apply against the same
        // diff_document_id simply re-applies the same Sequent-side items, which
        // is safe (the underlying per-voter operations are themselves
        // idempotent) but will post a second electoral log entry. Acceptable:
        // the frontend only calls apply when there's something outstanding to
        // apply in the first place, so an accidental retry here is a rare
        // double-click, not a normal path.
        match datafix_annotations.last_applied_sequence {
            Some(last_applied) if envelope.sequence < last_applied => {
                return Err(format!(
                    "Reconciliation round Sequence {} is older than the current round ({last_applied})",
                    envelope.sequence
                ));
            }
            Some(last_applied)
                if envelope.sequence == last_applied
                    && !datafix_annotations.last_apply_had_failures =>
            {
                return Err(format!(
                    "Reconciliation round Sequence {} was already applied successfully; this envelope is diff-only",
                    envelope.sequence
                ));
            }
            _ => {}
        }
    }

    if !envelope.apply_allowed {
        return Err(format!(
            "Reconciliation round Sequence {} is a diff-only convergence check and cannot be applied",
            envelope.sequence
        ));
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
        indices_by_voter
            .entry(item.voter_username.clone())
            .or_default()
            .push(index);
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

    // Generate-time failures are intentionally absent from the executable
    // Sequent patch, but they are still outstanding failures for this round:
    // surface them in the apply result and retain same-Sequence retry
    // eligibility until a later run resolves them.
    let mut row_failures: Vec<(String, String)> = envelope
        .items
        .iter()
        .filter(|item| item.category == ReconciliationChangeCategory::ROW_FAILURE)
        .map(|item| {
            (
                item.voter_username.clone(),
                item.failure_reason
                    .clone()
                    .unwrap_or_else(|| "Row was excluded while generating the diff".to_string()),
            )
        })
        .collect();
    let mut applied_voters_count: usize = 0;
    let mut applied_items: Vec<DiffItem> = Vec::new();

    // VOTER_ADDED voters — the dominant category by volume whenever the
    // realm is badly out of sync with the file — are bulk-created via
    // direct writes to Keycloak's own tables instead of one Admin API call
    // each (see bulk_create's module doc for why this is necessary at
    // scale). Every other category still goes through the sequential
    // per-voter Admin API path below, unchanged.
    let mut voters_added: HashMap<String, Vec<DiffItem>> = HashMap::new();
    let mut other_voters: HashMap<String, Vec<usize>> = HashMap::new();
    for (voter_username, indices) in indices_by_voter {
        let voter_items: Vec<DiffItem> = indices
            .iter()
            .map(|&index| sequent_items[index].clone())
            .collect();
        if voter_items
            .iter()
            .any(|item| item.category == ReconciliationChangeCategory::ROW_FAILURE)
        {
            continue; // never applied - excluded at generate time already, defensive only
        }
        if voter_items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::VOTER_ADDED)
        {
            voters_added.insert(voter_username, voter_items);
        } else {
            other_voters.insert(voter_username, indices);
        }
    }

    if !voters_added.is_empty() {
        let voter_group_name = std::env::var("KEYCLOAK_VOTER_GROUP_NAME")
            .map_err(|err| format!("Error getting env var KEYCLOAK_VOTER_GROUP_NAME: {err:?}"))?;
        let voters_added_count = voters_added.len();
        let (bulk_applied, bulk_failures) = apply_voters_added_bulk(
            &hasura_transaction,
            &keycloak_transaction,
            &body.tenant_id,
            &body.election_event_id,
            &realm,
            &voter_group_name,
            &voters_added,
        )
        .await
        .map_err(|err| format!("Error bulk-creating added voters: {err:?}"))?;
        applied_voters_count += voters_added_count - bulk_failures.len();
        applied_items.extend(bulk_applied);
        row_failures.extend(bulk_failures);
    }

    for (voter_username, indices) in &other_voters {
        let voter_items: Vec<DiffItem> = indices
            .iter()
            .map(|&index| sequent_items[index].clone())
            .collect();

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
            Ok(VoterApplyOutcome::Applied) => {
                applied_voters_count += 1;
                applied_items.extend(voter_items);
            }
            Ok(VoterApplyOutcome::Failed { reason }) => {
                row_failures.push((voter_username.clone(), reason))
            }
            Err(err) => row_failures.push((voter_username.clone(), format!("{err:?}"))),
        }
    }

    // Electoral log: every apply attempt gets a run-level entry, including a
    // run where every row failed. The artifact contains only old/new items
    // that were actually applied.
    let artifact = serde_json::to_vec(&applied_items)
        .map_err(|err| format!("Error serializing reconciliation audit artifact: {err}"))?;
    let slug = std::env::var("ENV_SLUG").map_err(|err| format!("Missing ENV_SLUG: {err}"))?;
    let board_name = get_event_board(&body.tenant_id, &body.election_event_id, &slug);
    let electoral_log = ElectoralLog::new(
        &hasura_transaction,
        &body.tenant_id,
        Some(&body.election_event_id),
        &board_name,
    )
    .await
    .map_err(|err| format!("Error initializing reconciliation electoral log: {err:?}"))?;
    electoral_log
        .post_external_reconciliation(
            body.election_event_id.clone(),
            ExternalReconciliationKind::ChangesApplied,
            envelope.sequence,
            envelope.generated_at,
            envelope.source_sha256.clone(),
            None,
            Some(artifact),
            Some(body.applied_by_user_id.clone()),
            body.applied_by_username.clone(),
        )
        .await
        .map_err(|err| format!("Error storing reconciliation electoral log: {err:?}"))?;

    if let ReconciliationPatchSource::Datafix { .. } = &body.source {
        set_datafix_reconciliation_state(
            &hasura_transaction,
            &body.tenant_id,
            &body.election_event_id,
            envelope.sequence,
            !row_failures.is_empty(),
        )
        .await
        .map_err(|err| format!("Error storing Datafix reconciliation state: {err:?}"))?;
    }

    keycloak_transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing Keycloak transaction: {err}"))?;

    hasura_transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing transaction: {err}"))?;

    Ok((applied_voters_count, row_failures))
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
    let document = get_document(
        hasura_transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        document_id,
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("Document {document_id} not found"))?;
    let temp_file = get_document_as_temp_file(tenant_id, &document).await?;
    let bytes = std::fs::read(temp_file.path())?;
    Ok(serde_json::from_slice(&bytes)?)
}
