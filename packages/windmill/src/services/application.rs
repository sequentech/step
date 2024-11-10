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

#[instrument(skip(hasura_transaction), err)]
pub async fn verify_application(
    hasura_transaction: &Transaction<'_>,
    applicant_id: &str,
    applicant_data: &Value,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &Option<String>,
    labels: &Option<Value>,
    annotations: &Option<Value>,
) -> Result<()> {
    // TODO Search user

    // TODO User match matrix

    // If user not matched create a manual verification application
    insert_application(
        hasura_transaction,
        tenant_id,
        election_event_id,
        area_id,
        &applicant_id,
        applicant_data,
        labels,
        annotations,
        ApplicationType::MANUAL,
        ApplicationStatus::PENDING,
    )
    .await?;

    // TODO Respond with user found if success

    Ok(())
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
