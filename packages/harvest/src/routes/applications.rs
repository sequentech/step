// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use std::iter::Map;
use std::str::FromStr;

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use crate::types::optional::OptionalId;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use keycloak::types::{
    CredentialRepresentation, UPAttribute, UPConfig, UserRepresentation,
};
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::Value;
use tracing::instrument;
use windmill::postgres::application;
use windmill::services::application::{
    confirm_application, verify_application,
};
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::tasks::send_template::send_template;
use windmill::types::application::{ApplicationStatus, ApplicationType};

#[derive(Deserialize, Debug)]
pub struct ApplicationVerifyBody {
    applicant_id: String,
    applicant_data: Value,
    tenant_id: String,
    election_event_id: String,
    area_id: Option<String>,
    labels: Option<Value>,
    annotations: Option<Value>,
}

#[instrument(skip(claims))]
#[post("/verify-application", format = "json", data = "<body>")]
pub async fn verify_user_application(
    claims: jwt::JwtClaims,
    body: Json<ApplicationVerifyBody>,
) -> Result<Json<String>, JsonError> {
    let input = body.into_inner();

    info!("Verifiying application: {input:?}");

    let required_perm: Permissions = Permissions::SERVICE_ACCOUNT;
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{:?}", e),
            ErrorCode::Unauthorized,
        )
    })?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{:?}", e),
                ErrorCode::GetTransactionFailed,
            )
        })?;

    verify_application(
        &hasura_transaction,
        &input.applicant_id,
        &input.applicant_data,
        &input.tenant_id,
        &input.election_event_id,
        &None,
        &input.labels,
        &input.annotations,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("{:?}", e),
            ErrorCode::InternalServerError,
        )
    })?;

    let _commit = hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    Ok(Json("Success".to_string()))
}

#[derive(Deserialize, Debug)]
pub struct ApplicationConfirmationBody {
    tenant_id: String,
    election_event_id: String,
    area_id: Option<String>,
    id: String,
    user_id: String,
}

#[instrument(skip(claims))]
#[post("/confirm-application", format = "json", data = "<body>")]
pub async fn confirm_user_application(
    claims: jwt::JwtClaims,
    body: Json<ApplicationConfirmationBody>,
) -> Result<Json<String>, JsonError> {
    let input = body.into_inner();

    info!("Confirming application: {input:?}");

    let required_perm: Permissions = Permissions::APPLICATION_WRITE;
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![required_perm],
    )
    .map_err(|e| {
        ErrorResponse::new(
            Status::Unauthorized,
            &format!("{:?}", e),
            ErrorCode::Unauthorized,
        )
    })?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error obtaining hasura pool: {:?}", e),
                ErrorCode::InternalServerError,
            )
        })?;

    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("Error obtaining transaction: {:?}", e),
                ErrorCode::GetTransactionFailed,
            )
        })?;

    let application = confirm_application(
        &hasura_transaction,
        &input.id,
        &input.tenant_id,
        &input.election_event_id,
        &input.user_id,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error confirming application {:?}", e),
            ErrorCode::InternalServerError,
        )
    })?;

    let realm = get_event_realm(&input.tenant_id, &input.election_event_id);

    let client = KeycloakAdminClient::new().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error obtaining the client: {:?}", e),
            ErrorCode::InternalServerError,
        )
    })?;

    match application {
        Some(application) => {
            let mut credentials = None;
            // Get attributes to store
            let attributes_to_store: Vec<String> =
                if let Some(Value::Object(annotations_map)) =
                    application.annotations
                {
                    let update_attributes =
                        annotations_map.get("update-attributes");

                    credentials = if let Some(value) =
                        annotations_map.get("credentials")
                    {
                        serde_json::from_value::<Vec<CredentialRepresentation>>(
                            value.clone(),
                        )
                        .ok()
                    } else {
                        None
                    };

                    if let Some(Value::String(value)) = update_attributes {
                        value.split(',').map(|s| s.trim().to_string()).collect()
                    } else {
                        todo!();
                    }
                } else {
                    todo!();
                };

            // Get applicant data
            let applicant_data = if let Value::Object(applicant_data_map) =
                application.applicant_data
            {
                applicant_data_map
            } else {
                todo!();
            };

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
                .map(|value| value.first().unwrap().to_owned());
            let first_name = attributes
                .remove("firstName")
                .map(|value| value.first().unwrap().to_owned());
            let last_name = attributes
                .remove("lastName")
                .map(|value| value.first().unwrap().to_owned());
            let _username = attributes
                .remove("username")
                .map(|value| value.first().unwrap().to_owned());

            client.edit_user_with_credentials(
                &realm,
                &input.user_id,
                None,
                Some(attributes),
                email,
                first_name,
                last_name,
                None,
                credentials,
                Some(false),
            )
        }
        None => {
            todo!();
        }
    }
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("Error editing user: {:?}", e),
            ErrorCode::InternalServerError,
        )
    })?;

    let _commit = hasura_transaction.commit().await.map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("commit failed: {e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    // TODO Send confirmation email or SMS

    Ok(Json("Success".to_string()))
}
