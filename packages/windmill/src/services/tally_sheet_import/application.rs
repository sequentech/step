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
use tracing::instrument;
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
        ReviewTallySheetOutcome,
    },
    tally_sheet_import::{
        get_tally_sheet_import_by_id, get_tally_sheet_import_items_for_review,
        insert_tally_sheet_import, insert_tally_sheet_import_items,
        update_tally_sheet_import_items_status, update_tally_sheet_import_items_status_by_ids,
        update_tally_sheet_import_status, update_tally_sheet_import_status_with_conflict_count,
        TallySheetImportItemReviewSnapshot,
    },
};

use super::{
    csv::parse_canonical_csv,
    diff::{classify_change, render_ballot_box_csv},
    errors::TallySheetImportError,
    hash::{hash_area_contest_results, hash_bytes},
    validation::{contest_max_marks_per_ballot, validate_import_content},
};

const DISPLAY_NAME_FALLBACK_LANG: &str = "en";

/// Reads the display name (alias, falling back to name) out of a contest's
/// or candidate's `presentation` jsonb i18n map, preferring English and
/// otherwise falling back to any other available language. `description` is
/// a separate, free-text field and is not the display name shown elsewhere
/// in the app (see `useAliasRenderer` on the frontend).
fn presentation_display_name(presentation: &Option<Value>) -> Option<String> {
    let i18n = presentation.as_ref()?.get("i18n")?.as_object()?;

    let read_field = |lang: &str, field: &str| -> Option<String> {
        i18n.get(lang)?
            .get(field)?
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };

    read_field(DISPLAY_NAME_FALLBACK_LANG, "alias")
        .or_else(|| read_field(DISPLAY_NAME_FALLBACK_LANG, "name"))
        .or_else(|| {
            i18n.keys()
                .find_map(|lang| read_field(lang, "alias").or_else(|| read_field(lang, "name")))
        })
}

