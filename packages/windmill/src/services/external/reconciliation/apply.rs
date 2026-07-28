// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Applies the `target = Sequent` side of a reconciliation diff. Per-voter
//! atomic (spec, "Implementation Requirements": "a row failure does not
//! abort the process") — one voter's failure marks that voter's items
//! FAILED and moves on, exactly like `tally_sheet_import`'s
//! `find_stale_baseline_conflicts` gates a single item without aborting the
//! whole import.
//!
//! Deliberately source-agnostic: every existing-voter category reduces to
//! one generic Keycloak edit (`apply_generic_voter_edit`) built entirely from
//! what the `DiffItem`s already carry (`SequentReconciliationField`) — this
//! module never calls into `api_datafix`, so it has no idea the diff came
//! from Datafix, only that it targets Sequent. `VOTER_ADDED` is handled only
//! by `bulk_create`, so there is one authoritative creation path.
//!
//! The per-voter cast-vote guards (`ensure_voter_has_no_active_vote`-style
//! checks before a disable/re-enable actually lands) are re-implemented here
//! rather than reused from the inbound Datafix API, since those take a
//! `DatafixClaims` (the inbound request guard) that doesn't exist on this
//! path — this module re-validates the same underlying condition
//! (`VoterCastVoteState`, including unresolved and valid votes) directly.

use crate::postgres::area::{get_area_by_id, get_area_id_from_event_by_name};
use crate::postgres::cast_vote::get_voter_cast_vote_state;
use crate::services::external::reconciliation::diff::DiffItem;
use crate::services::external::types::{ReconciliationChangeCategory, SequentReconciliationField};
use crate::services::external::utils::{
    external_voter_lock_key, get_user_id, voted_via_internet, voted_via_not_internet_channel,
    DATAFIX_VOTER_LOCK_SECS,
};
use crate::services::pg_lock::PgLock;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::keycloak::{User, AREA_ID_ATTR_NAME, ATTR_RESET_VALUE, VOTED_CHANNEL};
use std::collections::{HashMap, HashSet};
use tracing::{error, info, instrument};
use uuid::Uuid;

/// Outcome of applying one voter's queued changes.
#[derive(Debug)]
pub enum VoterApplyOutcome {
    Applied,
    Failed { reason: String },
}

/// Applies every `target = Sequent` item for one voter (`items` — all sharing
/// `voter_username`, already filtered to exclude `ROW_FAILURE`) under the
/// same per-voter Datafix advisory lock the inbound API and `edit_user` use,
/// so this can never interleave with an inbound Datafix call or an outbound
/// `SetVoted`/`SetNotVoted` for the same voter.
#[instrument(skip(hasura_transaction, keycloak_transaction, items), fields(voter_username = %voter_username), err)]
pub async fn apply_voter_changes(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    voter_username: &str,
    items: &[DiffItem],
) -> Result<VoterApplyOutcome> {
    let lock = PgLock::acquire(
        external_voter_lock_key(tenant_id, election_event_id, voter_username),
        Uuid::new_v4().to_string(),
        ISO8601::now() + chrono::Duration::seconds(DATAFIX_VOTER_LOCK_SECS),
    )
    .await?;

    let result = apply_voter_changes_locked(
        hasura_transaction,
        keycloak_transaction,
        tenant_id,
        election_event_id,
        realm,
        voter_username,
        items,
    )
    .await;

    if let Err(err) = lock.release().await {
        error!("Error releasing the Datafix voter lock during reconciliation apply: {err}");
    }

    result
}

