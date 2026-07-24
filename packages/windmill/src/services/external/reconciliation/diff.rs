// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::external::datafix_types::{
    DatafixReconciliationField, ParsedDatafixReconciliationRow, FILE_CHANNEL_INTERNET,
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
use tracing::instrument;

/// One (voter, field) change destined for either the Datafix patch or a
/// direct Sequent apply. Never persisted on its own — always as part of a
/// `Vec<DiffItem>` inside one of the two documents `generate_reconciliation_patches`
/// uploads: the full `ReconciliationDiff` envelope (both sides, for review)
/// and the Sequent-only patch document (`target = Sequent` items, what
/// `apply_reconciliation_patch` actually applies). Each is written once and
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
    /// The `target = Sequent`, non-`ROW_FAILURE` items, already serialized as
    /// their own document (see `patch::build_sequent_patch_json`) so apply
    /// doesn't need to re-filter `items` below.
    pub sequent_patch_document_id: String,
    /// Every item, both sides, including `ROW_FAILURE`s — what the review UI
    /// renders.
    pub items: Vec<DiffItem>,
}

/// Runs the full two-pass diff described above against one page of the
/// snapshot stream (see `users::fetch_realm_voter_snapshots_page`).
/// `file_rows_by_username` should hold every parsed file row, indexed by
/// `VoterID`, so each page can be compared in a single lookup pass;
/// `seen_usernames` is threaded through across pages so the caller can run
/// the reverse pass (D, a voter Sequent has that the file doesn't) once the
/// last page comes back empty. `source` carries whatever config the file's
/// own origin needs for classification (for Datafix, its `CountyMun`, from
/// the event's own `VoterviewRequest::county_mun`).
#[instrument(skip_all, fields(page_size = snapshots_page.len()))]
pub fn diff_snapshot_page(
    snapshots_page: &[VoterSnapshot],
    file_rows_by_username: &HashMap<String, ParsedDatafixReconciliationRow>,
    source: &ReconciliationPatchSource,
    seen_usernames: &mut HashSet<String>,
) -> Vec<DiffItem> {
    let mut items = Vec::new();
    for snapshot in snapshots_page {
        let Some(row) = file_rows_by_username.get(&snapshot.username) else {
            // Voter not in the file at all — handled by the reverse pass once
            // the whole snapshot stream (not just this page) has been walked,
            // since "not seen anywhere in the file" can only be known then.
            continue;
        };
        seen_usernames.insert(snapshot.username.clone());
        items.extend(classify_file_row(row, Some(snapshot), source));
    }
    items
}

