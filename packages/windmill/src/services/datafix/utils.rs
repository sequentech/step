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
use sequent_core::types::hasura::core::{Area, ElectionEvent};
use sequent_core::types::keycloak::{
    ATTR_RESET_VALUE, VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE,
};
use std::collections::HashMap;
use tracing::{error, instrument, warn};
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
) -> Result<(String, DatafixAnnotations), DatafixError> {
    let election_events = get_all_tenant_election_events(hasura_transaction, tenant_id)
        .await
        .map_err(|err| {
            error!("Error getting election events: {err:?}");
            DatafixError::internal(format!("Error getting election events: {err}"))
        })?;

    let mut matching_events = find_events_by_datafix_id(&election_events, requester_datafix_id);

    // A Datafix id shared by several events is a configuration mistake that
    // makes the request impossible to attribute, so it is rejected instead of
    // silently served by whichever event happens to be found first.
    if matching_events.len() > 1 {
        let event_ids: Vec<String> = matching_events
            .into_iter()
            .map(|(event_id, _)| event_id)
            .collect();
        let error = ambiguous_datafix_id_error(requester_datafix_id, &event_ids);
        error!("{}", error.detail);
        log_ambiguous_datafix_id(hasura_transaction, tenant_id, &event_ids, &error.detail).await;

        return Err(error);
    }

    matching_events.pop().ok_or_else(|| {
        warn!("Datafix annotations not found. Requested datafix ID: {requester_datafix_id}");
        DatafixError::new(
            DatafixErrorCode::EventNotFound,
            format!("Datafix event not found for datafix id {requester_datafix_id}"),
        )
    })
}

/// Every event whose `datafix:id` annotation matches the requester's, so the
/// caller can tell the single configured event from an ambiguous one. Events
/// without the Datafix marker, and those whose Datafix configuration does not
/// deserialize, are skipped.
#[instrument(skip(election_events))]
fn find_events_by_datafix_id(
    election_events: &[ElectionEventDatafix],
    requester_datafix_id: &str,
) -> Vec<(String, DatafixAnnotations)> {
    election_events
        .iter()
        .filter(|event| {
            event
                .0
                .annotations
                .as_ref()
                .and_then(|annotations| annotations.get(DATAFIX_ID_KEY))
                .is_some()
        })
        .filter_map(|event| match event.get_annotations() {
            Ok(annotations) => Some((event.0.id.clone(), annotations)),
            Err(err) => {
                error!(
                    "Error deserializing datafix annotations of event {}: {err}",
                    event.0.id
                );
                None
            }
        })
        .filter(|(_, annotations)| requester_datafix_id.eq(&annotations.id))
        .collect()
}

/// The failure answered when several events are configured with the same
/// Datafix id: an internal error naming every event holding it, since the
/// misconfiguration is on the Sequent side and only an administrator can fix
/// it.
#[instrument]
fn ambiguous_datafix_id_error(requester_datafix_id: &str, event_ids: &[String]) -> DatafixError {
    DatafixError::internal(format!(
        "Datafix id {requester_datafix_id} is configured in {} election events ({}), so the target event is ambiguous",
        event_ids.len(),
        event_ids.join(", ")
    ))
}

/// Records the ambiguous Datafix id in the electoral log of every event holding
/// it — the request belongs to none of them in particular, so each one logs the
/// misconfiguration. Logging failures are swallowed so auditing never turns
/// into a second failure.
#[instrument(skip(hasura_transaction))]
async fn log_ambiguous_datafix_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    event_ids: &[String],
    detail: &str,
) {
    for event_id in event_ids {
        if let Err(err) = post_operation_result_to_electoral_log(
            hasura_transaction,
            tenant_id,
            event_id,
            None,
            None,
            None,
            ExtApiRequestDirection::Inbound,
            detail.to_string(),
        )
        .await
        {
            error!("Unable to record the ambiguous Datafix id in the electoral log of event {event_id}: {err}");
        }
    }
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

/// An event area resolved from the ward, school support code and poll of a
/// Datafix request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedArea {
    pub id: String,
    pub name: String,
}

/// Returns the area matching the request. If it cannot find the area id by name returns an error.
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
) -> Result<ResolvedArea, DatafixError> {
    let event_areas = event_areas(hasura_transaction, tenant_id, election_event_id).await?;

    resolve_area_by_name(&event_areas, &compose_area_name(voter_info))
}

