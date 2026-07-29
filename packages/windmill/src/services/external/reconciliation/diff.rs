// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::external::datafix_types::{
    channels_equal, file_channel_to_keycloak, keycloak_channel_to_file, DatafixReconciliationField,
    ParsedDatafixReconciliationRow, FILE_CHANNEL_INTERNET,
};
use crate::services::external::types::{
    ReconciliationChangeCategory, ReconciliationPatchSource, ReconciliationPatchTarget,
    SequentReconciliationField,
};
use crate::services::users::VoterSnapshot;
use sequent_core::types::keycloak::{
    ATTR_RESET_VALUE, DATE_OF_BIRTH, DISABLE_COMMENT, DISABLE_REASON_DELETE_CALL,
    DISABLE_REASON_MARKVOTED_CALL, VOTED_CHANNEL,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{info, instrument};

/// One (voter, field) change destined for either the Datafix patch or a
/// direct Sequent apply. Never persisted on its own — always as part of a
/// serialized into one of the two documents `generate_reconciliation_patches`
/// uploads: the full `ReconciliationDiff` envelope (both sides, for review)
/// and the Sequent-side NDJSON apply stream. Each is written once and
/// never mutated afterward, so unlike an earlier draft of this type there's
/// no `apply_status` field here — the outcome of applying lives in the
/// row-failures document and the electoral log artifact instead, not back on
/// the item that was applied. The field this item changes, and its own
/// `(old, new)` pair, both live on `target` itself
/// (`ReconciliationPatchTarget::Datafix`/`Sequent`) — not as separate
/// `old_value`/`new_value` members here, since a field is never meaningful
/// apart from its own old/new values and which field enum applies depends on
/// which side `target` is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffItem {
    pub voter_username: String,
    #[serde(flatten)]
    pub target: ReconciliationPatchTarget,
    pub category: ReconciliationChangeCategory,
    pub failure_reason: Option<String>,
}

/// The full content of the "diff envelope" document `generate_reconciliation_patches`
/// uploads once per round, referenced from `task_execution.annotations.document_id`.
/// There is no `external_reconciliation_import` table or row of any kind — this
/// document *is* the record. The frontend fetches and parses the whole thing
/// to render both diff tables; `apply_reconciliation_patch` re-fetches and
/// re-parses it server-side (never trusting client-supplied fields for the
/// checks below) to find the Sequent patch document to apply and to
/// re-validate the external side is clean.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationDiff {
    pub sequence: i64,
    pub generated_at: i64,
    pub source_sha256: String,
    /// `None` when the external-side diff was empty. Apply is only allowed
    /// once this is `None` — a non-empty external side means that system's
    /// own process hasn't converged yet.
    pub external_patch_document_id: Option<String>,
    pub external_patch_sha256: Option<String>,
    /// Every `target = Sequent` item, serialized as an NDJSON stream so apply
    /// can process one voter at a time. It includes `ROW_FAILURE` items for
    /// structured reporting; those entries are never treated as mutations.
    pub sequent_patch_document_id: String,
    /// False for an already-successful Sequence: the envelope is a diff-only
    /// convergence check and must not be applied again. True for a new round
    /// or a same-Sequence retry whose previous apply had row failures.
    #[serde(default = "default_apply_allowed")]
    pub apply_allowed: bool,
    /// Every item, both sides, including `ROW_FAILURE`s — what the review UI
    /// renders.
    pub items: Vec<DiffItem>,
}

fn default_apply_allowed() -> bool {
    true
}

/// Original Datafix area columns indexed by Sequent's composed area name.
/// The composed name is not generally reversible (components may contain
/// hyphens and optional components are omitted), so the reverse voter pass
/// learns this mapping from rows in the same reconciliation file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatafixAreaFields {
    pub ward: String,
    pub poll: String,
    pub school_support_code: String,
}

pub type DatafixAreaFieldsByName = HashMap<String, Option<DatafixAreaFields>>;

pub fn index_datafix_area_fields(
    index: &mut DatafixAreaFieldsByName,
    rows: &[ParsedDatafixReconciliationRow],
) {
    for row in rows {
        let name = composed_area_name(row);
        let fields = DatafixAreaFields {
            ward: row.ward.clone(),
            poll: row.poll.clone(),
            school_support_code: row.school_support_code.clone(),
        };
        index
            .entry(name)
            .and_modify(|current| {
                if current.as_ref() != Some(&fields) {
                    *current = None;
                }
            })
            .or_insert(Some(fields));
    }
}

/// Runs the forward pass for one batch of file rows (see
/// `services::external::reconciliation::csv::ReconciliationRowBatches`):
/// classifies each row against its Sequent snapshot, if this batch's
/// `fetch_realm_voter_snapshots_by_usernames` call found one —
/// `snapshots_by_username` holds only this batch's matches, not the whole
/// realm, so a missing entry means this file row's voter doesn't exist in
/// Sequent at all (D, forward direction, handled inside `classify_file_row`
/// same as before). `source` carries whatever config the file's own origin
/// needs for classification (for Datafix, its `CountyMun`, from the event's
/// own `VoterviewRequest::county_mun`).
#[instrument(skip_all, fields(batch_size = file_rows.len()))]
pub fn diff_file_row_batch(
    file_rows: &[ParsedDatafixReconciliationRow],
    snapshots_by_username: &HashMap<String, VoterSnapshot>,
    source: &ReconciliationPatchSource,
) -> Vec<DiffItem> {
    file_rows
        .iter()
        .flat_map(|row| classify_file_row(row, snapshots_by_username.get(&row.voter_id), source))
        .collect()
}

