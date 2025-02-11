// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::get_event_areas;
use crate::postgres::election_event::{get_all_tenant_election_events, ElectionEventDatafix};
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::users::{get_users_by_username, lookup_users, FilterOption, ListUsersFilter};
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use rand::{distributions::Uniform, Rng};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::Annotations;
use sequent_core::serialization::deserialize_with_path::{deserialize_str, deserialize_value};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::{
    User, UserArea, AREA_ID_ATTR_NAME, ATTR_RESET_VALUE, DATE_OF_BIRTH, DISABLE_COMMENT,
    DISABLE_REASON_DELETE_CALL, DISABLE_REASON_MARKVOTED_CALL, VOTED_CHANNEL,
};
use sequent_core::util::date_time::verify_date_format_ymd;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum_macros::{Display, EnumString};
use tracing::{error, info, instrument, warn};

#[derive(Deserialize, Debug)]
pub struct VoterInformationBody {
    voter_id: String,
    ward: String,
    schoolboard: Option<String>,
    poll: Option<String>,
    birthdate: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MarkVotedBody {
    voter_id: String,
    channel: String,
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
pub struct DatafixAnnotations {
    id: String,
    password_policy: PasswordPolicy,
}

#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, EnumString)]
pub enum BasePolicy {
    #[strum(serialize = "id-password-concatenated")]
    #[serde(rename = "id-password-concatenated")]
    IdPswConcat,
    #[default]
    #[strum(serialize = "password-only")]
    #[serde(rename = "password-only")]
    PswOnly,
}

#[derive(Default, Display, Serialize, Deserialize, Debug, Clone, EnumString)]
pub enum CharactersPolicy {
    #[strum(serialize = "numeric")]
    #[serde(rename = "numeric")]
    Numeric,
    #[default]
    #[strum(serialize = "alphanumeric")]
    #[serde(rename = "alphanumeric")]
    Alphanumeric,
}

#[derive(Deserialize, Serialize, Debug)]
struct PasswordPolicy {
    base: BasePolicy,
    size: usize,
    characters: CharactersPolicy,
}

impl ValidateAnnotations for ElectionEventDatafix {
    type Item = DatafixAnnotations;

    fn get_annotations(&self) -> Result<Self::Item> {
        let annotations_value = self
            .0
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing election event annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_value)?;
        let id = match annotations.get("datafix:id") {
            Some(id) => id.clone(),
            None => return Err(anyhow!("datafix:id not found")),
        };

        let password_policy: PasswordPolicy = match annotations.get("datafix:password_policy") {
            Some(value_as_str) => deserialize_str(value_as_str)?,
            None => return Err(anyhow!("datafix:id not found")),
        };

        Ok(DatafixAnnotations {
            id,
            password_policy,
        })
    }
}

/// Gets the election_event_id and the DatafixAnnotations of the event that has the datafix id in its annotations.
#[instrument(skip(hasura_transaction))]
async fn get_event_id_and_datafix_annotations(
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

    let mut itr: std::slice::Iter<'_, ElectionEventDatafix> = election_events.iter();
    let mut next_event = itr.next(); // Use while let Some(event) = itr.next()... once the compiler gets updated.

    // Search for the datafix event id in all the annotations
    while next_event.is_some() {
        let event = next_event.unwrap();
        let datafix_id_value = event
            .0
            .annotations
            .as_ref()
            .and_then(|v| v.get("datafix:id"));
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
    return Err(DatafixResponse::new(Status::BadRequest));
}

impl PasswordPolicy {
    #[instrument]
    fn generate_password(self, voter_id: &str) -> String {
        let pin = match self.characters {
            CharactersPolicy::Numeric => {
                let mut pass = String::new();
                let mut rng = rand::thread_rng();
                for _ in 0..self.size {
                    pass.push_str(rng.gen_range(0..10).to_string().as_str());
                }
                pass
            }
            CharactersPolicy::Alphanumeric => rand::thread_rng()
                .sample_iter(Uniform::new(char::from(32), char::from(126))) // In the range of the ascii characters
                .take(self.size)
                .map(char::from)
                .collect(),
        };
        match self.base {
            BasePolicy::IdPswConcat => format!("{}{}", voter_id, pin),
            BasePolicy::PswOnly => pin,
        }
    }
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
    let area_childs = vec![voter_info.schoolboard.clone(), voter_info.poll.clone()];
    for subarea in &area_childs {
        if let Some(subarea) = subarea {
            area_concat.push_str(format!(" - {subarea}").as_str());
        }
    }
    // Get the areas for this election_event_id
    let event_areas = get_event_areas(hasura_transaction, tenant_id, election_event_id)
        .await
        .map_err(|e| {
            error!("Error getting event areas: {e:?}");
            DatafixResponse::new(Status::InternalServerError)
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
        .and_then(|area| Some(area.id.clone()));

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
    let user_ids = get_users_by_username(keycloak_transaction, &realm, username)
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
        id: None,
        attributes: attributes.clone(),
        email: None,
        email_verified: None,
        enabled: Some(true),
        first_name: None,
        last_name: None,
        username: Some(username.to_string()),
        area: Some(area),
        votes_info: None,
    };
    let _user = client
        .create_user(&realm, &user, attributes, None)
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
            None,
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
            error!("Error creating user: {e:?}");
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
            error!("Error creating user: {e:?}");
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
            error!("Error creating user: {e:?}");
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
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        error!("Error getting KeycloakAdminClient: {e:?}");
        DatafixResponse::new(Status::InternalServerError)
    })?;

    let filter = ListUsersFilter {
        tenant_id: tenant_id.to_string(),
        election_event_id: Some(election_event_id),
        realm: realm.clone(),
        username: Some(FilterOption::IsEqual(username.to_string())),
        ..ListUsersFilter::default()
    };

    // If a voter is disabled, do not generate a PIN
    let user_id = match lookup_users(hasura_transaction, keycloak_transaction, filter).await {
        Ok(users) if users.len() == 1 => {
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

    let _user = client
        .edit_user(
            &realm, &user_id, None, // Enable/disable
            None, // attributes
            None, None, None, None, password, None,
        )
        .await
        .map_err(|e| {
            error!("Error creating user: {e:?}");
            DatafixResponse::new(Status::InternalServerError)
        })?;

    Ok(pin)
}
