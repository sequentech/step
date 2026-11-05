// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area::get_areas;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::ballot_box_reads::get_event_ballot_box;
use crate::services::cast_votes::get_users_with_vote_info_in_elections;
use crate::services::database::PgConfig;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box_status::VoterBallotState;
use electoral_log::adapters::postgres::PostgresStore;
use futures::TryStreamExt;
use keycloak::types::GroupRepresentation;
use keycloak::KeycloakError;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient, PubKeycloakAdmin};
use sequent_core::types::keycloak::*;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::{
    collections::{HashMap, HashSet},
    convert::From,
};
use strum_macros::{Display, EnumString};

use crate::services::sql_utils::{escape_sql_identifier, escape_sql_literal};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use tokio::fs::File;
use tokio::io::{copy, AsyncWriteExt, BufWriter};
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tokio_util::io::StreamReader;
use tracing::error;
use tracing::{debug, info, instrument};
use uuid::Uuid;

pub const VALIDATE_ID_ATTR_NAME: &str = "sequent.read-only.id-card-number-validated";
pub const DELEGATE_TO_ATTR_NAME: &str = "delegate-vote-to";
pub const VALIDATE_ID_REGISTERED_VOTER: &str = "VERIFIED";
/// Transaction-local table holding the voters with a valid ballot in the
/// election and area being tallied.
const VOTERS_WITH_VALID_BALLOT_TABLE: &str = "tally_voters_with_valid_ballot";

/// A voter's current Sequent-side state, as far as reconciliation cares.
#[derive(Debug, Clone)]
pub struct VoterSnapshot {
    pub username: String,
    /// Keycloak's own internal user id (`user_entity.id`) — same value the
    /// ballot box's `voter_id` carries (as text there), so this is what
    /// actually matches against the event-wide cast-vote state map;
    /// `username` is a separate, mutable-in-theory identifier not safe to key
    /// that lookup on.
    pub voter_id: Uuid,
    pub enabled: bool,
    /// The voter's resolved area name (`WARD-SCHOOLBOARD-POLL`, uppercased —
    /// see `reconciliation::snapshot`), `None` if their `area-id` attribute
    /// doesn't resolve to a known area (or is unset).
    pub area_name: Option<String>,
    pub dob: Option<String>,
    /// Raw `voted-channel` attribute value; `None` means no channel is
    /// recorded, which is not proof the voter has not voted (see
    /// `has_valid_internet_vote`). File-side comparisons normalize this value
    /// explicitly at the boundary.
    pub voted_channel: Option<String>,
    pub has_valid_internet_vote: bool,
    pub has_unresolved_internet_vote: bool,
    /// Raw `disable-comment` attribute value (`sequent_core::types::keycloak::DISABLE_COMMENT`),
    /// used by `diff.rs` to classify an already-disabled voter — see the
    /// classification rules in `reconciliation::diff::classify_file_row`.
    pub disable_comment: Option<String>,
}

const VOTER_SNAPSHOT_PAGE_SIZE: i64 = 5_000;

/// One keyset-paginated page of the realm's voter-group users, ordered by
/// username, with every attribute. Restricting the query to the configured
/// voter group prevents service accounts and administrators in the same
/// realm from being emitted as Sequent-only voters. `after_username` is the
/// last username of the previous page (`None` for the first page); an empty
/// result means the scan is done.
///
/// Kept as a direct query against Keycloak's own Postgres rather than the
/// Admin REST API (`GET /admin/realms/{realm}/users`, see
/// `sequent_core::services::keycloak::admin_client`): that endpoint only
/// offers offset-based (`first`/`max`) pagination, which is not
/// concurrency-safe — a user created or deleted elsewhere in the realm while
/// a scan is in progress shifts every later offset, silently skipping or
/// duplicating voters. The keyset scan here (`username > $2`) doesn't have
/// that failure mode, which reconciliation (needing to see every voter
/// exactly once) depends on.
#[instrument(skip(keycloak_transaction, areas_by_id), err)]
pub async fn fetch_realm_voter_snapshots_page(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    voter_group_name: &str,
    after_username: Option<&str>,
    areas_by_id: &HashMap<String, String>,
) -> Result<Vec<VoterSnapshot>> {
    let statement = keycloak_transaction
        .prepare(
            r#"
                SELECT
                    u.id AS voter_id,
                    u.username,
                    u.enabled,
                    json_object_agg(ua.name, ua.value) FILTER (WHERE ua.name IS NOT NULL) AS attributes
                FROM user_entity u
                INNER JOIN realm AS ra ON ra.id = u.realm_id
                INNER JOIN user_group_membership ugm ON ugm.user_id = u.id
                INNER JOIN keycloak_group kg ON kg.id = ugm.group_id AND kg.realm_id = u.realm_id
                LEFT JOIN user_attribute ua ON ua.user_id = u.id
                WHERE ra.name = $1
                    AND kg.name = $2
                    AND u.username > $3
                GROUP BY u.id, u.username, u.enabled
                ORDER BY u.username
                LIMIT $4
            "#,
        )
        .await?;

    let rows: Vec<Row> = keycloak_transaction
        .query(
            &statement,
            &[
                &realm,
                &voter_group_name,
                &after_username.unwrap_or(""),
                &VOTER_SNAPSHOT_PAGE_SIZE,
            ],
        )
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| voter_snapshot_row_to_snapshot(row, areas_by_id))
        .collect())
}

/// Batch-fetches voter-group snapshots for exactly the usernames named in
/// `usernames` (typically one reconciliation-file batch's worth of
/// `VoterID`s), in a single round trip via `= ANY($2)` — the file-driven
/// counterpart to `fetch_realm_voter_snapshots_page`'s Sequent-driven
/// pagination, so reconciliation never needs the whole file's rows resident
/// in memory to look voters up. A username with no matching row simply
/// isn't present in the result; the caller treats that as "this file row's
/// voter doesn't exist in Sequent" (D, forward direction).
#[instrument(skip(keycloak_transaction, areas_by_id, usernames), err)]
pub async fn fetch_realm_voter_snapshots_by_usernames(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    voter_group_name: &str,
    usernames: &[String],
    areas_by_id: &HashMap<String, String>,
) -> Result<Vec<VoterSnapshot>> {
    if usernames.is_empty() {
        return Ok(Vec::new());
    }

    let statement = keycloak_transaction
        .prepare(
            r#"
                SELECT
                    u.id AS voter_id,
                    u.username,
                    u.enabled,
                    json_object_agg(ua.name, ua.value) FILTER (WHERE ua.name IS NOT NULL) AS attributes
                FROM user_entity u
                INNER JOIN realm AS ra ON ra.id = u.realm_id
                INNER JOIN user_group_membership ugm ON ugm.user_id = u.id
                INNER JOIN keycloak_group kg ON kg.id = ugm.group_id AND kg.realm_id = u.realm_id
                LEFT JOIN user_attribute ua ON ua.user_id = u.id
                WHERE ra.name = $1
                    AND kg.name = $2
                    AND u.username = ANY($3)
                GROUP BY u.id, u.username, u.enabled
            "#,
        )
        .await?;

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &[&realm, &voter_group_name, &usernames])
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| voter_snapshot_row_to_snapshot(row, areas_by_id))
        .collect())
}

#[instrument(skip(row, areas_by_id))]
fn voter_snapshot_row_to_snapshot(
    row: Row,
    areas_by_id: &HashMap<String, String>,
) -> Option<VoterSnapshot> {
    let voter_id: String = row.get("voter_id");
    let voter_id = Uuid::parse_str(&voter_id).ok()?;
    let username: String = row.get("username");
    let enabled: bool = row.get("enabled");
    let attributes: Option<serde_json::Value> = row.get("attributes");
    let attributes = attributes.unwrap_or(serde_json::Value::Null);

    let attr = |key: &str| -> Option<String> {
        attributes
            .get(key)
            .and_then(|value| value.as_str())
            .map(str::to_string)
    };

    let area_id = attr(AREA_ID_ATTR_NAME);
    let area_name = area_id.and_then(|id| areas_by_id.get(&id).cloned());

    Some(VoterSnapshot {
        username,
        voter_id,
        enabled,
        area_name,
        dob: attr(DATE_OF_BIRTH),
        voted_channel: attr(VOTED_CHANNEL),
        has_valid_internet_vote: false, // filled in by reconciliation's event-wide vote-state query
        has_unresolved_internet_vote: false, // filled in by reconciliation's event-wide vote-state query
        disable_comment: attr(DISABLE_COMMENT),
    })
}

#[instrument(skip(hasura_transaction), err)]
async fn get_area_ids(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<String>,
    election_id: Option<String>,
    area_id: Option<String>,
    param_number: i32,
) -> Result<(Option<Vec<String>>, String, String)> {
    let tenant_uuid = parse_uuid_v4(&tenant_id)?;
    let election_event_uuid: Option<Uuid> = election_event_id
        .map(|val| parse_uuid_v4(&val))
        .transpose()
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;

    if election_event_uuid.is_none() {
        return Ok((None, "".to_string(), "".to_string()));
    }
    let is_explicit_election_filter = election_id.is_some();
    let election_uuid: Option<Uuid> = election_id
        .map(|val| parse_uuid_v4(&val))
        .transpose()
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;

    let is_explicit_area_filter = area_id.is_some();
    let area_ids: Vec<String> = match area_id {
        Some(area_id_value) => vec![area_id_value],
        None => {
            // LEFT JOINed (not INNER) so areas with no contest at all still come back
            // when no specific election is requested ($3 IS NULL) — otherwise voters
            // in such an area silently vanish from the list instead of showing up so
            // an admin can reassign them. When $3 IS a specific election, the WHERE
            // still requires a matching contest, so callers like get_total_voters
            // (participation report denominator) keep excluding areas that can't
            // vote in that election — this must stay that way, or reports miscount.
            let areas_statement = hasura_transaction
                .prepare(
                    r#"
                SELECT DISTINCT
                    a.id::VARCHAR
                FROM
                    sequent_backend.area a
                LEFT JOIN
                    sequent_backend.area_contest ac ON a.id = ac.area_id AND ac.tenant_id = $1 AND ac.election_event_id = $2
                LEFT JOIN
                    sequent_backend.contest c ON ac.contest_id = c.id AND c.tenant_id = $1 AND c.election_event_id = $2
                WHERE
                    a.tenant_id = $1 AND
                    a.election_event_id = $2 AND
                    ($3::uuid IS NULL OR c.election_id = $3::uuid);
            "#,
                )
                .await?;
            let rows: Vec<Row> = hasura_transaction
                .query(
                    &areas_statement,
                    &[&tenant_uuid, &election_event_uuid, &election_uuid],
                )
                .await
                .map_err(|err| anyhow!("Error running the areas query: {}", err))?;
            let area_ids: Vec<String> = rows
                .into_iter()
                .map(|row| -> Result<String> {
                    Ok(row
                        .try_get::<&str, String>("id")
                        .map_err(|err| anyhow!("Error getting the area id of a row: {}", err))?)
                })
                .collect::<Result<Vec<String>>>()
                .map_err(|err| anyhow!("Error getting the areas ids: {}", err))?;
            area_ids
        }
    };

    debug!("area_ids: {area_ids:?}");
    // LEFT JOIN so voters with no area-id attribute still produce a row
    // (area_attr.user_id IS NULL) instead of being dropped by the join.
    let area_ids_join_clause = format!(
        r#"
    LEFT JOIN
        user_attribute AS area_attr ON u.id = area_attr.user_id AND area_attr.name = '{AREA_ID_ATTR_NAME}'
    "#,
    );
    let area_ids_where_clause = if is_explicit_area_filter || is_explicit_election_filter {
        // A specific area, or a specific election within the event, was
        // requested — keep strict matching. Relaxing this for the
        // election-scoped case would let voters with no area attribute
        // count toward that election's totals (e.g. get_total_voters,
        // the participation report denominator) even though they have no
        // contest to vote in for it.
        format!(
            r#"
    AND area_attr.value = ANY(${})
    "#,
            param_number,
        )
    } else {
        // Fully unscoped request (no area, no election): also surface voters
        // with no area assigned so they show up to be reviewed/reassigned.
        format!(
            r#"
    AND (
        area_attr.value = ANY(${}) OR area_attr.user_id IS NULL
    )
    "#,
            param_number,
        )
    };

    Ok((Some(area_ids), area_ids_join_clause, area_ids_where_clause))
}

