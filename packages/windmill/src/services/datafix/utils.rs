// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::types::*;
use crate::postgres::area::get_event_areas;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::election_event::update_election_event_annotations;
use crate::postgres::election_event::{get_all_tenant_election_events, ElectionEventDatafix};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::electoral_log::ElectoralLog;
use crate::services::protocol_manager::get_event_board;
use crate::services::users::get_users_by_username;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::{ExtApiName, ExtApiRequestDirection};
use sequent_core::ballot::Annotations;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::UserArea;
use sequent_core::types::keycloak::{
    ATTR_RESET_VALUE, VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE,
};
use std::collections::HashMap;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;

pub const DATAFIX_ID_KEY: &str = "datafix:id";
pub const DATAFIX_PSW_POLICY_KEY: &str = "datafix:password_policy";
pub const DATAFIX_VOTERVIEW_REQ_KEY: &str = "datafix:voterview_request";
/// Last `Sequence` (from a reconciliation file's `#META` line) actually applied
/// for this event — the monotonic gate against importing a stale file. Kept
/// per-provider under its own annotation key (like the three above), not a
/// dedicated `election_event` column, since a future non-Datafix voter
/// registry integration would need its own independent sequence, not share
/// this one.
pub const DATAFIX_LAST_APPLIED_SEQUENCE_KEY: &str = "datafix:last_applied_sequence";
/// Whether the most recent reconciliation apply had per-row failures. A true
/// value permits retrying that same Sequence; a successful apply clears it.
pub const DATAFIX_LAST_APPLY_HAD_FAILURES_KEY: &str = "datafix:last_apply_had_failures";
/// Lifetime of the per-voter Datafix advisory lock. Must exceed the slowest
/// VoterView round-trip so the lock outlives an in-flight SOAP call.
pub const DATAFIX_VOTER_LOCK_SECS: i64 = 300;

/// Advisory-lock key that serializes all Datafix work for one
/// voter within an event — outbound `SetVoted`, disable-release, inbound
/// mark/unmark, and reconciliation apply all take this same lock, so none of
/// them can interleave for the same voter.
#[instrument]
pub fn datafix_voter_lock_key(tenant_id: &str, election_event_id: &str, voter_id: &Uuid) -> String {
    format!("datafix-voter-{tenant_id}-{election_event_id}-{voter_id}")
}

/// Returns true if the voter has voted via Sequent´s system -
/// this is if VOTED_CHANNEL attribute is set to VOTED_CHANNEL_INTERNET_VALUE.
#[instrument(skip_all)]
pub fn voted_via_internet(attributes: &HashMap<String, Vec<String>>) -> bool {
    match attributes.iter().find(|tupple| tupple.0.eq(VOTED_CHANNEL)) {
        Some((_, v)) => {
            matches!(v.last(), Some(channel) if channel.eq_ignore_ascii_case(VOTED_CHANNEL_INTERNET_VALUE))
        }
        None => false,
    }
}

