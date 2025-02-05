// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id, get_elections, update_election_voting_status};
use crate::postgres::election_event::{
    get_all_tenant_election_events, get_election_event_by_id, update_election_event_status,
};
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::users::get_users_by_username;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{User, UserArea};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, warn};

#[derive(Deserialize, Debug)]
pub struct VoterInformationBody {
    voter_id: String,
    ward: String,
    schoolboard: Option<String>,
    poll: Option<String>,
    birthdate: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatafixResponse {
    pub code: u16,
    pub message: String,
}

pub type JsonErrorResponse = Json<DatafixResponse>;

impl DatafixResponse {
    #[instrument]
    pub fn new(status: Status) -> JsonErrorResponse {
        Json(DatafixResponse {
            code: status.code,
            message: status.reason().unwrap_or_default().to_string(),
        })
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct DatafixAnnotations {
    id: String,
    password_policy: PasswordPolicy,
}

#[derive(Deserialize, Serialize, Debug)]
struct PasswordPolicy {
    base: String,
    size: u8,
    characters: String,
}

/// Gets the election_event_id and the DatafixAnnotations of the event that has the datafix id in its annotations.
#[instrument(skip(hasura_transaction))]
async fn get_datafix_annotations(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    requester_datafix_id: &str,
) -> Result<(String, DatafixAnnotations), JsonErrorResponse> {
    let election_events = get_all_tenant_election_events(hasura_transaction, tenant_id)
        .await
        .map_err(|err| {
            error!("Error getting election events: {err}");
            DatafixResponse::new(Status::BadRequest)
        })?;

    let mut itr: std::slice::Iter<'_, ElectionEvent> = election_events.iter();
    let mut next_event = itr.next(); // Use while let Some(event) = itr.next()... once the compiler gets updated.

    // Search for the datafix event id in all the annotations
    while next_event.is_some() {
        let event = next_event.unwrap();
        let datafix_object = event.annotations.as_ref().and_then(|v| v.get("DATAFIX"));
        info!("datafix_object: {datafix_object:?}");
        // If there is a Datafix object, deserialize it:
        if let Some(datafix_value) = datafix_object {
            match deserialize_value::<DatafixAnnotations>(datafix_value.clone()) {
                // Return Ok only in case of matching the ID of the requester:
                Ok(annotations_datafix) if requester_datafix_id.eq(&annotations_datafix.id) => {
                    return Ok((event.id.clone(), annotations_datafix));
                }
                Ok(annotations_datafix) => {
                    info!(
                        "Not matching id: {} found in event: {}",
                        annotations_datafix.id, event.id
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
    return Err(DatafixResponse::new(Status::BadRequest));
}

/// Disable the voter, datafix users are not actually deleted but just disabled.
/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction))]
pub async fn disable_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    username: &str,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let (election_event_id, _) =
        get_datafix_annotations(hasura_transaction, tenant_id, datafix_event_id).await?;
    info!("election_event_id: {election_event_id}");
    let realm = get_event_realm(tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let user_ids = get_users_by_username(hasura_transaction, &realm, username)
        .await
        .map_err(|e| {
            error!("Error getting users by username: {e:?}");
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let user_id = match user_ids.len() {
        0 => {
            error!("Error getting users by username: Not Found");
            return Err(DatafixResponse::new(Status::NotFound));
        }
        1 => user_ids[0].clone(),
        _ => {
            error!("Error getting users by username: Multiple users Found");
            return Err(DatafixResponse::new(Status::NotFound));
        }
    };

    let _user = client
        .edit_user(
            &realm,
            &user_id,
            Some(false),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            error!("Error editing user: {e:?}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
    Ok(DatafixResponse::new(Status::Ok))
}

/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction))]
pub async fn add_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_info: &VoterInformationBody,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let username = &voter_info.voter_id;
    let (election_event_id, _) =
        get_datafix_annotations(hasura_transaction, tenant_id, datafix_event_id).await?;

    let realm = get_event_realm(tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let area = Some(UserArea {
        id: None, // TODO: Arrange the areas from the voter_info and verify them
        name: None,
    });
    let user = User {
        id: None,
        attributes: None,
        email: None,
        email_verified: None,
        enabled: None,
        first_name: None,
        last_name: None,
        username: Some(username.to_string()),
        area,
        votes_info: None,
    };
    let _user = client
        .create_user(
            // TODO: Check Which values are required to create an user?
            &realm, &user, None, None,
        )
        .await
        .map_err(|e| {
            error!("Error creating user: {e:?}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
    Ok(DatafixResponse::new(Status::Ok))
}
