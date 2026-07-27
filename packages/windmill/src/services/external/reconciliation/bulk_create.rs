// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Bulk-creates `VOTER_ADDED` voters via direct writes to Keycloak's own
//! Postgres tables (`user_entity`/`user_attribute`/`user_group_membership`),
//! bypassing the Admin REST API entirely — modeled on the existing
//! `services::import::import_users::import_users_file` bulk-import path.
//! A reconciliation round against a realm that's badly out of sync with a
//! 100k+-row file is dominated by `VOTER_ADDED` items; going through
//! Keycloak's REST API one voter at a time at that volume is impractical
//! (confirmed empirically — even after fixing a Keycloak-side connection
//! leak, per-voter `create_user` calls remained far too slow to finish in
//! practice), while this writes many voters per statement.
//!
//! Every other reconciliation category (edits to existing voters) still
//! goes through the Admin API in `apply.rs`, unchanged — this file only
//! ever creates brand-new users, never touches an existing one.
//!
//! Trade-off, worth being explicit about: this bypasses whatever validation
//! Keycloak's own API enforces beyond what we replicate here (the username
//! length rule we've directly observed Keycloak reject — 3 to 40
//! characters) and Keycloak's event system entirely, so nothing that
//! listens for Keycloak admin events fires for these voters.

use crate::postgres::keycloak_realm::get_realm_id;
use crate::services::external::reconciliation::diff::DiffItem;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::{
    AREA_ID_ATTR_NAME, DATE_OF_BIRTH, TENANT_ID_ATTR_NAME, VOTED_CHANNEL,
};
use std::collections::{HashMap, HashSet};
use tracing::{instrument, warn};

/// Voters are inserted this many at a time — bounds the blast radius of a
/// single bad batch (a constraint violation the database itself can't route
/// around, unlike `ON CONFLICT DO NOTHING` for duplicate usernames) to one
/// batch instead of the whole run, while still writing many voters per
/// round trip instead of one.
const BULK_INSERT_BATCH_SIZE: usize = 2_000;
const KEYCLOAK_USERNAME_MIN_LEN: usize = 3;
const KEYCLOAK_USERNAME_MAX_LEN: usize = 40;

/// The result of looking an area name up among an election event's areas —
/// area names are expected to be unique, but aren't enforced as such at the
/// database level, so a lookup must be able to report "more than one
/// matched" as distinct from "none matched" rather than silently picking a
/// winner.
enum AreaLookup {
    Found(String),
    Ambiguous,
}

/// One `VOTER_ADDED` voter's fields extracted from its `DiffItem`s, area
/// name not yet resolved to an id (batched separately across every pending
/// voter, see `apply_voters_added_bulk`).
struct PendingVoterAdd {
    voter_username: String,
    enabled: bool,
    date_of_birth: Option<String>,
    voted_channel: Option<String>,
    area_name: Option<String>,
}

/// Mirrors `apply::keycloak_edit_from_items`' extraction for the one shape
/// `VOTER_ADDED` items always come in (`AreaName`, two `KeycloakUA` items
/// for `dateOfBirth`/`voted-channel`, `Enabled`) — see `diff::voter_added_to_sequent`.
fn extract_pending_voter_add(voter_username: &str, items: &[DiffItem]) -> PendingVoterAdd {
    let mut enabled = true;
    let mut date_of_birth = None;
    let mut voted_channel = None;
    let mut area_name = None;

    for item in items {
        let Some(field) = item.target.sequent_field() else {
            continue;
        };
        if let Some(new_enabled) = field.new_enabled() {
            enabled = new_enabled;
        }
        if let Some(name) = field.new_area_name() {
            area_name = Some(name.to_string());
        }
        if let Some(attributes) = field.new_keycloak_attributes() {
            if let Some(dob) = attributes.get(DATE_OF_BIRTH) {
                date_of_birth = Some(dob.clone());
            }
            if let Some(channel) = attributes.get(VOTED_CHANNEL) {
                voted_channel = Some(channel.clone());
            }
        }
    }

    PendingVoterAdd {
        voter_username: voter_username.to_string(),
        enabled,
        date_of_birth,
        voted_channel,
        area_name,
    }
}

