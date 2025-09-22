// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::types::*;
use super::utils::*;

use crate::services::users::{list_users, FilterOption, ListUsersFilter};
use anyhow::Result;
use deadpool_postgres::Transaction;

use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::keycloak::{
    User, AREA_ID_ATTR_NAME, ATTR_RESET_VALUE, DATE_OF_BIRTH, DISABLE_COMMENT,
    DISABLE_REASON_DELETE_CALL, DISABLE_REASON_MARKVOTED_CALL, TENANT_ID_ATTR_NAME, VOTED_CHANNEL,
};
use sequent_core::util::date_time::verify_date_format_ymd;
use std::collections::HashMap;
use std::env;
use tracing::{error, info, instrument, warn};
/// Disable the voter, datafix users are not actually deleted but just disabled.
/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction, keycloak_transaction))]
pub async fn disable_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    username: &str,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let (election_event_id, _) =
        get_event_id_and_datafix_annotations(hasura_transaction, tenant_id, datafix_event_id)
            .await?;
    info!("election_event_id: {election_event_id}");
    let realm = get_event_realm(tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let user_id = get_user_id(keycloak_transaction, &realm, username).await?;
    let mut hash_map = HashMap::new();
    hash_map.insert(
        DISABLE_COMMENT.to_string(),
        vec![DISABLE_REASON_DELETE_CALL.to_string()],
    );
    let attributes = Some(hash_map);

    let _user = client
        .edit_user(
            &realm,
            &user_id,
            Some(false),
            attributes,
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
        get_event_id_and_datafix_annotations(hasura_transaction, tenant_id, datafix_event_id)
            .await?;

    let realm = get_event_realm(tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let area = find_user_area_by_name(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        voter_info,
    )
    .await?;

    // Both area and birthdate have to go into the attributes HashMap. They will be taken from there but not from the User struct.
    let mut hash_map = HashMap::new();
    hash_map.insert(
        AREA_ID_ATTR_NAME.to_string(),
        vec![area.id.clone().unwrap_or_default()],
    );
    hash_map.insert(TENANT_ID_ATTR_NAME.to_string(), vec![tenant_id.to_string()]);
    // Area is required in the input body but the birthdate is not.
    if let Some(birthdate) = voter_info.birthdate.clone() {
        verify_date_format_ymd(&birthdate).map_err(|e| {
            error!("Birthdate format is not correct: {e:?}");
            DatafixResponse::new(Status::BadRequest)
        })?;
        hash_map.insert(DATE_OF_BIRTH.to_string(), vec![birthdate]);
    }
    let attributes = Some(hash_map);
    let user = User {
        attributes: attributes.clone(),
        enabled: Some(true),
        username: Some(username.to_string()),
        area: Some(area),
        ..User::default()
    };
    let voter_group_name = env::var("KEYCLOAK_VOTER_GROUP_NAME").map_err(|e| {
        error!("Error getting env var KEYCLOAK_VOTER_GROUP_NAME: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;
    let _user = client
        .create_user(&realm, &user, attributes, Some(vec![voter_group_name]))
        .await
        .map_err(|e| {
            error!("Error creating user: {e:?}");
            DatafixResponse::new(Status::InternalServerError)
        })?;
    Ok(DatafixResponse::new(Status::Ok))
}

/// There are 2 things that can be updated, the area and the birthdate.
/// Note: voter_id in Datafix API represents the username in Keycloak/Sequent´s system.
#[instrument(skip(hasura_transaction, keycloak_transaction))]
pub async fn update_datafix_voter(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_info: &VoterInformationBody,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let username = voter_info.voter_id.clone();
    let (election_event_id, _) =
        get_event_id_and_datafix_annotations(hasura_transaction, tenant_id, datafix_event_id)
            .await?;

    let realm = get_event_realm(tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let area = find_user_area_by_name(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        voter_info,
    )
    .await?;
    // Both area and birthdate have to go into the attributes HashMap. They will be taken from there but not from the User struct.
    let mut hash_map = HashMap::new();
    hash_map.insert(
        AREA_ID_ATTR_NAME.to_string(),
        vec![area.id.unwrap_or_default()],
    );
    // Area is required in the input body but birthdate is not.
    if let Some(birthdate) = voter_info.birthdate.clone() {
        verify_date_format_ymd(&birthdate).map_err(|e| {
            error!("Birthdate format is not correct: {e:?}");
            DatafixResponse::new(Status::BadRequest)
        })?;
        hash_map.insert(DATE_OF_BIRTH.to_string(), vec![birthdate]);
    }
    let attributes = Some(hash_map);

    let user_id = get_user_id(keycloak_transaction, &realm, &username).await?;
    let _user = client
        .edit_user(
            &realm,
            &user_id,
            voter_info.enabled,
            attributes,
            None,
            None,
            None,
            Some(username),
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

/// Mark a voter as having voted via a given channel
/// Also disables the voter so it cannot vote online
#[instrument(skip(hasura_transaction, keycloak_transaction))]
pub async fn mark_as_voted_via_channel(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_body: &MarkVotedBody,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let username = voter_body.voter_id.clone();
    let (election_event_id, _) =
        get_event_id_and_datafix_annotations(hasura_transaction, tenant_id, datafix_event_id)
            .await?;

    let realm = get_event_realm(tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let mut hash_map = HashMap::new();
    hash_map.insert(VOTED_CHANNEL.to_string(), vec![voter_body.channel.clone()]);
    hash_map.insert(
        DISABLE_COMMENT.to_string(),
        vec![DISABLE_REASON_MARKVOTED_CALL.to_string()],
    );
    let attributes = Some(hash_map);

    let user_id = get_user_id(keycloak_transaction, &realm, &username).await?;
    let _user = client
        .edit_user(
            &realm,
            &user_id,
            Some(false), // Disable the voter
            attributes,
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

/// Unmark a voter as having voted, set the attribute to None
/// Also enables the voter
#[instrument(skip(hasura_transaction, keycloak_transaction))]
pub async fn unmark_voter_as_voted(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    voter_id: &str,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let username = voter_id.to_string();
    let (election_event_id, _) =
        get_event_id_and_datafix_annotations(hasura_transaction, tenant_id, datafix_event_id)
            .await?;

    let realm = get_event_realm(tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let mut hash_map = HashMap::new();
    hash_map.insert(
        VOTED_CHANNEL.to_string(),
        vec![ATTR_RESET_VALUE.to_string()],
    );
    hash_map.insert(
        DISABLE_COMMENT.to_string(),
        vec![ATTR_RESET_VALUE.to_string()],
    );
    let attributes = Some(hash_map);
    let user_id = get_user_id(keycloak_transaction, &realm, &username).await?;
    let _user = client
        .edit_user(
            &realm,
            &user_id,
            Some(true), // Enable the voter again
            attributes,
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

/// Generate a new password.
#[instrument(skip(hasura_transaction, keycloak_transaction))]
pub async fn replace_voter_pin(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tenant_id: &str,
    datafix_event_id: &str,
    username: &str,
) -> Result<String, JsonErrorResponse> {
    let (election_event_id, datafix_annotations) =
        get_event_id_and_datafix_annotations(hasura_transaction, tenant_id, datafix_event_id)
            .await?;
    info!("election_event_id: {election_event_id}");
    let realm = get_event_realm(tenant_id, &election_event_id);

    let filter = ListUsersFilter {
        tenant_id: tenant_id.to_string(),
        election_event_id: Some(election_event_id),
        realm: realm.clone(),
        username: Some(FilterOption::IsEqual(username.to_string())),
        ..ListUsersFilter::default()
    };

    // If a voter is disabled, do not generate a PIN
    let user_id = match list_users(hasura_transaction, keycloak_transaction, filter).await {
        Ok((users, 1)) => {
            let user = users
                .last()
                .map(|val_ref| val_ref.to_owned())
                .unwrap_or_default();
            if !user.enabled.unwrap_or(true) {
                warn!("Cannot replace pin because the user is disabled.");
                return Err(DatafixResponse::new(Status::BadRequest));
            }
            user.id.unwrap_or_default()
        }
        Ok(_) => {
            warn!("Error getting users by username: Must be only one user per username");
            return Err(DatafixResponse::new(Status::NotFound));
        }
        Err(e) => {
            error!("Error looking up user: {e:?}");
            return Err(DatafixResponse::new(Status::InternalServerError));
        }
    };

    let pin = datafix_annotations
        .password_policy
        .generate_password(&username);
    let password = Some(pin.clone());

    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let _user = client
        .edit_user(
            &realm, &user_id, None, // Enable/disable
            None, // attributes
            None, None, None, None, password, None,
        )
        .await
        .map_err(|e| {
            error!("Error editing user: {e:?}");
            DatafixResponse::new(Status::InternalServerError)
        })?;

    Ok(pin)
}