/// Validates every item's old snapshot and derives safety guards from the
/// complete category set before writing. A voter can legitimately have
/// mixed categories (for example profile update plus deletion), so dispatch
/// must never depend on item ordering.
#[instrument(skip(hasura_transaction, keycloak_transaction, items), err)]
async fn apply_voter_changes_locked(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    voter_username: &str,
    items: &[DiffItem],
) -> Result<VoterApplyOutcome> {
    if items.is_empty() {
        return Ok(VoterApplyOutcome::Applied); // nothing to do
    }

    let categories: HashSet<_> = items.iter().map(|item| item.category).collect();
    if categories.contains(&ReconciliationChangeCategory::VOTER_ADDED) {
        return Ok(VoterApplyOutcome::Failed {
            reason: "VOTER_ADDED must be handled by the bulk-create path".to_string(),
        });
    }
    if categories.iter().any(|category| {
        matches!(
            category,
            ReconciliationChangeCategory::VOTED_INTERNET
                | ReconciliationChangeCategory::DELETION_REVERTED
                | ReconciliationChangeCategory::ROW_FAILURE
        )
    }) {
        return Ok(VoterApplyOutcome::Failed {
            reason: "Datafix-side or row-failure item reached Sequent apply".to_string(),
        });
    }

    let user_id = get_user_id(keycloak_transaction, realm, voter_username)
        .await
        .map_err(|err| anyhow!("Error resolving voter user id: {err:?}"))?;
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error getting KeycloakAdminClient: {err:?}"))?;
    let current_user = client
        .get_user(realm, &user_id)
        .await
        .map_err(|err| anyhow!("Error loading current voter snapshot: {err:?}"))?;

    if let Some(reason) =
        validate_old_values(hasura_transaction, tenant_id, &current_user, items).await?
    {
        return Ok(VoterApplyOutcome::Failed { reason });
    }

    let needs_no_active_vote = categories.iter().any(|category| {
        matches!(
            category,
            ReconciliationChangeCategory::VOTED_OTHER_CHANNEL
                | ReconciliationChangeCategory::VOTED_UNMARKED
                | ReconciliationChangeCategory::DISABLED_DELETE_CALL
                | ReconciliationChangeCategory::REENABLED
        )
    });
    if needs_no_active_vote {
        let state = get_voter_cast_vote_state(
            hasura_transaction,
            &parse_uuid_v4(tenant_id)?,
            &parse_uuid_v4(election_event_id)?,
            &user_id,
        )
        .await?;
        if state.has_unresolved_vote || state.has_valid_vote {
            return Ok(VoterApplyOutcome::Failed {
                reason: "Voter now has an active Internet ballot; the reconciliation change is stale and was not applied"
                    .to_string(),
            });
        }
    }

    if categories.contains(&ReconciliationChangeCategory::REENABLED) {
        let attributes = current_user.attributes.clone().unwrap_or_default();
        if voted_via_internet(&attributes) || voted_via_not_internet_channel(&attributes) {
            return Ok(VoterApplyOutcome::Failed {
                reason: "Voter still has a voted-channel attribute and cannot be re-enabled"
                    .to_string(),
            });
        }
    }

    apply_generic_voter_edit(
        hasura_transaction,
        tenant_id,
        election_event_id,
        realm,
        &user_id,
        items,
    )
    .await
}