#[instrument(skip_all, err)]
pub async fn preview_tally_sheet_import(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    source_format: TallySheetImportSourceFormat,
    selected_channel: VotingChannel,
    canonical_csv_bytes: &[u8],
    // Errors already known before parsing, e.g. from converting a source
    // XML file to canonical CSV (a problem scoped to one Contest there
    // skips just that contest rather than failing the whole import, so it
    // surfaces here instead, same as any other validation error).
    conversion_validation_errors: Vec<TallySheetImportValidationError>,
) -> Result<TallySheetImportPreview> {
    let (parsed_imports, parse_errors) = parse_canonical_csv(canonical_csv_bytes);
    let mut validation_errors = conversion_validation_errors;
    validation_errors.extend(parse_errors);
    let mut items = Vec::new();
    let mut resolution_cache = BallotBoxResolutionCache::default();
    let mut baseline_cache: HashMap<BallotBoxKey, Option<TallySheet>> = HashMap::new();

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
                params: HashMap::from([
                    (
                        "rowChannel".to_string(),
                        parsed_import.key.channel.to_string(),
                    ),
                    ("selectedChannel".to_string(), selected_channel.to_string()),
                ]),
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
            &mut resolution_cache,
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
                    params: HashMap::new(),
                });
                continue;
            }
        };

        validation_errors.extend(validate_import_content(
            &parsed_import.key.channel,
            &parsed_import.key.area_name,
            &parsed_import.key.contest_external_id,
            &resolved.content,
            contest_max_marks_per_ballot(&resolved.contest),
        ));

        let ballot_box_key = BallotBoxKey {
            election_id: resolved.contest.election_id.clone(),
            area_id: resolved.area.id.clone(),
            contest_id: resolved.contest.id.clone(),
            channel: parsed_import.key.channel.to_string(),
        };
        let baseline = if let Some(cached_baseline) = baseline_cache.get(&ballot_box_key) {
            cached_baseline.clone()
        } else {
            let latest = get_latest_approved_tally_sheet(
                transaction,
                tenant_id,
                election_event_id,
                &resolved.contest.election_id,
                &resolved.area.id,
                &resolved.contest.id,
                &parsed_import.key.channel,
            )
            .await?;
            baseline_cache.insert(ballot_box_key, latest.clone());
            latest
        };
        let previous = baseline.as_ref().and_then(|sheet| sheet.content.clone());
        let previous_csv = previous.as_ref().map(|content| {
            render_ballot_box_csv(
                content,
                &resolved.candidate_names_by_id,
                &resolved.candidate_external_ids_by_id,
            )
        });
        let incoming_csv = render_ballot_box_csv(
            &resolved.content,
            &resolved.candidate_names_by_id,
            &resolved.candidate_external_ids_by_id,
        );
        let incoming_content_hash = hash_area_contest_results(&resolved.content)?;
        let change_type = classify_change(previous.as_ref(), &resolved.content)?;

        items.push(TallySheetImportPreviewItem {
            channel: parsed_import.key.channel.clone(),
            area_id: resolved.area.id,
            area_name: parsed_import.key.area_name,
            contest_id: resolved.contest.id,
            contest_name: presentation_display_name(&resolved.contest.presentation)
                .or(resolved.contest.description)
                .unwrap_or_default(),
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

#[instrument(skip_all, err)]
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
    conversion_validation_errors: Vec<TallySheetImportValidationError>,
    // Free-form record of how this import was produced (e.g. which source
    // element supplied the area names). Written verbatim to the import's
    // `annotations` — a derived record, never an input to the conversion.
    annotations: Option<Value>,
) -> Result<TallySheetImport> {
    let preview = preview_tally_sheet_import(
        transaction,
        tenant_id,
        election_event_id,
        document_id,
        source_format.clone(),
        selected_channel.clone(),
        canonical_csv_bytes,
        conversion_validation_errors,
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
            annotations.as_ref(),
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
        annotations.as_ref(),
    )
    .await?;

    let mut import_items = Vec::with_capacity(preview.items.len());

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

        import_items.push(TallySheetImportItem {
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
        });
    }

    insert_tally_sheet_import_items(transaction, &import_items).await?;

    Ok(import)
}

