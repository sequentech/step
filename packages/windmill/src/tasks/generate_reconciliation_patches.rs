// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Parses an uploaded reconciliation file, computes both diffs at once
//! (spec: "Both diffs... are calculated at once"), and uploads three
//! documents: the Sequent-patch JSON `apply_reconciliation_patch` later
//! applies from, the downloadable Datafix patch CSV (only if that side is
//! non-empty), and a "diff envelope" JSON referencing both plus every item —
//! see `reconciliation::diff::ReconciliationDiff`. There is no
//! `datafix_reconciliation_import` table or row of any kind; the envelope
//! document *is* the record, and its id is the one thing recorded on
//! `task_execution.annotations.document_id`. Named `GENERATE_RECONCILIATION_PATCHES`
//! to match the `ETasksExecution` value already committed on the frontend,
//! even though it also computes the diff, not just the patch.
//!
//! Both the input (the uploaded file) and the output (the three documents
//! above) are handled in fixed-size batches rather than fully materialized
//! in memory: the file is read incrementally via
//! `reconciliation::csv::ReconciliationRowBatches`, each batch's matching
//! Sequent voters are fetched in one round trip via
//! `users::fetch_realm_voter_snapshots_by_usernames`, and each batch's
//! resulting `DiffItem`s are written straight into the three open output
//! files via `reconciliation::patch::DiffItemArrayWriter`/
//! `ExternalPatchCsvWriter` — nothing here ever holds the whole diff (or the
//! whole file) resident in memory at once, which a 100k+-row reconciliation
//! run otherwise would.

use crate::postgres::area::get_event_areas;
use crate::postgres::cast_vote::get_voter_cast_vote_states_for_event;
use crate::postgres::document::get_document;
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::electoral_log::ElectoralLog;
use crate::services::external::reconciliation::csv::{
    split_meta_and_csv, ReconciliationRowBatches,
};
use crate::services::external::reconciliation::diff::{
    diff_file_row_batch, diff_unmatched_sequent_voters, index_datafix_area_fields,
    DatafixAreaFieldsByName, DiffItem, ReconciliationDiff,
};
use crate::services::external::reconciliation::patch::{
    is_sequent_patch_item, sha256_hex, DiffItemArrayWriter, ExternalPatchCsvWriter,
};
use crate::services::external::types::ReconciliationPatchSource;
use crate::services::protocol_manager::get_event_board;
use crate::services::serialize_tasks_logs::append_general_log;
use crate::services::tally_sheet_import::hash::hash_bytes;
use crate::services::tasks_execution::{update, update_complete, update_fail};
use crate::services::users::{
    fetch_realm_voter_snapshots_by_usernames, fetch_realm_voter_snapshots_page, VoterSnapshot,
};
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use electoral_log::messages::newtypes::ExternalReconciliationKind;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Write};
use tracing::instrument;

/// Rows are read from the file and matched against Keycloak this many at a
/// time — matches `users::VOTER_SNAPSHOT_PAGE_SIZE`, so the file-driven
/// forward pass and the Sequent-driven reverse pass move the same amount of
/// data per round trip.
const RECONCILIATION_BATCH_SIZE: usize = 5_000;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenerateReconciliationPatchesBody {
    pub tenant_id: String,
    pub election_event_id: String,
    pub source_document_id: String,
    pub requested_by_user_id: String,
    pub requested_by_username: Option<String>,
}

