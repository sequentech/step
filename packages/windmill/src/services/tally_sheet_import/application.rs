// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::{BTreeSet, HashMap};

use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::{Area, Candidate, Contest, TallySheet};
use sequent_core::types::tally_sheet_import::{
    TallySheetImport, TallySheetImportChangeType, TallySheetImportItem, TallySheetImportItemStatus,
    TallySheetImportPreview, TallySheetImportPreviewItem, TallySheetImportReviewDecision,
    TallySheetImportSourceFormat, TallySheetImportStatus, TallySheetImportSummary,
    TallySheetImportValidationError,
};
use sequent_core::types::tally_sheets::{
    AreaContestResults, CandidateResults, TallySheetStatus, VotingChannel,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::postgres::{
    area::get_area_by_name,
    area_contest::area_contest_exists,
    candidate::get_candidates_by_contest_id,
    contest::get_contest_by_external_id,
    tally_sheet::{
        get_latest_approved_tally_sheet, get_latest_ballot_box_tally_sheet,
        get_latest_ballot_box_version, insert_tally_sheet, lock_ballot_box_version_assignment,
        review_tally_sheet_status, soft_delete_tally_sheet_leftover_versions,
    },
    tally_sheet_import::{
        get_tally_sheet_import_by_id, get_tally_sheet_import_items, insert_tally_sheet_import,
        insert_tally_sheet_import_item, update_tally_sheet_import_items_status,
        update_tally_sheet_import_items_status_by_ids, update_tally_sheet_import_status,
        update_tally_sheet_import_status_with_conflict_count,
    },
};

use super::{
    csv::parse_canonical_csv,
    diff::{classify_change, render_ballot_box_csv},
    hash::{hash_area_contest_results, hash_bytes},
    validation::validate_import_content,
};

pub async fn preview_tally_sheet_import(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    source_format: TallySheetImportSourceFormat,
    selected_channel: VotingChannel,
    canonical_csv_bytes: &[u8],
) -> Result<TallySheetImportPreview> {
    let (parsed_imports, mut validation_errors) = parse_canonical_csv(canonical_csv_bytes);
    let mut items = Vec::new();

    for parsed_import in parsed_imports {
        if parsed_import.key.channel != selected_channel {
            validation_errors.push(TallySheetImportValidationError {
                code: "selected_channel_mismatch".to_string(),
                message: format!(
                    "CSV row channel {} does not match selected import channel {}",
                    parsed_import.key.channel, selected_channel
                ),
                channel: Some(parsed_import.key.channel.clone()),
                area_name: Some(parsed_import.key.area_name.clone()),
                contest_external_id: Some(parsed_import.key.contest_external_id.clone()),
                candidate_external_id: None,
                field: Some("channel".to_string()),
            });
            continue;
        }

        let resolved = match resolve_ballot_box_import(
            transaction,
            tenant_id,
            election_event_id,
            &parsed_import.key.area_name,
            &parsed_import.key.contest_external_id,
            parsed_import.content,
        )
        .await
        {
            Ok(resolved) => resolved,
            Err(error) => {
                validation_errors.push(TallySheetImportValidationError {
                    code: "unresolved_ballot_box".to_string(),
                    message: error.to_string(),
                    channel: Some(parsed_import.key.channel.clone()),
                    area_name: Some(parsed_import.key.area_name.clone()),
                    contest_external_id: Some(parsed_import.key.contest_external_id.clone()),
                    candidate_external_id: None,
                    field: None,
                });
                continue;
            }
        };

        validation_errors.extend(validate_import_content(
            &parsed_import.key.channel,
            &parsed_import.key.area_name,
            &parsed_import.key.contest_external_id,
            &resolved.content,
        ));

        let baseline = get_latest_approved_tally_sheet(
            transaction,
            tenant_id,
            election_event_id,
            &resolved.contest.election_id,
            &resolved.area.id,
            &resolved.contest.id,
            &parsed_import.key.channel,
        )
        .await?;
        let previous = baseline.as_ref().and_then(|sheet| sheet.content.clone());
        let previous_csv = previous
            .as_ref()
            .map(|content| render_ballot_box_csv(content, &resolved.candidate_names_by_id));
        let incoming_csv =
            render_ballot_box_csv(&resolved.content, &resolved.candidate_names_by_id);
        let incoming_content_hash = hash_area_contest_results(&resolved.content)?;
        let change_type = classify_change(previous.as_ref(), &resolved.content)?;

        items.push(TallySheetImportPreviewItem {
            channel: parsed_import.key.channel.clone(),
            area_id: resolved.area.id,
            area_name: parsed_import.key.area_name,
            contest_id: resolved.contest.id,
            contest_name: resolved.contest.description.unwrap_or_default(),
            election_id: resolved.contest.election_id,
            baseline_tally_sheet_id: baseline.as_ref().map(|sheet| sheet.id.clone()),
            baseline_version: baseline.as_ref().map(|sheet| sheet.version),
            baseline_content_hash: previous
                .as_ref()
                .map(hash_area_contest_results)
                .transpose()?,
            previous,
            incoming: resolved.content,
            previous_csv,
            incoming_csv,
            incoming_content_hash,
            change_type,
            source_refs: Some(json!({
                "area_name": resolved.source_area_name,
                "contest_external_id": resolved.source_contest_external_id,
                "candidate_external_ids": resolved.source_candidate_external_ids,
            })),
        });
    }

    let summary = summarize_preview_items(&items, validation_errors.len());
    Ok(TallySheetImportPreview {
        document_id: document_id.to_string(),
        source_format,
        selected_channel,
        summary,
        items,
        validation_errors,
    })
}

pub async fn create_tally_sheet_import(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    source_file_name: Option<&str>,
    source_format: TallySheetImportSourceFormat,
    selected_channel: VotingChannel,
    canonical_csv_bytes: &[u8],
    source_bytes: &[u8],
    created_by_user_id: &str,
) -> Result<TallySheetImport> {
    let preview = preview_tally_sheet_import(
        transaction,
        tenant_id,
        election_event_id,
        document_id,
        source_format.clone(),
        selected_channel.clone(),
        canonical_csv_bytes,
    )
    .await?;

    if !preview.validation_errors.is_empty() {
        return insert_tally_sheet_import(
            transaction,
            tenant_id,
            election_event_id,
            document_id,
            source_file_name,
            Some(&hash_bytes(source_bytes)),
            &source_format,
            &selected_channel,
            &TallySheetImportStatus::FAILED_VALIDATION,
            created_by_user_id,
            &preview.summary,
            Some(&serde_json::to_value(&preview.validation_errors)?),
            Some(&hash_bytes(canonical_csv_bytes)),
        )
        .await;
    }

    let import = insert_tally_sheet_import(
        transaction,
        tenant_id,
        election_event_id,
        document_id,
        source_file_name,
        Some(&hash_bytes(source_bytes)),
        &source_format,
        &selected_channel,
        &TallySheetImportStatus::PENDING_REVIEW,
        created_by_user_id,
        &preview.summary,
        None,
        Some(&hash_bytes(canonical_csv_bytes)),
    )
    .await?;

    for item in preview.items {
        let generated_tally_sheet_id = match item.change_type {
            TallySheetImportChangeType::NEW | TallySheetImportChangeType::CHANGED => {
                lock_ballot_box_version_assignment(
                    transaction,
                    tenant_id,
                    election_event_id,
                    &item.election_id,
                    &item.area_id,
                    &item.contest_id,
                    &item.channel,
                )
                .await?;
                let version = get_latest_ballot_box_version(
                    transaction,
                    tenant_id,
                    election_event_id,
                    &item.election_id,
                    &item.area_id,
                    &item.contest_id,
                    &item.channel,
                )
                .await?
                    + 1;
                let tally_sheet = insert_tally_sheet(
                    transaction,
                    tenant_id,
                    election_event_id,
                    &item.election_id,
                    &item.contest_id,
                    &item.area_id,
                    &item.incoming,
                    &item.channel,
                    created_by_user_id,
                    TallySheetStatus::PENDING,
                    version,
                    Some(&import.id),
                )
                .await?;
                Some(tally_sheet.id)
            }
            TallySheetImportChangeType::UNCHANGED => None,
        };

        insert_tally_sheet_import_item(
            transaction,
            &TallySheetImportItem {
                id: Uuid::new_v4().to_string(),
                tenant_id: tenant_id.to_string(),
                election_event_id: election_event_id.to_string(),
                import_id: import.id.clone(),
                election_id: item.election_id,
                area_id: item.area_id,
                contest_id: item.contest_id,
                channel: item.channel,
                generated_tally_sheet_id,
                baseline_approved_tally_sheet_id: item.baseline_tally_sheet_id,
                baseline_approved_version: item.baseline_version,
                baseline_content_hash: item.baseline_content_hash,
                incoming_content_hash: item.incoming_content_hash,
                change_type: item.change_type,
                status: TallySheetImportItemStatus::PENDING_REVIEW,
                previous_csv: item.previous_csv,
                incoming_csv: item.incoming_csv,
                source_refs: item.source_refs,
                validation_warnings: None,
                labels: None,
                annotations: None,
            },
        )
        .await?;
    }

    Ok(import)
}

pub async fn review_tally_sheet_import(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    import_id: &str,
    decision: TallySheetImportReviewDecision,
    reviewed_by_user_id: &str,
) -> Result<TallySheetImport> {
    let import = get_tally_sheet_import_by_id(transaction, tenant_id, election_event_id, import_id)
        .await?
        .ok_or_else(|| anyhow!("Tally sheet import {import_id} not found"))?;
    if import.status != TallySheetImportStatus::PENDING_REVIEW {
        return Err(anyhow!(
            "Tally sheet import {import_id} cannot be reviewed from status {}",
            import.status
        ));
    }

    let items =
        get_tally_sheet_import_items(transaction, tenant_id, election_event_id, import_id).await?;
    if decision == TallySheetImportReviewDecision::APPROVE {
        let conflicted_item_ids =
            find_stale_baseline_conflicts(transaction, tenant_id, election_event_id, &items)
                .await?;

        if !conflicted_item_ids.is_empty() {
            update_tally_sheet_import_items_status_by_ids(
                transaction,
                tenant_id,
                election_event_id,
                import_id,
                &conflicted_item_ids,
                &TallySheetImportItemStatus::CONFLICTED,
            )
            .await?;
            return update_tally_sheet_import_status_with_conflict_count(
                transaction,
                tenant_id,
                election_event_id,
                import_id,
                &TallySheetImportStatus::CONFLICTED,
                conflicted_item_ids.len(),
            )
            .await;
        }
    }

    let (sheet_status, item_status, import_status) = match decision {
        TallySheetImportReviewDecision::APPROVE => (
            TallySheetStatus::APPROVED,
            TallySheetImportItemStatus::APPROVED,
            TallySheetImportStatus::APPROVED,
        ),
        TallySheetImportReviewDecision::DISAPPROVE => (
            TallySheetStatus::DISAPPROVED,
            TallySheetImportItemStatus::DISAPPROVED,
            TallySheetImportStatus::DISAPPROVED,
        ),
    };

    for item in &items {
        let Some(tally_sheet_id) = item.generated_tally_sheet_id.as_ref() else {
            continue;
        };
        let Some(tally_sheet) = review_tally_sheet_status(
            transaction,
            tenant_id,
            election_event_id,
            tally_sheet_id,
            reviewed_by_user_id,
            sheet_status.clone(),
        )
        .await?
        else {
            return Err(anyhow!("Generated tally sheet {tally_sheet_id} not found"));
        };
        if sheet_status == TallySheetStatus::APPROVED {
            soft_delete_tally_sheet_leftover_versions(transaction, &tally_sheet).await?;
        }
    }

    update_tally_sheet_import_items_status(
        transaction,
        tenant_id,
        election_event_id,
        import_id,
        &item_status,
    )
    .await?;
    update_tally_sheet_import_status(
        transaction,
        tenant_id,
        election_event_id,
        import_id,
        &import_status,
    )
    .await
}

async fn find_stale_baseline_conflicts(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    items: &[TallySheetImportItem],
) -> Result<Vec<String>> {
    let mut conflicted_item_ids = Vec::new();

    for item in items {
        lock_ballot_box_version_assignment(
            transaction,
            tenant_id,
            election_event_id,
            &item.election_id,
            &item.area_id,
            &item.contest_id,
            &item.channel,
        )
        .await?;

        let latest_approved = get_latest_approved_tally_sheet(
            transaction,
            tenant_id,
            election_event_id,
            &item.election_id,
            &item.area_id,
            &item.contest_id,
            &item.channel,
        )
        .await?;

        if !baseline_matches_import_item(item, latest_approved.as_ref())? {
            conflicted_item_ids.push(item.id.clone());
            continue;
        }

        if generated_tally_sheet_is_stale(transaction, tenant_id, election_event_id, item).await? {
            conflicted_item_ids.push(item.id.clone());
        }
    }

    Ok(conflicted_item_ids)
}

async fn generated_tally_sheet_is_stale(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    item: &TallySheetImportItem,
) -> Result<bool> {
    let Some(generated_tally_sheet_id) = item.generated_tally_sheet_id.as_ref() else {
        return Ok(false);
    };

    let latest_sheet = get_latest_ballot_box_tally_sheet(
        transaction,
        tenant_id,
        election_event_id,
        &item.election_id,
        &item.area_id,
        &item.contest_id,
        &item.channel,
    )
    .await?;

    let Some(latest_sheet) = latest_sheet else {
        return Ok(true);
    };

    Ok(
        latest_sheet.id.as_str() != generated_tally_sheet_id.as_str()
            || latest_sheet.status != TallySheetStatus::PENDING.to_string(),
    )
}

fn baseline_matches_import_item(
    item: &TallySheetImportItem,
    latest_approved: Option<&TallySheet>,
) -> Result<bool> {
    let latest_id = latest_approved.map(|sheet| sheet.id.clone());
    let latest_version = latest_approved.map(|sheet| sheet.version);
    let latest_hash = latest_approved
        .and_then(|sheet| sheet.content.as_ref())
        .map(hash_area_contest_results)
        .transpose()?;

    Ok(latest_id == item.baseline_approved_tally_sheet_id
        && latest_version == item.baseline_approved_version
        && latest_hash == item.baseline_content_hash)
}

struct ResolvedBallotBoxImport {
    area: Area,
    contest: Contest,
    content: AreaContestResults,
    candidate_names_by_id: HashMap<String, String>,
    source_area_name: String,
    source_contest_external_id: String,
    source_candidate_external_ids: Vec<String>,
}

async fn resolve_ballot_box_import(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_name: &str,
    contest_external_id: &str,
    mut content: AreaContestResults,
) -> Result<ResolvedBallotBoxImport> {
    let area = get_area_by_name(transaction, tenant_id, election_event_id, area_name)
        .await?
        .ok_or_else(|| anyhow!("Area '{area_name}' not found"))?;
    let contest = get_contest_by_external_id(
        transaction,
        tenant_id,
        election_event_id,
        contest_external_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Contest external id '{contest_external_id}' not found"))?;
    if !area_contest_exists(
        transaction,
        tenant_id,
        election_event_id,
        &area.id,
        &contest.id,
    )
    .await?
    {
        return Err(anyhow!(
            "Area '{area_name}' is not assigned to contest external id '{contest_external_id}'"
        ));
    }

    let candidates =
        get_candidates_by_contest_id(transaction, tenant_id, election_event_id, &contest.id)
            .await?;
    let mut candidates_by_external_id = HashMap::new();
    let mut duplicate_candidate_external_ids = BTreeSet::new();
    let mut candidates_missing_external_id = Vec::new();
    let mut candidate_names_by_id = HashMap::new();

    for candidate in &candidates {
        let candidate_name = candidate.description.clone().unwrap_or_default();
        candidate_names_by_id.insert(candidate.id.clone(), candidate_name.clone());
        let Some(external_id) = candidate
            .external_id
            .as_deref()
            .map(str::trim)
            .filter(|external_id| !external_id.is_empty())
        else {
            candidates_missing_external_id.push(if candidate_name.is_empty() {
                candidate.id.clone()
            } else {
                candidate_name
            });
            continue;
        };

        if candidates_by_external_id
            .insert(external_id.to_string(), candidate)
            .is_some()
        {
            duplicate_candidate_external_ids.insert(external_id.to_string());
        }
    }

    if !candidates_missing_external_id.is_empty() {
        candidates_missing_external_id.sort();
        return Err(anyhow!(
            "Contest external id '{}' has candidate(s) without an external_id required for import: {}",
            contest_external_id,
            candidates_missing_external_id.join(", ")
        ));
    }

    if !duplicate_candidate_external_ids.is_empty() {
        return Err(anyhow!(
            "Contest external id '{}' has duplicate candidate external_id value(s): {}",
            contest_external_id,
            duplicate_candidate_external_ids
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        ));
    }

    let incoming_candidate_external_ids = content
        .candidate_results
        .keys()
        .cloned()
        .collect::<BTreeSet<String>>();
    let missing_candidate_external_ids = candidates_by_external_id
        .keys()
        .filter(|external_id| !incoming_candidate_external_ids.contains(*external_id))
        .cloned()
        .collect::<BTreeSet<String>>();
    if !missing_candidate_external_ids.is_empty() {
        return Err(anyhow!(
            "Import for contest external id '{}' is missing candidate_votes rows for candidate external_id value(s): {}",
            contest_external_id,
            missing_candidate_external_ids
                .into_iter()
                .collect::<Vec<String>>()
                .join(", ")
        ));
    }

    let mut source_candidate_external_ids = content
        .candidate_results
        .keys()
        .cloned()
        .collect::<Vec<String>>();
    source_candidate_external_ids.sort();
    let mut resolved_candidate_results = HashMap::new();

    for (candidate_external_id, candidate_result) in content.candidate_results {
        let candidate = candidates_by_external_id.get(&candidate_external_id).ok_or_else(|| {
            anyhow!(
                "Candidate external id '{candidate_external_id}' not found in contest external id '{contest_external_id}'"
            )
        })?;
        resolved_candidate_results.insert(
            candidate.id.clone(),
            CandidateResults {
                candidate_id: candidate.id.clone(),
                total_votes: candidate_result.total_votes,
            },
        );
    }

    content.area_id = area.id.clone();
    content.contest_id = contest.id.clone();
    content.candidate_results = resolved_candidate_results;

    Ok(ResolvedBallotBoxImport {
        area,
        contest,
        content,
        candidate_names_by_id,
        source_area_name: area_name.to_string(),
        source_contest_external_id: contest_external_id.to_string(),
        source_candidate_external_ids,
    })
}

fn summarize_preview_items(
    items: &[TallySheetImportPreviewItem],
    validation_error_count: usize,
) -> TallySheetImportSummary {
    let mut summary = TallySheetImportSummary {
        imported_ballot_box_count: items.len(),
        validation_error_count,
        ..TallySheetImportSummary::default()
    };

    for item in items {
        match item.change_type {
            TallySheetImportChangeType::NEW => summary.new_ballot_box_count += 1,
            TallySheetImportChangeType::CHANGED => summary.changed_ballot_box_count += 1,
            TallySheetImportChangeType::UNCHANGED => summary.unchanged_ballot_box_count += 1,
        }
    }

    summary
}