/// D, reverse direction, run against one page of Sequent's own voter walk
/// (`users::fetch_realm_voter_snapshots_page`) after every batch of the file
/// has been scanned, so `all_file_usernames` (every `VoterID` seen across
/// every batch) is complete: an enabled voter whose username was never seen
/// in the file at all is reported into the Datafix patch via
/// `voter_missing_from_file`. Disabled voters need no report here — if
/// Sequent already considers them gone, there's nothing for Datafix to
/// catch up on regardless of whether the file mentions them.
#[instrument(skip_all, fields(page_size = snapshots_page.len()))]
pub fn diff_unmatched_sequent_voters(
    snapshots_page: &[VoterSnapshot],
    all_file_usernames: &HashSet<String>,
    source: &ReconciliationPatchSource,
    area_fields_by_name: &DatafixAreaFieldsByName,
) -> Vec<DiffItem> {
    snapshots_page
        .iter()
        .filter(|snapshot| snapshot.enabled && !all_file_usernames.contains(&snapshot.username))
        .flat_map(|snapshot| {
            voter_missing_from_file(&snapshot.username, snapshot, source, area_fields_by_name)
        })
        .collect()
}

/// Classifies a single file row against the voter's current Sequent snapshot
/// (`None` if Sequent doesn't have this voter at all — source D, forward
/// direction). Returns zero or more `DiffItem`s: most rows produce 0-2 items,
/// but the "voted via other channel on a voter holding a valid Internet
/// ballot" exception produces **two** — a `ROW_FAILURE` (excluded from
/// apply) *and* a `VOTED_INTERNET`-style correction so the Datafix patch
/// still keeps `Channel=INTERNET` on their side for the rest of the round
/// (spec, "Concrete examples" B) — easy to miss, worth a dedicated test.
#[instrument(skip_all, fields(voter_username = %row.voter_id))]
fn classify_file_row(
    row: &ParsedDatafixReconciliationRow,
    snapshot: Option<&VoterSnapshot>,
    source: &ReconciliationPatchSource,
) -> Vec<DiffItem> {
    let ReconciliationPatchSource::Datafix { county_mun } = source;
    let event_county_mun = county_mun.as_str();
    if row.county_mun != event_county_mun && row.deleted != "true" {
        return vec![DiffItem {
            voter_username: row.voter_id.clone(),
            // Excluded from both diffs regardless; `CountyMun` has no Sequent
            // equivalent field, so there is nothing to name here.
            target: ReconciliationPatchTarget::Sequent(None),
            category: ReconciliationChangeCategory::ROW_FAILURE,
            failure_reason: Some(format!(
                "CountyMun ({}) does not match this election event's CountyMun ({event_county_mun}) \
                 and the row is not marked Deleted — Datafix processing error.",
                row.county_mun,
            )),
        }];
    }

    let Some(snapshot) = snapshot else {
        // A file cannot manufacture an Internet ballot for a voter Sequent
        // does not even know. Correct Datafix first; the next clean file will
        // add the voter with Channel=NONE.
        if channels_equal(&row.channel, FILE_CHANNEL_INTERNET) {
            return vec![diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                    row.channel.clone(),
                    ATTR_RESET_VALUE.to_string(),
                )),
                ReconciliationChangeCategory::VOTED_UNMARKED,
            )];
        }
        return voter_added_to_sequent(row);
    };

    let mut items = Vec::new();
    let file_says_none = channels_equal(&row.channel, ATTR_RESET_VALUE);
    let file_says_internet = channels_equal(&row.channel, FILE_CHANNEL_INTERNET);
    let stored_channel = snapshot
        .voted_channel
        .as_deref()
        .unwrap_or(ATTR_RESET_VALUE);
    let stored_disable_comment = snapshot
        .disable_comment
        .as_deref()
        .unwrap_or(ATTR_RESET_VALUE);
    let unmarking_stored_channel = file_says_none
        && !snapshot.has_valid_internet_vote
        && !snapshot.has_unresolved_internet_vote
        && !channels_equal(stored_channel, ATTR_RESET_VALUE);
    let applying_other_channel = !file_says_none
        && !file_says_internet
        && !snapshot.has_valid_internet_vote
        && !snapshot.has_unresolved_internet_vote;

    // A) Sequent holds a valid Internet ballot; Datafix says NONE.
    if file_says_none && snapshot.has_valid_internet_vote {
        items.push(diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                row.channel.clone(),
                FILE_CHANNEL_INTERNET.to_string(),
            )),
            ReconciliationChangeCategory::VOTED_INTERNET,
        ));
    } else if file_says_none && snapshot.has_unresolved_internet_vote {
        items.push(row_failure(
            &row.voter_id,
            "Voter has an in-progress Internet ballot; Channel=NONE cannot be reconciled until the ballot resolves",
        ));
    } else if unmarking_stored_channel {
        // File-driven equivalent of `/unmark-voted`. Compute the final
        // disable reason here, together with the channel reset, so a
        // Deleted=true row never emits two competing writes to the same
        // attribute. A manual/admin disable is not undone by an unmark: only
        // MARKVOTED_CALL owns the enable transition and its comment.
        let desired_disable_comment = if row.deleted == "true" {
            DISABLE_REASON_DELETE_CALL
        } else if !snapshot.enabled && stored_disable_comment != DISABLE_REASON_MARKVOTED_CALL {
            stored_disable_comment
        } else {
            ATTR_RESET_VALUE
        };
        let mut old_attributes =
            HashMap::from([(VOTED_CHANNEL.to_string(), stored_channel.to_string())]);
        let mut new_attributes =
            HashMap::from([(VOTED_CHANNEL.to_string(), ATTR_RESET_VALUE.to_string())]);
        if desired_disable_comment != stored_disable_comment {
            old_attributes.insert(
                DISABLE_COMMENT.to_string(),
                stored_disable_comment.to_string(),
            );
            new_attributes.insert(
                DISABLE_COMMENT.to_string(),
                desired_disable_comment.to_string(),
            );
        }
        items.push(diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::KeycloakUA(
                old_attributes,
                new_attributes,
            ))),
            ReconciliationChangeCategory::VOTED_UNMARKED,
        ));
        if !snapshot.enabled
            && row.deleted != "true"
            && stored_disable_comment == DISABLE_REASON_MARKVOTED_CALL
        {
            items.push(diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    false, true,
                ))),
                ReconciliationChangeCategory::VOTED_UNMARKED,
            ));
        }
    } else if file_says_internet && snapshot.has_unresolved_internet_vote {
        items.push(row_failure(
            &row.voter_id,
            "Voter has an in-progress Internet ballot; Channel=INTERNET cannot be confirmed until the ballot resolves",
        ));
    } else if file_says_internet && !snapshot.has_valid_internet_vote {
        // The reverse half of source-of-truth A: Datafix cannot claim an
        // Internet vote that Sequent has no active record of.
        items.push(diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                row.channel.clone(),
                ATTR_RESET_VALUE.to_string(),
            )),
            ReconciliationChangeCategory::VOTED_UNMARKED,
        ));
    }

    // B) Datafix reports another channel. The same active-ballot guard used
    // by the real-time API applies to both valid and in-progress ballots.
    if !file_says_none && !file_says_internet {
        if snapshot.has_valid_internet_vote {
            // Exception: cannot resolve automatically — row failure now, and
            // Sequent still wins for the rest of the round (spec, example B).
            items.push(DiffItem {
                voter_username: row.voter_id.clone(),
                target: ReconciliationPatchTarget::Sequent(Some(
                    SequentReconciliationField::KeycloakUA(
                        HashMap::from([(VOTED_CHANNEL.to_string(), stored_channel.to_string())]),
                        HashMap::from([(VOTED_CHANNEL.to_string(), row.channel.clone())]),
                    ),
                )),
                category: ReconciliationChangeCategory::ROW_FAILURE,
                failure_reason: Some(format!(
                    "Voter holds a valid Internet ballot; voted-via-other-channel ({}) cannot be \
                     resolved automatically — release the voter via edit_user after the freeze \
                     ends.",
                    row.channel,
                )),
            });
            items.push(diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                    row.channel.clone(),
                    FILE_CHANNEL_INTERNET.to_string(),
                )),
                ReconciliationChangeCategory::VOTED_INTERNET,
            ));
        } else if snapshot.has_unresolved_internet_vote {
            items.push(row_failure(
                &row.voter_id,
                &format!(
                    "Voter has an in-progress Internet ballot; voted-via-other-channel ({}) cannot be applied until it resolves",
                    row.channel
                ),
            ));
        } else {
            // Both the voted channel and why the voter is being disabled are
            // plain Keycloak attributes — carried together so `apply` writes
            // them in a single edit without knowing this came from Datafix.
            // Deleted=true takes precedence over MARKVOTED_CALL in the same
            // canonical item so the delete block below has nothing to
            // overwrite later.
            let desired_disable_comment = if row.deleted == "true" {
                DISABLE_REASON_DELETE_CALL
            } else {
                DISABLE_REASON_MARKVOTED_CALL
            };
            if !channels_equal(stored_channel, &row.channel)
                || stored_disable_comment != desired_disable_comment
            {
                items.push(diff_item(
                    &row.voter_id,
                    ReconciliationPatchTarget::Sequent(Some(
                        SequentReconciliationField::KeycloakUA(
                            HashMap::from([
                                (VOTED_CHANNEL.to_string(), stored_channel.to_string()),
                                (
                                    DISABLE_COMMENT.to_string(),
                                    stored_disable_comment.to_string(),
                                ),
                            ]),
                            HashMap::from([
                                (
                                    VOTED_CHANNEL.to_string(),
                                    file_channel_to_keycloak(&row.channel),
                                ),
                                (
                                    DISABLE_COMMENT.to_string(),
                                    desired_disable_comment.to_string(),
                                ),
                            ]),
                        ),
                    )),
                    ReconciliationChangeCategory::VOTED_OTHER_CHANNEL,
                ));
            }
            if snapshot.enabled {
                items.push(diff_item(
                    &row.voter_id,
                    ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                        true, false,
                    ))),
                    ReconciliationChangeCategory::VOTED_OTHER_CHANNEL,
                ));
            }
        }
    }

    // C) Profile fields — Datafix wins. Ward/Poll/SchoolSupportCode are
    // compared as a single composed area name (see the module doc in
    // snapshot.rs for why they can't be compared field-by-field), surfaced
    // here as an AreaName-field change for display/patch purposes.
    let file_area_name = composed_area_name(row);
    if snapshot.area_name.as_deref() != Some(file_area_name.as_str()) {
        items.push(diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::AreaName(
                snapshot
                    .area_name
                    .clone()
                    .unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
                file_area_name.clone(),
            ))),
            ReconciliationChangeCategory::PROFILE_UPDATE,
        ));
    }
    if Some(row.dob.as_str()) != snapshot.dob.as_deref() {
        items.push(diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::KeycloakUA(
                HashMap::from([(
                    DATE_OF_BIRTH.to_string(),
                    snapshot
                        .dob
                        .clone()
                        .unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
                )]),
                HashMap::from([(DATE_OF_BIRTH.to_string(), row.dob.clone())]),
            ))),
            ReconciliationChangeCategory::PROFILE_UPDATE,
        ));
    }

    // C) Deleted=true — Datafix wins unless an active Internet ballot makes
    // the transition unsafe. Profile changes above still reconcile disabled
    // voters in the same round.
    if row.deleted == "true" {
        if snapshot.has_valid_internet_vote {
            items.push(diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Deleted(
                    "true".to_string(),
                    "false".to_string(),
                )),
                ReconciliationChangeCategory::DELETION_REVERTED,
            ));
        } else if snapshot.has_unresolved_internet_vote {
            items.push(row_failure(
                &row.voter_id,
                "Voter has an in-progress Internet ballot and cannot be deleted until it resolves",
            ));
        } else {
            // An authoritative non-Internet channel already emitted the
            // same enabled=false transition; every other Deleted=true path
            // gets it here exactly once.
            if snapshot.enabled && !applying_other_channel {
                items.push(diff_item(
                    &row.voter_id,
                    ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                        true, false,
                    ))),
                    ReconciliationChangeCategory::DISABLED_DELETE_CALL,
                ));
            }
            // Channel transitions in A/B already assigned DELETE_CALL as the
            // canonical final reason. Do not emit a duplicate attribute
            // write whose winner would otherwise depend on item ordering.
            if !unmarking_stored_channel
                && !applying_other_channel
                && stored_disable_comment != DISABLE_REASON_DELETE_CALL
            {
                items.push(diff_item(
                    &row.voter_id,
                    ReconciliationPatchTarget::Sequent(Some(
                        SequentReconciliationField::KeycloakUA(
                            HashMap::from([(
                                DISABLE_COMMENT.to_string(),
                                stored_disable_comment.to_string(),
                            )]),
                            HashMap::from([(
                                DISABLE_COMMENT.to_string(),
                                DISABLE_REASON_DELETE_CALL.to_string(),
                            )]),
                        ),
                    )),
                    ReconciliationChangeCategory::DISABLED_DELETE_CALL,
                ));
            }
        }
    } else if !snapshot.enabled
        && stored_disable_comment == DISABLE_REASON_DELETE_CALL
        && file_says_none
        && channels_equal(stored_channel, ATTR_RESET_VALUE)
    {
        items.push(diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                false, true,
            ))),
            ReconciliationChangeCategory::REENABLED,
        ));
    }

    // A row-level safety failure excludes every Sequent mutation for that
    // voter in this round. Datafix corrections may remain (for example a
    // valid Internet ballot correcting PAPER back to INTERNET), because the
    // external patch must still move the source file toward convergence.
    let failure_reasons: Vec<String> = items
        .iter()
        .filter(|item| item.category == ReconciliationChangeCategory::ROW_FAILURE)
        .filter_map(|item| item.failure_reason.clone())
        .collect();
    if !failure_reasons.is_empty() {
        items.retain(|item| {
            item.category != ReconciliationChangeCategory::ROW_FAILURE && item.target.is_datafix()
        });
        items.push(row_failure(&row.voter_id, &failure_reasons.join("; ")));
    }

    items
}