/// D, reverse direction, run once after the full snapshot stream is
/// exhausted: every file row whose `VoterID` was never matched to a snapshot
/// (source D, forward direction — the file has a voter Sequent doesn't).
#[instrument(skip_all)]
pub fn diff_unmatched_file_rows(
    file_rows_by_username: &HashMap<String, ParsedDatafixReconciliationRow>,
    seen_usernames: &HashSet<String>,
    source: &ReconciliationPatchSource,
) -> Vec<DiffItem> {
    file_rows_by_username
        .iter()
        .filter(|(username, _)| !seen_usernames.contains(*username))
        .flat_map(|(_, row)| classify_file_row(row, None, source))
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
        return voter_added_to_sequent(row);
    };

    if !snapshot.enabled && !snapshot.has_valid_internet_vote {
        // An already-disabled voter is reconciled on the Deleted field
        // alone, keyed off why Sequent disabled them — see
        // classify_disabled_voter below.
        // Voter has not voted. If it had valid vote could not be disabled, because its vote is discarded then.
        return classify_disabled_voter(row, snapshot);
    }

    let mut items = Vec::new();

    // A) Sequent holds a valid Internet ballot; Datafix says NONE.
    if row.channel == ATTR_RESET_VALUE && snapshot.has_valid_internet_vote {
        items.push(diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Datafix(DatafixReconciliationField::Channel(
                row.channel.clone(),
                FILE_CHANNEL_INTERNET.to_string(),
            )),
            ReconciliationChangeCategory::VOTED_INTERNET,
        ));
    }

    // B) Datafix reports another channel Sequent doesn't have recorded.
    if row.channel != ATTR_RESET_VALUE
        && row.channel != FILE_CHANNEL_INTERNET
        && snapshot.voted_channel.as_deref() != Some(row.channel.as_str())
    {
        if snapshot.has_valid_internet_vote {
            // Exception: cannot resolve automatically — row failure now, and
            // Sequent still wins for the rest of the round (spec, example B).
            items.push(DiffItem {
                voter_username: row.voter_id.clone(),
                target: ReconciliationPatchTarget::Sequent(Some(
                    SequentReconciliationField::KeycloakUA(
                        HashMap::from([(
                            VOTED_CHANNEL.to_string(),
                            FILE_CHANNEL_INTERNET.to_string(),
                        )]),
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
        } else {
            // Both the voted channel and why the voter is being disabled are
            // plain Keycloak attributes — carried together so `apply` writes
            // them in a single edit without knowing this came from Datafix.
            items.push(diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::KeycloakUA(
                    HashMap::from([(
                        VOTED_CHANNEL.to_string(),
                        snapshot
                            .voted_channel
                            .clone()
                            .unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
                    )]),
                    HashMap::from([
                        (VOTED_CHANNEL.to_string(), row.channel.clone()),
                        (
                            DISABLE_COMMENT.to_string(),
                            DISABLE_REASON_MARKVOTED_CALL.to_string(),
                        ),
                    ]),
                ))),
                ReconciliationChangeCategory::VOTED_OTHER_CHANNEL,
            ));
            items.push(diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    true, false,
                ))),
                ReconciliationChangeCategory::VOTED_OTHER_CHANNEL,
            ));
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
                    snapshot.dob.clone().unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
                )]),
                HashMap::from([(DATE_OF_BIRTH.to_string(), row.dob.clone())]),
            ))),
            ReconciliationChangeCategory::PROFILE_UPDATE,
        ));
    }

    // C) Deleted=true — Datafix wins, unless the voter has already voted via internet.
    // (snapshot.enabled is guaranteed true here — the disabled case already
    // returned via classify_disabled_voter above.)
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
        } else {
            items.push(diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    true, false,
                ))),
                ReconciliationChangeCategory::DISABLED_DELETE_CALL,
            ));
            items.push(diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::KeycloakUA(
                    HashMap::from([(DISABLE_COMMENT.to_string(), ATTR_RESET_VALUE.to_string())]),
                    HashMap::from([(
                        DISABLE_COMMENT.to_string(),
                        DISABLE_REASON_DELETE_CALL.to_string(),
                    )]),
                ))),
                ReconciliationChangeCategory::DISABLED_DELETE_CALL,
            ));
        }
    }

    items
}

/// D, forward direction: the file has a voter Sequent doesn't. Per "Patch
/// Files Format", every field Sequent actually stores is present with `_old =
/// NONE` since Sequent has no prior record at all — expanded here into one
/// `DiffItem` per `SequentReconciliationField`, all sharing `VOTER_ADDED`/
/// `target = Sequent`. `CountyMun` has no Sequent equivalent, so unlike the
/// `DatafixReconciliationField::NAMES` set used for the outbound patch CSV,
/// this only covers the four fields Sequent itself understands. Note
/// `apply::apply_voter_added` always creates the voter enabled regardless of
/// the `Enabled` item's value here (pre-existing behavior, not this diff's
/// concern) — it's still emitted for display parity with the other fields.
#[instrument(skip_all, fields(voter_username = %row.voter_id))]
fn voter_added_to_sequent(row: &ParsedDatafixReconciliationRow) -> Vec<DiffItem> {
    let file_area_name = composed_area_name(row);
    let file_enabled = row.deleted != "true";
    vec![
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
                HashMap::from([(VOTED_CHANNEL.to_string(), row.channel.clone())]),
            ))),
            ReconciliationChangeCategory::VOTER_ADDED,
        ),
        diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                true,
                file_enabled,
            ))),
            ReconciliationChangeCategory::VOTER_ADDED,
        ),
    ]
}