/// Resolves every distinct area name in one query instead of one per voter —
/// mirrors `postgres::area::get_area_id_from_event_by_name`'s own
/// found/not-found/ambiguous cases (a duplicate area name is a data problem
/// in the election event, not something to silently pick a winner for).
#[instrument(skip(hasura_transaction, area_names), err)]
async fn resolve_area_ids_bulk(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_names: &[String],
) -> Result<HashMap<String, AreaLookup>> {
    if area_names.is_empty() {
        return Ok(HashMap::new());
    }

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    name,
                    array_agg(id) AS ids
                FROM
                    sequent_backend.area
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    name = ANY($3)
                GROUP BY name
            "#,
        )
        .await
        .context("Error preparing bulk area lookup")?;

    let tenant_uuid = sequent_core::services::uuid_validation::parse_uuid_v4(tenant_id)?;
    let event_uuid = sequent_core::services::uuid_validation::parse_uuid_v4(election_event_id)?;
    let rows = hasura_transaction
        .query(&statement, &[&tenant_uuid, &event_uuid, &area_names])
        .await
        .context("Error running bulk area lookup")?;

    let mut result = HashMap::new();
    for row in rows {
        let name: String = row.try_get("name")?;
        let ids: Vec<uuid::Uuid> = row.try_get("ids")?;
        let lookup = match ids.len() {
            1 => AreaLookup::Found(ids[0].to_string()),
            _ => AreaLookup::Ambiguous,
        };
        result.insert(name, lookup);
    }
    Ok(result)
}