/// D, forward direction: the file has a voter Sequent doesn't. Per "Patch
/// Files Format", every field Sequent actually stores is present with `_old =
/// NONE` since Sequent has no prior record at all — expanded here into one
/// `DiffItem` per `SequentReconciliationField`, all sharing `VOTER_ADDED`/
/// `target = Sequent`. `CountyMun` has no Sequent equivalent, so unlike the
/// `DatafixReconciliationField::NAMES` set used for the outbound patch CSV,
/// this only covers the fields Sequent itself understands. The bulk apply
/// path consumes the emitted `Enabled` and `disable-comment` values exactly,
/// including file rows already marked deleted or voted via another channel.
#[instrument(skip_all, fields(voter_username = %row.voter_id))]
fn voter_added_to_sequent(row: &ParsedDatafixReconciliationRow) -> Vec<DiffItem> {
    let file_area_name = composed_area_name(row);
    let voted_other_channel = !channels_equal(&row.channel, ATTR_RESET_VALUE);
    let file_enabled = row.deleted != "true" && !voted_other_channel;
    let disable_comment = if row.deleted == "true" {
        Some(DISABLE_REASON_DELETE_CALL)
    } else if voted_other_channel {
        Some(DISABLE_REASON_MARKVOTED_CALL)
    } else {
        None
    };
    let mut items = vec![
        diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::AreaName(
                ATTR_RESET_VALUE.to_string(),
                file_area_name,
            ))),
            ReconciliationChangeCategory::VOTER_ADDED,
        ),
        diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::KeycloakUA(
                HashMap::from([(DATE_OF_BIRTH.to_string(), ATTR_RESET_VALUE.to_string())]),
                HashMap::from([(DATE_OF_BIRTH.to_string(), row.dob.clone())]),
            ))),
            ReconciliationChangeCategory::VOTER_ADDED,
        ),
        diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::KeycloakUA(
                HashMap::from([(VOTED_CHANNEL.to_string(), ATTR_RESET_VALUE.to_string())]),
                HashMap::from([(
                    VOTED_CHANNEL.to_string(),
                    file_channel_to_keycloak(&row.channel),
                )]),
            ))),
            ReconciliationChangeCategory::VOTER_ADDED,
        ),
        diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                false,
                file_enabled,
            ))),
            ReconciliationChangeCategory::VOTER_ADDED,
        ),
    ];
    if let Some(disable_comment) = disable_comment {
        items.push(diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::KeycloakUA(
                HashMap::from([(DISABLE_COMMENT.to_string(), ATTR_RESET_VALUE.to_string())]),
                HashMap::from([(DISABLE_COMMENT.to_string(), disable_comment.to_string())]),
            ))),
            ReconciliationChangeCategory::VOTER_ADDED,
        ));
    }
    items
}