/// Computes both reconciliation diffs for one uploaded file and uploads the
/// three documents described in the module doc. Nothing here mutates voter
/// data (that's `apply_reconciliation_patch`'s job).
#[instrument(
    skip_all,
    fields(
        tenant_id = %body.tenant_id,
        election_event_id = %body.election_event_id
    ),
    err
)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn generate_reconciliation_patches(
    body: GenerateReconciliationPatchesBody,
    task_execution: TasksExecution,
) -> Result<()> {
    let mut task_execution = task_execution;
    match run_generate_reconciliation_patches(&body, &mut task_execution).await {
        Ok(diff_document_id) => {
            update_complete(&task_execution, Some(diff_document_id))
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

#[instrument(skip(body, task_execution), err)]
async fn run_generate_reconciliation_patches(
    body: &GenerateReconciliationPatchesBody,
    task_execution: &mut TasksExecution,
) -> std::result::Result<String, String> {
    let mut hasura_client = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Hasura client: {err}"))?;
    let hasura_transaction = hasura_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Hasura transaction: {err}"))?;

    let document = get_document(
        &hasura_transaction,
        &body.tenant_id,
        Some(body.election_event_id.clone()),
        &body.source_document_id,
    )
    .await
    .map_err(|err| format!("Error loading uploaded document: {err:?}"))?
    .ok_or_else(|| "Uploaded reconciliation file not found".to_string())?;
    let temp_file = get_document_as_temp_file(&body.tenant_id, &document)
        .await
        .map_err(|err| format!("Error downloading uploaded reconciliation file: {err:?}"))?;

    // Hash the whole file once (so Datafix's own generated hash can later be
    // compared against it manually) and read its `#META` line, then drop the
    // bytes — the batch loop below re-reads the same temp file path
    // incrementally instead of keeping the whole file resident.
    let file_bytes = std::fs::read(temp_file.path())
        .map_err(|err| format!("Error reading uploaded file: {err}"))?;
    let source_sha256 = hash_bytes(&file_bytes);
    let (meta, _) = split_meta_and_csv(&file_bytes)
        .map_err(|err| format!("Invalid reconciliation metadata: {err}"))?;
    drop(file_bytes);

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|err| format!("Error loading election event: {err:?}"))?;
    let datafix_annotations = ElectionEventDatafix(election_event)
        .get_annotations()
        .map_err(|err| format!("Election event has no valid Datafix configuration: {err}"))?;

    let apply_allowed = apply_permission_for_sequence(
        meta.sequence,
        datafix_annotations.last_applied_sequence,
        datafix_annotations.last_apply_had_failures,
    )?;
    checkpoint(
        task_execution,
        &format!("Sequence {} accepted; scanning the file.", meta.sequence),
    )
    .await;

    let source = ReconciliationPatchSource::Datafix {
        county_mun: datafix_annotations.voterview_request.county_mun.clone(),
    };
    let realm = get_event_realm(&body.tenant_id, &body.election_event_id);
    let voter_group_name = std::env::var("KEYCLOAK_VOTER_GROUP_NAME")
        .map_err(|err| format!("Error getting env var KEYCLOAK_VOTER_GROUP_NAME: {err:?}"))?;

    let mut keycloak_client = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Keycloak client: {err}"))?;
    let keycloak_transaction = keycloak_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Keycloak transaction: {err}"))?;

    let areas_by_id: HashMap<String, String> = get_event_areas(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|err| format!("Error loading event areas: {err:?}"))?
    .into_iter()
    .filter_map(|area| area.name.map(|name| (area.id, name)))
    .collect();

    let voter_cast_vote_states = get_voter_cast_vote_states_for_event(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|err| format!("Error loading active voter ballot states: {err:?}"))?;

    // Three output documents, each written incrementally as batches are
    // processed below, instead of serialized once from one fully-materialized
    // diff. `sequent_patch_writer`/`envelope_items_writer` share the exact
    // same `DiffItemArrayWriter` type, just fed different filtered subsets of
    // each batch (see `is_sequent_patch_item`).
    let sequent_patch_temp = tempfile::NamedTempFile::new()
        .map_err(|err| format!("Error creating the Sequent patch temp file: {err}"))?;
    let mut sequent_patch_writer = DiffItemArrayWriter::start(BufWriter::new(
        sequent_patch_temp
            .reopen()
            .map_err(|err| format!("Error reopening the Sequent patch temp file: {err}"))?,
    ))
    .map_err(|err| format!("Error starting the Sequent patch: {err}"))?;

    let envelope_temp = tempfile::NamedTempFile::new()
        .map_err(|err| format!("Error creating the diff envelope temp file: {err}"))?;
    let mut envelope_writer = BufWriter::new(
        envelope_temp
            .reopen()
            .map_err(|err| format!("Error reopening the diff envelope temp file: {err}"))?,
    );
    // `items` is written first (before `sequence`/`external_patch_document_id`/
    // etc., unlike `ReconciliationDiff`'s own field order) precisely so it
    // can be streamed before the fields that aren't known until the whole
    // file has been scanned — field order has no bearing on how serde_json
    // deserializes this back into `ReconciliationDiff`, which matches by name.
    envelope_writer
        .write_all(b"{\"items\":")
        .map_err(|err| format!("Error starting the diff envelope: {err}"))?;
    let mut envelope_items_writer = DiffItemArrayWriter::start(envelope_writer)
        .map_err(|err| format!("Error starting the diff envelope items: {err}"))?;

    let external_patch_temp = tempfile::NamedTempFile::new()
        .map_err(|err| format!("Error creating the Datafix patch temp file: {err}"))?;
    let mut external_patch_writer = ExternalPatchCsvWriter::start(
        BufWriter::new(
            external_patch_temp
                .reopen()
                .map_err(|err| format!("Error reopening the Datafix patch temp file: {err}"))?,
        ),
        meta.sequence,
        meta.generated_at,
    )
    .map_err(|err| format!("Error starting the Datafix patch: {err}"))?;

    // Forward pass: read the file in batches, batch-fetch the matching
    // Sequent snapshots for exactly this batch's VoterIDs (one round trip
    // per batch via `= ANY($usernames)`, not one per row and not the whole
    // realm at once), classify, and stream each batch's items straight into
    // the three writers above. `all_file_usernames` only keeps the usernames
    // (not the full parsed rows) across batches, for the reverse pass below.
    let mut file_reader = ReconciliationRowBatches::open(temp_file.path())
        .map_err(|err| format!("Error opening the reconciliation file for reading: {err}"))?;
    let mut all_file_usernames: HashSet<String> = HashSet::new();
    let mut area_fields_by_name = DatafixAreaFieldsByName::new();
    let mut total_rows: usize = 0;

    loop {
        let file_rows = file_reader
            .next_batch(RECONCILIATION_BATCH_SIZE)
            .map_err(|err| {
                if err.line == 0 {
                    format!(
                        "Reconciliation file has an invalid CSV header: {}",
                        err.message
                    )
                } else {
                    format!(
                        "Reconciliation file has a malformed row at line {}: {}",
                        err.line, err.message
                    )
                }
            })?;
        if file_rows.is_empty() {
            break;
        }
        total_rows += file_rows.len();

        let usernames: Vec<String> = file_rows.iter().map(|row| row.voter_id.clone()).collect();
        for username in &usernames {
            if !all_file_usernames.insert(username.clone()) {
                return Err(format!(
                    "Reconciliation file contains duplicate VoterID '{username}'"
                ));
            }
        }
        index_datafix_area_fields(&mut area_fields_by_name, &file_rows);

        let mut snapshots = fetch_realm_voter_snapshots_by_usernames(
            &keycloak_transaction,
            &realm,
            &voter_group_name,
            &usernames,
            &areas_by_id,
        )
        .await
        .map_err(|err| format!("Error fetching voter snapshots for a file batch: {err:?}"))?;
        for snapshot in snapshots.iter_mut() {
            if let Some(state) = voter_cast_vote_states.get(&snapshot.voter_id_string) {
                snapshot.has_valid_internet_vote = state.has_valid_vote;
                snapshot.has_unresolved_internet_vote = state.has_unresolved_vote;
            }
        }
        let snapshots_by_username: HashMap<String, VoterSnapshot> = snapshots
            .into_iter()
            .map(|snapshot| (snapshot.username.clone(), snapshot))
            .collect();

        let batch_items = diff_file_row_batch(&file_rows, &snapshots_by_username, &source);
        write_batch_to_all_outputs(
            &mut envelope_items_writer,
            &mut sequent_patch_writer,
            &mut external_patch_writer,
            &batch_items,
            &file_rows,
        )?;

        checkpoint(
            task_execution,
            &format!("Processed {total_rows} row(s) so far."),
        )
        .await;
    }

    // Reverse pass: page through Sequent's own voters (unchanged pagination)
    // to find enabled voters the file never mentioned in any batch — the
    // `voter_missing_from_file` case, wired up here for the first time (see
    // `diff::diff_unmatched_sequent_voters`'s doc).
    let mut after_username: Option<String> = None;
    loop {
        let mut page = fetch_realm_voter_snapshots_page(
            &keycloak_transaction,
            &realm,
            &voter_group_name,
            after_username.as_deref(),
            &areas_by_id,
        )
        .await
        .map_err(|err| format!("Error fetching voter snapshot page: {err:?}"))?;
        if page.is_empty() {
            break;
        }
        for snapshot in page.iter_mut() {
            if let Some(state) = voter_cast_vote_states.get(&snapshot.voter_id_string) {
                snapshot.has_valid_internet_vote = state.has_valid_vote;
                snapshot.has_unresolved_internet_vote = state.has_unresolved_vote;
            }
        }
        after_username = page.last().map(|snapshot| snapshot.username.clone());

        let reverse_items = diff_unmatched_sequent_voters(
            &page,
            &all_file_usernames,
            &source,
            &area_fields_by_name,
        );
        write_batch_to_all_outputs(
            &mut envelope_items_writer,
            &mut sequent_patch_writer,
            &mut external_patch_writer,
            &reverse_items,
            &[], // no file rows exist for these voters; every field falls back to NONE
        )?;
    }
    checkpoint(task_execution, "Computed the full diff.").await;

    // Finish and upload the Sequent patch (always produced, never
    // downloadable, purely apply_reconciliation_patch's input).
    let sequent_patch_writer_inner = sequent_patch_writer
        .finish()
        .map_err(|err| format!("Error finishing the Sequent patch: {err}"))?;
    flush_writer(sequent_patch_writer_inner)
        .map_err(|err| format!("Error finishing the Sequent patch: {err}"))?;
    let sequent_patch_size = file_size(sequent_patch_temp.path())
        .map_err(|err| format!("Error sizing the Sequent patch: {err}"))?;
    let sequent_patch_document_id = upload_document_from_temp_file(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
        &format!("sequent_patch_seq{}.json", meta.sequence),
        "application/json",
        sequent_patch_size,
        sequent_patch_temp.path(),
    )
    .await
    .map_err(|err| format!("Error uploading the Sequent patch: {err:?}"))?;
    checkpoint(
        task_execution,
        &format!("Uploaded the Sequent patch ({sequent_patch_size} bytes)."),
    )
    .await;

    // Finish and upload the downloadable external (Datafix) patch CSV —
    // only if non-empty.
    let mut external_patch_document_id = None;
    let mut external_patch_sha256 = None;
    if let Some(external_patch_writer_inner) = external_patch_writer
        .finish()
        .map_err(|err| format!("Error finishing the Datafix patch: {err}"))?
    {
        flush_writer(external_patch_writer_inner)
            .map_err(|err| format!("Error finishing the Datafix patch: {err}"))?;
        let csv_size = file_size(external_patch_temp.path())
            .map_err(|err| format!("Error sizing the Datafix patch: {err}"))?;
        let csv_bytes = std::fs::read(external_patch_temp.path())
            .map_err(|err| format!("Error reading back the Datafix patch for hashing: {err}"))?;
        let hash = sha256_hex(&csv_bytes);
        drop(csv_bytes);
        let file_name = format!("datafix_patch_seq{}.csv", meta.sequence);
        let uploaded_id = upload_document_from_temp_file(
            &hasura_transaction,
            &body.tenant_id,
            &body.election_event_id,
            &file_name,
            "text/csv",
            csv_size,
            external_patch_temp.path(),
        )
        .await
        .map_err(|err| format!("Error uploading Datafix patch: {err:?}"))?;
        external_patch_document_id = Some(uploaded_id);
        external_patch_sha256 = Some(hash);
        checkpoint(
            task_execution,
            &format!("Uploaded the Datafix patch ({csv_size} bytes)."),
        )
        .await;
    }

    // Finish the diff envelope: close the `items` array, then append the
    // fields that could only be known once every batch (and the Datafix
    // patch, if any) had been processed.
    let envelope_writer_inner = envelope_items_writer
        .finish()
        .map_err(|err| format!("Error finishing the diff envelope: {err}"))?;
    let mut envelope_writer_inner = envelope_writer_inner;
    write_envelope_tail(
        &mut envelope_writer_inner,
        meta.sequence,
        meta.generated_at,
        &source_sha256,
        external_patch_document_id.as_deref(),
        external_patch_sha256.as_deref(),
        &sequent_patch_document_id,
        apply_allowed,
    )
    .map_err(|err| format!("Error finishing the diff envelope: {err}"))?;
    flush_writer(envelope_writer_inner)
        .map_err(|err| format!("Error finishing the diff envelope: {err}"))?;
    let envelope_size = file_size(envelope_temp.path())
        .map_err(|err| format!("Error sizing the diff envelope: {err}"))?;
    let envelope_document_id = upload_document_from_temp_file(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
        &format!("diff_seq{}.json", meta.sequence),
        "application/json",
        envelope_size,
        envelope_temp.path(),
    )
    .await
    .map_err(|err| format!("Error uploading the diff envelope: {err:?}"))?;

    // Electoral log: "patch generated" run-level entry.
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
            ExternalReconciliationKind::PatchGenerated,
            meta.sequence,
            meta.generated_at,
            source_sha256.clone(),
            external_patch_sha256.clone(),
            None,
            Some(body.requested_by_user_id.clone()),
            body.requested_by_username.clone(),
        )
        .await
        .map_err(|err| format!("Error storing reconciliation electoral log: {err:?}"))?;

    hasura_transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing transaction: {err}"))?;

    Ok(envelope_document_id)
}

/// Enforces the ticket's `<=` stale rule while preserving its two explicit
/// equal-Sequence cases: row-failure retries may apply; successful rounds are
/// generated only for a diff-only convergence check.
fn apply_permission_for_sequence(
    sequence: i64,
    last_applied_sequence: Option<i64>,
    last_apply_had_failures: bool,
) -> std::result::Result<bool, String> {
    match last_applied_sequence {
        Some(last_applied) if sequence < last_applied => Err(format!(
            "Reconciliation file Sequence {sequence} is older than the last applied Sequence {last_applied}"
        )),
        Some(last_applied) if sequence == last_applied => Ok(last_apply_had_failures),
        _ => Ok(true),
    }
}

/// Writes one batch's items into all three open output writers — the
/// per-batch step shared by both the forward (file-driven) and reverse
/// (Sequent-driven) passes above.
fn write_batch_to_all_outputs<EW: Write, SW: Write, CW: Write>(
    envelope_items_writer: &mut DiffItemArrayWriter<EW>,
    sequent_patch_writer: &mut DiffItemArrayWriter<SW>,
    external_patch_writer: &mut ExternalPatchCsvWriter<CW>,
    batch_items: &[DiffItem],
    file_rows: &[crate::services::external::datafix_types::ParsedDatafixReconciliationRow],
) -> std::result::Result<(), String> {
    envelope_items_writer
        .write_batch(batch_items.iter())
        .map_err(|err| format!("Error writing diff envelope batch: {err:?}"))?;
    sequent_patch_writer
        .write_batch(
            batch_items
                .iter()
                .filter(|item| is_sequent_patch_item(item)),
        )
        .map_err(|err| format!("Error writing Sequent patch batch: {err:?}"))?;

    let file_rows_by_username: HashMap<String, _> = file_rows
        .iter()
        .map(|row| (row.voter_id.clone(), row.clone()))
        .collect();
    external_patch_writer
        .write_batch(batch_items, &file_rows_by_username)
        .map_err(|err| format!("Error writing Datafix patch batch: {err}"))?;
    Ok(())
}

/// Appends the diff envelope's metadata fields after the already-closed
/// `items` array and closes the object. Kept as one explicit hand-written
/// object rather than `serde_json::to_writer(&ReconciliationDiff {..})`
/// because `items` must be streamed before these fields are known — see the
/// call site's comment.
fn write_envelope_tail<W: Write>(
    writer: &mut W,
    sequence: i64,
    generated_at: i64,
    source_sha256: &str,
    external_patch_document_id: Option<&str>,
    external_patch_sha256: Option<&str>,
    sequent_patch_document_id: &str,
    apply_allowed: bool,
) -> std::io::Result<()> {
    write!(
        writer,
        ",\"sequence\":{sequence},\"generated_at\":{generated_at},"
    )?;
    write!(writer, "\"source_sha256\":{},", json_string(source_sha256))?;
    write!(
        writer,
        "\"external_patch_document_id\":{},",
        json_optional_string(external_patch_document_id)
    )?;
    write!(
        writer,
        "\"external_patch_sha256\":{},",
        json_optional_string(external_patch_sha256)
    )?;
    write!(
        writer,
        "\"sequent_patch_document_id\":{},\"apply_allowed\":{apply_allowed}}}",
        json_string(sequent_patch_document_id),
    )?;
    Ok(())
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn json_optional_string(value: Option<&str>) -> String {
    match value {
        Some(value) => json_string(value),
        None => "null".to_string(),
    }
}

/// Flushes a `BufWriter` so every byte written through it is guaranteed to
/// have reached the underlying file before its size is read back from disk —
/// shared by every output document's finishing step below.
fn flush_writer<W: Write>(mut writer: BufWriter<W>) -> std::io::Result<()> {
    writer.flush()
}

/// Reads back the size of an already-flushed output file from disk, since
/// `BufWriter` itself doesn't track total bytes written.
fn file_size(path: &std::path::Path) -> std::io::Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

/// Uploads an already-written temp file as a `Document` — the counterpart to
/// a "serialize to one buffer, then upload" helper, for documents built
/// incrementally by the streaming writers above instead.
#[instrument(skip(hasura_transaction), err)]
async fn upload_document_from_temp_file(
    hasura_transaction: &deadpool_postgres::Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    file_name: &str,
    media_type: &str,
    file_size: u64,
    temp_file_path: &std::path::Path,
) -> anyhow::Result<String> {
    let uploaded = upload_and_return_document(
        hasura_transaction,
        temp_file_path.to_str().unwrap_or_default(),
        file_size,
        media_type,
        tenant_id,
        Some(election_event_id.to_string()),
        file_name,
        None,
        false,
    )
    .await?;
    Ok(uploaded.id)
}

/// Appends `message` to the task's log and persists it immediately (status
/// stays `IN_PROGRESS`), so a crash partway through this task leaves a
/// record of how far processing got instead of nothing beyond "Task
/// started". Mutates `task_execution.logs` in place: the final
/// `update_complete`/`update_fail` call is built from that same field
/// (celery task arguments are captured once at enqueue time and don't
/// reflect this task's own DB writes), so without this the last checkpoint
/// would otherwise be overwritten by a summary built from the task's
/// original, pre-run logs. Persisting a checkpoint is best-effort: a
/// failure here is logged, not propagated — a diagnostic write must never
/// abort the reconciliation run it's only there to report on.
async fn checkpoint(task_execution: &mut TasksExecution, message: &str) {
    let new_logs = match serde_json::to_value(append_general_log(&task_execution.logs, message)) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!("Error serializing reconciliation checkpoint log: {err:?}");
            return;
        }
    };
    task_execution.logs = Some(new_logs.clone());
    if let Err(err) = update(
        &task_execution.tenant_id,
        &task_execution.id,
        TasksExecutionStatus::IN_PROGRESS,
        new_logs,
        None,
    )
    .await
    {
        tracing::warn!("Error persisting reconciliation checkpoint log: {err:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::apply_permission_for_sequence;

    #[test]
    fn sequence_gate_distinguishes_retry_from_convergence_check() {
        assert_eq!(apply_permission_for_sequence(0, None, false), Ok(true));
        assert!(apply_permission_for_sequence(4, Some(5), true).is_err());
        assert_eq!(apply_permission_for_sequence(5, Some(5), true), Ok(true));
        assert_eq!(apply_permission_for_sequence(5, Some(5), false), Ok(false));
        assert_eq!(apply_permission_for_sequence(6, Some(5), false), Ok(true));
    }
}