/// Resolves the request's area and, from the same areas fetch, the name of the
/// area the voter is leaving — Keycloak stores only the area id, so naming the
/// previous area in the electoral log needs the event's areas anyway. The
/// previous name is `None` when the voter had no area, or when its area is no
/// longer one of the event's (the entry still records the raw previous id).
#[instrument(skip_all)]
pub async fn find_user_area_and_previous_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    voter_info: &VoterInformationBody,
    previous_area_id: Option<&str>,
) -> Result<(ResolvedArea, Option<String>), DatafixError> {
    let event_areas = event_areas(hasura_transaction, tenant_id, election_event_id).await?;
    let resolved_area = resolve_area_by_name(&event_areas, &compose_area_name(voter_info))?;
    let previous_area_name =
        previous_area_id.and_then(|area_id| area_name_by_id(&event_areas, area_id));

    Ok((resolved_area, previous_area_name))
}

#[instrument(skip(hasura_transaction))]
async fn event_areas(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<Area>, DatafixError> {
    get_event_areas(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|e| {
            error!("Error getting event areas: {e:?}");
            DatafixError::internal(format!("Error getting event areas: {e}"))
        })
}

/// Finds the area whose name matches the composed `Ward-SchoolSupportCode-Poll`.
#[instrument(skip(event_areas))]
fn resolve_area_by_name(
    event_areas: &[Area],
    area_concat: &str,
) -> Result<ResolvedArea, DatafixError> {
    event_areas
        .iter()
        .find(|area| {
            area.name
                .as_ref()
                .is_some_and(|name| name.as_str() == area_concat)
        })
        .map(|area| ResolvedArea {
            id: area.id.clone(),
            name: area_concat.to_string(),
        })
        .ok_or_else(|| {
            error!("Error. Area not found for {area_concat}");
            DatafixError::new(
                DatafixErrorCode::AreaNotFound,
                format!("Area not found for {area_concat}"),
            )
        })
}

/// The name of the event area with this id, `None` when the event has no such
/// area or the area has no name.
#[instrument(skip(event_areas))]
fn area_name_by_id(event_areas: &[Area], area_id: &str) -> Option<String> {
    event_areas
        .iter()
        .find(|area| area.id.eq(area_id))
        .and_then(|area| area.name.clone())
}

/// Get user id by username
#[instrument(skip(keycloak_transaction))]
pub async fn get_user_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    username: &str,
) -> Result<String, DatafixError> {
    let user_ids = get_users_by_username(keycloak_transaction, realm, username)
        .await
        .map_err(|e| {
            error!("Error getting users by username: {e:?}");
            DatafixError::internal(format!("Error getting users by username: {e}"))
        })?;

    match user_ids.len() {
        0 => {
            error!("Error getting users by username: Not Found");
            Err(DatafixError::new(
                DatafixErrorCode::VoterNotFound,
                "Voter not found",
            ))
        }
        1 => Ok(user_ids[0].clone()),
        _ => {
            error!("Error getting users by username: Multiple users Found");
            Err(DatafixError::internal(
                "Multiple users found for the username",
            ))
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
    username: Option<&str>,
    area_id: Option<&str>,
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
            username.map(str::to_string),
            area_id.map(str::to_string),
            direction,
            ExtApiName::Datafix,
            operation,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn datafix_event(
        event_id: &str,
        annotations: Option<serde_json::Value>,
    ) -> ElectionEventDatafix {
        ElectionEventDatafix(ElectionEvent {
            id: event_id.to_string(),
            created_at: None,
            updated_at: None,
            labels: None,
            annotations,
            tenant_id: "tenant".to_string(),
            description: None,
            presentation: None,
            bulletin_board_reference: None,
            is_archived: false,
            voting_channels: None,
            status: None,
            user_boards: None,
            encryption_protocol: "protocol".to_string(),
            is_audit: None,
            audit_election_event_id: None,
            public_key: None,
            statistics: None,
            external_id: None,
        })
    }

    fn datafix_annotations(datafix_id: &str) -> serde_json::Value {
        json!({
            DATAFIX_ID_KEY: datafix_id,
            DATAFIX_PSW_POLICY_KEY: r#"{"base":"password-only","size":6,"characters":"numeric"}"#,
            DATAFIX_VOTERVIEW_REQ_KEY: r#"{"url":"https://example.invalid","usr":"user","psw":"secret","county_mun":"county"}"#,
        })
    }

    #[test]
    fn finds_the_event_configured_with_the_requested_datafix_id() {
        let events = vec![
            datafix_event("event-1", Some(datafix_annotations("other-datafix-id"))),
            datafix_event("event-2", Some(datafix_annotations("requested-datafix-id"))),
            datafix_event("event-3", None),
        ];

        let matches = find_events_by_datafix_id(&events, "requested-datafix-id");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "event-2");
        assert_eq!(matches[0].1.id, "requested-datafix-id");
    }

    #[test]
    fn finds_every_event_sharing_the_same_datafix_id() {
        let events = vec![
            datafix_event("event-1", Some(datafix_annotations("requested-datafix-id"))),
            datafix_event("event-2", Some(datafix_annotations("other-datafix-id"))),
            datafix_event("event-3", Some(datafix_annotations("requested-datafix-id"))),
        ];

        let event_ids: Vec<String> = find_events_by_datafix_id(&events, "requested-datafix-id")
            .into_iter()
            .map(|(event_id, _)| event_id)
            .collect();
        assert_eq!(event_ids, vec!["event-1", "event-3"]);
    }

    #[test]
    fn finds_no_event_when_none_carries_the_requested_datafix_id() {
        let events = vec![
            datafix_event("event-1", Some(datafix_annotations("other-datafix-id"))),
            datafix_event("event-2", None),
        ];

        assert!(find_events_by_datafix_id(&events, "requested-datafix-id").is_empty());
    }

    #[test]
    fn skips_events_whose_datafix_configuration_is_invalid() {
        let events = vec![
            datafix_event(
                "event-1",
                Some(json!({DATAFIX_ID_KEY: "requested-datafix-id"})),
            ),
            datafix_event("event-2", Some(datafix_annotations("requested-datafix-id"))),
        ];

        let matches = find_events_by_datafix_id(&events, "requested-datafix-id");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, "event-2");
    }

    #[test]
    fn ambiguous_datafix_id_error_names_every_event_holding_it() {
        let error = ambiguous_datafix_id_error(
            "requested-datafix-id",
            &["event-1".to_string(), "event-3".to_string()],
        );

        assert_eq!(error.code, DatafixErrorCode::InternalError);
        assert!(error.detail.contains("requested-datafix-id"), "{error}");
        assert!(error.detail.contains("event-1"), "{error}");
        assert!(error.detail.contains("event-3"), "{error}");
    }

    fn area(area_id: &str, name: Option<&str>) -> Area {
        Area {
            id: area_id.to_string(),
            tenant_id: "tenant".to_string(),
            election_event_id: "event".to_string(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            name: name.map(str::to_string),
            description: None,
            r#type: None,
            parent_id: None,
            presentation: None,
        }
    }

    #[test]
    fn resolves_the_area_whose_name_matches_the_composed_one() {
        let areas = vec![
            area("area-1", Some("WARD-1")),
            area("area-2", None),
            area("area-3", Some("WARD-2-POLL-5")),
        ];

        let resolved = resolve_area_by_name(&areas, "WARD-2-POLL-5").expect("area not found");
        assert_eq!(resolved.id, "area-3");
        assert_eq!(resolved.name, "WARD-2-POLL-5");
    }

    #[test]
    fn rejects_a_composed_name_no_area_carries() {
        let areas = vec![area("area-1", Some("WARD-1"))];

        let error = resolve_area_by_name(&areas, "WARD-2").expect_err("unknown area accepted");
        assert_eq!(error.code, DatafixErrorCode::AreaNotFound);
    }

    #[test]
    fn names_the_previous_area_by_its_id() {
        let areas = vec![area("area-1", Some("WARD-1")), area("area-2", None)];

        assert_eq!(
            area_name_by_id(&areas, "area-1"),
            Some("WARD-1".to_string())
        );
        assert_eq!(area_name_by_id(&areas, "area-2"), None);
        assert_eq!(area_name_by_id(&areas, "area-unknown"), None);
    }

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