/// Returns true if the voter has voted via a secondary channel, PAPER, PHONE, ETC -
/// this is if VOTED_CHANNEL attribute is set to anything else than Internet.
#[instrument(skip_all)]
pub fn voted_via_not_internet_channel(attributes: &HashMap<String, Vec<String>>) -> bool {
    match attributes.iter().find(|tupple| tupple.0.eq(VOTED_CHANNEL)) {
        Some((_, v)) => {
            matches!(v.last(), Some(channel) if !channel.eq_ignore_ascii_case(ATTR_RESET_VALUE) && !channel.eq_ignore_ascii_case(VOTED_CHANNEL_INTERNET_VALUE) && !channel.is_empty())
        }
        None => false,
    }
}
/// Gets the election_event_id and the DatafixAnnotations of the event that has the datafix id in its annotations.
#[instrument(skip(hasura_transaction))]
pub async fn get_event_id_and_datafix_annotations(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    requester_datafix_id: &str,
) -> Result<(String, DatafixAnnotations), JsonErrorResponse> {
    let election_events = get_all_tenant_election_events(hasura_transaction, tenant_id)
        .await
        .map_err(|err| {
            error!("Error getting election events: {err}");
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    let mut itr: std::slice::Iter<'_, ElectionEventDatafix> = election_events.iter();
    let mut next_event = itr.next(); // Use while let Some(event) = itr.next()... once the compiler gets updated.

    // Search for the datafix event id in all the annotations
    while let Some(event) = next_event {
        let datafix_id_value = event
            .0
            .annotations
            .as_ref()
            .and_then(|v| v.get(DATAFIX_ID_KEY));
        info!("datafix_id_value: {datafix_id_value:?}");
        // If there is a Datafix object, deserialize it:
        if datafix_id_value.is_some() {
            match event.get_annotations() {
                // Return Ok only in case of matching the ID of the requester:
                Ok(annotations_datafix) if requester_datafix_id.eq(&annotations_datafix.id) => {
                    return Ok((event.0.id.clone(), annotations_datafix));
                }
                Ok(annotations_datafix) => {
                    info!(
                        "Not matching id: {} found in event: {}",
                        annotations_datafix.id, event.0.id
                    );
                }
                Err(err) => {
                    error!("Error deserializing datafix annotations: {err}");
                }
            }
        }

        next_event = itr.next();
    }

    warn!("Datafix annotations not found. Requested datafix ID: {requester_datafix_id}");
    return Err(DatafixResponse::error(DatafixErrorCode::EventNotFound));
}

/// Composes the area name from the voter information, following the naming contract:
/// a concatenation of `Ward-SchoolSupportCode-Poll`. `None` (or empty) values are
/// ignored (e.g. `WARD-POLL` when there is no SchoolSupportCode,
/// `WARD-SCHOOL` when there is no Poll). All values are uppercased.
/// `pub(crate)` (rather than private) so `reconciliation::diff` can reuse the
/// exact same Ward-SchoolSupportCode-Poll composition/uppercasing rule when
/// comparing a reconciliation file row's area against a voter's resolved
/// `Area::name`.
#[instrument(skip_all)]
pub(crate) fn compose_area_name(voter_info: &VoterInformationBody) -> String {
    let mut parts = vec![voter_info.ward.clone()];

    if let Some(schoolboard) = &voter_info.schoolboard {
        if !schoolboard.is_empty() {
            parts.push(schoolboard.clone());
        }
    }

    if let Some(poll) = &voter_info.poll {
        if !poll.is_empty() {
            parts.push(poll.clone());
        }
    }

    parts.join("-").to_uppercase()
}

/// Returns the UserArea object. If it cannot find the area id by name returns an error.
/// Area names are a concatenation of Ward-SchoolSupportCode-Poll. The contract: <br>
/// If any of the values is empty or None, it is omitted. <br>
/// i.e. Ward-Poll (no SchoolSupportCode), Ward-SchoolSupportCode (no Poll) <br>
/// All values are set to uppercase
#[instrument(skip_all)]
pub async fn find_user_area_by_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    voter_info: &VoterInformationBody,
) -> Result<UserArea, JsonErrorResponse> {
    // Compose the full area name from the voter information
    let area_concat = compose_area_name(voter_info);
    let event_areas = get_event_areas(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|e| {
            error!("Error getting event areas: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    // Find the id that matches the full name.
    let area_id = event_areas
        .iter()
        .find(|area| {
            if let Some(name) = &area.name {
                name.eq(&area_concat)
            } else {
                false
            }
        })
        .map(|area| area.id.clone());

    match area_id {
        Some(id) => Ok(UserArea {
            id: Some(id),
            name: Some(area_concat),
        }),
        None => {
            error!("Error. Area not found for {}", area_concat);
            Err(DatafixResponse::error(DatafixErrorCode::AreaNotFound))
        }
    }
}

/// Get user id by username
#[instrument(skip(keycloak_transaction))]
pub async fn get_user_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    username: &str,
) -> Result<String, JsonErrorResponse> {
    let user_ids = get_users_by_username(keycloak_transaction, realm, username)
        .await
        .map_err(|e| {
            error!("Error getting users by username: {e:?}");
            DatafixResponse::error(DatafixErrorCode::InternalError)
        })?;

    match user_ids.len() {
        0 => {
            error!("Error getting users by username: Not Found");
            return Err(DatafixResponse::error(DatafixErrorCode::VoterNotFound));
        }
        1 => Ok(user_ids[0].clone()),
        _ => {
            error!("Error getting users by username: Multiple users Found");
            return Err(DatafixResponse::error(DatafixErrorCode::InternalError));
        }
    }
}

/// Get the ElectionEvent and check if its a datafix election event (has datafix:id annotations).
#[instrument(skip(hasura_transaction), err)]
pub async fn is_datafix_election_event_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<bool> {
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;

    Ok(datafix_annotations(&election_event)?.is_some())
}

/// Check if its a datafix election event (has datafix:id annotations).
#[instrument(skip(election_event))]
pub fn is_datafix_election_event(election_event: &ElectionEvent) -> bool {
    datafix_annotations(election_event).ok().flatten().is_some()
}

/// Returns `None` for an ordinary event and validates the full Datafix
/// configuration whenever the event contains the Datafix marker.
#[instrument(skip(election_event), err)]
pub fn datafix_annotations(election_event: &ElectionEvent) -> Result<Option<DatafixAnnotations>> {
    let is_configured = election_event
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(DATAFIX_ID_KEY))
        .is_some();

    if !is_configured {
        return Ok(None);
    }

    ElectionEventDatafix(election_event.clone())
        .get_annotations()
        .map(Some)
        .map_err(|err| anyhow!("Invalid Datafix election event configuration: {err}"))
}

/// Stores the event's last applied Sequence and whether that apply had row
/// failures. Same-Sequence writes update the retry flag; moving backwards is
/// rejected. Read-modify-write of the whole
/// `annotations` blob, matching every other annotation writer in this codebase (e.g.
/// `update_election_event_sbei_users`) rather than a raw `jsonb_set`
/// compare-and-swap — reconciliation apply is an infrequent, deliberate admin
/// action, not a hot concurrent path.
#[instrument(skip(hasura_transaction), err)]
pub async fn set_datafix_reconciliation_state(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    sequence: i64,
    had_failures: bool,
) -> Result<()> {
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    let annotations_value = election_event
        .annotations
        .clone()
        .ok_or_else(|| anyhow!("Missing election event annotations"))?;
    let mut annotations: Annotations = deserialize_value(annotations_value)?;

    let current: Option<i64> = annotations
        .get(DATAFIX_LAST_APPLIED_SEQUENCE_KEY)
        .and_then(|value| value.parse().ok());
    if current.is_some_and(|current| current > sequence) {
        return Err(anyhow!(
            "Cannot move Datafix reconciliation state backwards from Sequence {} to {sequence}",
            current.unwrap_or_default()
        ));
    }

    annotations.insert(
        DATAFIX_LAST_APPLIED_SEQUENCE_KEY.to_string(),
        sequence.to_string(),
    );
    annotations.insert(
        DATAFIX_LAST_APPLY_HAD_FAILURES_KEY.to_string(),
        had_failures.to_string(),
    );
    let annotations_value = serde_json::to_value(&annotations)?;
    update_election_event_annotations(
        hasura_transaction,
        tenant_id,
        election_event_id,
        annotations_value,
    )
    .await
}

#[instrument(skip_all, fields(direction = %direction), err)]
pub async fn post_operation_result_to_electoral_log(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    user_id: Option<&str>,
    username: &str,
    direction: ExtApiRequestDirection,
    operation: String,
) -> Result<()> {
    let slug = std::env::var("ENV_SLUG").map_err(|err| anyhow!("Missing ENV_SLUG: {err}"))?;
    let board_name = get_event_board(tenant_id, election_event_id, &slug);
    let electoral_log = ElectoralLog::new(
        hasura_transaction,
        tenant_id,
        Some(election_event_id),
        &board_name,
    )
    .await?;

    electoral_log
        .post_external_api_request(
            tenant_id.to_string(),
            election_event_id.to_string(),
            None,
            user_id.map(str::to_string),
            Some(username.to_string()),
            direction,
            ExtApiName::Datafix,
            operation,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voter_info(
        ward: &str,
        schoolboard: Option<&str>,
        poll: Option<&str>,
    ) -> VoterInformationBody {
        VoterInformationBody {
            voter_id: "voter-1".to_string(),
            ward: ward.to_string(),
            schoolboard: schoolboard.map(str::to_string),
            poll: poll.map(str::to_string),
            birthdate: None,
            enabled: None,
        }
    }

    #[test]
    fn composes_all_parts_when_present() {
        let info = voter_info("ward", Some("school"), Some("poll"));
        assert_eq!(compose_area_name(&info), "WARD-SCHOOL-POLL");
    }

    #[test]
    fn renders_missing_poll_omitted() {
        let info = voter_info("ward", Some("school"), None);
        assert_eq!(compose_area_name(&info), "WARD-SCHOOL");
    }

    #[test]
    fn renders_both_optionals_missing_omitted() {
        let info = voter_info("ward", None, None);
        assert_eq!(compose_area_name(&info), "WARD");
    }

    #[test]
    fn treats_empty_string_the_same_as_none() {
        let info = voter_info("ward", Some(""), Some("poll"));
        assert_eq!(compose_area_name(&info), "WARD-POLL");
    }

    #[test]
    fn uppercases_all_values() {
        let info = voter_info("ward", Some("school"), Some("poll"));
        assert_eq!(compose_area_name(&info), "WARD-SCHOOL-POLL");
        let mixed = voter_info("Ward-A", Some("Sb_2"), Some("p3"));
        assert_eq!(compose_area_name(&mixed), "WARD-A-SB_2-P3");
    }
}