/// D, reverse direction: Sequent has an enabled voter the file doesn't
/// mention at all — reported into the Datafix patch so their side adds it.
#[instrument(skip_all)]
fn voter_missing_from_file(
    username: &str,
    snapshot: &VoterSnapshot,
    source: &ReconciliationPatchSource,
    area_fields_by_name: &DatafixAreaFieldsByName,
) -> Vec<DiffItem> {
    let ReconciliationPatchSource::Datafix { county_mun } = source;
    let Some(area_name) = snapshot.area_name.as_deref() else {
        return vec![row_failure(
            username,
            "Sequent-only voter has no resolvable area; cannot build a complete Datafix add patch",
        )];
    };
    let Some(Some(area_fields)) = area_fields_by_name.get(area_name) else {
        return vec![row_failure(
            username,
            &format!(
                "Cannot recover Ward/Poll/SchoolSupportCode for Sequent area '{area_name}' from this reconciliation file"
            ),
        )];
    };
    let fields = [
        DatafixReconciliationField::CountyMun(ATTR_RESET_VALUE.to_string(), county_mun.clone()),
        DatafixReconciliationField::Ward(ATTR_RESET_VALUE.to_string(), area_fields.ward.clone()),
        DatafixReconciliationField::Poll(ATTR_RESET_VALUE.to_string(), area_fields.poll.clone()),
        DatafixReconciliationField::SchoolSupportCode(
            ATTR_RESET_VALUE.to_string(),
            area_fields.school_support_code.clone(),
        ),
        DatafixReconciliationField::DoB(
            ATTR_RESET_VALUE.to_string(),
            snapshot
                .dob
                .clone()
                .unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
        ),
        DatafixReconciliationField::Channel(
            ATTR_RESET_VALUE.to_string(),
            if snapshot.has_valid_internet_vote {
                FILE_CHANNEL_INTERNET.to_string()
            } else {
                snapshot
                    .voted_channel
                    .as_deref()
                    .map(keycloak_channel_to_file)
                    .unwrap_or_else(|| ATTR_RESET_VALUE.to_string())
            },
        ),
        DatafixReconciliationField::Deleted(ATTR_RESET_VALUE.to_string(), "false".to_string()),
    ];
    fields
        .into_iter()
        .map(|field| {
            diff_item(
                username,
                ReconciliationPatchTarget::Datafix(field),
                ReconciliationChangeCategory::VOTER_ADDED,
            )
        })
        .collect()
}

