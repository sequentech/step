// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Applies the `target = Sequent` diff of an already-computed reconciliation
//! round — the Sequent-patch document `generate_reconciliation_patches`
//! produced, not recomputed (spec: "calculated in the first diff, not
//! recalculated"). Per-voter atomic; failures are collected and reported at
//! the end rather than aborting (spec, "Implementation Requirements"). There
//! is no `datafix_reconciliation_import` row to mutate, and no downloadable
//! row-failures document either — the outcome is reported as a bounded,
//! structured task annotation and a single electoral log entry, not written
//! back onto anything else.

use crate::postgres::document::get_document;
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::documents::get_document_as_temp_file;
use crate::services::electoral_log::ElectoralLog;
use crate::services::external::reconciliation::apply::{apply_voter_changes, VoterApplyOutcome};
use crate::services::external::reconciliation::bulk_create::apply_voters_added_bulk;
use crate::services::external::reconciliation::diff::{DiffItem, ReconciliationApplyEnvelope};
use crate::services::external::types::{ReconciliationChangeCategory, ReconciliationPatchSource};
use crate::services::external::utils::set_datafix_reconciliation_state;
use crate::services::protocol_manager::get_event_board;
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::tasks_execution::{update_fail, update_with_annotations};
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use electoral_log::messages::newtypes::ExternalReconciliationKind;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use tracing::{info, instrument};

const VOTER_ADD_APPLY_BATCH_SIZE: usize = 5_000;
const MAX_RECONCILIATION_ROW_FAILURE_DETAILS: usize = 1_000;

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

#[derive(Serialize)]
struct ReconciliationTaskRowFailure {
    voter_id: String,
    reason: String,
}

#[derive(Serialize)]
struct ApplyReconciliationTaskAnnotations {
    /// Reserved for task-result document compatibility. Row-failure details
    /// are currently carried as the bounded sample below.
    document_id: Option<String>,
    reconciliation_row_failure_count: usize,
    reconciliation_row_failures_truncated: bool,
    reconciliation_row_failures: Vec<ReconciliationTaskRowFailure>,
}

/// Bounded task-facing failure summary. Retry eligibility and operator-facing
/// totals use `total_count`; only the first N details are retained for the
/// task annotation and browser table.
#[derive(Debug, Default)]
struct RowFailureSummary {
    total_count: usize,
    details: Vec<(String, String)>,
}

impl RowFailureSummary {
    fn record(&mut self, voter_id: String, reason: String) {
        self.total_count += 1;
        if self.details.len() < MAX_RECONCILIATION_ROW_FAILURE_DETAILS {
            self.details.push((voter_id, reason));
        }
    }

    fn extend(&mut self, failures: impl IntoIterator<Item = (String, String)>) {
        for (voter_id, reason) in failures {
            self.record(voter_id, reason);
        }
    }

    fn has_failures(&self) -> bool {
        self.total_count > 0
    }

    fn is_truncated(&self) -> bool {
        self.total_count > self.details.len()
    }
}

/// Enforces the NDJSON contract while retaining only the completed voter ids.
/// A repeated non-contiguous voter is a malformed apply artifact, not another
/// independent row, because its old-value validation would observe mutations
/// made by the first group.
#[derive(Debug, Default)]
struct VoterGroupTracker {
    current: Option<String>,
    completed: HashSet<String>,
}

impl VoterGroupTracker {
    fn switch_to(&mut self, voter_id: &str) -> std::result::Result<Option<String>, String> {
        if self.current.as_deref() == Some(voter_id) {
            return Ok(None);
        }
        if self.completed.contains(voter_id) {
            return Err(format!(
                "Invalid Sequent apply stream: voter {voter_id} appears in more than one non-contiguous group"
            ));
        }

        let previous = self.current.replace(voter_id.to_string());
        if let Some(completed) = previous.as_ref() {
            self.completed.insert(completed.clone());
        }
        Ok(previous)
    }

    fn finish(self) -> Option<String> {
        self.current
    }
}