/// Which optional multiplicity column the voter dump emits, if any. Delegated
/// voting and voter-weighted voting are mutually exclusive, so this is one
/// choice rather than two independent flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoterMultiplicityColumn {
    None,
    DelegateCount,
    VoteWeight,
}

/// A voter field the tally census reads when the tally runs. Changing one
/// can leave a cast ballot out of the tally or change how many times it
/// counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CensusAttribute {
    Enabled,
    AreaId,
    AuthorizedElectionIds,
    VoteWeight,
    DelegateVoteTo,
}

impl CensusAttribute {
    const KEYCLOAK_ATTRIBUTES: [CensusAttribute; 4] = [
        CensusAttribute::AreaId,
        CensusAttribute::AuthorizedElectionIds,
        CensusAttribute::VoteWeight,
        CensusAttribute::DelegateVoteTo,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CensusAttribute::Enabled => "enabled",
            CensusAttribute::AreaId => AREA_ID_ATTR_NAME,
            CensusAttribute::AuthorizedElectionIds => AUTHORIZED_ELECTION_IDS_NAME,
            CensusAttribute::VoteWeight => VOTE_WEIGHT_ATTR_NAME,
            CensusAttribute::DelegateVoteTo => DELEGATE_TO_ATTR_NAME,
        }
    }
}

impl std::fmt::Display for CensusAttribute {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

fn census_values(values: Option<&Vec<String>>) -> Vec<&str> {
    let mut values: Vec<&str> = values.into_iter().flatten().map(String::as_str).collect();
    values.sort_unstable();
    values
}

/// The census attributes an edit of `current` would change. `enabled` and
/// `attributes` are what the edit writes; an attribute it leaves out keeps
/// its value, as `KeycloakAdminClient::edit_user` merges attributes.
pub fn changed_census_attributes(
    current: &User,
    enabled: Option<bool>,
    attributes: &HashMap<String, Vec<String>>,
) -> Vec<CensusAttribute> {
    let mut changed = vec![];
    if enabled.is_some_and(|enabled| Some(enabled) != current.enabled) {
        changed.push(CensusAttribute::Enabled);
    }
    for attribute in CensusAttribute::KEYCLOAK_ATTRIBUTES {
        let Some(requested) = attributes.get(attribute.name()) else {
            continue;
        };
        let current_values = current
            .attributes
            .as_ref()
            .and_then(|current| current.get(attribute.name()));
        if census_values(Some(requested)) != census_values(current_values) {
            changed.push(attribute);
        }
    }
    changed
}

/// Why an edit changing `changed` must be refused: the voter has a valid or
/// in-progress ballot.
pub fn census_change_refusal(
    state: &VoterBallotState,
    changed: &[CensusAttribute],
) -> Option<String> {
    if changed.is_empty() || !state.has_active_vote() {
        return None;
    }
    let names: Vec<String> = changed.iter().map(ToString::to_string).collect();
    Some(format!(
        "Cannot change {} of a voter who has cast a ballot",
        names.join(", ")
    ))
}

/// The delegates whose delegator count an edit of `current` changes. A
/// delegate's ballot counts once for each voter delegating to them, so
/// these are the ballots a delegation change reweights.
pub fn reweighted_delegates(
    current: &User,
    attributes: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let Some(requested) = attributes.get(DELEGATE_TO_ATTR_NAME) else {
        return vec![];
    };
    let current_values = current
        .attributes
        .as_ref()
        .and_then(|current| current.get(DELEGATE_TO_ATTR_NAME));
    let mut balance: HashMap<&str, i64> = HashMap::new();
    for username in current_values.into_iter().flatten() {
        *balance.entry(username.as_str()).or_default() -= 1;
    }
    for username in requested {
        *balance.entry(username.as_str()).or_default() += 1;
    }
    let mut delegates: Vec<String> = balance
        .into_iter()
        .filter(|(username, count)| *count != 0 && !username.is_empty())
        .map(|(username, _)| username.to_string())
        .collect();
    delegates.sort_unstable();
    delegates
}

/// Why an edit of the election event voter `current` must be refused, if it
/// must: it changes a census attribute that counts a valid or in-progress
/// ballot. That is the voter's own ballot or, for a delegation change, the
/// ballot of each delegate the change adds the voter to or removes them
/// from.
#[instrument(
    skip(hasura_transaction, keycloak_transaction, current, attributes),
    err
)]
pub async fn census_edit_refusal(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    current: &User,
    enabled: Option<bool>,
    attributes: &HashMap<String, Vec<String>>,
) -> Result<Option<String>> {
    let changed = changed_census_attributes(current, enabled, attributes);
    if changed.is_empty() {
        return Ok(None);
    }
    let user_id = current
        .id
        .as_deref()
        .ok_or_else(|| anyhow!("The voter has no id"))?;
    let state = crate::services::ballot_box::get_voter_ballot_state(
        hasura_transaction,
        tenant_id,
        election_event_id,
        user_id,
    )
    .await?;
    if let Some(reason) = census_change_refusal(&state, &changed) {
        return Ok(Some(reason));
    }

    let realm = get_event_realm(tenant_id, election_event_id);
    for delegate in reweighted_delegates(current, attributes) {
        for delegate_id in get_users_by_username(keycloak_transaction, &realm, &delegate).await? {
            let state = crate::services::ballot_box::get_voter_ballot_state(
                hasura_transaction,
                tenant_id,
                election_event_id,
                &delegate_id,
            )
            .await?;
            if state.has_active_vote() {
                return Ok(Some(format!(
                    "Cannot change {} while the delegate {delegate} has cast a ballot",
                    CensusAttribute::DelegateVoteTo
                )));
            }
        }
    }
    Ok(None)
}

fn census_group_membership(alias: &str, group: Option<&str>) -> String {
    group
        .map(|group| {
            format!(
                "AND EXISTS (SELECT 1 FROM user_group_membership gm JOIN keycloak_group g \
         ON g.id = gm.group_id AND g.realm_id = {alias}.realm_id \
         WHERE gm.user_id = {alias}.id AND g.name = '{}')",
                escape_sql_literal(group),
            )
        })
        .unwrap_or_default()
}

/// The extra column of the voters CSV, if any.
///
/// At most one extra column is emitted, always at index 1. Both variants are
/// correlated scalar subqueries rather than joins so that `GROUP BY u.id`
/// still holds.
fn multiplicity_select(
    multiplicity_column: VoterMultiplicityColumn,
    area_id_escaped: &str,
    election_alias_escaped: &str,
    voter_group: Option<&str>,
) -> String {
    match multiplicity_column {
        VoterMultiplicityColumn::DelegateCount => {
            // A delegator must be a voter of this realm, area and election who
            // names only this delegate and has no valid ballot of their own.
            let no_service_account_delegators = service_account_exclusion("delegator");
            let delegator_group = census_group_membership("delegator", voter_group);
            format!(
                r#"
            ,(
                SELECT
                    COUNT(DISTINCT delegator.id)
                FROM
                    user_entity AS delegator
                JOIN
                    user_attribute AS ua_delegate ON delegator.id = ua_delegate.user_id
                JOIN
                    user_attribute AS ua_delegator_area ON delegator.id = ua_delegator_area.user_id AND ua_delegator_area.name = '{AREA_ID_ATTR_NAME}'
                LEFT JOIN
                    user_attribute AS ua_delegator_elections ON delegator.id = ua_delegator_elections.user_id AND ua_delegator_elections.name = '{AUTHORIZED_ELECTION_IDS_NAME}'
                WHERE
                    delegator.realm_id = u.realm_id AND
                    {no_service_account_delegators} {delegator_group} AND
                    delegator.enabled IS TRUE AND
                    ua_delegate.name = '{DELEGATE_TO_ATTR_NAME}' AND
                    ua_delegate.value = u.username AND
                    ua_delegator_area.value = '{area_id_escaped}' AND
                    (ua_delegator_elections.value = '{election_alias_escaped}' OR ua_delegator_elections.value IS NULL) AND
                    NOT EXISTS (
                        SELECT 1
                        FROM user_attribute AS ua_other_delegate
                        WHERE
                            ua_other_delegate.user_id = delegator.id AND
                            ua_other_delegate.name = '{DELEGATE_TO_ATTR_NAME}' AND
                            ua_other_delegate.value IS DISTINCT FROM u.username
                    ) AND
                    NOT EXISTS (
                        SELECT 1
                        FROM pg_temp.{VOTERS_WITH_VALID_BALLOT_TABLE} AS voted
                        WHERE voted.voter_id = delegator.id
                    )
            ) AS delegate_count
        "#
            )
        }
        VoterMultiplicityColumn::VoteWeight => {
            // COALESCE is mandatory: a voter without the attribute yields SQL NULL,
            // which COPY .. FORMAT CSV writes as an empty field. The cast to bigint
            // makes the aggregate numeric rather than lexicographic, which
            // matters because Keycloak allows several rows for one
            // (user_id, name) pair. MIN rather than MAX so an unexpected
            // duplicate cannot silently grant more voting power than the
            // smallest value recorded for the voter.
            format!(
                r#"
            ,(
                SELECT
                    COALESCE(MIN((NULLIF(ua_weight.value, ''))::bigint), {DEFAULT_VOTE_WEIGHT})
                FROM
                    user_attribute AS ua_weight
                WHERE
                    ua_weight.user_id = u.id AND
                    ua_weight.name = '{VOTE_WEIGHT_ATTR_NAME}'
            ) AS vote_weight
        "#
            )
        }
        VoterMultiplicityColumn::None => "".to_string(),
    }
}

#[instrument(skip(keycloak_transaction, voters_with_valid_ballot), err)]
pub async fn list_keycloak_enabled_users_by_area_id_and_authorized_elections(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
    election_alias: &str,
    output_file: &PathBuf,
    multiplicity_column: VoterMultiplicityColumn,
    voters_with_valid_ballot: &[String],
) -> Result<()> {
    list_keycloak_enabled_users_with_identity(
        keycloak_transaction,
        realm,
        area_id,
        election_alias,
        output_file,
        multiplicity_column,
        voters_with_valid_ballot,
        CensusIdentity::VoterId,
    )
    .await
}

/// Keycloak's delegation lookup needs real IDs in its transaction-local table.
/// This conversion is internal only; ballot files keep their public pseudonyms.
pub async fn census_voters_from_ballot_identities(
    store: &PostgresStore,
    event: &str,
    identities: &[String],
) -> Result<Vec<String>> {
    if store.ballot_identity(event).await?
        != electoral_log::adapters::privacy::BallotIdentity::Pseudonym
    {
        return Ok(identities.to_vec());
    }
    let mut voters = Vec::with_capacity(identities.len());
    for batch in identities.chunks(5_000) {
        let mapping = store.voters_for_pseudonyms(event, batch).await?;
        for identity in batch {
            voters.push(
                mapping
                    .get(identity)
                    .cloned()
                    .context("The tally identity mapping is unavailable")?,
            );
        }
    }
    Ok(voters)
}

/// Identity column of a census artifact. Privacy mode writes only pseudonyms;
/// the real census identities are transformed in memory before a file is made.
pub enum CensusIdentity<'a> {
    VoterId,
    Pseudonym {
        store: &'a PostgresStore,
        event: &'a str,
    },
}

#[derive(Debug, PartialEq, Eq)]
struct CensusRow {
    identity: String,
    multiplicity: Option<i64>,
}

