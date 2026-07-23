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
//! Deliberately source-agnostic: every category but `VOTER_ADDED` reduces to
//! one generic Keycloak edit (`apply_generic_voter_edit`) built entirely from
//! what the `DiffItem`s already carry (`SequentReconciliationField`) — this
//! module never calls into `api_datafix`, so it has no idea the diff came
//! from Datafix, only that it targets Sequent. `VOTER_ADDED` still needs its
//! own path since creating a voter is a different Keycloak call
//! (`create_user`, not `edit_user`) with no existing `user_id` to edit yet.
//!
//! The per-voter cast-vote guards (`ensure_voter_has_no_active_vote`-style
//! checks before a disable/re-enable actually lands) are re-implemented here
//! rather than reused from the inbound Datafix API, since those take a
//! `DatafixClaims` (the inbound request guard) that doesn't exist on this
//! path — this module re-validates the same underlying condition
//! (`has_valid_cast_vote`) directly instead.

use crate::postgres::area::get_event_areas;
use crate::postgres::cast_vote::has_valid_cast_vote;
use crate::services::external::reconciliation::diff::DiffItem;
use crate::services::external::types::ReconciliationChangeCategory;
use crate::services::external::utils::{
    datafix_voter_lock_key, get_user_id, DATAFIX_VOTER_LOCK_SECS,
};
use crate::services::pg_lock::PgLock;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::{User, AREA_ID_ATTR_NAME, TENANT_ID_ATTR_NAME};
use std::collections::HashMap;
use std::env;
use tracing::{error, instrument};
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
        datafix_voter_lock_key(tenant_id, election_event_id, voter_username),
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

/// Picks the apply path for one voter's items. Every category maps onto the
/// same generic Keycloak edit except `VOTER_ADDED` (a new voter — see the
/// module doc) and `VOTED_OTHER_CHANNEL`/`REENABLED`, which each need a
/// business-safety guard re-checked right before writing (a vote may have
/// landed since the diff was computed) on top of that same generic edit.
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
    // Every item for a voter shares the same category (recordToDiffRows-style
    // flattening in diff.rs always emits same-category items per voter for a
    // single reconciliation pass), so the first item's category picks the
    // branch below.
    let Some(category) = items.first().map(|item| item.category) else {
        return Ok(VoterApplyOutcome::Applied); // nothing to do
    };

    match category {
        ReconciliationChangeCategory::VOTED_OTHER_CHANNEL => {
            apply_voted_other_channel(
                hasura_transaction,
                keycloak_transaction,
                tenant_id,
                election_event_id,
                realm,
                voter_username,
                items,
            )
            .await
        }
        ReconciliationChangeCategory::REENABLED => {
            apply_reenable(
                hasura_transaction,
                keycloak_transaction,
                tenant_id,
                election_event_id,
                realm,
                voter_username,
                items,
            )
            .await
        }
        ReconciliationChangeCategory::DISABLED_DELETE_CALL
        | ReconciliationChangeCategory::PROFILE_UPDATE => {
            apply_generic_voter_edit(
                hasura_transaction,
                keycloak_transaction,
                tenant_id,
                election_event_id,
                realm,
                voter_username,
                items,
            )
            .await
        }
        ReconciliationChangeCategory::VOTER_ADDED => {
            apply_voter_added(
                hasura_transaction,
                tenant_id,
                election_event_id,
                realm,
                items,
            )
            .await
        }
        ReconciliationChangeCategory::VOTED_INTERNET
        | ReconciliationChangeCategory::DELETION_REVERTED
        | ReconciliationChangeCategory::ROW_FAILURE => {
            // These categories are always target = Datafix (patch-bound) or
            // excluded from apply entirely — should never reach here for a
            // target = Sequent item. Treated as a no-op rather than a panic,
            // since a category/target mismatch is a diff.rs bug, not
            // something this per-voter apply loop should crash on.
            Ok(VoterApplyOutcome::Applied)
        }
    }
}

