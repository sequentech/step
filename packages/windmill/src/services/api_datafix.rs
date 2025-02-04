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

/// Gets the election_event_id of the event that has the datafix id in its annotations.
#[instrument(skip(hasura_transaction))]
async fn get_datafix_election_event_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
) -> Result<String, JsonErrorResponse> {
    let election_events = get_all_tenant_election_events(hasura_transaction, tenant_id)
        .await
        .map_err(|err| {
            error!("Error getting election events: {err}");
            DatafixResponse::new(Status::BadRequest)
        })?;

    info!("election_events: {:?}", election_events);
    let mut election_event_id_match = None;
    let mut itr: std::slice::Iter<'_, ElectionEvent> = election_events.iter();
    let mut event_opt = itr.next(); // Use while let Some(event) = itr.next()... once the compiler gets updated.
    while event_opt.is_some() && election_event_id_match.is_none() {
        let event = event_opt.unwrap();
        let annotations_datafix_id = event.annotations.clone().and_then(|v| {
            v.get("DATAFIX")
                .and_then(|v| v.get("id").map(|v| v.to_string()))
        });
        election_event_id_match = match annotations_datafix_id {
            Some(annotations_datafix_id) if annotations_datafix_id.eq(datafix_event_id) => {
                Some(event.id.clone())
            }
            _ => None,
        };
        event_opt = itr.next();
    }

    match election_event_id_match {
        Some(election_event_id) => Ok(election_event_id),
        None => {
            warn!("Datafix event id: {datafix_event_id} not found");
            return Err(DatafixResponse::new(Status::BadRequest));
        }
    }
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
    let election_event_id =
        get_datafix_election_event_id(hasura_transaction, tenant_id, datafix_event_id).await?;

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
    let election_event_id =
        get_datafix_election_event_id(hasura_transaction, tenant_id, datafix_event_id).await?;

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
