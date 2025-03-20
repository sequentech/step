// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::connection::DatafixClaims;

use electoral_log::messages::newtypes::ExtApiRequestDirection;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::permissions::Permissions;
use serde::Serialize;
use tracing::{error, info, instrument};
use windmill::services;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::datafix::types::*;
use windmill::services::datafix::utils::{
    get_event_id_and_datafix_annotations, get_user_id,
    post_operation_result_to_electoral_log,
};

#[instrument(skip_all)]
fn authorize_user(claims: &DatafixClaims) -> Result<(), JsonErrorResponse> {
    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })
}

#[instrument(skip(claims))]
#[post("/add-voter", format = "json", data = "<body>")]
pub async fn add_voter(
    claims: DatafixClaims,
    body: Json<VoterInformationBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterInformationBody = body.into_inner();
    authorize_user(&claims)?;
    handle_voter_operation(
        EndpointNames::AddVoter,
        &claims,
        VoterOperationInput::VoterInfo(input),
    )
    .await
}

#[instrument(skip(claims))]
#[post("/update-voter", format = "json", data = "<body>")]
pub async fn update_voter(
    claims: DatafixClaims,
    body: Json<VoterInformationBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterInformationBody = body.into_inner();
    authorize_user(&claims)?;
    handle_voter_operation(
        EndpointNames::UpdateVoter,
        &claims,
        VoterOperationInput::VoterInfo(input),
    )
    .await
}

#[instrument(skip(claims))]
#[post("/delete-voter", format = "json", data = "<body>")]
pub async fn delete_voter(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();
    authorize_user(&claims)?;
    handle_voter_operation(
        EndpointNames::DeleteVoter,
        &claims,
        VoterOperationInput::VoterId(input),
    )
    .await
}

#[instrument(skip(claims))]
#[post("/unmark-voted", format = "json", data = "<body>")]
pub async fn unmark_voted(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();
    authorize_user(&claims)?;
    handle_voter_operation(
        EndpointNames::UnmarkVoted,
        &claims,
        VoterOperationInput::VoterId(input),
    )
    .await
}

#[instrument(skip(claims))]
#[post("/mark-voted", format = "json", data = "<body>")]
pub async fn mark_voted(
    claims: DatafixClaims,
    body: Json<MarkVotedBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: MarkVotedBody = body.into_inner();
    authorize_user(&claims)?;
    handle_voter_operation(
        EndpointNames::MarkVoted,
        &claims,
        VoterOperationInput::MarkVoted(input),
    )
    .await
}

#[derive(Serialize, Debug)]
pub struct ReplacePinOutput {
    pin: String,
}

#[instrument(skip(claims))]
#[post("/replace-pin", format = "json", data = "<body>")]
pub async fn replace_pin(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<ReplacePinOutput>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();
    authorize_user(&claims)?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Error getting hasura client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Error starting hasura transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let (election_event_id, datafix_annotations) =
        get_event_id_and_datafix_annotations(
            &hasura_transaction,
            &claims.tenant_id,
            &claims.datafix_event_id,
        )
        .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let user_id =
        get_user_id(&keycloak_transaction, &realm, &input.voter_id).await?;

    let endpoint_name = EndpointNames::ReplacePin;
    let (pin_result, operation) =
        match services::datafix::api_datafix::replace_voter_pin(
            &hasura_transaction,
            &keycloak_transaction,
            &claims.tenant_id,
            &claims.datafix_event_id,
            &input.voter_id,
            &election_event_id,
            &realm,
            &datafix_annotations,
        )
        .await
        {
            Ok(pin) => (Ok(pin), format!("{endpoint_name} Succeeded")),
            Err(err) => (Err(err), format!("{endpoint_name} Failed")),
        };

    post_operation_result_to_electoral_log(
        &hasura_transaction,
        &claims.tenant_id,
        &election_event_id,
        &user_id,
        &input.voter_id,
        ExtApiRequestDirection::Inbound,
        operation,
    )
    .await;
    let pin = pin_result?;
    Ok(Json(ReplacePinOutput { pin }))
}

async fn handle_voter_operation(
    endpoint_name: EndpointNames,
    claims: &DatafixClaims,
    input: VoterOperationInput,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Error getting hasura client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Error starting hasura transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|e| {
            error!("Error getting keycloak client {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|e| {
            error!("Error starting keycloak transaction {}", e);
            DatafixResponse::new(Status::InternalServerError)
        })?;

    let (election_event_id, _) = get_event_id_and_datafix_annotations(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await?;
    let realm = get_event_realm(&claims.tenant_id, &election_event_id);

    let (result, username) = match (endpoint_name, input) {
        (EndpointNames::AddVoter, VoterOperationInput::VoterInfo(input)) => (
            services::datafix::api_datafix::add_datafix_voter(
                &hasura_transaction,
                &claims.tenant_id,
                &claims.datafix_event_id,
                &input,
                &election_event_id,
                &realm,
            )
            .await,
            input.voter_id,
        ),

        (EndpointNames::UpdateVoter, VoterOperationInput::VoterInfo(input)) => {
            (
                services::datafix::api_datafix::update_datafix_voter(
                    &hasura_transaction,
                    &keycloak_transaction,
                    &claims.tenant_id,
                    &claims.datafix_event_id,
                    &input,
                    &election_event_id,
                    &realm,
                )
                .await,
                input.voter_id,
            )
        }

        (EndpointNames::DeleteVoter, VoterOperationInput::VoterId(input)) => (
            services::datafix::api_datafix::disable_datafix_voter(
                &hasura_transaction,
                &keycloak_transaction,
                &claims.tenant_id,
                &claims.datafix_event_id,
                &input.voter_id,
                &realm,
            )
            .await,
            input.voter_id,
        ),

        (EndpointNames::UnmarkVoted, VoterOperationInput::VoterId(input)) => (
            services::datafix::api_datafix::unmark_voter_as_voted(
                &hasura_transaction,
                &keycloak_transaction,
                &claims.tenant_id,
                &claims.datafix_event_id,
                &input.voter_id,
                &realm,
            )
            .await,
            input.voter_id,
        ),

        (EndpointNames::MarkVoted, VoterOperationInput::MarkVoted(input)) => (
            services::datafix::api_datafix::mark_as_voted_via_channel(
                &hasura_transaction,
                &keycloak_transaction,
                &claims.tenant_id,
                &claims.datafix_event_id,
                &input,
                &realm,
            )
            .await,
            input.voter_id,
        ),

        _ => return Err(DatafixResponse::new(Status::InternalServerError)),
    };

    let user_id = get_user_id(&keycloak_transaction, &realm, &username).await?;

    let operation = match result.is_ok() {
        true => format!("{endpoint_name} Succeeded"),
        false => format!("{endpoint_name} Failed"),
    };

    post_operation_result_to_electoral_log(
        &hasura_transaction,
        &claims.tenant_id,
        &election_event_id,
        &user_id,
        &username,
        ExtApiRequestDirection::Inbound,
        operation,
    )
    .await;

    result
}