/// D, reverse direction: Sequent has an enabled voter the file doesn't
/// mention at all — reported into the Datafix patch so their side adds it.
#[instrument(skip_all)]
fn voter_missing_from_file(username: &str, snapshot: &VoterSnapshot) -> Vec<DiffItem> {
    let fields = [
        DatafixReconciliationField::Ward(
            ATTR_RESET_VALUE.to_string(),
            snapshot
                .area_name
                .clone()
                .unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
        ),
        DatafixReconciliationField::DoB(
            ATTR_RESET_VALUE.to_string(),
            snapshot.dob.clone().unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
        ),
        DatafixReconciliationField::Channel(
            ATTR_RESET_VALUE.to_string(),
            snapshot
                .voted_channel
                .clone()
                .unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
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

/// Reconciles the `Deleted` field for a voter Sequent already has disabled,
/// based on *why* — read off `disable_comment`
/// (`sequent_core::types::keycloak::DISABLE_COMMENT`) — rather than assuming
/// every disabled voter should read as `Deleted=true`.
#[instrument(skip_all, fields(voter_username = %row.voter_id))]
fn classify_disabled_voter(
    row: &ParsedDatafixReconciliationRow,
    snapshot: &VoterSnapshot,
) -> Vec<DiffItem> {
    let thereis_deleted_call_comment = match snapshot.disable_comment.as_deref() {
        Some(DISABLE_REASON_DELETE_CALL) => true,
        // DISABLE_REASON_MARKVOTED_CALL, was disabled via update-voter api, or anything else (admin-disabled,
        // typically the SetNotVoted release flow)
        _ => false,
    };
    let is_file_deleted = row.deleted == "true";

    if is_file_deleted == thereis_deleted_call_comment {
        return vec![]; // already converged, nothing to reconcile
    } else if !is_file_deleted && thereis_deleted_call_comment {
        // File says false, Sequent has them disabled purely from a Datafix
        // delete call with no vote involved — Datafix no longer considers
        // them deleted, so re-enable to follow (guarded the same way an
        // inbound re-enable is — reuse ensure_inbound_reenable_is_safe at
        // apply time, see reconciliation::apply).
        vec![diff_item(
            &row.voter_id,
            ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                false, true,
            ))),
            ReconciliationChangeCategory::REENABLED,
        )]
    } else {
        // The other case: Sequent has them disabled for a reason other than
        // a Datafix delete call but the file says Deleted=true (should be a DISABLE_REASON_DELETE_CALL).
        // The options are: DISABLE_REASON_MARKVOTED_CALL, a DISABLE_REASON_DELETE_CALL that got overriden or lost, or any other admin-set disable reason (typically the SetNotVoted release flow).
        // In either case leave it disabled because it already converges (Deleted=true means disabled (soft delete) in Sequent, so the voter is still present in the system and can be re-enabled later if needed).
        // This case Datafix wins, although the voter is already disabled in Sequent, but we do it add the DISABLE_REASON_DELETE_CALL.
        vec![
            diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    false, false,
                ))),
                ReconciliationChangeCategory::DISABLED_DELETE_CALL,
            ),
            diff_item(
                &row.voter_id,
                ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::KeycloakUA(
                    HashMap::from([(
                        DISABLE_COMMENT.to_string(),
                        snapshot
                            .disable_comment
                            .clone()
                            .unwrap_or_else(|| ATTR_RESET_VALUE.to_string()),
                    )]),
                    HashMap::from([(
                        DISABLE_COMMENT.to_string(),
                        DISABLE_REASON_DELETE_CALL.to_string(),
                    )]),
                ))),
                ReconciliationChangeCategory::DISABLED_DELETE_CALL,
            ),
        ]
    }
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

    compose_area_name(&VoterInformationBody {
        voter_id: row.voter_id.clone(),
        ward: row.ward.clone(),
        schoolboard: optional_field(&row.school_support_code),
        poll: optional_field(&row.poll),
        birthdate: None,
        enabled: None,
    })
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
            enabled: true,
            area_name: Some("01-P-000".to_string()),
            dob: Some("1990-01-01".to_string()),
            voted_channel: None,
            has_valid_internet_vote: false,
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
                    true, true,
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
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::DISABLED_DELETE_CALL));
        assert!(items.iter().any(|item| {
            item.target
                == ReconciliationPatchTarget::Sequent(Some(SequentReconciliationField::Enabled(
                    false, false,
                )))
        }));
        let attributes_item = items
            .iter()
            .find_map(|item| item.target.sequent_field()?.new_keycloak_attributes())
            .expect("one item carries the Keycloak attributes");
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
        assert_eq!(items.len(), 2);
        assert!(items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::DISABLED_DELETE_CALL));
    }

    #[test]
    fn voter_missing_from_file_is_reported_externally() {
        let items = voter_missing_from_file("voter-1", &enabled_snapshot());
        assert_eq!(items.len(), 4);
        assert!(items
            .iter()
            .all(|item| item.category == ReconciliationChangeCategory::VOTER_ADDED));
        assert!(items.iter().all(|item| item.target.is_datafix()));
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
