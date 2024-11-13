// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::cast_votes::get_users_with_vote_info;
use crate::services::database::PgConfig;
use crate::{
    postgres::application::{insert_application, update_confirm_application},
    postgres::area::get_areas,
    types::application::ApplicationStatus,
    types::application::ApplicationType,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use futures::stream::Filter;
use keycloak::types::CredentialRepresentation;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::hasura::core::Application;
use sequent_core::types::keycloak::User;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    convert::From,
};
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

use super::users::{lookup_users, FilterOption, ListUsersFilter};

#[instrument(skip(hasura_transaction, keycloak_transaction), err)]
pub async fn verify_application(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    applicant_id: &str,
    applicant_data: &Value,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &Option<String>,
    labels: &Option<Value>,
    annotations: &Option<Value>,
) -> Result<Option<User>> {
    let realm = get_event_realm(tenant_id, election_event_id);

    // Generate a filter with applicant data
    let filter = get_filter_from_applicant_data(
        tenant_id.to_string(),
        Some(election_event_id.to_string()),
        None,
        None,
        realm,
        None,
        annotations,
        applicant_data,
    )?;

    // Uses applicant data to lookup possible users
    let (users, _count) = lookup_users(hasura_transaction, keycloak_transaction, filter).await?;

    info!("Users found: {:?}", users);

    // Finds an user from the list of found possible users
    let (user, status, applicantion_type) =
        automatic_verification(users, annotations, applicant_data)?;

    info!("Result - user: {:?} status: {:?} application_type: {:?}", user, status, applicantion_type);

    // Insert application
    insert_application(
        hasura_transaction,
        tenant_id,
        election_event_id,
        area_id,
        applicant_id,
        applicant_data,
        labels,
        annotations,
        applicantion_type,
        status,
    )
    .await?;

    Ok(user)
}

fn get_filter_from_applicant_data(
    tenant_id: String,
    election_event_id: Option<String>,
    election_id: Option<String>,
    area_id: Option<String>,
    realm: String,
    search: Option<String>,
    annotations: &Option<Value>,
    applicant_data: &Value,
) -> Result<ListUsersFilter> {
    let applicant_data_map = applicant_data
        .as_object()
        .ok_or(anyhow!("Error converting applicant_data to map"))?
        .clone();

    let annotations_map = annotations
        .clone()
        .ok_or(anyhow!("Error missing annotations"))?
        .as_object()
        .ok_or(anyhow!("Error converting annotations to map"))?
        .clone();

    let search_attributes: String = annotations_map
        .get("search-attributes")
        .and_then(|value| value.as_str().map(|value| value.to_string()))
        .map(|value| value)
        .ok_or(anyhow!(
            "Error obtaining search_attributes from annotations"
        ))?;

    let mut first_name = None;
    let mut last_name = None;
    let mut username = None;
    let mut email = None;
    let mut attributes_map = HashMap::new();

    for attribute in search_attributes.split(",") {
        match attribute {
            "firstName" => {
                first_name = applicant_data_map
                    .get("firstName")
                    .and_then(|value| value.as_str().map(|value| value.to_string()))
                    .and_then(|value| Some(FilterOption::IsLike(value.to_string())));
            }
            "lastName" => {
                last_name = applicant_data_map
                    .get("lastName")
                    .and_then(|value| value.as_str().map(|value| value.to_string()))
                    .and_then(|value| Some(FilterOption::IsLike(value.to_string())));
            }
            "username" => {
                username = applicant_data_map
                    .get("username")
                    .and_then(|value| value.as_str().map(|value| value.to_string()))
                    .and_then(|value| Some(FilterOption::IsLike(value.to_string())));
            }
            "email" => {
                email = applicant_data_map
                    .get("email")
                    .and_then(|value| value.as_str().map(|value| value.to_string()))
                    .and_then(|value| Some(FilterOption::IsLike(value.to_string())));
            }
            _ => {
                let value = applicant_data_map
                    .get(attribute)
                    .and_then(|value| value.as_str().map(|value| value.to_string()))
                    .ok_or(anyhow!("Error obtaining {attribute} from applicant data"))?;

                attributes_map.insert(attribute.to_string(), value);
            }
        }
    }

    let attributes = if attributes_map.is_empty() {
        None
    } else {
        Some(attributes_map)
    };

    Ok(ListUsersFilter {
        tenant_id,
        election_event_id,
        election_id,
        area_id,
        realm,
        search,
        first_name,
        last_name,
        username,
        email,
        limit: None,
        offset: None,
        user_ids: None,
        attributes,
        email_verified: None,
        enabled: None,
        sort: None,
        has_voted: None,
        authorized_to_election_alias: None,
    })
}

