// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::types::*;
use crate::postgres::area::get_event_areas;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::election_event::{get_all_tenant_election_events, ElectionEventDatafix};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::users::get_users_by_username;
use anyhow::Result;
use deadpool_postgres::Transaction;
use rocket::http::Status;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::UserArea;
use sequent_core::types::keycloak::{
    ATTR_RESET_VALUE, VOTED_CHANNEL, VOTED_CHANNEL_INTERNET_VALUE,
};
use std::collections::HashMap;
use tracing::{error, info, instrument, warn};

pub const DATAFIX_ID_KEY: &str = "datafix:id";
pub const DATAFIX_PSW_POLICY_KEY: &str = "datafix:password_policy";
pub const DATAFIX_VOTERVIEW_REQ_KEY: &str = "datafix:voterview_request";

/// Returns true if the voter has voted via Sequent´s system -
/// this is if VOTED_CHANNEL attribute is set to VOTED_CHANNEL_INTERNET_VALUE.
#[instrument()]
pub fn voted_via_internet(attributes: &HashMap<String, Vec<String>>) -> bool {
    match attributes.iter().find(|tupple| tupple.0.eq(VOTED_CHANNEL)) {
        Some((_, v)) => {
            matches!(v.last(), Some(channel) if channel.eq(VOTED_CHANNEL_INTERNET_VALUE))
        }
        None => false,
    }
}

/// Returns true if the voter has voted via a secondary channel, PAPER, PHONE, ETC -
/// this is if VOTED_CHANNEL attribute is set to anything else than Internet.
#[instrument()]
pub fn voted_via_not_internet_channel(attributes: &HashMap<String, Vec<String>>) -> bool {
    match attributes.iter().find(|tupple| tupple.0.eq(VOTED_CHANNEL)) {
        Some((_, v)) => {
            matches!(v.last(), Some(channel) if channel != ATTR_RESET_VALUE && channel != VOTED_CHANNEL_INTERNET_VALUE && !channel.is_empty())
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
            DatafixResponse::new(Status::InternalServerError)
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
    return Err(DatafixResponse::new(Status::NotFound));
}

/// Returns the UserArea object. If it cannot find the area id by name returns an error.
#[instrument(skip_all)]
pub async fn find_user_area_by_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    voter_info: &VoterInformationBody,
) -> Result<UserArea, JsonErrorResponse> {
    // Compose the full area name from the voter information
    let mut area_concat: String = voter_info.ward.clone();
    let area_childs = [voter_info.schoolboard.clone(), voter_info.poll.clone()];
    for subarea in area_childs.iter().flatten() {
        area_concat.push_str(format!("-{subarea}").as_str());
    }
    // Get the areas for this election_event_id
    let event_areas = get_event_areas(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|e| {
            error!("Error getting event areas: {e:?}");
            DatafixResponse::new(Status::InternalServerError)
        })?;

    area_concat = area_concat.to_uppercase();
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
            Err(DatafixResponse::new(Status::NotFound))
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
            DatafixResponse::new(Status::InternalServerError)
        })?;

    match user_ids.len() {
        0 => {
            error!("Error getting users by username: Not Found");
            return Err(DatafixResponse::new(Status::NotFound));
        }
        1 => Ok(user_ids[0].clone()),
        _ => {
            error!("Error getting users by username: Multiple users Found");
            return Err(DatafixResponse::new(Status::NotFound));
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

    Ok(is_datafix_election_event(&election_event))
}

/// Check if its a datafix election event (has datafix:id annotations).
#[instrument(skip(election_event))]
pub fn is_datafix_election_event(election_event: &ElectionEvent) -> bool {
    let datafix_event = ElectionEventDatafix(election_event.clone());
    datafix_event.get_annotations().ok().is_some()
}