#[instrument(skip_all, err)]
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
        .ok_or_else(|| TallySheetImportError::NotFound(import_id.to_string()))?;
    if !matches!(
        import.status,
        TallySheetImportStatus::PENDING_REVIEW | TallySheetImportStatus::CONFLICTED
    ) {
        return Err(TallySheetImportError::InvalidReviewState {
            import_id: import_id.to_string(),
            status: import.status.to_string(),
        }
        .into());
    }

    let items = get_tally_sheet_import_items_for_review(
        transaction,
        tenant_id,
        election_event_id,
        import_id,
    )
    .await?;
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
        let tally_sheet = match review_tally_sheet_status(
            transaction,
            tenant_id,
            election_event_id,
            tally_sheet_id,
            reviewed_by_user_id,
            sheet_status.clone(),
        )
        .await?
        {
            ReviewTallySheetOutcome::Reviewed(tally_sheet) => tally_sheet,
            ReviewTallySheetOutcome::NotPending(tally_sheet) => {
                return Err(anyhow!(
                    "Generated tally sheet {tally_sheet_id} cannot be reviewed from status {}",
                    tally_sheet.status
                ));
            }
            ReviewTallySheetOutcome::NotFound => {
                return Err(anyhow!("Generated tally sheet {tally_sheet_id} not found"));
            }
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

#[instrument(skip_all, err)]
async fn find_stale_baseline_conflicts(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    items: &[TallySheetImportItemReviewSnapshot],
) -> Result<Vec<String>> {
    let mut conflicted_item_ids = Vec::new();

    for item in items {
        if item.generated_tally_sheet_id.is_none() {
            // UNCHANGED items never created/replaced a tally sheet, so they can't have a stale baseline.
            continue;
        }

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

#[instrument(skip_all, err)]
async fn generated_tally_sheet_is_stale(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    item: &TallySheetImportItemReviewSnapshot,
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
            || latest_sheet.status != TallySheetStatus::PENDING,
    )
}

fn baseline_matches_import_item(
    item: &TallySheetImportItemReviewSnapshot,
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
    candidate_external_ids_by_id: HashMap<String, String>,
    source_area_name: String,
    source_contest_external_id: String,
    source_candidate_external_ids: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct BallotBoxKey {
    election_id: String,
    area_id: String,
    contest_id: String,
    channel: String,
}

#[derive(Default)]
struct BallotBoxResolutionCache {
    areas_by_name: HashMap<String, Area>,
    contests_by_external_id: HashMap<String, Contest>,
    area_contest_membership: HashMap<(String, String), bool>,
    candidates_by_contest_id: HashMap<String, Vec<Candidate>>,
}

#[instrument(skip_all, err)]
async fn resolve_ballot_box_import(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_name: &str,
    contest_external_id: &str,
    mut content: AreaContestResults,
    cache: &mut BallotBoxResolutionCache,
) -> Result<ResolvedBallotBoxImport> {
    let area = if let Some(cached_area) = cache.areas_by_name.get(area_name) {
        cached_area.clone()
    } else {
        let loaded_area = get_area_by_name(transaction, tenant_id, election_event_id, area_name)
            .await?
            .ok_or_else(|| anyhow!("Area '{area_name}' not found"))?;
        cache
            .areas_by_name
            .insert(area_name.to_string(), loaded_area.clone());
        loaded_area
    };
    let contest =
        if let Some(cached_contest) = cache.contests_by_external_id.get(contest_external_id) {
            cached_contest.clone()
        } else {
            let loaded_contest = get_contest_by_external_id(
                transaction,
                tenant_id,
                election_event_id,
                contest_external_id,
            )
            .await?
            .ok_or_else(|| anyhow!("Contest external id '{contest_external_id}' not found"))?;
            cache
                .contests_by_external_id
                .insert(contest_external_id.to_string(), loaded_contest.clone());
            loaded_contest
        };
    let area_contest_key = (area.id.clone(), contest.id.clone());
    let area_has_contest =
        if let Some(cached_membership) = cache.area_contest_membership.get(&area_contest_key) {
            *cached_membership
        } else {
            let exists = area_contest_exists(
                transaction,
                tenant_id,
                election_event_id,
                &area.id,
                &contest.id,
            )
            .await?;
            cache
                .area_contest_membership
                .insert(area_contest_key, exists);
            exists
        };
    if !area_has_contest {
        return Err(anyhow!(
            "Area '{area_name}' is not assigned to contest external id '{contest_external_id}'"
        ));
    }

    let candidates = if let Some(cached_candidates) =
        cache.candidates_by_contest_id.get(&contest.id)
    {
        cached_candidates.clone()
    } else {
        let loaded_candidates =
            get_candidates_by_contest_id(transaction, tenant_id, election_event_id, &contest.id)
                .await?;
        cache
            .candidates_by_contest_id
            .insert(contest.id.clone(), loaded_candidates.clone());
        loaded_candidates
    };
    let mut candidates_by_external_id = HashMap::new();
    let mut duplicate_candidate_external_ids = BTreeSet::new();
    let mut candidates_missing_external_id = Vec::new();
    let mut candidate_names_by_id = HashMap::new();
    let mut candidate_external_ids_by_id = HashMap::new();

    for candidate in &candidates {
        let candidate_name = presentation_display_name(&candidate.presentation)
            .or_else(|| candidate.description.clone())
            .unwrap_or_default();
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
        candidate_external_ids_by_id.insert(candidate.id.clone(), external_id.to_string());

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
        candidate_external_ids_by_id,
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