fn automatic_verification(
    users: Vec<User>,
    annotations: &Option<Value>,
    applicant_data: &Value,
) -> Result<(Option<User>, ApplicationStatus, ApplicationType)> {
    let mut matched_user: Option<User> = None;
    let mut matched_status = ApplicationStatus::REJECTED;
    let mut matched_type = ApplicationType::AUTOMATIC;

    let annotations_map = annotations
        .clone()
        .ok_or(anyhow!("Error missing annotations"))?
        .as_object()
        .ok_or(anyhow!("Error converting annotations to map"))?
        .clone();

    let search_attributes: String = annotations_map
        .get("search-attributes")
        .and_then(|value| value.as_str().map(|value| value.to_string()))
        .map(|value| value)
        .ok_or(anyhow!(
            "Error obtaining search_attributes from annotations"
        ))?;

    for user in users {
        let (mismatches, fields_match) =
            check_mismatches(&user, applicant_data, search_attributes.clone())?;

        if mismatches <= 1 {
            return Ok((
                Some(user),
                ApplicationStatus::ACCEPTED,
                ApplicationType::AUTOMATIC,
            ));
        } else if mismatches == 2 {
            if !fields_match.get("country").unwrap_or(&false) {
                matched_user = None;
                matched_status = ApplicationStatus::PENDING;
                matched_type = ApplicationType::MANUAL;
            } else if !fields_match.get("middleName").unwrap_or(&false)
                && !fields_match.get("lastName").unwrap_or(&false)
            {
                matched_user = None;
                matched_status = ApplicationStatus::PENDING;
                matched_type = ApplicationType::MANUAL;
            }
        } else if matched_status != ApplicationStatus::PENDING {
            matched_user = None;
            matched_status = ApplicationStatus::REJECTED;
            matched_type = ApplicationType::AUTOMATIC;
        }
    }

    Ok((matched_user, matched_status, matched_type))
}

fn check_mismatches(
    user: &User,
    applicant_data: &Value,
    fields_to_check: String,
) -> Result<(usize, HashMap<String, bool>)> {
    let applicant_data = applicant_data
        .as_object()
        .ok_or(anyhow!("Error parsing application applicant data"))?
        .clone();

    let mut match_result = HashMap::new();
    let mut missmatches = 0;

    info!("Checking user with id: {:?}", user.id);

    for field_to_check in fields_to_check.split(",") {
        // Extract field from application
        let applicant_field_value = applicant_data
            .get(field_to_check)
            .and_then(|value| value.as_str())
            .map(|value| value.to_string().to_lowercase());
        // Extract field from user
        let user_field_value = match field_to_check {
            "firstName" => &user.first_name,
            "lastName" => &user.last_name,
            "username" => &user.username,
            "email" => &user.email,
            _ => &user
                .attributes.as_ref()
                .and_then(|attributes| attributes.get(field_to_check))
                .and_then(|values| values.first())
                .map(|value| value.to_string()),
        }; 

        let user_field_value = user_field_value.clone().map(|value| value.to_lowercase());

        let is_match = applicant_field_value == user_field_value;

        // Check match
        match_result.insert(field_to_check.to_string(), is_match);

        if !is_match {
            missmatches += 1;
        }
    }

    info!("Missmatches {:?}", missmatches);
    info!("Match Result {:?}", match_result);

    Ok((missmatches, match_result))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn confirm_application(
    hasura_transaction: &Transaction<'_>,
    id: &str,
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
) -> Result<(Application, User)> {
    // Update the application to ACCEPTED
    let application = update_confirm_application(
        hasura_transaction,
        &id,
        &tenant_id,
        &election_event_id,
        user_id,
        ApplicationStatus::ACCEPTED,
    )
    .await
    .map_err(|err| anyhow!("Error updating application: {}", err))?;

    // Update user attributes and credentials
    let realm = get_event_realm(tenant_id, election_event_id);
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error obtaining keycloak admin client: {}", err))?;

    // Obtain application annotations
    let annotations = application
        .annotations
        .clone()
        .ok_or(anyhow!("Error obtaining application annotations"))?
        .as_object()
        .ok_or(anyhow!("Error parsing application annotations"))?
        .clone();

    // Obtain application credentials
    let credentials = annotations
        .get("credentials")
        .map(|value| {
            serde_json::from_value::<Vec<CredentialRepresentation>>(value.clone())
                .map_err(|err| anyhow!("Error parsing application credentials: {}", err))
        })
        .transpose()?;

    // Obtain voter attributes to update
    let attributes_to_store: Vec<String> = annotations
        .get("update-attributes")
        .ok_or(anyhow!(
            "Error obtaining update-attributes from application annotations"
        ))?
        .as_str()
        .ok_or(anyhow!(
            "Error parsing update-attributes from application annotations"
        ))?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Get applicant data
    let applicant_data = application
        .applicant_data
        .clone()
        .as_object()
        .ok_or(anyhow!("Error parsing application applicant data"))?
        .clone();

    // Parse applicant data to update user
    let mut attributes: HashMap<String, Vec<String>> = applicant_data
        .iter()
        .filter(|(key, _value)| attributes_to_store.contains(key))
        .map(|(key, value)| {
            (
                key.to_owned(),
                value
                    .to_string()
                    .split(";")
                    .map(|value| value.trim_matches('"').to_string())
                    .collect(),
            )
        })
        .collect();

    let email = attributes
        .remove("email")
        .and_then(|value| value.first().cloned());
    let first_name = attributes
        .remove("firstName")
        .and_then(|value| value.first().cloned());
    let last_name = attributes
        .remove("lastName")
        .and_then(|value| value.first().cloned());
    let _username = attributes
        .remove("username")
        .and_then(|value| value.first().cloned());

    let user = client
        .edit_user_with_credentials(
            &realm,
            &user_id,
            None,
            Some(attributes),
            email,
            first_name,
            last_name,
            None,
            credentials,
            Some(false),
        )
        .await
        .map_err(|err| anyhow!("Error updating user: {}", err))?;

    // TODO Send confirmation email or SMS

    Ok((application, user))
}