/// The one generic Keycloak edit every category above (except `VOTER_ADDED`)
/// reduces to: resolve the existing voter's `user_id`, collect whatever
/// `enabled` transition and Keycloak attributes `items` carry, and write them
/// in a single `edit_user` call. Has no notion of *why* — that judgment
/// belongs entirely to whichever origin produced the diff (see
/// `SequentReconciliationField` in `services::external::types`).
#[instrument(skip(hasura_transaction, items), err)]
async fn apply_generic_voter_edit(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    user_id: &str,
    items: &[DiffItem],
) -> Result<VoterApplyOutcome> {
    let (enabled, mut attributes) = keycloak_edit_from_items(items);
    resolve_area_attribute(
        hasura_transaction,
        tenant_id,
        election_event_id,
        items,
        &mut attributes,
    )
    .await?;

    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error getting KeycloakAdminClient: {err:?}"))?;
    let result = client
        .edit_user(
            realm,
            user_id,
            enabled,
            Some(attributes),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await;
    Ok(match result {
        Ok(_) => VoterApplyOutcome::Applied,
        Err(err) => VoterApplyOutcome::Failed {
            reason: format!("{err:?}"),
        },
    })
}

/// Compare-and-set validation for the snapshot captured during generation.
/// A mismatch is a row failure, not a task failure: another voter can still
/// be applied safely.
async fn validate_old_values(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    current_user: &User,
    items: &[DiffItem],
) -> Result<Option<String>> {
    let attributes = current_user
        .attributes
        .as_ref()
        .cloned()
        .unwrap_or_default();
    let current_attribute = |key: &str| {
        attributes
            .get(key)
            .and_then(|values| values.last())
            .map(String::as_str)
            .unwrap_or(ATTR_RESET_VALUE)
    };

    for item in items {
        let Some(field) = item.target.sequent_field() else {
            continue;
        };
        match field {
            SequentReconciliationField::Enabled(expected, _) => {
                let actual = current_user.enabled.unwrap_or(false);
                if actual != *expected {
                    return Ok(Some(format!(
                        "Stale snapshot for enabled: expected {expected}, found {actual}"
                    )));
                }
            }
            SequentReconciliationField::KeycloakUA(expected, _) => {
                for (key, expected_value) in expected {
                    let actual = current_attribute(key);
                    let matches = if key == VOTED_CHANNEL {
                        actual.eq_ignore_ascii_case(expected_value)
                    } else {
                        actual == expected_value
                    };
                    if !matches {
                        return Ok(Some(format!(
                            "Stale snapshot for attribute '{key}': expected '{expected_value}', found '{actual}'"
                        )));
                    }
                }
            }
            SequentReconciliationField::AreaName(expected, _) => {
                let current_area_id = current_attribute(AREA_ID_ATTR_NAME);
                let actual = if current_area_id == ATTR_RESET_VALUE {
                    ATTR_RESET_VALUE.to_string()
                } else {
                    get_area_by_id(hasura_transaction, tenant_id, current_area_id)
                        .await?
                        .and_then(|area| area.name)
                        .unwrap_or_else(|| ATTR_RESET_VALUE.to_string())
                };
                if actual != *expected {
                    return Ok(Some(format!(
                        "Stale snapshot for area: expected '{expected}', found '{actual}'"
                    )));
                }
            }
        }
    }
    Ok(None)
}

/// Merges every `KeycloakUA` attribute across `items` into one map, keyed
/// exactly as Keycloak expects, and derives the `enabled` transition from any
/// `Enabled` item's new value. Purely mechanical: diff.rs already decided
/// every value, this just collects them.
fn keycloak_edit_from_items(items: &[DiffItem]) -> (Option<bool>, HashMap<String, Vec<String>>) {
    let mut enabled = None;
    let mut attributes: HashMap<String, Vec<String>> = HashMap::new();
    for item in items {
        let Some(field) = item.target.sequent_field() else {
            continue;
        };
        if let Some(new_enabled) = field.new_enabled() {
            enabled = Some(new_enabled);
        }
        if let Some(keycloak_attributes) = field.new_keycloak_attributes() {
            for (key, value) in keycloak_attributes {
                attributes.insert(key.clone(), vec![value.clone()]);
            }
        }
    }
    (enabled, attributes)
}

/// Resolves any `AreaName` item's composed name to a Sequent `area-id` and
/// merges it into `attributes` — a plain Sequent-domain lookup (an `Area` by
/// its already-composed name), not anything Datafix-specific: the field the
/// file used to compose that name (Ward/Poll/SchoolSupportCode) is Datafix's
/// concern alone, not this apply path's.
#[instrument(skip(hasura_transaction, items))]
async fn resolve_area_attribute(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    items: &[DiffItem],
    attributes: &mut HashMap<String, Vec<String>>,
) -> Result<()> {
    info!("Resolving area attribute for voter changes: {items:?}");
    let Some(area_name) = items
        .iter()
        .find_map(|item| item.target.sequent_field()?.new_area_name())
    else {
        return Ok(());
    };
    info!("Resolving area name {area_name:?} to area-id for voter changes");
    let area_id =
        get_area_id_from_event_by_name(hasura_transaction, tenant_id, election_event_id, area_name)
            .await?;
    attributes.insert(AREA_ID_ATTR_NAME.to_string(), vec![area_id]);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::external::types::ReconciliationPatchTarget;
    use sequent_core::types::keycloak::{DATE_OF_BIRTH, DISABLE_COMMENT};

    fn item(field: SequentReconciliationField) -> DiffItem {
        DiffItem {
            voter_username: "voter-1".to_string(),
            target: ReconciliationPatchTarget::Sequent(Some(field)),
            category: ReconciliationChangeCategory::PROFILE_UPDATE,
            failure_reason: None,
        }
    }

    #[test]
    fn merges_mixed_category_fields_into_one_keycloak_edit() {
        let items = vec![
            item(SequentReconciliationField::Enabled(true, false)),
            item(SequentReconciliationField::KeycloakUA(
                HashMap::new(),
                HashMap::from([(DATE_OF_BIRTH.to_string(), "1990-01-01".to_string())]),
            )),
            item(SequentReconciliationField::KeycloakUA(
                HashMap::new(),
                HashMap::from([(
                    DISABLE_COMMENT.to_string(),
                    "Disabled by reconciliation".to_string(),
                )]),
            )),
        ];

        let (enabled, attributes) = keycloak_edit_from_items(&items);
        assert_eq!(enabled, Some(false));
        assert_eq!(
            attributes.get(DATE_OF_BIRTH),
            Some(&vec!["1990-01-01".to_string()])
        );
        assert_eq!(
            attributes.get(DISABLE_COMMENT),
            Some(&vec!["Disabled by reconciliation".to_string()])
        );
    }
}
