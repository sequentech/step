// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Context;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::{
    get_event_realm, get_realm_password_policy, KeycloakAdminClient,
};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
use uuid::Uuid;
use windmill::postgres::tasks_execution::get_task_by_id;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::tasks_execution::{post, update_fail};
use windmill::services::voter_information_letter::{
    read_secret, save_secret, VoterInformationLetterSecret,
};
use windmill::types::tasks::ETasksExecution;

const POLICY_ERROR: &str =
    "Password Policy is not configured. Set it under Election Event Data before generating a letter.";

#[derive(Debug, Deserialize)]
pub struct GenerateVoterInformationLetterInput {
    election_event_id: String,
    voter_id: String,
}

#[derive(Debug, Serialize)]
pub struct GenerateVoterInformationLetterOutput {
    document_id: String,
    pdf_password: String,
    task_execution: TasksExecution,
}

#[derive(Debug, Deserialize)]
pub struct GetVoterInformationLetterPasswordInput {
    task_id: String,
}

#[derive(Serialize)]
pub struct GetVoterInformationLetterPasswordOutput {
    pdf_password: String,
}

fn internal_error(message: &str) -> JsonError {
    ErrorResponse::new(
        Status::InternalServerError,
        message,
        ErrorCode::InternalServerError,
    )
}

async fn store_secret_for_task(
    tenant_id: &str,
    election_event_id: &str,
    task_id: &str,
    secret: &VoterInformationLetterSecret,
) -> anyhow::Result<()> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get database client")?;
    let transaction = client
        .transaction()
        .await
        .context("Failed to start secret transaction")?;
    save_secret(&transaction, tenant_id, election_event_id, task_id, secret)
        .await
        .context("Failed to store Voter Information Letter secret")?;
    transaction
        .commit()
        .await
        .context("Failed to commit Voter Information Letter secret")
}

#[instrument(skip_all)]
#[post(
    "/generate-voter-information-letter",
    format = "json",
    data = "<input>"
)]
pub async fn generate_voter_information_letter(
    claims: JwtClaims,
    input: Json<GenerateVoterInformationLetterInput>,
) -> Result<Json<GenerateVoterInformationLetterOutput>, JsonError> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::VOTER_INFORMATION_LETTER],
    )
    .map_err(|_| {
        ErrorResponse::new(
            Status::Forbidden,
            "Authorization failed",
            ErrorCode::Unauthorized,
        )
    })?;

    let input = input.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let policy = get_realm_password_policy(
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|error| {
        error!("Failed to read the election event password policy: {error:#}");
        internal_error("Failed to read the election event password policy")
    })?;
    policy.validate_for_generation().map_err(|_| {
        ErrorResponse::new(
            Status::BadRequest,
            POLICY_ERROR,
            ErrorCode::PasswordPolicyNotConfigured,
        )
    })?;

    KeycloakAdminClient::new()
        .await
        .map_err(|_| internal_error("Failed to initialize Keycloak client"))?
        .get_user(
            &get_event_realm(&tenant_id, &input.election_event_id),
            &input.voter_id,
        )
        .await
        .map_err(|_| {
            ErrorResponse::new(
                Status::NotFound,
                "Voter not found",
                ErrorCode::VoterInformationLetterUnavailable,
            )
        })?;

    let secret = VoterInformationLetterSecret {
        voter_password: policy.generate_password().map_err(|error| {
            error!(
                "Failed to generate a credential from the election event password policy: {error:#}"
            );
            internal_error("Failed to generate a voter credential")
        })?,
        pdf_password: Uuid::new_v4().simple().to_string(),
    };

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());
    let task_execution = post(
        &tenant_id,
        Some(&input.election_event_id),
        ETasksExecution::VOTER_INFORMATION_LETTER,
        &executer_name,
    )
    .await
    .map_err(|error| {
        error!("Failed to create Voter Information Letter task: {error:#}");
        internal_error("Failed to create Voter Information Letter task")
    })?;

    if let Err(error) = store_secret_for_task(
        &tenant_id,
        &input.election_event_id,
        &task_execution.id,
        &secret,
    )
    .await
    {
        error!(
            task_id = %task_execution.id,
            "Failed to prepare Voter Information Letter document access: {error:#}"
        );
        update_fail(
            &task_execution,
            "Failed to prepare Voter Information Letter generation",
        )
        .await
        .ok();
        return Err(internal_error(
            "Failed to prepare Voter Information Letter generation",
        ));
    }

    let document_id = Uuid::new_v4().to_string();
    let celery_app = get_celery_app().await;
    if let Err(_send_error) = celery_app
        .send_task(
            windmill::tasks::voter_information_letter::generate_voter_information_letter::new(
                tenant_id,
                input.election_event_id,
                input.voter_id,
                document_id.clone(),
                task_execution.clone(),
            ),
        )
        .await
    {
        error!(
            task_id = %task_execution.id,
            "Failed to enqueue Voter Information Letter task"
        );
        update_fail(
            &task_execution,
            "Failed to enqueue Voter Information Letter generation",
        )
        .await
        .ok();
        return Err(internal_error(
            "Failed to enqueue Voter Information Letter task",
        ));
    }

    Ok(Json(GenerateVoterInformationLetterOutput {
        document_id,
        pdf_password: secret.pdf_password,
        task_execution,
    }))
}

