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

use crate::postgres::area::get_event_areas;
use crate::postgres::cast_vote::get_usernames_with_valid_cast_vote;
use crate::postgres::document::get_document;
use crate::postgres::election_event::{get_election_event_by_id, ElectionEventDatafix};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::electoral_log::ElectoralLog;
use crate::services::external::reconciliation::csv::{
    parse_reconciliation_rows, split_meta_and_csv,
};
use crate::services::external::reconciliation::diff::{
    diff_snapshot_page, diff_unmatched_file_rows, ReconciliationDiff,
};
use crate::services::external::reconciliation::patch::{
    build_external_patch_csv, build_sequent_patch_json, sha256_hex,
};
use crate::services::external::types::ReconciliationPatchSource;
use crate::services::protocol_manager::get_event_board;
use crate::services::tally_sheet_import::hash::hash_bytes;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::services::users::fetch_realm_voter_snapshots_page;
use crate::types::error::{Error, Result};
use celery::error::TaskError;
use electoral_log::messages::newtypes::ExternalReconciliationKind;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::TasksExecution;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenerateReconciliationPatchesBody {
    pub tenant_id: String,
    pub election_event_id: String,
    pub source_document_id: String,
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
    match run_generate_reconciliation_patches(&body).await {
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

#[instrument(skip(body), err)]
async fn run_generate_reconciliation_patches(
    body: &GenerateReconciliationPatchesBody,
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
    let file_bytes = std::fs::read(temp_file.path())
        .map_err(|err| format!("Error reading uploaded file: {err}"))?;
    // Hash for the electoral log record below (so Datafix's own generated hash can later be
    // compared against it manually); it isn't a security check on the upload itself.
    let source_sha256 = hash_bytes(&file_bytes);
    let (meta, csv_bytes) = split_meta_and_csv(&file_bytes);
    let (rows, row_parse_errors) = parse_reconciliation_rows(csv_bytes);
    if !row_parse_errors.is_empty() {
        let details = row_parse_errors
            .iter()
            .map(|err| format!("line {}: {}", err.line, err.message))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "Reconciliation file has {} malformed row(s): {details}",
            row_parse_errors.len()
        ));
    }

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

    // Sequence gating (view-time rule, no mode flag): reject only if strictly
    // less than the last-applied Sequence — a genuinely superseded file.
    // Equal-to is always allowed, so a plain convergence re-check and a
    // same-Sequence retry both just work with no special-casing.
    if meta.sequence < datafix_annotations.last_applied_sequence {
        hasura_transaction.commit().await.ok();
        return Err(format!(
            "Reconciliation file Sequence {} is not newer than the last applied Sequence {}",
            meta.sequence, datafix_annotations.last_applied_sequence
        ));
    }

    let source = ReconciliationPatchSource::Datafix {
        county_mun: datafix_annotations.voterview_request.county_mun.clone(),
    };
    let realm = get_event_realm(&body.tenant_id, &body.election_event_id);

    let file_rows_by_username: HashMap<String, _> = rows
        .into_iter()
        .map(|row| (row.voter_id.clone(), row))
        .collect();

    let mut keycloak_client = get_keycloak_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting Keycloak client: {err}"))?;
    let keycloak_transaction = keycloak_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting Keycloak transaction: {err}"))?;

    let areas_by_id: HashMap<String, String> =
        get_event_areas(&hasura_transaction, &body.tenant_id, &body.election_event_id)
            .await
            .map_err(|err| format!("Error loading event areas: {err:?}"))?
            .into_iter()
            .filter_map(|area| area.name.map(|name| (area.id, name)))
            .collect();

    let valid_voters = get_usernames_with_valid_cast_vote(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
    )
    .await
    .map_err(|err| format!("Error loading voters with a valid cast vote: {err:?}"))?;

    let mut all_items = Vec::new();
    let mut seen_usernames = std::collections::HashSet::new();
    let mut after_username: Option<String> = None;
    loop {
        let mut page = fetch_realm_voter_snapshots_page(
            &keycloak_transaction,
            &realm,
            after_username.as_deref(),
            &areas_by_id,
        )
        .await
        .map_err(|err| format!("Error fetching voter snapshot page: {err:?}"))?;
        if page.is_empty() {
            break;
        }
        for snapshot in page.iter_mut() {
            snapshot.has_valid_internet_vote = valid_voters.contains(&snapshot.username);
        }
        after_username = page.last().map(|snapshot| snapshot.username.clone());
        all_items.extend(diff_snapshot_page(
            &page,
            &file_rows_by_username,
            &source,
            &mut seen_usernames,
        ));
    }
    all_items.extend(diff_unmatched_file_rows(
        &file_rows_by_username,
        &seen_usernames,
        &source,
    ));

    // Document 1: the Sequent patch — always produced, never downloadable,
    // purely apply_reconciliation_patch's input.
    let sequent_patch_bytes = build_sequent_patch_json(&all_items)
        .map_err(|err| format!("Error serializing the Sequent patch: {err}"))?;
    let sequent_patch_document_id = upload_json_document(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
        &format!("sequent_patch_seq{}.json", meta.sequence),
        &sequent_patch_bytes,
    )
    .await
    .map_err(|err| format!("Error uploading the Sequent patch: {err:?}"))?;

    // Document 2: the downloadable external (Datafix) patch CSV — only if non-empty.
    let patch_csv = build_external_patch_csv(
        &all_items,
        &file_rows_by_username,
        meta.sequence,
        meta.generated_at,
    );
    let mut external_patch_document_id = None;
    let mut external_patch_sha256 = None;
    if let Some(patch_csv) = &patch_csv {
        let hash = sha256_hex(patch_csv);
        let file_name = format!("datafix_patch_seq{}.csv", meta.sequence);
        let mut temp_file = tempfile::NamedTempFile::new()
            .map_err(|err| format!("Error creating temp file: {err}"))?;
        temp_file
            .write_all(patch_csv.as_bytes())
            .map_err(|err| format!("Error writing patch CSV: {err}"))?;
        let uploaded = upload_and_return_document(
            &hasura_transaction,
            temp_file.path().to_str().unwrap_or_default(),
            patch_csv.len() as u64,
            "text/csv",
            &body.tenant_id,
            Some(body.election_event_id.clone()),
            &file_name,
            None,
            false,
        )
        .await
        .map_err(|err| format!("Error uploading Datafix patch: {err:?}"))?;
        external_patch_document_id = Some(uploaded.id.clone());
        external_patch_sha256 = Some(hash);
    }

    // Document 3: the diff envelope — always produced. Its id is the one
    // thing recorded on task_execution.annotations.document_id; the frontend
    // fetches and parses it for everything else.
    let envelope = ReconciliationDiff {
        sequence: meta.sequence,
        generated_at: meta.generated_at,
        source_sha256: source_sha256.clone(),
        external_patch_document_id: external_patch_document_id.clone(),
        external_patch_sha256: external_patch_sha256.clone(),
        sequent_patch_document_id,
        items: all_items,
    };
    let envelope_bytes = serde_json::to_vec(&envelope)
        .map_err(|err| format!("Error serializing the diff envelope: {err}"))?;
    let envelope_document_id = upload_json_document(
        &hasura_transaction,
        &body.tenant_id,
        &body.election_event_id,
        &format!("diff_seq{}.json", meta.sequence),
        &envelope_bytes,
    )
    .await
    .map_err(|err| format!("Error uploading the diff envelope: {err:?}"))?;

    // Electoral log: "patch generated" run-level entry.
    let slug = std::env::var("ENV_SLUG").map_err(|err| format!("Missing ENV_SLUG: {err}"))?;
    let board_name = get_event_board(&body.tenant_id, &body.election_event_id, &slug);
    if let Ok(electoral_log) = ElectoralLog::new(
        &hasura_transaction,
        &body.tenant_id,
        Some(&body.election_event_id),
        &board_name,
    )
    .await
    {
        electoral_log
            .post_external_reconciliation(
                body.election_event_id.clone(),
                ExternalReconciliationKind::PatchGenerated,
                meta.sequence,
                meta.generated_at,
                source_sha256.clone(),
                external_patch_sha256.clone(),
                None,
                None,
                None,
            )
            .await
            .ok();
    }

    hasura_transaction
        .commit()
        .await
        .map_err(|err| format!("Error committing transaction: {err}"))?;

    Ok(envelope_document_id)
}

/// Uploads raw JSON bytes as a `Document` — shared by the Sequent-patch and
/// diff-envelope uploads above, neither of which is CSV text like the
/// Datafix patch.
#[instrument(skip(hasura_transaction, bytes), err)]
async fn upload_json_document(
    hasura_transaction: &deadpool_postgres::Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    file_name: &str,
    bytes: &[u8],
) -> anyhow::Result<String> {
    let mut temp_file = tempfile::NamedTempFile::new()?;
    temp_file.write_all(bytes)?;
    let uploaded = upload_and_return_document(
        hasura_transaction,
        temp_file.path().to_str().unwrap_or_default(),
        bytes.len() as u64,
        "application/json",
        tenant_id,
        Some(election_event_id.to_string()),
        file_name,
        None,
        false,
    )
    .await?;
    Ok(uploaded.id)
}