/// Bulk-inserts every `VOTER_ADDED` voter in `voters` directly into
/// Keycloak's tables. Returns the same `(applied_items, row_failures)` shape
/// `run_apply_reconciliation_patch` already collects from the sequential
/// Admin-API path, so the two paths merge into one report.
#[instrument(skip_all, fields(voter_count = voters.len()), err)]
pub async fn apply_voters_added_bulk(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    realm: &str,
    voter_group_name: &str,
    voters: &HashMap<String, Vec<DiffItem>>,
) -> Result<(Vec<DiffItem>, Vec<(String, String)>)> {
    let mut applied_items = Vec::new();
    let mut row_failures: Vec<(String, String)> = Vec::new();

    if voters.is_empty() {
        return Ok((applied_items, row_failures));
    }

    let mut pending = Vec::with_capacity(voters.len());
    for (voter_username, items) in voters {
        let candidate = extract_pending_voter_add(voter_username, items);
        if candidate.voter_username.len() < KEYCLOAK_USERNAME_MIN_LEN
            || candidate.voter_username.len() > KEYCLOAK_USERNAME_MAX_LEN
        {
            row_failures.push((
                voter_username.clone(),
                format!(
                    "VoterID length {} is outside Keycloak's allowed username length ({}-{})",
                    candidate.voter_username.len(),
                    KEYCLOAK_USERNAME_MIN_LEN,
                    KEYCLOAK_USERNAME_MAX_LEN
                ),
            ));
            continue;
        }
        pending.push(candidate);
    }

    let area_names: Vec<String> = pending
        .iter()
        .filter_map(|candidate| candidate.area_name.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let areas_by_name = resolve_area_ids_bulk(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &area_names,
    )
    .await?;

    let mut ready: Vec<(PendingVoterAdd, Option<String>)> = Vec::with_capacity(pending.len());
    for candidate in pending {
        let area_id = match &candidate.area_name {
            None => None,
            Some(name) => match areas_by_name.get(name) {
                Some(AreaLookup::Found(id)) => Some(id.clone()),
                Some(AreaLookup::Ambiguous) => {
                    row_failures.push((
                        candidate.voter_username.clone(),
                        format!(
                            "Found multiple areas with name '{name}' in election event '{election_event_id}'"
                        ),
                    ));
                    continue;
                }
                None => {
                    row_failures.push((
                        candidate.voter_username.clone(),
                        format!(
                            "No area found with name '{name}' in election event '{election_event_id}'"
                        ),
                    ));
                    continue;
                }
            },
        };
        ready.push((candidate, area_id));
    }

    if ready.is_empty() {
        return Ok((applied_items, row_failures));
    }

    let realm_id = get_realm_id(keycloak_transaction, realm.to_string())
        .await
        .context("Error resolving realm id for bulk voter creation")?;
    let group_id = get_group_id(keycloak_transaction, &realm_id, voter_group_name).await?;

    for batch in ready.chunks(BULK_INSERT_BATCH_SIZE) {
        match insert_voter_batch(keycloak_transaction, tenant_id, &realm_id, &group_id, batch).await
        {
            Ok(inserted_usernames) => {
                for (candidate, _area_id) in batch {
                    if inserted_usernames.contains(&candidate.voter_username) {
                        if let Some(items) = voters.get(&candidate.voter_username) {
                            applied_items.extend(items.clone());
                        }
                    } else {
                        row_failures.push((
                            candidate.voter_username.clone(),
                            "Voter already existed in Keycloak (skipped by ON CONFLICT)"
                                .to_string(),
                        ));
                    }
                }
            }
            Err(err) => {
                warn!("Bulk voter-add batch of {} failed: {err:?}", batch.len());
                for (candidate, _area_id) in batch {
                    row_failures.push((
                        candidate.voter_username.clone(),
                        format!("Bulk insert batch failed: {err:?}"),
                    ));
                }
            }
        }
    }

    Ok((applied_items, row_failures))
}

#[instrument(skip(keycloak_transaction), err)]
async fn get_group_id(
    keycloak_transaction: &Transaction<'_>,
    realm_id: &str,
    group_name: &str,
) -> Result<String> {
    let statement = keycloak_transaction
        .prepare(
            r#"
                SELECT id FROM keycloak_group WHERE realm_id = $1 AND name = $2
            "#,
        )
        .await
        .context("Error preparing keycloak_group lookup")?;
    let row = keycloak_transaction
        .query_opt(&statement, &[&realm_id, &group_name])
        .await
        .context("Error running keycloak_group lookup")?
        .ok_or_else(|| anyhow!("Keycloak group '{group_name}' not found in realm '{realm_id}'"))?;
    row.try_get("id").context("Error reading group id")
}

/// Inserts one batch of ready-to-write voters in three statements (users,
/// attributes, group membership) instead of one round trip per voter.
/// Duplicate usernames are skipped via `ON CONFLICT DO NOTHING` rather than
/// aborting the batch — the caller reports any skipped username as a row
/// failure. Returns the usernames that were actually inserted.
#[instrument(skip(keycloak_transaction, batch), fields(batch_size = batch.len()), err)]
async fn insert_voter_batch(
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    realm_id: &str,
    group_id: &str,
    batch: &[(PendingVoterAdd, Option<String>)],
) -> Result<HashSet<String>> {
    let usernames: Vec<String> = batch
        .iter()
        .map(|(candidate, _)| candidate.voter_username.clone())
        .collect();
    let enabled_flags: Vec<bool> = batch
        .iter()
        .map(|(candidate, _)| candidate.enabled)
        .collect();

    let mut attr_usernames: Vec<String> = Vec::new();
    let mut attr_names: Vec<String> = Vec::new();
    let mut attr_values: Vec<String> = Vec::new();
    for (candidate, area_id) in batch {
        let mut push_attr = |name: &str, value: String| {
            attr_usernames.push(candidate.voter_username.clone());
            attr_names.push(name.to_string());
            attr_values.push(value);
        };
        push_attr(TENANT_ID_ATTR_NAME, tenant_id.to_string());
        if let Some(dob) = &candidate.date_of_birth {
            push_attr(DATE_OF_BIRTH, dob.clone());
        }
        if let Some(channel) = &candidate.voted_channel {
            push_attr(VOTED_CHANNEL, channel.clone());
        }
        if let Some(area_id) = area_id {
            push_attr(AREA_ID_ATTR_NAME, area_id.clone());
        }
    }

    let statement = keycloak_transaction
        .prepare(
            r#"
                WITH input_users AS (
                    SELECT * FROM UNNEST($2::text[], $3::boolean[]) AS u(username, enabled)
                ),
                new_users AS (
                    INSERT INTO user_entity (
                        id, realm_id, username, enabled, email_verified,
                        created_timestamp, not_before
                    )
                    SELECT
                        gen_random_uuid()::text, $1, iu.username, iu.enabled, true,
                        (extract(epoch from now()) * 1000)::bigint, 0
                    FROM input_users iu
                    ON CONFLICT (realm_id, username) DO NOTHING
                    RETURNING id, username
                ),
                input_attrs AS (
                    SELECT * FROM UNNEST($4::text[], $5::text[], $6::text[]) AS a(username, name, value)
                ),
                attrs_inserted AS (
                    INSERT INTO user_attribute (id, user_id, name, value)
                    SELECT gen_random_uuid()::text, nu.id, ia.name, ia.value
                    FROM input_attrs ia
                    JOIN new_users nu ON nu.username = ia.username
                    RETURNING 1
                ),
                groups_inserted AS (
                    INSERT INTO user_group_membership (group_id, user_id, membership_type)
                    SELECT $7, nu.id, 'UNMANAGED'
                    FROM new_users nu
                    RETURNING 1
                )
                SELECT username FROM new_users
            "#,
        )
        .await
        .context("Error preparing bulk voter insert")?;

    let rows = keycloak_transaction
        .query(
            &statement,
            &[
                &realm_id,
                &usernames,
                &enabled_flags,
                &attr_usernames,
                &attr_names,
                &attr_values,
                &group_id,
            ],
        )
        .await
        .context("Error executing bulk voter insert")?;

    rows.into_iter()
        .map(|row| {
            row.try_get::<_, String>("username")
                .map_err(anyhow::Error::from)
        })
        .collect()
}
