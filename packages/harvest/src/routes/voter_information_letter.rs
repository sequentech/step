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
    PasswordPolicyGenerationError,
};
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};
use uuid::Uuid;
use windmill::services::celery_app::get_celery_app;
use windmill::services::database::get_hasura_pool;
use windmill::services::document_password::save_password;
use windmill::services::electoral_log::ElectoralLogAdminContext;
use windmill::services::tasks_execution::{post, update_fail};
use windmill::types::tasks::ETasksExecution;

const POLICY_NOT_CONFIGURED_ERROR: &str =
    "Password Policy is not configured. Set it under Election Event Data before generating a letter.";
const POLICY_MINIMUM_LENGTH_MISSING_ERROR: &str =
    "Password Policy must include a minimum length before generating a letter.";
const POLICY_CHARACTER_CLASS_MISSING_ERROR: &str =
    "Password Policy must include at least one character class before generating a letter.";

#[derive(Debug, Deserialize)]
pub struct GenerateVoterInformationLetterInput {
    election_event_id: String,
    voter_id: String,
}

#[derive(Serialize)]
pub struct GenerateVoterInformationLetterOutput {
    document_id: String,
    pdf_password: String,
    task_execution: TasksExecution,
}

fn internal_error(message: &str) -> JsonError {
    ErrorResponse::new(
        Status::InternalServerError,
        message,
        ErrorCode::InternalServerError,
    )
}

fn password_policy_generation_error(
    error: PasswordPolicyGenerationError,
) -> JsonError {
    let (message, code) = match error {
        PasswordPolicyGenerationError::NotConfigured => (
            POLICY_NOT_CONFIGURED_ERROR,
            ErrorCode::PasswordPolicyNotConfigured,
        ),
        PasswordPolicyGenerationError::MinimumLengthMissing => (
            POLICY_MINIMUM_LENGTH_MISSING_ERROR,
            ErrorCode::PasswordPolicyMinimumLengthMissing,
        ),
        PasswordPolicyGenerationError::CharacterClassMissing => (
            POLICY_CHARACTER_CLASS_MISSING_ERROR,
            ErrorCode::PasswordPolicyCharacterClassMissing,
        ),
        _ => {
            return ErrorResponse::new(
                Status::BadRequest,
                &error.to_string(),
                ErrorCode::InvalidPasswordPolicy,
            )
        }
    };

    ErrorResponse::new(Status::BadRequest, message, code)
}

async fn store_document_password(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    password: &str,
) -> anyhow::Result<String> {
    let mut client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .context("Failed to get database client")?;
    let transaction = client
        .transaction()
        .await
        .context("Failed to start secret transaction")?;
    let secret_id = save_password(
        &transaction,
        tenant_id,
        Some(election_event_id),
        document_id,
        password,
    )
    .await
    .context("Failed to store document password")?;
    transaction
        .commit()
        .await
        .context("Failed to commit document password secret")?;
    Ok(secret_id)
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
        vec![
            Permissions::VOTER_INFORMATION_LETTER,
            Permissions::DOCUMENT_PASSWORD_READ,
        ],
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
    policy
        .validate_for_generation()
        .map_err(password_policy_generation_error)?;

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

    let document_id = Uuid::new_v4().to_string();
    let pdf_password = Uuid::new_v4().simple().to_string();
    let password_secret_id = match store_document_password(
        &tenant_id,
        &input.election_event_id,
        &document_id,
        &pdf_password,
    )
    .await
    {
        Ok(secret_id) => secret_id,
        Err(error) => {
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
    };

    let password_change_initiator = ElectoralLogAdminContext {
        user_id: claims.hasura_claims.user_id.clone(),
        username: claims.preferred_username.clone(),
        authorized_election_ids: claims
            .hasura_claims
            .authorized_election_ids
            .clone(),
        area_id: claims.hasura_claims.area_id.clone(),
    };
    let celery_app = get_celery_app().await;
    if let Err(_send_error) = celery_app
        .send_task(
            windmill::tasks::voter_information_letter::generate_voter_information_letter::new(
                tenant_id,
                input.election_event_id,
                input.voter_id,
                document_id.clone(),
                password_secret_id,
                password_change_initiator,
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
        pdf_password,
        task_execution,
    }))
}

#[cfg(test)]
mod tests {
    use super::password_policy_generation_error;
    use rocket::http::Status;
    use sequent_core::services::keycloak::PasswordPolicyGenerationError;

    #[test]
    fn maps_missing_policy_to_its_action_error_code() {
        let response = password_policy_generation_error(
            PasswordPolicyGenerationError::NotConfigured,
        );

        assert_eq!(Status::BadRequest, response.0);
        assert_eq!(
            "PasswordPolicyNotConfigured",
            response.1 .0.extensions.code
        );
    }

    #[test]
    fn maps_generation_preconditions_to_distinct_action_error_codes() {
        let missing_length = password_policy_generation_error(
            PasswordPolicyGenerationError::MinimumLengthMissing,
        );
        let missing_class = password_policy_generation_error(
            PasswordPolicyGenerationError::CharacterClassMissing,
        );

        assert_eq!(
            "PasswordPolicyMinimumLengthMissing",
            missing_length.1 .0.extensions.code
        );
        assert_eq!(
            "PasswordPolicyCharacterClassMissing",
            missing_class.1 .0.extensions.code
        );
        assert_ne!(missing_length.1 .0.message, missing_class.1 .0.message);
    }

    #[test]
    fn maps_other_invalid_generation_configuration_without_calling_it_missing()
    {
        let response = password_policy_generation_error(
            PasswordPolicyGenerationError::MinimumExceedsMaximum,
        );

        assert_eq!("InvalidPasswordPolicy", response.1 .0.extensions.code);
        assert!(response.1 .0.message.contains("cannot exceed maximum"));
    }
}