#[instrument(skip_all)]
#[post(
    "/get-voter-information-letter-password",
    format = "json",
    data = "<input>"
)]
pub async fn get_voter_information_letter_password(
    claims: JwtClaims,
    input: Json<GetVoterInformationLetterPasswordInput>,
) -> Result<Json<GetVoterInformationLetterPasswordOutput>, JsonError> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![
            Permissions::TASKS_READ,
            Permissions::VOTER_INFORMATION_LETTER,
        ],
    )
    .map_err(|_| {
        ErrorResponse::new(
            Status::Forbidden,
            "Authorization failed",
            ErrorCode::Unauthorized,
        )
    })?;

    let task = get_task_by_id(&input.task_id).await.map_err(|_| {
        ErrorResponse::new(
            Status::NotFound,
            "Voter Information Letter task not found",
            ErrorCode::VoterInformationLetterUnavailable,
        )
    })?;
    let expected_type = ETasksExecution::VOTER_INFORMATION_LETTER.to_string();
    if task.tenant_id != claims.hasura_claims.tenant_id
        || task.task_type != expected_type
        || task.execution_status != "SUCCESS"
    {
        return Err(ErrorResponse::new(
            Status::NotFound,
            "Voter Information Letter password is not available",
            ErrorCode::VoterInformationLetterUnavailable,
        ));
    }
    let election_event_id =
        task.election_event_id.as_deref().ok_or_else(|| {
            ErrorResponse::new(
                Status::NotFound,
                "Voter Information Letter password is not available",
                ErrorCode::VoterInformationLetterUnavailable,
            )
        })?;

    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|_| internal_error("Failed to get database client"))?;
    let transaction = client
        .transaction()
        .await
        .map_err(|_| internal_error("Failed to start database transaction"))?;
    let secret =
        read_secret(&transaction, &task.tenant_id, election_event_id, &task.id)
            .await
            .map_err(|_| internal_error("Failed to retrieve the PDF password"))?
            .ok_or_else(|| {
                ErrorResponse::new(
                    Status::NotFound,
                    "Voter Information Letter password is not available",
                    ErrorCode::VoterInformationLetterUnavailable,
                )
            })?;
    transaction
        .commit()
        .await
        .map_err(|_| internal_error("Failed to finish password retrieval"))?;

    Ok(Json(GetVoterInformationLetterPasswordOutput {
        pdf_password: secret.pdf_password,
    }))
}