#[instrument(skip_all)]
fn diff_item(
    voter_username: &str,
    target: ReconciliationPatchTarget,
    category: ReconciliationChangeCategory,
) -> DiffItem {
    DiffItem {
        voter_username: voter_username.to_string(),
        target,
        category,
        failure_reason: None,
    }
}

#[instrument(skip_all)]
fn row_failure(voter_username: &str, reason: &str) -> DiffItem {
    DiffItem {
        voter_username: voter_username.to_string(),
        target: ReconciliationPatchTarget::Sequent(None),
        category: ReconciliationChangeCategory::ROW_FAILURE,
        failure_reason: Some(reason.to_string()),
    }
}

/// Composes a file row's Ward-SchoolSupportCode-Poll into the same format as
/// `Area::name`, reusing `external::utils::compose_area_name` (via a minimal
/// `VoterInformationBody`) so the join/uppercase rule is defined in exactly
/// one place.
#[instrument(skip_all)]
fn composed_area_name(row: &ParsedDatafixReconciliationRow) -> String {
    use crate::services::external::datafix_types::VoterInformationBody;
    use crate::services::external::utils::compose_area_name;

    // The file's own "no value" sentinel (`ATTR_RESET_VALUE`) must not be
    // concatenated into the composed name as a literal segment — translate it
    // to `None` first so `compose_area_name`'s own empty-value handling
    // (built for the inbound API's `Option<String>` contract) omits it.
    let optional_field = |value: &str| (value != ATTR_RESET_VALUE).then(|| value.to_string());

    let composed_area = compose_area_name(&VoterInformationBody {
        voter_id: row.voter_id.clone(),
        ward: row.ward.clone(),
        schoolboard: optional_field(&row.school_support_code),
        poll: optional_field(&row.poll),
        birthdate: None,
        enabled: None,
    });
    info!(%composed_area, "Composed area name from file row");
    composed_area
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(voter_id: &str, channel: &str, deleted: &str) -> ParsedDatafixReconciliationRow {
        ParsedDatafixReconciliationRow {
            county_mun: "0014".to_string(),
            voter_id: voter_id.to_string(),
            dob: "1990-01-01".to_string(),
            ward: "01".to_string(),
            poll: "000".to_string(),
            school_support_code: "P".to_string(),
            channel: channel.to_string(),
            deleted: deleted.to_string(),
        }
    }

    fn datafix_source(county_mun: &str) -> ReconciliationPatchSource {
        ReconciliationPatchSource::Datafix {
            county_mun: county_mun.to_string(),
        }
    }

    fn enabled_snapshot() -> VoterSnapshot {
        VoterSnapshot {
            username: "voter-1".to_string(),
            voter_id_string: "voter-1-id".to_string(),
            enabled: true,
            area_name: Some("01-P-000".to_string()),
            dob: Some("1990-01-01".to_string()),
            voted_channel: None,
            has_valid_internet_vote: false,
            has_unresolved_internet_vote: false,
            disable_comment: None,
        }
    }

    #[test]
    fn county_mun_mismatch_is_a_row_failure() {
        let mut bad_row = row("voter-1", ATTR_RESET_VALUE, "false");
        bad_row.county_mun = "0099".to_string();
        let items = classify_file_row(&bad_row, Some(&enabled_snapshot()), &datafix_source("0014"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category, ReconciliationChangeCategory::ROW_FAILURE);
        assert_eq!(items[0].target, ReconciliationPatchTarget::Sequent(None));
    }

    #[test]
    fn county_mun_mismatch_is_ignored_when_deleted() {
        let mut bad_row = row("voter-1", ATTR_RESET_VALUE, "true");
        bad_row.county_mun = "0099".to_string();
        let items = classify_file_row(&bad_row, Some(&enabled_snapshot()), &datafix_source("0014"));
        assert!(items
            .iter()
            .all(|item| item.category != ReconciliationChangeCategory::ROW_FAILURE));
    }

    #[test]
    fn a_voted_internet_ballot_wins_over_none_in_the_file() {
        let snapshot = VoterSnapshot {
            has_valid_internet_vote: true,
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "false"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].category,
            ReconciliationChangeCategory::VOTED_INTERNET
        );
        assert_eq!(
            items[0].target,
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                ATTR_RESET_VALUE.to_string(),
                FILE_CHANNEL_INTERNET.to_string()
            ))
        );
    }

    #[test]
    fn other_channel_wins_when_sequent_has_not_voted() {
        let items = classify_file_row(
            &row("voter-1", "PAPER", "false"),
            Some(&enabled_snapshot()),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::VOTED_OTHER_CHANNEL));
        let attributes_item = items
            .iter()
            .find_map(|item| item.target.sequent_field()?.new_keycloak_attributes())
            .expect("one item carries the Keycloak attributes");
        assert_eq!(
            attributes_item.get(VOTED_CHANNEL),
            Some(&"PAPER".to_string())
        );
        assert_eq!(
            attributes_item.get(DISABLE_COMMENT),
            Some(&DISABLE_REASON_MARKVOTED_CALL.to_string())
        );
        assert!(items.iter().any(|item| {
            item.target
                == ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    true, false,
                )))
        }));
    }

    #[test]
    fn other_channel_wins_and_keeps_the_voters_prior_channel_as_the_old_value() {
        // A voter already recorded with some channel (not the one the file now
        // reports) must not show "NONE" as the diff's old value — only a
        // voter with no prior channel at all should.
        let snapshot = VoterSnapshot {
            voted_channel: Some("PHONE".to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", "PAPER", "false"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        let keycloak_field = items
            .iter()
            .find_map(|item| match item.target.sequent_field() {
                Some(field @ SequentReconciliationField::KeycloakUA(..)) => Some(field),
                _ => None,
            })
            .expect("one item carries the Keycloak attributes");
        let SequentReconciliationField::KeycloakUA(old, new) = keycloak_field else {
            unreachable!("matched above");
        };
        assert_eq!(old.get(VOTED_CHANNEL), Some(&"PHONE".to_string()));
        assert_eq!(new.get(VOTED_CHANNEL), Some(&"PAPER".to_string()));
    }

    #[test]
    fn other_channel_on_a_valid_internet_ballot_produces_a_failure_and_a_correction() {
        let snapshot = VoterSnapshot {
            has_valid_internet_vote: true,
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", "PAPER", "false"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .any(|item| item.category == ReconciliationChangeCategory::ROW_FAILURE));
        assert!(items.iter().any(|item| {
            item.category == ReconciliationChangeCategory::VOTED_INTERNET
                && item.target
                    == ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                        "PAPER".to_string(),
                        FILE_CHANNEL_INTERNET.to_string(),
                    ))
        }));
    }

    #[test]
    fn deleted_true_disables_a_voter_who_has_not_voted() {
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "true"),
            Some(&enabled_snapshot()),
            &datafix_source("0014"),
        );
        assert!(items.iter().any(|item| {
            item.category == ReconciliationChangeCategory::DISABLED_DELETE_CALL
                && item.target
                    == ReconciliationPatchTarget::Sequent(Some(
                        SequentReconciliationField::Enabled(true, false),
                    ))
        }));
    }

    #[test]
    fn deleted_true_is_reverted_for_a_voter_who_has_voted() {
        let snapshot = VoterSnapshot {
            has_valid_internet_vote: true,
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "true"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert!(items.iter().any(|item| {
            item.category == ReconciliationChangeCategory::DELETION_REVERTED
                && item.target
                    == ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Deleted(
                        "true".to_string(),
                        "false".to_string(),
                    ))
        }));
        assert!(items
            .iter()
            .all(|item| item.category != ReconciliationChangeCategory::DISABLED_DELETE_CALL));
    }

    #[test]
    fn unknown_voter_is_added_to_sequent_with_none_old_values() {
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "false"),
            None,
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 4); // one per SequentReconciliationField
        assert!(items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::VOTER_ADDED));
        assert!(items.iter().all(|item| item.target.is_sequent()));
        let area_item = items
            .iter()
            .find_map(|item| item.target.sequent_field())
            .and_then(|field| field.new_area_name().map(|_| field))
            .expect("one item carries the area name");
        assert!(matches!(
            area_item,
            SequentReconciliationField::AreaName(old, _) if old == ATTR_RESET_VALUE
        ));
        assert!(items.iter().any(|item| {
            item.target
                == ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    false, true,
                )))
        }));
    }

    #[test]
    fn disabled_voter_deleted_externally_converges_when_file_still_says_deleted() {
        let snapshot = VoterSnapshot {
            enabled: false,
            disable_comment: Some(DISABLE_REASON_DELETE_CALL.to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "true"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert!(items.is_empty());
    }

    #[test]
    fn disabled_voter_deleted_externally_is_reenabled_when_file_says_not_deleted() {
        let snapshot = VoterSnapshot {
            enabled: false,
            disable_comment: Some(DISABLE_REASON_DELETE_CALL.to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "false"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category, ReconciliationChangeCategory::REENABLED);
        assert_eq!(
            items[0].target,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                false, true
            )))
        );
    }

    #[test]
    fn disabled_voter_marked_voted_via_other_channel_is_retagged_as_delete_call() {
        let snapshot = VoterSnapshot {
            enabled: false,
            disable_comment: Some(DISABLE_REASON_MARKVOTED_CALL.to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", "PAPER", "true"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 1);
        let attributes_item = items
            .iter()
            .find_map(|item| item.target.sequent_field()?.new_keycloak_attributes())
            .expect("the canonical channel item carries the delete reason");
        assert_eq!(
            attributes_item.get(DISABLE_COMMENT),
            Some(&DISABLE_REASON_DELETE_CALL.to_string())
        );
    }

    #[test]
    fn admin_disabled_voter_with_no_sentinel_comment_is_also_retagged_as_delete_call() {
        let snapshot = VoterSnapshot {
            enabled: false,
            disable_comment: Some("Released after voting online".to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "true"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 1);
        assert!(items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::DISABLED_DELETE_CALL));
    }

    #[test]
    fn voter_missing_from_file_is_reported_externally() {
        let area_fields = HashMap::from([(
            "01-P-000".to_string(),
            Some(DatafixAreaFields {
                ward: "01".to_string(),
                poll: "000".to_string(),
                school_support_code: "P".to_string(),
            }),
        )]);
        let items = voter_missing_from_file(
            "voter-1",
            &enabled_snapshot(),
            &datafix_source("0014"),
            &area_fields,
        );
        assert_eq!(items.len(), 7);
        assert!(items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::VOTER_ADDED));
        assert!(items.iter().all(|item| item.target.is_datafix()));
        assert!(items.iter().any(|item| {
            item.target
                == ReconciliationPatchTarget::Datafix(DatafixReconciliationField::CountyMun(
                    ATTR_RESET_VALUE.to_string(),
                    "0014".to_string(),
                ))
        }));
        assert!(items.iter().any(|item| {
            item.target
                == ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Poll(
                    ATTR_RESET_VALUE.to_string(),
                    "000".to_string(),
                ))
        }));
    }

    #[test]
    fn keycloak_internet_casing_converges_and_reverse_patch_is_uppercase() {
        let snapshot = VoterSnapshot {
            voted_channel: Some("Internet".to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", FILE_CHANNEL_INTERNET, "false"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].target,
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                FILE_CHANNEL_INTERNET.to_string(),
                ATTR_RESET_VALUE.to_string(),
            ))
        );

        let area_fields = HashMap::from([(
            "01-P-000".to_string(),
            Some(DatafixAreaFields {
                ward: "01".to_string(),
                poll: "000".to_string(),
                school_support_code: "P".to_string(),
            }),
        )]);
        let reverse =
            voter_missing_from_file("voter-1", &snapshot, &datafix_source("0014"), &area_fields);
        assert!(reverse.iter().any(|item| {
            item.target
                == ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                    ATTR_RESET_VALUE.to_string(),
                    FILE_CHANNEL_INTERNET.to_string(),
                ))
        }));
    }

    #[test]
    fn file_none_unmarks_a_stored_non_internet_vote() {
        let snapshot = VoterSnapshot {
            enabled: false,
            voted_channel: Some("PAPER".to_string()),
            disable_comment: Some(DISABLE_REASON_MARKVOTED_CALL.to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "false"),
            Some(&snapshot),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::VOTED_UNMARKED));
        assert!(items.iter().any(|item| {
            item.target
                == ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    false, true,
                )))
        }));
    }

    #[test]
    fn unmark_and_delete_emit_one_canonical_disable_comment() {
        let snapshot = VoterSnapshot {
            enabled: false,
            voted_channel: Some("PAPER".to_string()),
            disable_comment: Some(DISABLE_REASON_MARKVOTED_CALL.to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "true"),
            Some(&snapshot),
            &datafix_source("0014"),
        );

        assert_eq!(items.len(), 1);
        let SequentReconciliationField::KeycloakUA(_, new_attributes) = items[0]
            .target
            .sequent_field()
            .expect("unmark emits a Keycloak attribute change")
        else {
            panic!("expected Keycloak attributes");
        };
        assert_eq!(
            new_attributes.get(VOTED_CHANNEL),
            Some(&ATTR_RESET_VALUE.to_string())
        );
        assert_eq!(
            new_attributes.get(DISABLE_COMMENT),
            Some(&DISABLE_REASON_DELETE_CALL.to_string())
        );
    }

    #[test]
    fn unmark_preserves_an_existing_delete_reason_and_converges_in_one_round() {
        let snapshot = VoterSnapshot {
            enabled: false,
            voted_channel: Some("PAPER".to_string()),
            disable_comment: Some(DISABLE_REASON_DELETE_CALL.to_string()),
            ..enabled_snapshot()
        };
        let file_row = row("voter-1", ATTR_RESET_VALUE, "true");
        let items = classify_file_row(&file_row, Some(&snapshot), &datafix_source("0014"));

        assert_eq!(items.len(), 1);
        let SequentReconciliationField::KeycloakUA(_, new_attributes) = items[0]
            .target
            .sequent_field()
            .expect("unmark emits a Keycloak attribute change")
        else {
            panic!("expected Keycloak attributes");
        };
        assert_eq!(
            new_attributes,
            &HashMap::from([(VOTED_CHANNEL.to_string(), ATTR_RESET_VALUE.to_string())])
        );

        let after_apply = VoterSnapshot {
            voted_channel: Some(ATTR_RESET_VALUE.to_string()),
            ..snapshot
        };
        assert!(
            classify_file_row(&file_row, Some(&after_apply), &datafix_source("0014")).is_empty()
        );
    }

    #[test]
    fn unmark_does_not_enable_or_clear_an_admin_disabled_voter() {
        let manual_reason = "Disabled manually";
        let snapshot = VoterSnapshot {
            enabled: false,
            voted_channel: Some("PAPER".to_string()),
            disable_comment: Some(manual_reason.to_string()),
            ..enabled_snapshot()
        };
        let items = classify_file_row(
            &row("voter-1", ATTR_RESET_VALUE, "false"),
            Some(&snapshot),
            &datafix_source("0014"),
        );

        assert_eq!(items.len(), 1);
        let SequentReconciliationField::KeycloakUA(_, new_attributes) = items[0]
            .target
            .sequent_field()
            .expect("unmark emits a Keycloak attribute change")
        else {
            panic!("expected Keycloak attributes");
        };
        assert_eq!(
            new_attributes,
            &HashMap::from([(VOTED_CHANNEL.to_string(), ATTR_RESET_VALUE.to_string())])
        );
        assert!(items.iter().all(|item| !matches!(
            item.target.sequent_field(),
            Some(SequentReconciliationField::Enabled(_, _))
        )));
    }

    #[test]
    fn file_internet_without_an_active_ballot_is_corrected_to_none() {
        let items = classify_file_row(
            &row("voter-1", FILE_CHANNEL_INTERNET, "false"),
            Some(&enabled_snapshot()),
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].category,
            ReconciliationChangeCategory::VOTED_UNMARKED
        );
        assert_eq!(
            items[0].target,
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                FILE_CHANNEL_INTERNET.to_string(),
                ATTR_RESET_VALUE.to_string(),
            ))
        );
    }

    #[test]
    fn unresolved_internet_ballot_blocks_other_channel_and_deletion() {
        let snapshot = VoterSnapshot {
            has_unresolved_internet_vote: true,
            ..enabled_snapshot()
        };
        for file_row in [
            row("voter-1", "PAPER", "false"),
            row("voter-1", ATTR_RESET_VALUE, "true"),
            row("voter-1", FILE_CHANNEL_INTERNET, "false"),
        ] {
            let items = classify_file_row(&file_row, Some(&snapshot), &datafix_source("0014"));
            assert!(items
                .iter()
                .any(|item| item.category == ReconciliationChangeCategory::ROW_FAILURE));
            assert!(items.iter().all(|item| {
                item.category != ReconciliationChangeCategory::VOTED_OTHER_CHANNEL
                    && item.category != ReconciliationChangeCategory::DISABLED_DELETE_CALL
            }));
        }
    }

    #[test]
    fn a_new_other_channel_voter_is_created_disabled_with_keycloak_casing_and_reason() {
        let items = classify_file_row(
            &row("voter-1", "PAPER", "false"),
            None,
            &datafix_source("0014"),
        );
        assert_eq!(items.len(), 5);
        assert!(items.iter().any(|item| {
            item.target
                == ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    false, false,
                )))
        }));
        assert!(items.iter().any(|item| {
            item.target.sequent_field().is_some_and(|field| {
                field.new_keycloak_attributes().is_some_and(|attributes| {
                    attributes.get(DISABLE_COMMENT)
                        == Some(&DISABLE_REASON_MARKVOTED_CALL.to_string())
                })
            })
        }));
    }

    #[test]
    fn composed_area_name_omits_the_files_none_sentinel() {
        // The file's own "NONE" sentinel for an absent SchoolSupportCode/Poll
        // must not end up as a literal "-NONE-" segment in the composed name.
        let mut file_row = row("voter-1", ATTR_RESET_VALUE, "false");
        assert_eq!(composed_area_name(&file_row), "01-P-000");

        file_row.school_support_code = ATTR_RESET_VALUE.to_string();
        assert_eq!(composed_area_name(&file_row), "01-000");

        file_row.poll = ATTR_RESET_VALUE.to_string();
        assert_eq!(composed_area_name(&file_row), "01");
    }
}