#[instrument(skip(hasura_transaction, keycloak_transaction, items), err)]
async fn apply_voted_other_channel(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    voter_username: &str,
    items: &[DiffItem],
) -> Result<VoterApplyOutcome> {
    // The exception from source-of-truth B: re-check the valid-ballot guard
    // right before writing (a vote may have landed since the diff was
    // computed).
    let user_id = get_user_id(keycloak_transaction, realm, voter_username)
        .await
        .map_err(|err| anyhow!("Error resolving voter user id: {err:?}"))?;
    if has_valid_cast_vote(hasura_transaction, tenant_id, election_event_id, &user_id).await? {
        return Ok(VoterApplyOutcome::Failed {
            reason: "Voter now holds a valid Internet ballot; voted-via-other-channel cannot be \
                     applied — resolve manually via edit_user after the freeze ends."
                .to_string(),
        });
    }
    apply_generic_voter_edit(
        hasura_transaction,
        keycloak_transaction,
        tenant_id,
        election_event_id,
        realm,
        voter_username,
        items,
    )
    .await
}

#[instrument(skip(hasura_transaction, keycloak_transaction, items), err)]
async fn apply_reenable(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    voter_username: &str,
    items: &[DiffItem],
) -> Result<VoterApplyOutcome> {
    // Mirrors the inbound /update-voter(enabled=true) guard
    // (`ensure_inbound_reenable_is_safe`), re-implemented against
    // tenant/election_event_id directly since that guard is coupled to the
    // inbound request's `DatafixClaims`.
    let user_id = get_user_id(keycloak_transaction, realm, voter_username)
        .await
        .map_err(|err| anyhow!("Error resolving voter user id: {err:?}"))?;
    if has_valid_cast_vote(hasura_transaction, tenant_id, election_event_id, &user_id).await? {
        return Ok(VoterApplyOutcome::Failed {
            reason: "Voter now holds a valid Internet ballot; cannot re-enable".to_string(),
        });
    }
    apply_generic_voter_edit(
        hasura_transaction,
        keycloak_transaction,
        tenant_id,
        election_event_id,
        realm,
        voter_username,
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
#[instrument(skip(hasura_transaction, keycloak_transaction, items), err)]
async fn apply_generic_voter_edit(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    voter_username: &str,
    items: &[DiffItem],
) -> Result<VoterApplyOutcome> {
    let user_id = get_user_id(keycloak_transaction, realm, voter_username)
        .await
        .map_err(|err| anyhow!("Error resolving voter user id: {err:?}"))?;

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
            &user_id,
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

/// Creates a Sequent voter for a file row Sequent didn't have at all (D,
/// forward direction). Unlike every other category, this can't reduce to
/// `apply_generic_voter_edit` — there is no existing `user_id` to edit, so it
/// needs Keycloak's `create_user` instead — but the attribute/area handling
/// is the same generic machinery.
#[instrument(skip(hasura_transaction, items), err)]
async fn apply_voter_added(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    items: &[DiffItem],
) -> Result<VoterApplyOutcome> {
    let voter_username = items
        .first()
        .map(|item| item.voter_username.clone())
        .unwrap_or_default();

    // The `enabled` transition items may carry is ignored here: a
    // newly-added voter is always created enabled (pre-existing behavior,
    // not something this generalization changes).
    let (_enabled, mut attributes) = keycloak_edit_from_items(items);
    resolve_area_attribute(
        hasura_transaction,
        tenant_id,
        election_event_id,
        items,
        &mut attributes,
    )
    .await?;
    attributes.insert(TENANT_ID_ATTR_NAME.to_string(), vec![tenant_id.to_string()]);

    let voter_group_name = env::var("KEYCLOAK_VOTER_GROUP_NAME")
        .map_err(|err| anyhow!("Error getting env var KEYCLOAK_VOTER_GROUP_NAME: {err:?}"))?;
    let user = User {
        username: Some(voter_username),
        enabled: Some(true),
        ..User::default()
    };
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error getting KeycloakAdminClient: {err:?}"))?;
    let result = client
        .create_user(realm, &user, Some(attributes), Some(vec![voter_group_name]))
        .await;
    Ok(match result {
        Ok(_) => VoterApplyOutcome::Applied,
        Err(err) => VoterApplyOutcome::Failed {
            reason: format!("{err:?}"),
        },
    })
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
    let Some(area_name) = items
        .iter()
        .find_map(|item| item.target.sequent_field()?.new_area_name())
    else {
        return Ok(());
    };

    let areas = get_event_areas(hasura_transaction, tenant_id, election_event_id).await?;
    if let Some(area_id) = areas
        .into_iter()
        .find(|area| area.name.as_deref() == Some(area_name))
        .map(|area| area.id)
    {
        attributes.insert(AREA_ID_ATTR_NAME.to_string(), vec![area_id]);
    }
    Ok(())
}
