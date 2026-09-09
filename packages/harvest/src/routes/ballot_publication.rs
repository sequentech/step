// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use crate::types::error_response::{ErrorCode, ErrorResponse, JsonError};
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::permissions::Permissions;
use sequent_core::{
    ballot::{ElectionEventPresentation, LockedDown},
    services::jwt::{has_gold_permission, JwtClaims},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::{
    postgres::election_event::get_election_event_by_id,
    services::{
        ballot_styles::ballot_publication::{
            add_ballot_publication, get_ballot_publication_diff,
            update_publish_ballot, BallotPublicationValidationError,
            PublicationDiff,
        },
        database::get_hasura_pool,
        tasks_execution::{
            post as post_task_execution,
            update_complete as update_task_execution_complete,
            update_fail as update_task_execution_fail,
        },
    },
    types::tasks::ETasksExecution,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateBallotPublicationInput {
    election_event_id: String,
    election_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateBallotPublicationOutput {
    ballot_publication_id: String,
    task_execution: TasksExecution,
}

#[instrument(skip(claims))]
#[post("/generate-ballot-publication", format = "json", data = "<body>")]
pub async fn generate_ballot_publication(
    body: Json<GenerateBallotPublicationInput>,
    claims: JwtClaims,
) -> Result<Json<GenerateBallotPublicationOutput>, (Status, String)> {
    if !has_gold_permission(&claims) {
        return Err((Status::Forbidden, "Insufficient privileges".into()));
    }

    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_WRITE],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    if let Some(election_event_presentation) = election_event.presentation {
        info!(
            "election_event_presentation {:?}",
            election_event_presentation
        );
        let maybe_err = deserialize_value::<ElectionEventPresentation>(
            election_event_presentation.clone(),
        );
        info!("presentation err {:?}", maybe_err);

        if deserialize_value::<ElectionEventPresentation>(
            election_event_presentation,
        )
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?
        .locked_down
            == Some(LockedDown::LOCKED_DOWN)
        {
            return Err((
                Status::Forbidden,
                "Election event is locked down".to_string(),
            ));
        }
    }

    let executer_name = claims
        .name
        .clone()
        .unwrap_or_else(|| claims.hasura_claims.user_id.clone());

    let (ballot_publication_id, task_execution) = add_ballot_publication(
        &hasura_transaction,
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.election_id.clone(),
        user_id.clone(),
        &executer_name,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    hasura_transaction.commit().await.map_err(|err| {
        (Status::InternalServerError, format!("Commit failed: {err}"))
    })?;

    Ok(Json(GenerateBallotPublicationOutput {
        ballot_publication_id,
        task_execution,
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotInput {
    election_event_id: String,
    ballot_publication_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishBallotOutput {
    ballot_publication_id: String,
}

#[instrument(skip(claims))]
#[post("/publish-ballot", format = "json", data = "<body>")]
pub async fn publish_ballot(
    body: Json<PublishBallotInput>,
    claims: JwtClaims,
) -> Result<Json<PublishBallotOutput>, JsonError> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_WRITE],
    )
    .map_err(|(status, message)| {
        let code =
            if status == Status::Unauthorized || status == Status::Forbidden {
                ErrorCode::Unauthorized
            } else {
                ErrorCode::UnknownError
            };
        ErrorResponse::new(status, &message, code)
    })?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();
    let user_id = claims.hasura_claims.user_id.clone();
    let username = claims
        .preferred_username
        .clone()
        .unwrap_or_else(|| "-".to_string());
    let executer_name = claims.name.clone().unwrap_or_else(|| user_id.clone());

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{e:?}"),
                ErrorCode::InternalServerError,
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            ErrorResponse::new(
                Status::InternalServerError,
                &format!("{e:?}"),
                ErrorCode::InternalServerError,
            )
        })?;
    let task_execution = post_task_execution(
        &tenant_id,
        Some(&input.election_event_id),
        ETasksExecution::PUBLISH_BALLOT,
        &executer_name,
    )
    .await
    .map_err(|e| {
        ErrorResponse::new(
            Status::InternalServerError,
            &format!("{e:?}"),
            ErrorCode::InternalServerError,
        )
    })?;

    let publish_result = update_publish_ballot(
        &hasura_transaction,
        user_id,
        username,
        tenant_id,
        input.election_event_id.clone(),
        input.ballot_publication_id.clone(),
    )
    .await;

    if let Err(error) = publish_result {
        let is_validation_error = error
            .downcast_ref::<BallotPublicationValidationError>()
            .is_some();
        let failure_message = error.to_string();
        let response_message = format!(
            "Publish task {} failed: {failure_message}",
            task_execution.id
        );

        if let Err(rollback_error) = hasura_transaction.rollback().await {
            let message =
                format!("{failure_message}\nRollback failed: {rollback_error}");
            update_task_execution_fail(&task_execution, &message)
                .await
                .ok();
            return Err(ErrorResponse::new(
                Status::InternalServerError,
                &response_message,
                ErrorCode::InternalServerError,
            ));
        }

        update_task_execution_fail(&task_execution, &failure_message)
            .await
            .map_err(|task_error| {
                ErrorResponse::new(
                    Status::InternalServerError,
                    &format!(
                        "{response_message}. The task failure could not be recorded: {task_error}"
                    ),
                    ErrorCode::InternalServerError,
                )
            })?;
        return Err(if is_validation_error {
            ErrorResponse::new(
                Status::BadRequest,
                &failure_message,
                ErrorCode::BallotPublicationValidation,
            )
        } else {
            ErrorResponse::new(
                Status::InternalServerError,
                &response_message,
                ErrorCode::InternalServerError,
            )
        });
    }

    if let Err(commit_error) = hasura_transaction.commit().await {
        let failure_message = format!("Commit failed: {commit_error}");
        update_task_execution_fail(&task_execution, &failure_message)
            .await
            .ok();
        return Err(ErrorResponse::new(
            Status::InternalServerError,
            &failure_message,
            ErrorCode::InternalServerError,
        ));
    }

    if let Err(task_error) =
        update_task_execution_complete(&task_execution, None).await
    {
        let failure_message = format!(
            "Ballot was published, but task {} could not be marked complete: {task_error}",
            task_execution.id
        );
        update_task_execution_fail(&task_execution, &failure_message)
            .await
            .ok();
        return Err(ErrorResponse::new(
            Status::InternalServerError,
            &failure_message,
            ErrorCode::InternalServerError,
        ));
    }

    Ok(Json(PublishBallotOutput {
        ballot_publication_id: input.ballot_publication_id.clone(),
    }))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetBallotPublicationChangesInput {
    election_event_id: String,
    ballot_publication_id: String,
    limit: Option<usize>,
}

#[instrument(skip(claims))]
#[post("/get-ballot-publication-changes", format = "json", data = "<body>")]
pub async fn get_ballot_publication_changes(
    body: Json<GetBallotPublicationChangesInput>,
    claims: JwtClaims,
) -> Result<Json<PublicationDiff>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::PUBLISH_READ],
    )?;
    let input = body.into_inner();
    let tenant_id = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    let diff = get_ballot_publication_diff(
        &hasura_transaction,
        tenant_id.clone(),
        input.election_event_id.clone(),
        input.ballot_publication_id.clone(),
        input.limit,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(diff))
}