fn pseudonymous_census_rows(
    rows: Vec<CensusRow>,
    mapping: &HashMap<String, String>,
) -> Result<Vec<CensusRow>> {
    let mut mapped = rows
        .into_iter()
        .map(|row| {
            Ok(CensusRow {
                identity: mapping
                    .get(&row.identity)
                    .cloned()
                    .context("The eligible census is missing privacy registrations")?,
                multiplicity: row.multiplicity,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    // The merge join requires lexical order of the actual identity column.
    mapped.sort_unstable_by(|left, right| left.identity.cmp(&right.identity));
    Ok(mapped)
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, err)]
pub async fn list_keycloak_enabled_users_with_identity(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
    election_alias: &str,
    output_file: &PathBuf,
    multiplicity_column: VoterMultiplicityColumn,
    voters_with_valid_ballot: &[String],
    identity: CensusIdentity<'_>,
) -> Result<()> {
    // COPY does not support parameters so we have to add them using format.
    // Validate area_id as v4 UUID before interpolating into SQL.
    parse_uuid_v4(area_id)?;
    let realm_escaped = escape_sql_literal(realm);
    let area_id_escaped = escape_sql_literal(area_id);
    let election_alias_escaped = escape_sql_literal(election_alias);

    let group_name = if matches!(&identity, CensusIdentity::Pseudonym { .. }) {
        Some(
            env::var("KEYCLOAK_VOTER_GROUP_NAME")
                .context("KEYCLOAK_VOTER_GROUP_NAME is required for the private census")?,
        )
    } else {
        None
    };
    let multiplicity_statement = multiplicity_select(
        multiplicity_column,
        &area_id_escaped,
        &election_alias_escaped,
        group_name.as_deref(),
    );

    if multiplicity_column == VoterMultiplicityColumn::DelegateCount {
        keycloak_transaction
            .batch_execute(&format!(
                "CREATE TEMP TABLE IF NOT EXISTS {VOTERS_WITH_VALID_BALLOT_TABLE} \
                 (voter_id TEXT PRIMARY KEY) ON COMMIT DROP; \
                 TRUNCATE pg_temp.{VOTERS_WITH_VALID_BALLOT_TABLE};"
            ))
            .await
            .context("Error creating the table of voters with a valid ballot")?;
        keycloak_transaction
            .execute(
                &format!(
                    "INSERT INTO pg_temp.{VOTERS_WITH_VALID_BALLOT_TABLE} (voter_id) \
                     SELECT DISTINCT unnest($1::text[])"
                ),
                &[&voters_with_valid_ballot],
            )
            .await
            .context("Error loading the voters with a valid ballot")?;
    }

    let no_service_accounts = service_account_exclusion("u");
    let voter_group = census_group_membership("u", group_name.as_deref());

    let statement = format!(
        r#"
        SELECT
            u.id
            {multiplicity_statement}
        FROM
            user_entity AS u
        JOIN
            realm ra ON u.realm_id = ra.id
        LEFT JOIN
            user_attribute ua_area ON u.id = ua_area.user_id AND ua_area.name = '{AREA_ID_ATTR_NAME}'
        LEFT JOIN
            user_attribute ua_elections ON u.id = ua_elections.user_id AND ua_elections.name = '{AUTHORIZED_ELECTION_IDS_NAME}'
        WHERE
            ra.name = '{realm_escaped}' AND
            {no_service_accounts} {voter_group} AND
            u.enabled IS TRUE AND
            ua_area.value = '{area_id_escaped}' AND
            (ua_elections.value = '{election_alias_escaped}' OR ua_elections.value IS NULL)
        GROUP BY
            u.id
        ORDER BY
            u.id
    "#
    );

    if let CensusIdentity::Pseudonym { store, event } = identity {
        let rows = keycloak_transaction.query(&statement, &[]).await?;
        let rows = rows
            .into_iter()
            .map(|row| {
                Ok(CensusRow {
                    identity: row.try_get(0)?,
                    multiplicity: if multiplicity_column == VoterMultiplicityColumn::None {
                        None
                    } else {
                        Some(row.try_get::<_, i64>(1)?)
                    },
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut mapping = HashMap::new();
        for batch in rows.chunks(5_000) {
            let ids = batch
                .iter()
                .map(|row| row.identity.clone())
                .collect::<Vec<_>>();
            mapping.extend(store.pseudonyms_for_voters(event, &ids).await?);
        }
        let rows = pseudonymous_census_rows(rows, &mapping)?;
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(Vec::new());
        for row in rows {
            match row.multiplicity {
                Some(value) => writer.write_record([row.identity, value.to_string()])?,
                None => writer.write_record([row.identity])?,
            }
        }
        tokio::fs::write(output_file, writer.into_inner()?).await?;
        return Ok(());
    }

    let tokio_temp_file = File::create(output_file)
        .await
        .context("Could not create the census file")?;

    let copy_out_query = format!("COPY ({}) TO STDOUT WITH (FORMAT CSV)", statement);
    let mut writer = BufWriter::new(tokio_temp_file);

    debug!("copy_out_query: {copy_out_query}");

    let reader = keycloak_transaction.copy_out(&copy_out_query).await?;

    let adapt_pg_error_to_io_error = |pg_err: tokio_postgres::Error| {
        std::io::Error::new(std::io::ErrorKind::Other, pg_err.to_string())
    };
    let io_error_stream = reader.map_err(adapt_pg_error_to_io_error);

    let async_reader = StreamReader::new(io_error_stream);
    tokio::pin!(async_reader);

    let bytes_copied = copy(&mut async_reader, &mut writer).await?;

    info!("voters bytes_copied: {bytes_copied}");

    writer.flush().await?;

    Ok(())
}

/// The enabled voters of any of `areas` who may vote in the election with
/// alias `election_alias`, each counted once however many of those areas
/// they belong to, leaving out every voter who also belongs to one of
/// `except_areas`.
///
/// A voter qualifies exactly as in
/// [`list_keycloak_enabled_users_by_area_id_and_authorized_elections`]: an
/// enabled account that is not a service account, in the event's realm, whose
/// authorized elections are unset or include the election.
#[instrument(skip(keycloak_transaction), err)]
pub async fn count_keycloak_enabled_voters_of_areas(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    areas: &[String],
    except_areas: &[String],
    election_alias: &str,
) -> Result<u64> {
    if areas.is_empty() {
        return Ok(0);
    }
    let no_service_accounts = service_account_exclusion("u");
    let statement = format!(
        r#"
        SELECT
            COUNT(*) AS voters
        FROM
            user_entity AS u
        JOIN
            realm AS ra ON u.realm_id = ra.id
        WHERE
            ra.name = $1 AND
            {no_service_accounts} AND
            u.enabled IS TRUE AND
            EXISTS (
                SELECT 1 FROM user_attribute AS ua_area
                WHERE ua_area.user_id = u.id AND
                    ua_area.name = '{AREA_ID_ATTR_NAME}' AND
                    ua_area.value = ANY($2)
            ) AND
            NOT EXISTS (
                SELECT 1 FROM user_attribute AS ua_area
                WHERE ua_area.user_id = u.id AND
                    ua_area.name = '{AREA_ID_ATTR_NAME}' AND
                    ua_area.value = ANY($3)
            ) AND
            (
                NOT EXISTS (
                    SELECT 1 FROM user_attribute AS ua_elections
                    WHERE ua_elections.user_id = u.id AND
                        ua_elections.name = '{AUTHORIZED_ELECTION_IDS_NAME}'
                ) OR
                EXISTS (
                    SELECT 1 FROM user_attribute AS ua_elections
                    WHERE ua_elections.user_id = u.id AND
                        ua_elections.name = '{AUTHORIZED_ELECTION_IDS_NAME}' AND
                        (ua_elections.value = $4 OR ua_elections.value IS NULL)
                )
            )
    "#
    );
    let row = keycloak_transaction
        .query_one(
            statement.as_str(),
            &[&realm, &areas, &except_areas, &election_alias],
        )
        .await
        .context("Error counting the enabled voters of the areas")?;
    let voters: i64 = row.try_get("voters")?;
    Ok(voters.try_into()?)
}

/// SQL boolean operator used to chain filter clauses in WHERE conditions.
#[derive(Debug, Clone, Copy, EnumString, Display)]
pub enum SqlBooleanOperator {
    #[strum(serialize = " AND")]
    And,
    #[strum(serialize = "")]
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display, Serialize)]
pub enum FilterOption {
    /// Those elements that contain the string are returned.
    IsLike(String),
    /// ILIKE but with unaccent and replacing blanks by single wildcards to detect hyphens.
    IsLikeUnaccentHyphens(String),
    /// Those elements that do not contain the string are returned.
    IsNotLike(String),
    /// Those elements that match precisely the string are returned.
    IsEqual(String),
    /// Those elements that do not match precisely the string are returned.
    IsNotEqual(String),
    /// Equals but uses a generated normalized column and compares to normalized text.
    IsEqualNormalized(String),
    /// When it is true, those elements that are null or empty are returned. When it is false they are discarded.
    IsEmpty(bool),
    /// Option not valid or set to null instead of an object, then it should not filter anything, display all.
    InvalidOrNull,
}

impl FilterOption {
    /// Get the parametrized sql clause which is a condition to filter at the given column, to be used in the WHERE clause.
    /// This function returns a tuple with the clause and the optional param, for which the param number must be provided.
    ///
    ///
    /// It is recommended to pass as param_number the current count of parameters in the dynamic sql query.
    /// If the returned parameter is Some, then the param count must be incremented by 1.
    fn get_sql_filter_clause(
        &self,
        col_name: &str,
        param_number: i32,
        operator: SqlBooleanOperator,
    ) -> (String, Option<String>) {
        // col_name is always supplied by internal code (hardcoded string
        // literals), never from user input. We still escape it as a SQL
        // identifier as defense-in-depth.
        let col_name = escape_sql_identifier(col_name);
        match self {
            Self::IsLike(pattern) => (
                format!(
                    r#"(${param_number}::VARCHAR IS NULL OR {col_name} ILIKE ${param_number}){operator}"#,
                ),
                Some(format!("%{}%", pattern)),
            ),
            Self::IsLikeUnaccentHyphens(pattern) => {
                let pattern = pattern.replace(" ", "_"); // replace blanks by single wildcards to detect hyphens
                (
                    format!(
                        r#"(${param_number}::VARCHAR IS NULL OR UNACCENT({col_name}) ILIKE ${param_number}){operator} "#,
                    ),
                    Some(format!("%{}%", pattern)),
                )
            }
            Self::IsEqualNormalized(pattern) => (
                format!(
                    r#"(normalize_text({col_name}) = normalize_text(${param_number})){operator} "#,
                ),
                Some(format!("{}", pattern)),
            ),

            Self::IsNotLike(pattern) => (
                format!(
                    r#"({col_name} IS NULL OR {col_name} NOT ILIKE ${param_number}){operator} "#,
                ),
                Some(format!("%{}%", pattern)),
            ),
            Self::IsEqual(pattern) => (
                format!(r#"({col_name} = ${param_number}){operator} "#,),
                Some(pattern.into()),
            ),
            Self::IsNotEqual(pattern) => (
                format!(r#"({col_name} <> ${param_number}){operator} "#,),
                Some(pattern.into()),
            ),
            Self::IsEmpty(true) => (
                format!(r#"({col_name} IS NULL OR {col_name} = ''){operator} "#,),
                None,
            ),
            Self::IsEmpty(false) => (
                format!(r#"({col_name} IS NOT NULL AND {col_name} <> ''){operator} "#,),
                None,
            ),
            Self::InvalidOrNull => {
                ("".to_string(), None) // no filtering
            }
        }
    }
}

impl<'de> Deserialize<'de> for FilterOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: Value = Deserialize::deserialize(deserializer)?;
        let map: HashMap<String, Value> = deserialize_value(value).map_err(|e| {
            serde::de::Error::custom(format!("Error parsing FilterOption o HMap: {e:?}"))
        })?;
        // Get the first key and value
        let (op, pattern_val) = map
            .iter()
            .next()
            .ok_or_else(|| serde::de::Error::custom("Error parsing FilterOption from map"))?;

        let filter: FilterOption = FilterOption::from_str(op).map_err(|e| {
            serde::de::Error::custom(format!("Error parsing FilterOption from str: {e:?}"))
        })?;

        let filter = match filter {
            FilterOption::InvalidOrNull => FilterOption::InvalidOrNull,
            FilterOption::IsEmpty(_) => {
                FilterOption::IsEmpty(pattern_val.as_bool().ok_or_else(|| {
                    serde::de::Error::custom(format!(
                        "Expected boolean value for IsEmpty Value: {pattern_val:?}"
                    ))
                })?)
            }
            FilterOption::IsLike(_) => {
                FilterOption::IsLike(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
            FilterOption::IsLikeUnaccentHyphens(_) => FilterOption::IsLikeUnaccentHyphens(
                deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?,
            ),
            FilterOption::IsNotLike(_) => {
                FilterOption::IsNotLike(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
            FilterOption::IsEqual(_) => {
                FilterOption::IsEqual(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
            FilterOption::IsNotEqual(_) => {
                FilterOption::IsNotEqual(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
            FilterOption::IsEqualNormalized(_) => {
                FilterOption::IsNotEqual(deserialize_value(pattern_val.clone()).map_err(|e| {
                    serde::de::Error::custom(format!(
                        "Error parsing String value {pattern_val:?} for pattern: {e:?}"
                    ))
                })?)
            }
        };

        Ok(filter)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default, Serialize, Deserialize)]
pub struct ListUsersFilter {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub election_id: Option<String>,
    /// Server-derived participation scope; absent on older queued selections.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permitted_election_ids: Option<Vec<String>>,
    pub area_id: Option<String>,
    pub realm: String,
    pub search: Option<String>,
    pub first_name: Option<FilterOption>,
    pub last_name: Option<FilterOption>,
    pub username: Option<FilterOption>,
    pub email: Option<FilterOption>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub user_ids: Option<Vec<String>>,
    pub attributes: Option<HashMap<String, String>>,
    pub email_verified: Option<bool>,
    pub enabled: Option<bool>,
    pub sort: Option<HashMap<String, String>>,
    pub has_voted: Option<bool>,
    pub authorized_to_election_alias: Option<String>,
}

impl ListUsersFilter {
    pub fn new(tenant_id: &str, realm: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            realm: realm.to_string(),
            ..Default::default()
        }
    }
}

fn get_query_bool_condition(field: &str, value: Option<bool>) -> String {
    match value {
        Some(true) => format!(r#"AND u.{} = true"#, field),
        Some(false) => format!(r#"AND u.{} = false"#, field),
        None => "".to_string(),
    }
}

/// Keycloak materialises a `user_entity` row for every client that has service accounts
/// enabled (`service-account-ivr-service`, `service-account-realm-management`, ...). Those
/// rows live in the same realm as the voters but are not voters, so they must never reach a
/// voter count or listing. `service_account_client_link` holds the owning client's id and is
/// null for real users, which makes it an exact test rather than a username-prefix guess.
fn service_account_exclusion(alias: &str) -> String {
    format!("{alias}.service_account_client_link IS NULL")
}

/// Base parameters shared by every voter-scoped query: realm and optional id allowlist.
/// This is the single source of truth for where `filter.realm`/`filter.user_ids` land in the
/// `params` slice, so `voter_scope_clause` and the query's own filter clauses can reference
/// their positions without hardcoding or re-deriving them independently.
struct VoterScopeParams<'a> {
    params: Vec<&'a (dyn ToSql + Sync)>,
    realm_param: i32,
    user_ids_param: i32,
    next_param_number: i32,
}

/// Takes `realm`/`user_ids` by reference (rather than `&ListUsersFilter`) so callers keep
/// borrowing only those two fields — a whole-struct borrow here would conflict with the
/// later partial moves out of other `filter` fields (e.g. `filter.sort`).
fn voter_scope_params<'a>(
    realm: &'a String,
    user_ids: &'a Option<Vec<String>>,
) -> VoterScopeParams<'a> {
    VoterScopeParams {
        params: vec![realm, user_ids],
        realm_param: 1,
        user_ids_param: 2,
        next_param_number: 3,
    }
}

/// WHERE-clause head shared by the voter count and voter listing queries: scope to the realm,
/// drop service accounts, then apply the caller's filters. `filters_clause` is the caller's
/// already-composed column filters, which carries its own trailing boolean operator when set.
/// `realm_param`/`user_ids_param` must be the positions returned by `voter_scope_params` for
/// the same `params` vec, so the placeholders here always match where the values were pushed.
fn voter_scope_clause(filters_clause: &str, realm_param: i32, user_ids_param: i32) -> String {
    let no_service_accounts = service_account_exclusion("u");
    format!(
        r#"ra.name = ${realm_param} AND
            {no_service_accounts} AND
            {filters_clause}
            (u.id = ANY(${user_ids_param}) OR ${user_ids_param} IS NULL)"#
    )
}

/// Gets sort clause ORDER BY, and the field parameter (column name or configurable attribute).
/// Checks if the field is valid and return None otherwise.
///
/// Maps the order input from the user into one of the valid options (ASC or DESC) to avoid injection, since we cannot put them as an sql parameter.
fn get_sort_clause_and_field_param(
    sort: Option<HashMap<String, String>>,
    param_number: i32,
) -> (String, Option<String>) {
    user_sort_for_alias(sort, param_number, "u")
}

/// Aliases are fixed by the two SELECTs, never supplied by the client.
fn user_sort_for_alias(
    sort: Option<HashMap<String, String>>,
    param_number: i32,
    alias: &str,
) -> (String, Option<String>) {
    const ASC: &str = "ASC";
    const DESC: &str = "DESC";
    fn sanitize_string(s: &str) -> String {
        s.trim_matches('\'').to_string()
    }
    let (sort_field, verified_order) = match sort {
        Some(sort_fields) => {
            let field = sort_fields
                .get("'field'")
                .map(|f| sanitize_string(f))
                .unwrap_or_else(|| "id".to_string());

            let order = sort_fields
                .get("'order'")
                .map(|o| match sanitize_string(o).to_uppercase().as_str() {
                    ASC => ASC,
                    DESC => DESC,
                    _ => ASC,
                })
                .unwrap_or_else(|| ASC);
            (field, order.to_string())
        }
        None => ("id".to_string(), ASC.to_string()),
    };

    match sort_field.as_str() {
        "id" => (format!("ORDER BY {alias}.id {verified_order}"), None),
        "email" | "first_name" | "last_name" | "username" | "enabled" | "email_verified" => (
            format!(r#"ORDER BY {alias}.{sort_field} {verified_order}, {alias}.id ASC"#),
            None,
        ),
        "has_voted" | "actions" => (format!("ORDER BY {alias}.id ASC"), None),
        _ => (
            format!(
                r#"ORDER BY (SELECT min(value) FROM user_attribute ua WHERE ua.user_id = {alias}.id AND ua.name = ${param_number}) {verified_order}, {alias}.id ASC"#,
            ),
            Some(sort_field),
        ),
    }
}

#[cfg(test)]
mod privacy_sort_regressions {
    use super::*;

    #[test]
    fn privacy_participation_sort_has_a_unique_tie_breaker_and_safe_fallback() {
        for field in ["last_name", "enabled", "region", "has_voted", "actions"] {
            let (order, _) = get_sort_clause_and_field_param(
                Some(HashMap::from([
                    ("'field'".into(), field.into()),
                    ("'order'".into(), "DESC".into()),
                ])),
                3,
            );
            assert!(order.ends_with("u.id ASC"), "{field}: {order}");
        }
    }
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn count_keycloak_users(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
) -> Result<i32> {
    // Start by setting up the base parameters: realm and user_ids.
    let VoterScopeParams {
        mut params,
        realm_param,
        user_ids_param,
        mut next_param_number,
    } = voter_scope_params(&filter.realm, &filter.user_ids);

    // Build filter clauses for basic fields.
    let mut filters_clause = String::new();
    let mut filter_params: Vec<String> = Vec::new();
    for (col_name, filter_option) in [
        ("email", &filter.email),
        ("first_name", &filter.first_name),
        ("last_name", &filter.last_name),
        ("username", &filter.username),
    ] {
        if let Some(filter_obj) = filter_option {
            let (clause, param) = filter_obj.get_sql_filter_clause(
                col_name,
                next_param_number,
                SqlBooleanOperator::And,
            );
            filters_clause.push_str(&clause);
            if let Some(param) = param {
                next_param_number += 1;
                filter_params.push(param.to_string());
            }
        }
    }
    for filt_param in filter_params.iter() {
        params.push(filt_param);
    }
    let search_pattern = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| format!("%{text}%"));
    if let Some(pattern) = &search_pattern {
        filters_clause.push_str(&format!(
            "concat_ws(' ', u.username, u.first_name, u.last_name, u.email) ILIKE ${next_param_number} AND ",
        ));
        params.push(pattern);
        next_param_number += 1;
    }

    // Add area-related joins/filters.
    let (area_ids, area_ids_join_clause, area_ids_where_clause) = get_area_ids(
        hasura_transaction,
        &filter.tenant_id,
        filter.election_event_id.clone(),
        filter.election_id.clone(),
        filter.area_id.clone(),
        next_param_number,
    )
    .await?;
    if let Some(area_ids) = &area_ids {
        params.push(area_ids);
        next_param_number += 1;
    }

    // Handle optional authorized election alias filtering.
    let (election_alias, authorized_alias_join_clause, authorized_alias_where_clause) = match filter
        .authorized_to_election_alias
    {
        Some(election_alias) => (
            Some(election_alias),
            format!(
                r#"
                    LEFT JOIN 
                        user_attribute AS authorization_attr ON u.id = authorization_attr.user_id AND authorization_attr.name = ${}
                    "#,
                next_param_number
            ),
            format!(
                r#"
                    AND (authorization_attr.value = ${} OR authorization_attr.user_id IS NULL)
                    "#,
                next_param_number + 1
            ),
        ),
        None => (None, String::new(), String::new()),
    };
    if election_alias.is_some() {
        params.push(&AUTHORIZED_ELECTION_IDS_NAME);
        params.push(&election_alias);
        next_param_number += 2;
    }

    // Append boolean conditions.
    let enabled_condition = get_query_bool_condition("enabled", filter.enabled);
    let email_verified_condition =
        get_query_bool_condition("email_verified", filter.email_verified);

    // Process dynamic attribute filters if any.
    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = Vec::new();
    if let Some(attributes) = &filter.attributes {
        for (key, value) in attributes {
            dynamic_attr_conditions.push(format!(
                r#"EXISTS (
                    SELECT 1 FROM user_attribute ua 
                    WHERE ua.user_id = u.id 
                      AND ua.name = ${} 
                      AND UNACCENT(ua.value) ILIKE ${}
                )"#,
                next_param_number,
                next_param_number + 1
            ));
            dynamic_attr_params.push(Some(key.trim_matches('\'').to_string()));
            dynamic_attr_params.push(Some(format!("%{value}%")));
            next_param_number += 2;
        }
    }
    for param in &dynamic_attr_params {
        params.push(param);
    }
    let dynamic_attr_clause = if dynamic_attr_conditions.is_empty() {
        String::new()
    } else {
        format!("AND ({})", dynamic_attr_conditions.join(" OR "))
    };

    // Build the count query using only the necessary filtering clauses.
    let scope_clause = voter_scope_clause(&filters_clause, realm_param, user_ids_param);
    let count_query = format!(
        r#"
        SELECT COUNT(*) AS total_count
        FROM user_entity AS u
        INNER JOIN realm AS ra ON ra.id = u.realm_id
        {area_ids_join_clause}
        {authorized_alias_join_clause}
        WHERE
            {scope_clause}
            {area_ids_where_clause}
            {authorized_alias_where_clause}
            {enabled_condition}
            {email_verified_condition}
            {dynamic_attr_clause}
        "#,
    );
    debug!("Count query: {count_query:?}");

    // Prepare and execute the count query.
    let stmt = keycloak_transaction.prepare(&count_query).await?;
    let row: Row = keycloak_transaction
        .query_one(&stmt, &params)
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let count: i32 = row.try_get::<&str, i64>("total_count")?.try_into()?;
    info!("Total eligible users: {count}");
    Ok(count)
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn list_users(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
) -> Result<(Vec<User>, i32)> {
    list_users_page(hasura_transaction, keycloak_transaction, filter, None, true).await
}

/// Internal keyset scans do not repeatedly recount the entire matching census.
async fn list_users_page(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
    after_user_id: Option<&str>,
    count_rows: bool,
) -> Result<(Vec<User>, i32)> {
    info!("filter: {filter:?}");
    let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
    let default_sql_limit = PgConfig::from_env()?.default_sql_limit;
    let query_limit: i64 =
        std::cmp::min(low_sql_limit, filter.limit.unwrap_or(default_sql_limit)).into();
    let query_offset: i64 = filter.offset.unwrap_or(0).into();

    let VoterScopeParams {
        mut params,
        realm_param,
        user_ids_param,
        mut next_param_number,
    } = voter_scope_params(&filter.realm, &filter.user_ids);

    let mut filters_clause = "".to_string();
    let mut filter_params: Vec<String> = vec![];
    for tuple in [
        ("email", &filter.email),
        ("first_name", &filter.first_name),
        ("last_name", &filter.last_name),
        ("username", &filter.username),
    ] {
        let (col_name, filter_option) = tuple;
        match filter_option {
            Some(filter_obj) => {
                let (clause, param) = filter_obj.get_sql_filter_clause(
                    col_name,
                    next_param_number,
                    SqlBooleanOperator::And,
                );
                filters_clause.push_str(&clause);
                if let Some(param) = param {
                    next_param_number += 1;
                    filter_params.push(param.to_string());
                }
            }
            None => {}
        }
    }
    for filt_param in filter_params.iter() {
        params.push(filt_param);
    }
    let search_pattern = filter
        .search
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| format!("%{text}%"));
    if let Some(pattern) = &search_pattern {
        filters_clause.push_str(&format!(
            "concat_ws(' ', u.username, u.first_name, u.last_name, u.email) ILIKE ${next_param_number} AND ",
        ));
        params.push(pattern);
        next_param_number += 1;
    }

    let (area_ids, area_ids_join_clause, area_ids_where_clause) = get_area_ids(
        hasura_transaction,
        &filter.tenant_id,
        filter.election_event_id.clone(),
        filter.election_id.clone(),
        filter.area_id.clone(),
        next_param_number,
    )
    .await?;

    if let Some(area_ids) = &area_ids {
        params.push(area_ids);
        next_param_number += 1;
    }

    let (election_alias, authorized_alias_join_clause, authorized_alias_where_clause) = match filter
        .authorized_to_election_alias
    {
        Some(election_alias) => (
            Some(election_alias),
            format!(
                r#"
            LEFT JOIN 
                user_attribute AS authorization_attr ON u.id = authorization_attr.user_id AND authorization_attr.name = ${}
            "#,
                next_param_number
            ),
            format!(
                r#"
            AND (
                authorization_attr.value = ${} OR authorization_attr.user_id IS NULL
            )
            "#,
                next_param_number + 1
            ),
        ),
        None => (None, "".to_string(), "".to_string()),
    };

    if election_alias.is_some() {
        params.push(&AUTHORIZED_ELECTION_IDS_NAME);
        params.push(&election_alias);
        next_param_number += 2;
    }

    let enabled_condition = get_query_bool_condition("enabled", filter.enabled);
    let email_verified_condition =
        get_query_bool_condition("email_verified", filter.email_verified);

    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = vec![];

    if let Some(attributes) = &filter.attributes {
        for (key, value) in attributes {
            dynamic_attr_conditions.push(format!(
                 r#"EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND UNACCENT(ua.value) ILIKE ${})"#,
                next_param_number,
                next_param_number + 1
            ));
            let val = Some(format!("%{value}%"));
            let formatted_keyy = key.trim_matches('\'').to_string();
            dynamic_attr_params.push(Some(formatted_keyy.clone()));
            dynamic_attr_params.push(val.clone());
            next_param_number += 2;
        }
    }
    for value in &dynamic_attr_params {
        params.push(value);
    }

    let dynamic_attr_clause = match dynamic_attr_conditions.is_empty() {
        true => "".to_string(),
        false => {
            format!(r#"AND({})"#, dynamic_attr_conditions.join(" OR "))
        }
    };

    if after_user_id.is_some() {
        filters_clause.push_str(&format!("u.id > ${next_param_number} AND "));
        params.push(&after_user_id);
        next_param_number += 1;
    }
    let keyset_scan = after_user_id.is_some();
    // A voter may have several matching area attributes. Group by the
    // user_entity primary key before applying participation pagination.
    let group_users = if keyset_scan || filter.has_voted.is_some() {
        "GROUP BY u.id"
    } else {
        ""
    };

    // The count query below has no ORDER BY clause, so it must not receive the
    // optional dynamic-attribute sort parameter used by the listing query.
    let count_params_len = params.len();
    let mut sort_params: Vec<Option<String>> = vec![];
    let requested_sort = if keyset_scan { None } else { filter.sort };
    let (sort_clause, field_param) =
        get_sort_clause_and_field_param(requested_sort.clone(), next_param_number);
    // Ordering inside a CTE does not order the containing SELECT.
    let (scan_order, _) = user_sort_for_alias(requested_sort, next_param_number, "lu");

    if field_param.is_some() {
        sort_params.push(field_param);
        next_param_number += 1;
    }
    for value in &sort_params {
        params.push(value);
    }

    debug!("parameters count: {}", next_param_number - 1);
    debug!("params {:?}", params);
    let scope_clause = voter_scope_clause(&filters_clause, realm_param, user_ids_param);
    let statement_str = format!(
        r#"
        WITH limited_users AS MATERIALIZED (
            SELECT
                u.id,
                u.email,
                u.email_verified,
                u.enabled,
                u.first_name,
                u.last_name,
                u.realm_id,
                u.username,
                u.created_timestamp
            FROM
                user_entity AS u
            INNER JOIN
                realm AS ra ON ra.id = u.realm_id
            {area_ids_join_clause}
            {authorized_alias_join_clause}
            WHERE
                {scope_clause}
                {area_ids_where_clause}
                {authorized_alias_where_clause}
                {enabled_condition}
                {email_verified_condition}
                {dynamic_attr_clause}
            {group_users}
            {sort_clause}
            LIMIT {query_limit} OFFSET {query_offset}
        )
        SELECT
            lu.id,
            lu.email,
            lu.email_verified,
            lu.enabled,
            lu.first_name,
            lu.last_name,
            lu.realm_id,
            lu.username,
            lu.created_timestamp,
            COALESCE(attr_json.attributes, '{{}}'::json) AS attributes
        FROM limited_users lu
        LEFT JOIN LATERAL (
            SELECT
                json_object_agg(attr.name, attr.values_array) AS attributes
            FROM (
                SELECT
                    ua.name,
                    json_agg(ua.value) AS values_array
                FROM user_attribute ua
                WHERE ua.user_id = lu.id
                GROUP BY ua.name
            ) attr
        ) attr_json ON TRUE
        {scan_order};
        "#
    );
    debug!("statement_str {statement_str:?}");

    let statement = keycloak_transaction.prepare(statement_str.as_str()).await?;
    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let realm: &str = &filter.realm;
    info!(
        "Count rows {} for realm={realm}, query_limit={query_limit}",
        rows.len()
    );

    // Count the amount of users for pagination
    let count_statement_str = format!(
        r#"
    SELECT
        COUNT(*) as total_count
    FROM
        user_entity AS u
    INNER JOIN
        realm AS ra ON ra.id = u.realm_id
    {area_ids_join_clause}
    {authorized_alias_join_clause}
    WHERE
        {scope_clause}
        {area_ids_where_clause}
        {authorized_alias_where_clause}
        {enabled_condition}
        {email_verified_condition}
        {dynamic_attr_clause}
    ;
    "#
    );
    debug!("statement_str {count_statement_str:?}");

    let count: i32 = if count_rows {
        let count_statement = keycloak_transaction
            .prepare(count_statement_str.as_str())
            .await?;
        let count_row: Row = keycloak_transaction
            .query_one(&count_statement, &params[..count_params_len])
            .await
            .map_err(|err| anyhow!("{}", err))?;

        count_row.try_get::<&str, i64>("total_count")?.try_into()?
    } else {
        0
    };

    // Process the users
    let users = rows
        .into_iter()
        .map(|row| -> Result<User> { row.try_into() })
        .collect::<Result<Vec<User>>>()?;
    if let Some(ref some_election_event_id) = filter.election_event_id {
        let unique_area_ids_set: HashSet<String> =
            users.iter().filter_map(|user| user.get_area_id()).collect();
        let area_ids: Vec<String> = unique_area_ids_set.into_iter().collect();

        let areas_by_ids = get_areas(
            hasura_transaction,
            filter.tenant_id.as_str(),
            some_election_event_id.as_str(),
            &area_ids,
        )
        .await
        .with_context(|| "can't find areas by ids")?;
        let get_area = |user: &User| {
            let area_id = user.get_area_id()?;
            return areas_by_ids.iter().find_map(|area| {
                let Some(ref area_dot_id) = area.id else {
                    return None;
                };
                if area_dot_id == &area_id {
                    Some(area.clone())
                } else {
                    None
                }
            });
        };
        let users_with_area = users
            .into_iter()
            .map(|user| {
                let area = get_area(&user);
                User {
                    area: area,
                    ..user.clone()
                }
            })
            .collect();
        Ok((users_with_area, count))
    } else {
        Ok((users, count))
    }
}

#[instrument(skip(hasura_transaction, keycloak_transaction, filter), err)]
pub async fn list_users_with_vote_info(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
) -> Result<(Vec<User>, i32)> {
    let tenant_id = filter.tenant_id.clone();
    let election_event_id = filter
        .election_event_id
        .clone()
        .ok_or(anyhow!("Election event id is empty"))?;
    let election_id = filter.election_id.clone();

    let filter_by_has_voted = filter.has_voted;
    let permitted_election_ids = filter.permitted_election_ids.clone();
    let (mut users, users_count) = list_users(hasura_transaction, keycloak_transaction, filter)
        .await
        .with_context(|| "Error listing users")?;

    let users: Vec<User> = get_users_with_vote_info_in_elections(
        hasura_transaction,
        tenant_id.as_str(),
        election_event_id.as_str(),
        election_id,
        users,
        filter_by_has_voted,
        permitted_election_ids.as_deref(),
    )
    .await
    .with_context(|| "Error listing users with vote info")?;

    Ok((users, users_count))
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn count_keycloak_enabled_users(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
) -> Result<i64> {
    let no_service_accounts = service_account_exclusion("u");
    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
                SELECT
                    COUNT(DISTINCT u.id) AS total_users
                FROM
                    user_entity AS u
                INNER JOIN
                    realm AS ra ON ra.id = u.realm_id
                WHERE
                    ra.name = $1 AND
                    {no_service_accounts} AND
                    u.enabled IS TRUE
                "#
            )
            .as_str(),
        )
        .await?;

    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm];

    let row = keycloak_transaction
        .query_one(&statement, &params)
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let user_count: i64 = row.get("total_users");
    Ok(user_count)
}

/// Use only for verifying application!, does not work as it seems for other situations, then use list_users instead.
#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn lookup_users(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
) -> Result<Vec<User>> {
    let low_sql_limit = PgConfig::from_env()
        .map_err(|e| anyhow!("Error getting low_sql_limit {e:?}"))?
        .low_sql_limit;

    let default_sql_limit = PgConfig::from_env()
        .map_err(|e| anyhow!("Error getting default_sql_limit {e:?}"))?
        .default_sql_limit;

    let query_limit: i64 =
        std::cmp::min(low_sql_limit, filter.limit.unwrap_or(default_sql_limit)).into();

    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&filter.realm, &query_limit];
    let mut next_param_number = 3;

    let mut filters_clause = "".to_string();
    let mut filter_params: Vec<String> = vec![];
    for tuple in [
        ("email", &filter.email),
        ("first_name", &filter.first_name),
        ("last_name", &filter.last_name),
        ("username", &filter.username),
    ] {
        let (col_name, filter_option) = tuple;
        match filter_option {
            Some(filter_obj) => {
                let (clause, param) = filter_obj.get_sql_filter_clause(
                    col_name,
                    next_param_number,
                    SqlBooleanOperator::None,
                );
                let clause = format!(
                    r#"
                    SELECT
                        ue.id, "{col_name}"
                    FROM user_entity ue
                    WHERE
                        {clause}
                    UNION ALL
                    "#
                );
                filters_clause.push_str(&clause);
                if let Some(param) = param {
                    next_param_number += 1;
                    filter_params.push(param.to_string());
                }
            }
            None => {}
        }
    }
    for filt_param in filter_params.iter() {
        params.push(filt_param);
    }

    let enabled_condition = get_query_bool_condition("enabled", filter.enabled);

    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = vec![];

    if let Some(attributes) = &filter.attributes {
        for (key, value) in attributes {
            dynamic_attr_conditions.push(format!(
                r#"(ua.name = ${} AND normalize_text(ua.value) = normalize_text(${}))"#,
                next_param_number,
                next_param_number + 1
            ));
            let val = Some(value.to_string());
            let formatted_keyy = key.trim_matches('\'').to_string();
            dynamic_attr_params.push(Some(formatted_keyy.clone()));
            dynamic_attr_params.push(val.clone());
            next_param_number += 2;
        }
    }
    for value in &dynamic_attr_params {
        params.push(value);
    }

    let dynamic_attr_clause = match dynamic_attr_conditions.is_empty() {
        true => "".to_string(),
        false => dynamic_attr_conditions.join(" OR "),
    };
    let no_service_accounts = service_account_exclusion("u");

    debug!("parameters count: {}", next_param_number - 1);
    debug!("params {:?}", params);

    let statement_str = format!(
        r#"
        WITH matched_ids AS (
            {filters_clause}
            SELECT
                ua.user_id, ua.name
            FROM user_attribute ua
            WHERE
                {dynamic_attr_clause}
        ),
        score_matches AS (
            SELECT mu.id, count(*) as match_score FROM matched_ids mu
            LEFT JOIN user_entity u ON u.id = mu.id
            LEFT JOIN realm ra ON ra.id = u.realm_id
            WHERE
                ra.name = $1
                AND {no_service_accounts}
                {enabled_condition}
            GROUP BY mu.id
        )
        SELECT match_score, u.id, u.email, u.email_verified, u.enabled, u.first_name, u.last_name, u.realm_id, u.username, u.created_timestamp, COALESCE(
                attr_json.attributes, '{{}}'::json
            ) AS attributes
        FROM
            score_matches rm
        INNER JOIN user_entity u ON u.id = rm.id
        LEFT JOIN LATERAL (
            SELECT json_object_agg(attr.name, attr.values_array) AS attributes
            FROM (
                    SELECT ua.name, json_agg(ua.value) AS values_array
                    FROM user_attribute ua
                    WHERE
                        ua.user_id = u.id
                    GROUP BY
                        ua.name
                ) attr
        ) attr_json ON true
        WHERE  match_score = (
                SELECT MAX(match_score)
                FROM score_matches
            )
        LIMIT $2
        "#
    );
    debug!("statement: {}", statement_str);

    let statement = keycloak_transaction.prepare(statement_str.as_str()).await?;
    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;
    let realm: &str = &filter.realm;
    debug!(
        "Count rows {} for realm={realm}, query_limit={query_limit}",
        rows.len()
    );

    let users = rows
        .into_iter()
        .map(|row| -> Result<User> { row.try_into() })
        .collect::<Result<Vec<User>>>()?;
    if let Some(ref some_election_event_id) = filter.election_event_id {
        let area_ids: Vec<String> = users.iter().filter_map(|user| user.get_area_id()).collect();
        let areas_by_ids = get_areas(
            hasura_transaction,
            filter.tenant_id.as_str(),
            some_election_event_id.as_str(),
            &area_ids,
        )
        .await
        .with_context(|| "can't find areas by ids")?;
        let get_area = |user: &User| {
            let area_id = user.get_area_id()?;
            return areas_by_ids.iter().find_map(|area| {
                let Some(ref area_dot_id) = area.id else {
                    return None;
                };
                if area_dot_id == &area_id {
                    Some(area.clone())
                } else {
                    None
                }
            });
        };
        let users_with_area = users
            .into_iter()
            .map(|user| {
                let area = get_area(&user);
                User {
                    area: area,
                    ..user.clone()
                }
            })
            .collect();
        Ok(users_with_area)
    } else {
        Ok(users)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumString, Display)]
pub enum AttributesFilterBy {
    IsLike,      // Those elements that contain the string are returned
    IsEqual,     // Those elements that match precisely the string are returned
    NotExist,    // Those elements that Not exist with givin value
    PartialLike, // Those elements that Not exist with givin value
}

#[derive(Debug, Clone)]
pub struct AttributesFilterOption {
    pub value: String,
    pub filter_by: AttributesFilterBy,
}

impl AttributesFilterOption {
    /// Return the sql condition to filter at the given column, to be used in the WHERE clause
    pub fn get_sql_filter_clause(&self, index: usize) -> String {
        let filter_option = self;
        match filter_option.filter_by {
            AttributesFilterBy::IsLike => {
                format!("EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE ${})",index - 1,index)
            }
            AttributesFilterBy::IsEqual => {
                format!("EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value = ${})",index -1, index)
            }
            AttributesFilterBy::NotExist => {
                format!("NOT EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value = ${})",index -1, index)
            }
            AttributesFilterBy::PartialLike => {
                format!("EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE '%' || ${} || '%')",index -1, index)
            }
        }
    }
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn count_keycloak_enabled_users_by_attrs(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    attrs: Option<HashMap<String, AttributesFilterOption>>, // bool : true = equal, false = isLike
) -> Result<i64> {
    let mut attr_conditions = Vec::new();
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&realm];

    if let Some(attributes) = &attrs {
        for (attr_name, attr_value) in attributes.iter() {
            let clause = attr_value.get_sql_filter_clause(params.len() + 2);
            params.push(attr_name);
            params.push(&attr_value.value);

            attr_conditions.push(clause);
        }
    }

    let attr_conditions_sql = if attr_conditions.is_empty() {
        r#"TRUE"#.to_string()
    } else {
        attr_conditions.join(r#" AND "#)
    };

    let no_service_accounts = service_account_exclusion("u");
    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
            SELECT
                COUNT(DISTINCT u.id) AS total_users
            FROM
                user_entity AS u
            INNER JOIN
                realm AS ra ON ra.id = u.realm_id
            WHERE
                ra.name = $1
                AND {no_service_accounts}
                AND u.enabled IS TRUE
                AND ({attr_conditions_sql})
            "#
            )
            .as_str(),
        )
        .await?;

    let row = keycloak_transaction
        .query_one(&statement, &params)
        .await
        .map_err(|err| anyhow!("Error executing the query: {}", err))?;

    let user_count: i64 = row.get("total_users");
    Ok(user_count)
}

// use std::error::Error;
// use reqwest::Client;

#[derive(Debug, Deserialize)]
struct Group {
    id: String,
    name: String,
}

#[instrument(skip(keycloak_transaction), err)]
pub async fn check_is_user_verified(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    user_id: &str,
) -> Result<bool> {
    let statement = keycloak_transaction
        .prepare(
            format!(
                r#"
            SELECT EXISTS (
                SELECT 1 
                FROM user_attribute ua
                INNER JOIN user_entity u ON ua.user_id = u.id
                INNER JOIN realm r ON u.realm_id = r.id
                WHERE r.name = $1 
                AND u.id = $2
                AND ua.name = '{VALIDATE_ID_ATTR_NAME}'
                AND ua.value = '{VALIDATE_ID_REGISTERED_VOTER}'
            ) AS is_verified;
            "#
            )
            .as_str(),
        )
        .await?;

    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &user_id];

    let row = keycloak_transaction
        .query_one(&statement, &params)
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let is_verified: bool = row.get("is_verified");
    Ok(is_verified)
}

/// Returns a vector with user ids.
/// It is up to the caller to handle when there are mutiple users with the same username or the vector is empty - not found.
#[instrument(err, skip(keycloak_transaction))]
pub async fn get_users_by_username(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    username: &str,
) -> Result<Vec<String>> {
    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &username];

    let statement = keycloak_transaction
        .prepare(&format!(
            r#"
        SELECT 
            u.id
        FROM 
            user_entity u
        INNER JOIN
            realm AS ra ON ra.id = u.realm_id
        LEFT JOIN LATERAL (
            SELECT
                json_object_agg(ua.name, ua.value) AS attributes
            FROM user_attribute ua
            WHERE ua.user_id = u.id
            GROUP BY ua.user_id
        ) attr_json ON true
        WHERE
            ra.name = $1
            AND u.username = $2
        "#,
        ))
        .await?;

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let user_ids = rows
        .into_iter()
        .filter_map(|row| row.get("id"))
        .collect::<Vec<String>>();

    Ok(user_ids)
}

/// Returns the username of the user id or None if it does not exist.
#[instrument(err, skip(keycloak_transaction))]
pub async fn get_username_by_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    user_id: &str,
) -> Result<Option<String>> {
    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &user_id];

    let statement = keycloak_transaction
        .prepare(&format!(
            r#"
        SELECT
            u.username
        FROM
            user_entity u
        INNER JOIN
            realm AS ra ON ra.id = u.realm_id
        LEFT JOIN LATERAL (
            SELECT
                json_object_agg(ua.name, ua.value) AS attributes
            FROM user_attribute ua
            WHERE ua.user_id = u.id
            GROUP BY ua.user_id
        ) attr_json ON true
        WHERE
            ra.name = $1
            AND u.id = $2
        "#,
        ))
        .await?;

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{err:?}"))?;

    let user_ids = rows
        .into_iter()
        .filter_map(|row| row.get("username"))
        .collect::<Vec<String>>();
    match user_ids.is_empty() {
        true => Ok(None),
        false => Ok(Some(user_ids[0].clone())),
    }
}

#[instrument(err, skip(keycloak_transaction))]
pub async fn get_user_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    user_id: &str,
) -> Result<Option<String>> {
    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &user_id];

    let statement = keycloak_transaction
        .prepare(&format!(
            r#"
        SELECT
             attr_json.attributes ->> '{AREA_ID_ATTR_NAME}' AS area_id
        FROM
            user_entity u
        INNER JOIN
            realm AS ra ON ra.id = u.realm_id
        LEFT JOIN LATERAL (
            SELECT
                json_object_agg(ua.name, ua.value) AS attributes
            FROM
                user_attribute ua
            WHERE
                ua.user_id = u.id
            GROUP BY
                ua.user_id
        ) attr_json ON true
        WHERE
            ra.name = $1
            AND u.id = $2
        "#
        ))
        .await?;

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{err:?}"))?;

    // Assuming there is at most one matching row, we extract the area_id (which might be null)
    if let Some(row) = rows.into_iter().next() {
        let area_id: Option<String> = row.get("area_id");
        Ok(area_id)
    } else {
        Ok(None)
    }
}

/// Voters of the filter's election event, or of one of its elections, with a valid
/// vote.
#[instrument(skip(hasura_transaction), err)]
pub async fn count_have_voted(
    hasura_transaction: &Transaction<'_>,
    filter: &ListUsersFilter,
    tenant_id: &str,
) -> Result<i32> {
    let election_event_id = filter
        .election_event_id
        .as_deref()
        .ok_or_else(|| anyhow!("Counting the voters who voted needs an election event"))?;
    let election_id = filter
        .election_id
        .as_deref()
        .map(parse_uuid_v4)
        .transpose()?
        .map(|id| id.to_string());
    let ballot_box = get_event_ballot_box(hasura_transaction, tenant_id, election_event_id).await?;
    let voters = if ballot_box.store.ballot_identity(election_event_id).await?
        == electoral_log::adapters::privacy::BallotIdentity::Pseudonym
    {
        ballot_box
            .store
            .privacy_participation_count(election_event_id, election_id.as_deref())
            .await?
    } else {
        ballot_box
            .store
            .participation(election_event_id, election_id.as_deref(), None)
            .await?
            .voters
    };
    Ok(voters.try_into()?)
}

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn list_users_has_voted(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    filter: ListUsersFilter,
    tenant_id: &str,
) -> Result<(Vec<User>, i32)> {
    let wanted = filter
        .has_voted
        .context("Participation filter not specified")?;
    let event = filter
        .election_event_id
        .as_deref()
        .context("Participation requires an event")?;
    let settings = PgConfig::from_env()?;
    let batch_size = settings
        .default_sql_batch_size
        .min(settings.low_sql_limit)
        .max(1);
    let mut after = String::new();
    let mut matching_ids = Vec::new();
    loop {
        let mut batch = filter.clone();
        batch.limit = Some(batch_size);
        batch.offset = Some(0);
        batch.sort = None;
        let (users, _) = list_users_page(
            hasura_transaction,
            keycloak_transaction,
            batch,
            Some(&after),
            false,
        )
        .await?;
        if users.is_empty() {
            break;
        }
        let next = users
            .iter()
            .filter_map(|user| user.id.as_ref())
            .max()
            .context("A listed voter has no identifier")?
            .clone();
        anyhow::ensure!(next > after, "Voter scan did not advance");
        let users = get_users_with_vote_info_in_elections(
            hasura_transaction,
            tenant_id,
            event,
            filter.election_id.clone(),
            users,
            None,
            filter.permitted_election_ids.as_deref(),
        )
        .await?;
        matching_ids.extend(
            users
                .into_iter()
                .filter(|user| {
                    let voted = user
                        .votes_info
                        .as_ref()
                        .is_some_and(|votes| !votes.is_empty());
                    voted == wanted
                })
                .filter_map(|user| user.id),
        );
        after = next;
    }
    matching_ids.sort_unstable();
    matching_ids.dedup();
    let total = matching_ids
        .len()
        .try_into()
        .context("Too many matching voters")?;
    if matching_ids.is_empty() || filter.limit == Some(0) {
        return Ok((vec![], total));
    }

    // The same matching identity set controls both the total and the normal
    // requested sort/page. Participation lookups never read privacy ballots.
    let mut page = filter.clone();
    page.user_ids = Some(matching_ids);
    let (users, _) =
        list_users_page(hasura_transaction, keycloak_transaction, page, None, false).await?;
    let users = get_users_with_vote_info_in_elections(
        hasura_transaction,
        tenant_id,
        event,
        filter.election_id.clone(),
        users,
        None,
        filter.permitted_election_ids.as_deref(),
    )
    .await?;
    Ok((users, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: &str = "10000000-0000-4000-8000-00000000000a";

    #[test]
    fn privacy_census_sorts_the_new_identity_and_preserves_multiplicity() {
        let mapping = HashMap::from([
            ("real-a".to_owned(), "ZZZZ-ZZZZ".to_owned()),
            ("real-b".to_owned(), "AAAA-AAAA".to_owned()),
        ]);
        // Both weight and delegate-count multiplicities survive the identity swap.
        for values in [[Some(7), Some(2)], [Some(0), Some(4)], [None, None]] {
            let rows = pseudonymous_census_rows(
                vec![
                    CensusRow {
                        identity: "real-a".to_owned(),
                        multiplicity: values[0],
                    },
                    CensusRow {
                        identity: "real-b".to_owned(),
                        multiplicity: values[1],
                    },
                ],
                &mapping,
            )
            .unwrap();
            assert_eq!(
                rows,
                vec![
                    CensusRow {
                        identity: "AAAA-AAAA".to_owned(),
                        multiplicity: values[1]
                    },
                    CensusRow {
                        identity: "ZZZZ-ZZZZ".to_owned(),
                        multiplicity: values[0]
                    },
                ]
            );
        }
    }

    #[test]
    fn privacy_census_refuses_missing_mapping_without_disclosing_identity() {
        let result = pseudonymous_census_rows(
            vec![CensusRow {
                identity: "unregistered-real-voter".to_owned(),
                multiplicity: None,
            }],
            &HashMap::new(),
        );
        let error = result.unwrap_err().to_string();
        assert!(!error.contains("unregistered-real-voter"));
        assert!(error.contains("missing privacy registrations"));
    }

    #[test]
    fn the_delegate_count_only_counts_eligible_delegators_without_a_ballot() {
        let select = multiplicity_select(
            VoterMultiplicityColumn::DelegateCount,
            AREA,
            "election-a",
            None,
        );

        assert!(select.contains("COUNT(DISTINCT delegator.id)"));
        assert!(select.contains("delegator.realm_id = u.realm_id"));
        assert!(select.contains(&format!("ua_delegator_area.value = '{AREA}'")));
        assert!(select.contains("ua_delegator_elections.value = 'election-a'"));
        assert!(select.contains(&format!("pg_temp.{VOTERS_WITH_VALID_BALLOT_TABLE}")));
        assert!(select.contains("AS delegate_count"));
    }

    #[test]
    fn the_vote_weight_column_defaults_a_missing_attribute() {
        let select = multiplicity_select(
            VoterMultiplicityColumn::VoteWeight,
            AREA,
            "election-a",
            None,
        );

        assert!(select.contains(&format!("{DEFAULT_VOTE_WEIGHT})")));
        assert!(select.contains("AS vote_weight"));
        assert!(!select.contains(VOTERS_WITH_VALID_BALLOT_TABLE));
    }

    #[test]
    fn no_multiplicity_adds_no_column() {
        assert_eq!(
            multiplicity_select(VoterMultiplicityColumn::None, AREA, "election-a", None),
            ""
        );
    }

    #[test]
    fn test_sql_boolean_operator_and_format() {
        let clause = format!("(col = $1){}", SqlBooleanOperator::And);
        assert_eq!(clause, "(col = $1) AND");
    }

    #[test]
    fn test_sql_boolean_operator_none_format() {
        let clause = format!("(col = $1){}", SqlBooleanOperator::None);
        assert_eq!(clause, "(col = $1)");
    }

    #[test]
    fn test_service_account_exclusion_targets_the_given_alias() {
        // The tally eligibility query joins user_entity twice, as `u` and as `delegator`.
        assert_eq!(
            service_account_exclusion("u"),
            "u.service_account_client_link IS NULL"
        );
        assert_eq!(
            service_account_exclusion("delegator"),
            "delegator.service_account_client_link IS NULL"
        );
    }

    #[test]
    fn test_voter_scope_clause_excludes_service_accounts() {
        let clause = voter_scope_clause("", 1, 2);
        assert!(
            clause.contains("u.service_account_client_link IS NULL"),
            "voter queries must not report Keycloak service accounts as voters: {clause}"
        );
    }

    #[test]
    fn test_voter_scope_clause_keeps_realm_and_caller_filters() {
        let filters_clause = format!(r#"("email" = $3){}"#, SqlBooleanOperator::And);
        let clause = voter_scope_clause(&filters_clause, 1, 2);

        assert!(clause.contains("ra.name = $1"));
        assert!(clause.contains(r#"("email" = $3) AND"#));
        assert!(clause.contains("(u.id = ANY($2) OR $2 IS NULL)"));
    }
    fn voter(enabled: bool, attributes: &[(&str, &[&str])]) -> User {
        User {
            enabled: Some(enabled),
            attributes: Some(
                attributes
                    .iter()
                    .map(|(name, values)| {
                        (
                            name.to_string(),
                            values.iter().map(|value| value.to_string()).collect(),
                        )
                    })
                    .collect(),
            ),
            ..User::default()
        }
    }

    fn requested(attributes: &[(&str, &[&str])]) -> HashMap<String, Vec<String>> {
        voter(true, attributes).attributes.unwrap_or_default()
    }

    #[test]
    fn census_changes_detect_every_tally_attribute() {
        let current = voter(
            true,
            &[
                (AREA_ID_ATTR_NAME, &["area-a"]),
                (AUTHORIZED_ELECTION_IDS_NAME, &["mayor", "council"]),
                (VOTE_WEIGHT_ATTR_NAME, &["1"]),
                (DELEGATE_TO_ATTR_NAME, &["alice"]),
            ],
        );
        let edit = requested(&[
            (AREA_ID_ATTR_NAME, &["area-b"]),
            (AUTHORIZED_ELECTION_IDS_NAME, &["mayor"]),
            (VOTE_WEIGHT_ATTR_NAME, &["5"]),
            (DELEGATE_TO_ATTR_NAME, &["bob"]),
        ]);

        assert_eq!(
            changed_census_attributes(&current, Some(false), &edit),
            vec![
                CensusAttribute::Enabled,
                CensusAttribute::AreaId,
                CensusAttribute::AuthorizedElectionIds,
                CensusAttribute::VoteWeight,
                CensusAttribute::DelegateVoteTo,
            ]
        );
    }

    #[test]
    fn census_changes_detect_added_and_removed_attributes() {
        let current = voter(true, &[(AUTHORIZED_ELECTION_IDS_NAME, &["mayor"])]);
        let edit = requested(&[
            (AUTHORIZED_ELECTION_IDS_NAME, &[]),
            (VOTE_WEIGHT_ATTR_NAME, &["3"]),
        ]);

        assert_eq!(
            changed_census_attributes(&current, None, &edit),
            vec![
                CensusAttribute::AuthorizedElectionIds,
                CensusAttribute::VoteWeight,
            ]
        );
    }

    #[test]
    fn echoed_or_unrelated_values_change_no_census_attribute() {
        let current = voter(
            true,
            &[
                (AREA_ID_ATTR_NAME, &["area-a"]),
                (AUTHORIZED_ELECTION_IDS_NAME, &["mayor", "council"]),
                ("first-language", &["en"]),
            ],
        );
        let edit = requested(&[
            (AREA_ID_ATTR_NAME, &["area-a"]),
            (AUTHORIZED_ELECTION_IDS_NAME, &["council", "mayor"]),
            (VOTE_WEIGHT_ATTR_NAME, &[]),
            ("first-language", &["fr"]),
        ]);

        assert!(changed_census_attributes(&current, Some(true), &edit).is_empty());
        assert!(changed_census_attributes(&current, None, &HashMap::new()).is_empty());
    }

    #[test]
    fn reweighted_delegates_are_the_delegates_gained_or_lost() {
        let current = voter(true, &[(DELEGATE_TO_ATTR_NAME, &["alice"])]);

        assert_eq!(
            reweighted_delegates(&current, &requested(&[(DELEGATE_TO_ATTR_NAME, &["bob"])])),
            vec!["alice".to_string(), "bob".to_string()]
        );
        assert_eq!(
            reweighted_delegates(&current, &requested(&[(DELEGATE_TO_ATTR_NAME, &[])])),
            vec!["alice".to_string()]
        );
        assert_eq!(
            reweighted_delegates(
                &voter(true, &[]),
                &requested(&[(DELEGATE_TO_ATTR_NAME, &["bob"])])
            ),
            vec!["bob".to_string()]
        );
    }

    #[test]
    fn an_unchanged_delegation_reweights_no_delegate() {
        let current = voter(true, &[(DELEGATE_TO_ATTR_NAME, &["alice"])]);

        assert!(
            reweighted_delegates(&current, &requested(&[(DELEGATE_TO_ATTR_NAME, &["alice"])]))
                .is_empty()
        );
        assert!(
            reweighted_delegates(&current, &requested(&[(AREA_ID_ATTR_NAME, &["area-b"])]))
                .is_empty()
        );
    }

    fn cast_vote_state(has_pending: bool, has_valid: bool) -> VoterBallotState {
        VoterBallotState {
            has_pending,
            has_valid,
        }
    }

    #[test]
    fn census_change_is_refused_for_a_voter_with_an_active_ballot() {
        let changed = [CensusAttribute::AreaId, CensusAttribute::VoteWeight];
        for state in [cast_vote_state(true, false), cast_vote_state(false, true)] {
            assert_eq!(
                census_change_refusal(&state, &changed).as_deref(),
                Some("Cannot change area-id, vote-weight of a voter who has cast a ballot")
            );
        }
    }

    #[test]
    fn census_change_is_allowed_without_an_active_ballot_or_without_changes() {
        assert_eq!(
            census_change_refusal(&cast_vote_state(false, false), &[CensusAttribute::Enabled]),
            None
        );
        assert_eq!(
            census_change_refusal(&cast_vote_state(false, true), &[]),
            None
        );
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL configured through KEYCLOAK_DB__*"]
    async fn delegate_count_only_counts_eligible_delegators_without_a_ballot() -> Result<()> {
        const AREA_ID: &str = "10000000-0000-4000-8000-00000000000a";
        const OTHER_AREA_ID: &str = "10000000-0000-4000-8000-00000000000b";
        const ELECTION_ALIAS: &str = "election-a";

        let pool = crate::services::database::generate_keycloak_pool().await?;
        let mut client = pool.get().await?;
        let transaction = client.transaction().await?;
        transaction
            .batch_execute(&format!(
                r#"
                CREATE TEMP TABLE realm (id TEXT PRIMARY KEY, name TEXT) ON COMMIT DROP;
                CREATE TEMP TABLE user_entity (
                    id TEXT PRIMARY KEY,
                    realm_id TEXT,
                    username TEXT,
                    enabled BOOLEAN,
                    service_account_client_link TEXT
                ) ON COMMIT DROP;
                CREATE TEMP TABLE user_attribute (
                    id SERIAL PRIMARY KEY,
                    user_id TEXT,
                    name TEXT,
                    value TEXT
                ) ON COMMIT DROP;
                INSERT INTO realm (id, name) VALUES ('realm-a', 'event-a'), ('realm-b', 'event-b');
                INSERT INTO user_entity (id, realm_id, username, enabled, service_account_client_link)
                VALUES ('bob', 'realm-a', 'bob', true, NULL),
                       ('zoe', 'realm-a', 'zoe', true, NULL),
                       ('voted', 'realm-a', 'voted', true, NULL),
                       ('disabled', 'realm-a', 'disabled', false, NULL),
                       ('other-area', 'realm-a', 'other-area', true, NULL),
                       ('other-election', 'realm-a', 'other-election', true, NULL),
                       ('this-election', 'realm-a', 'this-election', true, NULL),
                       ('repeated', 'realm-a', 'repeated', true, NULL),
                       ('two-delegates', 'realm-a', 'two-delegates', true, NULL),
                       ('eligible', 'realm-a', 'eligible', true, NULL),
                       ('service', 'realm-a', 'service', true, 'client'),
                       ('other-realm', 'realm-b', 'other-realm', true, NULL);
                INSERT INTO user_attribute (user_id, name, value)
                SELECT id, '{AREA_ID_ATTR_NAME}', '{AREA_ID}' FROM user_entity WHERE id <> 'other-area';
                INSERT INTO user_attribute (user_id, name, value)
                VALUES ('other-area', '{AREA_ID_ATTR_NAME}', '{OTHER_AREA_ID}'),
                       ('other-election', '{AUTHORIZED_ELECTION_IDS_NAME}', 'election-b'),
                       ('this-election', '{AUTHORIZED_ELECTION_IDS_NAME}', '{ELECTION_ALIAS}'),
                       ('voted', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('disabled', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('other-area', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('other-election', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('this-election', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('repeated', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('repeated', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('two-delegates', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('two-delegates', '{DELEGATE_TO_ATTR_NAME}', 'zoe'),
                       ('eligible', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('service', '{DELEGATE_TO_ATTR_NAME}', 'bob'),
                       ('other-realm', '{DELEGATE_TO_ATTR_NAME}', 'bob');
                "#
            ))
            .await?;

        let output_file = tempfile::NamedTempFile::new()?;
        list_keycloak_enabled_users_by_area_id_and_authorized_elections(
            &transaction,
            "event-a",
            AREA_ID,
            ELECTION_ALIAS,
            &output_file.path().to_path_buf(),
            VoterMultiplicityColumn::DelegateCount,
            &["bob".to_string(), "voted".to_string()],
        )
        .await?;
        transaction.rollback().await?;

        let delegate_counts: HashMap<String, u64> = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(output_file.reopen()?)
            .records()
            .map(|record| {
                let record = record?;
                Ok((record[0].to_string(), record[1].parse()?))
            })
            .collect::<Result<_>>()?;

        // Only repeated, this-election and eligible transfer their vote.
        assert_eq!(delegate_counts.get("bob"), Some(&3));
        assert_eq!(delegate_counts.get("zoe"), Some(&0));
        assert_eq!(delegate_counts.get("other-election"), None);
        Ok(())
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL configured through KEYCLOAK_DB__*"]
    async fn the_voters_of_areas_are_the_enabled_ones_each_counted_once() -> Result<()> {
        const NORTH: &str = "20000000-0000-4000-8000-00000000000a";
        const SOUTH: &str = "20000000-0000-4000-8000-00000000000b";
        // An area whose contests are all acclaimed: nothing to tally there.
        const ACCLAIMED: &str = "20000000-0000-4000-8000-00000000000c";
        const ELSEWHERE: &str = "20000000-0000-4000-8000-00000000000d";
        // Every voter of this area is disabled.
        const DISABLED: &str = "20000000-0000-4000-8000-00000000000e";
        // Its voter is authorized for an election without an external id,
        // which Keycloak names by its id.
        const BY_ID: &str = "20000000-0000-4000-8000-00000000000f";
        const ELECTION_ID: &str = "30000000-0000-4000-8000-000000000001";
        const ELECTION_ALIAS: &str = "election-a";

        let pool = crate::services::database::generate_keycloak_pool().await?;
        let mut client = pool.get().await?;
        let transaction = client.transaction().await?;
        transaction
            .batch_execute(&format!(
                r#"
                CREATE TEMP TABLE realm (id TEXT PRIMARY KEY, name TEXT) ON COMMIT DROP;
                CREATE TEMP TABLE user_entity (
                    id TEXT PRIMARY KEY,
                    realm_id TEXT,
                    username TEXT,
                    enabled BOOLEAN,
                    service_account_client_link TEXT
                ) ON COMMIT DROP;
                CREATE TEMP TABLE user_attribute (
                    id SERIAL PRIMARY KEY,
                    user_id TEXT,
                    name TEXT,
                    value TEXT
                ) ON COMMIT DROP;
                INSERT INTO realm (id, name) VALUES ('realm-a', 'event-a'), ('realm-b', 'event-b');
                INSERT INTO user_entity (id, realm_id, username, enabled, service_account_client_link)
                VALUES ('north-1', 'realm-a', 'north-1', true, NULL),
                       ('north-2', 'realm-a', 'north-2', true, NULL),
                       ('north-off', 'realm-a', 'north-off', false, NULL),
                       ('south-1', 'realm-a', 'south-1', true, NULL),
                       ('acclaimed-1', 'realm-a', 'acclaimed-1', true, NULL),
                       ('acclaimed-2', 'realm-a', 'acclaimed-2', true, NULL),
                       ('acclaimed-off', 'realm-a', 'acclaimed-off', false, NULL),
                       ('both', 'realm-a', 'both', true, NULL),
                       ('this-election', 'realm-a', 'this-election', true, NULL),
                       ('other-election', 'realm-a', 'other-election', true, NULL),
                       ('elsewhere', 'realm-a', 'elsewhere', true, NULL),
                       ('disabled-1', 'realm-a', 'disabled-1', false, NULL),
                       ('disabled-2', 'realm-a', 'disabled-2', false, NULL),
                       ('by-id', 'realm-a', 'by-id', true, NULL),
                       ('service', 'realm-a', 'service', true, 'client'),
                       ('other-realm', 'realm-b', 'other-realm', true, NULL);
                INSERT INTO user_attribute (user_id, name, value)
                VALUES ('north-1', '{AREA_ID_ATTR_NAME}', '{NORTH}'),
                       ('north-2', '{AREA_ID_ATTR_NAME}', '{NORTH}'),
                       ('north-off', '{AREA_ID_ATTR_NAME}', '{NORTH}'),
                       ('south-1', '{AREA_ID_ATTR_NAME}', '{SOUTH}'),
                       ('acclaimed-1', '{AREA_ID_ATTR_NAME}', '{ACCLAIMED}'),
                       ('acclaimed-2', '{AREA_ID_ATTR_NAME}', '{ACCLAIMED}'),
                       ('acclaimed-off', '{AREA_ID_ATTR_NAME}', '{ACCLAIMED}'),
                       ('both', '{AREA_ID_ATTR_NAME}', '{NORTH}'),
                       ('both', '{AREA_ID_ATTR_NAME}', '{ACCLAIMED}'),
                       ('this-election', '{AREA_ID_ATTR_NAME}', '{SOUTH}'),
                       ('this-election', '{AUTHORIZED_ELECTION_IDS_NAME}', 'election-b'),
                       ('this-election', '{AUTHORIZED_ELECTION_IDS_NAME}', '{ELECTION_ALIAS}'),
                       ('other-election', '{AREA_ID_ATTR_NAME}', '{SOUTH}'),
                       ('other-election', '{AUTHORIZED_ELECTION_IDS_NAME}', 'election-b'),
                       ('elsewhere', '{AREA_ID_ATTR_NAME}', '{ELSEWHERE}'),
                       ('disabled-1', '{AREA_ID_ATTR_NAME}', '{DISABLED}'),
                       ('disabled-2', '{AREA_ID_ATTR_NAME}', '{DISABLED}'),
                       ('by-id', '{AREA_ID_ATTR_NAME}', '{BY_ID}'),
                       ('by-id', '{AUTHORIZED_ELECTION_IDS_NAME}', '{ELECTION_ID}'),
                       ('service', '{AREA_ID_ATTR_NAME}', '{NORTH}'),
                       ('other-realm', '{AREA_ID_ATTR_NAME}', '{NORTH}');
                "#
            ))
            .await?;

        let areas = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        let count = |of: Vec<String>, except: Vec<String>| {
            let transaction = &transaction;
            async move {
                count_keycloak_enabled_voters_of_areas(
                    transaction,
                    "event-a",
                    &of,
                    &except,
                    ELECTION_ALIAS,
                )
                .await
            }
        };

        // north-1, north-2, both, south-1, this-election, acclaimed-1 and
        // acclaimed-2: `both` once although it is in two of the areas, and
        // nobody disabled, authorized elsewhere, a service account or from
        // another realm.
        assert_eq!(count(areas(&[NORTH, SOUTH, ACCLAIMED]), vec![]).await?, 7);
        // Each area on its own, as the per-area census reads them.
        assert_eq!(count(areas(&[NORTH]), vec![]).await?, 3);
        assert_eq!(count(areas(&[SOUTH]), vec![]).await?, 2);
        assert_eq!(count(areas(&[ACCLAIMED]), vec![]).await?, 3);
        // The acclaimed area beside two areas counted elsewhere: `both` is
        // already one of theirs.
        assert_eq!(count(areas(&[ACCLAIMED]), areas(&[NORTH, SOUTH])).await?, 2);
        assert_eq!(count(areas(&[DISABLED]), vec![]).await?, 0);
        assert_eq!(
            count(areas(&[NORTH, SOUTH, ACCLAIMED, DISABLED]), vec![]).await?,
            7
        );
        assert_eq!(count(vec![], vec![]).await?, 0);
        // Named by its id, the election has the voter authorized by its id;
        // named by an empty alias, it does not.
        let by_id = |alias: &'static str| {
            let transaction = &transaction;
            async move {
                count_keycloak_enabled_voters_of_areas(
                    transaction,
                    "event-a",
                    &[BY_ID.to_string()],
                    &[],
                    alias,
                )
                .await
            }
        };
        assert_eq!(by_id(ELECTION_ID).await?, 1);
        assert_eq!(by_id("").await?, 0);
        transaction.rollback().await?;
        Ok(())
    }
}