/// Applies every `target = Sequent` item of the round `diff_document_id`
/// points to, one voter at a time, per-row atomic (spec: "a row failure does
/// not abort the process" — every voter is still attempted). Row failures are
/// reported as structured task annotations and rendered by the wizard. A
/// completed per-row apply is `SUCCESS` even when some rows were safely
/// rejected; `FAILED` is reserved for an infrastructure/orchestration error.
/// Same-Sequence retry eligibility is tracked independently on the election
/// event whenever the complete row-failure count is non-zero.
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
                row_failures.total_count
            );

            let mut logs = task_execution.logs.clone();
            logs = append_task_log(&logs, &format!("Applied {applied_count} row(s)."));
            let closing_message = if !row_failures.has_failures() {
                "Task completed successfully".to_string()
            } else {
                format!(
                    "Task completed with {} row failure(s). See the reconciliation result.",
                    row_failures.total_count
                )
            };
            logs = append_task_log(&logs, &closing_message);

            let failures_truncated = row_failures.is_truncated();
            let failure_count = row_failures.total_count;
            let annotations = serde_json::to_value(ApplyReconciliationTaskAnnotations {
                document_id: None,
                reconciliation_row_failure_count: failure_count,
                reconciliation_row_failures_truncated: failures_truncated,
                reconciliation_row_failures: row_failures
                    .details
                    .into_iter()
                    .map(|(voter_id, reason)| ReconciliationTaskRowFailure { voter_id, reason })
                    .collect(),
            })
            .map_err(|err| {
                Error::String(format!(
                    "Error serializing reconciliation task result: {err}"
                ))
            })?;
            update_with_annotations(
                &task_execution.tenant_id,
                &task_execution.id,
                TasksExecutionStatus::SUCCESS,
                logs.unwrap_or_else(|| serde_json::Value::Array(vec![])),
                annotations,
            )
            .await
            .map_err(|err| {
                Error::String(format!("Error storing reconciliation task result: {err}"))
            })?;
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
/// count and a final result summary) instead of the single summary message
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
) -> std::result::Result<(usize, RowFailureSummary), String> {
    let mut hasura_client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura client: {err}"))?;
    let hasura_transaction = hasura_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err}"))?;

    let envelope: ReconciliationApplyEnvelope = fetch_json_document(
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
    let patch_document = get_document(
        &hasura_transaction,
        &body.tenant_id,
        Some(body.election_event_id.clone()),
        &envelope.sequent_patch_document_id,
    )
    .await
    .map_err(|err| format!("Error loading the Sequent apply stream: {err:?}"))?
    .ok_or_else(|| {
        format!(
            "Sequent apply stream document {} not found",
            envelope.sequent_patch_document_id
        )
    })?;
    let patch_temp = get_document_as_temp_file(&body.tenant_id, &patch_document)
        .await
        .map_err(|err| format!("Error downloading the Sequent apply stream: {err:?}"))?;

    let mut keycloak_client = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Keycloak client: {err}"))?;
    let keycloak_transaction = keycloak_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Keycloak transaction: {err}"))?;

    let voter_group_name = std::env::var("KEYCLOAK_VOTER_GROUP_NAME")
        .map_err(|err| format!("Error getting env var KEYCLOAK_VOTER_GROUP_NAME: {err:?}"))?;
    let mut row_failures = RowFailureSummary::default();
    let mut applied_voters_count: usize = 0;
    let mut pending_voters_added: HashMap<String, Vec<DiffItem>> = HashMap::new();

    // Consume one contiguous voter group at a time. `VoterGroupTracker`
    // rejects any voter that reappears after its first group, making the
    // generator/apply ordering contract self-enforcing.
    let patch_file = File::open(patch_temp.path())
        .map_err(|err| format!("Error opening the Sequent apply stream: {err}"))?;
    let stream =
        serde_json::Deserializer::from_reader(BufReader::new(patch_file)).into_iter::<DiffItem>();
    let mut voter_groups = VoterGroupTracker::default();
    let mut current_items: Vec<DiffItem> = Vec::new();
    for item in stream {
        let item = item.map_err(|err| format!("Invalid Sequent apply stream item: {err}"))?;
        if let Some(completed_voter) = voter_groups.switch_to(&item.voter_username)? {
            process_voter_group(
                &hasura_transaction,
                &keycloak_transaction,
                body,
                &realm,
                &voter_group_name,
                completed_voter,
                std::mem::take(&mut current_items),
                &mut pending_voters_added,
                &mut audit_writer,
                &mut applied_voters_count,
                &mut row_failures,
            )
            .await?;
        }
        current_items.push(item);
    }
    if let Some(voter_username) = voter_groups.finish() {
        process_voter_group(
            &hasura_transaction,
            &keycloak_transaction,
            body,
            &realm,
            &voter_group_name,
            voter_username,
            current_items,
            &mut pending_voters_added,
            &mut applied_voters_count,
            &mut row_failures,
        )
        .await?;
    }
    flush_voters_added(
        &hasura_transaction,
        &keycloak_transaction,
        body,
        &realm,
        &voter_group_name,
        &mut pending_voters_added,
        &mut applied_voters_count,
        &mut row_failures,
    )
    .await?;

    // Electoral log: every apply attempt gets a run-level entry, including a
    // run where every row failed.
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
            row_failures.has_failures(),
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

#[allow(clippy::too_many_arguments)]
async fn process_voter_group(
    hasura_transaction: &deadpool_postgres::Transaction<'_>,
    keycloak_transaction: &deadpool_postgres::Transaction<'_>,
    body: &ApplyReconciliationPatchBody,
    realm: &str,
    voter_group_name: &str,
    voter_username: String,
    voter_items: Vec<DiffItem>,
    pending_voters_added: &mut HashMap<String, Vec<DiffItem>>,
    applied_voters_count: &mut usize,
    row_failures: &mut RowFailureSummary,
) -> std::result::Result<(), String> {
    let generated_failures: Vec<String> = voter_items
        .iter()
        .filter(|item| item.category == ReconciliationChangeCategory::ROW_FAILURE)
        .map(|item| {
            item.failure_reason
                .clone()
                .unwrap_or_else(|| "Row was excluded while generating the diff".to_string())
        })
        .collect();
    if !generated_failures.is_empty() {
        row_failures.extend(
            generated_failures
                .into_iter()
                .map(|reason| (voter_username.clone(), reason)),
        );
        return Ok(());
    }

    if voter_items
        .iter()
        .all(|item| item.category == ReconciliationChangeCategory::VOTER_ADDED)
    {
        pending_voters_added.insert(voter_username, voter_items);
        if pending_voters_added.len() >= VOTER_ADD_APPLY_BATCH_SIZE {
            flush_voters_added(
                hasura_transaction,
                keycloak_transaction,
                body,
                realm,
                voter_group_name,
                pending_voters_added,
                applied_voters_count,
                row_failures,
            )
            .await?;
        }
        return Ok(());
    }

    match apply_voter_changes(
        hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
        realm,
        &voter_username,
        &voter_items,
    )
    .await
    {
        Ok(VoterApplyOutcome::Applied) => *applied_voters_count += 1,
        Ok(VoterApplyOutcome::Failed { reason }) => {
            row_failures.record(voter_username, reason);
        }
        Err(err) => row_failures.record(voter_username, format!("{err:?}")),
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn flush_voters_added(
    hasura_transaction: &deadpool_postgres::Transaction<'_>,
    keycloak_transaction: &deadpool_postgres::Transaction<'_>,
    body: &ApplyReconciliationPatchBody,
    realm: &str,
    voter_group_name: &str,
    pending_voters_added: &mut HashMap<String, Vec<DiffItem>>,
    applied_voters_count: &mut usize,
    row_failures: &mut RowFailureSummary,
) -> std::result::Result<(), String> {
    if pending_voters_added.is_empty() {
        return Ok(());
    }
    let voters_added = std::mem::take(pending_voters_added);
    let voters_added_count = voters_added.len();
    let bulk_failures = apply_voters_added_bulk(
        hasura_transaction,
        keycloak_transaction,
        &body.tenant_id,
        &body.election_event_id,
        realm,
        voter_group_name,
        &voters_added,
    )
    .await
    .map_err(|err| format!("Error bulk-creating added voters: {err:?}"))?;
    *applied_voters_count += voters_added_count - bulk_failures.len();
    row_failures.extend(bulk_failures);
    Ok(())
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
    let file = File::open(temp_file.path())?;
    Ok(serde_json::from_reader(BufReader::new(file))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_metadata_ignores_the_large_review_item_array() {
        let json = r#"{
            "items":[{"arbitrary":"review-only"}],
            "sequence":7,
            "generated_at":123,
            "source_sha256":"abc",
            "external_patch_document_id":null,
            "sequent_patch_document_id":"stream-id",
            "apply_allowed":true
        }"#;
        let envelope: ReconciliationApplyEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(envelope.sequence, 7);
        assert_eq!(envelope.sequent_patch_document_id, "stream-id");
    }

    #[test]
    fn task_annotations_expose_version_stable_structured_row_failures() {
        let annotations = serde_json::to_value(ApplyReconciliationTaskAnnotations {
            document_id: None,
            reconciliation_row_failure_count: 1,
            reconciliation_row_failures_truncated: false,
            reconciliation_row_failures: vec![ReconciliationTaskRowFailure {
                voter_id: "voter-1".to_string(),
                reason: "stale snapshot".to_string(),
            }],
        })
        .unwrap();

        assert_eq!(
            annotations["reconciliation_row_failures"][0]["voter_id"],
            "voter-1"
        );
        assert_eq!(
            annotations["reconciliation_row_failures"][0]["reason"],
            "stale snapshot"
        );
        assert_eq!(annotations["reconciliation_row_failure_count"], 1);
        assert_eq!(annotations["reconciliation_row_failures_truncated"], false);
    }

    #[test]
    fn row_failure_summary_caps_details_without_losing_the_total() {
        let mut summary = RowFailureSummary::default();
        for index in 0..MAX_RECONCILIATION_ROW_FAILURE_DETAILS + 2 {
            summary.record(format!("voter-{index}"), "failed".to_string());
        }

        assert_eq!(
            summary.total_count,
            MAX_RECONCILIATION_ROW_FAILURE_DETAILS + 2
        );
        assert_eq!(
            summary.details.len(),
            MAX_RECONCILIATION_ROW_FAILURE_DETAILS
        );
        assert!(summary.is_truncated());
    }

    #[test]
    fn voter_group_tracker_rejects_a_non_contiguous_repeat() {
        let mut tracker = VoterGroupTracker::default();
        assert_eq!(tracker.switch_to("voter-1").unwrap(), None);
        assert_eq!(tracker.switch_to("voter-1").unwrap(), None);
        assert_eq!(
            tracker.switch_to("voter-2").unwrap(),
            Some("voter-1".to_string())
        );
        assert!(tracker.switch_to("voter-1").is_err());
    }
}
