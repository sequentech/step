// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::authorization::authorize;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::connection::DatafixClaims;

use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde::Serialize;
use tracing::{error, info, instrument};
use windmill::services;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::datafix::types::*;

#[instrument(skip(claims))]
#[post("/add-voter", format = "json", data = "<body>")]
pub async fn add_voter(
    claims: DatafixClaims,
    body: Json<VoterInformationBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterInformationBody = body.into_inner();

    info!("Add voter: {input:?}");

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

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

    services::datafix::api_datafix::add_datafix_voter(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
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

    info!("Update voter: {input:?}");

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

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

    services::datafix::api_datafix::update_datafix_voter(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
    )
    .await
}

#[derive(Deserialize, Debug)]
pub struct VoterIdBody {
    voter_id: String,
}

#[instrument(skip(claims))]
#[post("/delete-voter", format = "json", data = "<body>")]
pub async fn delete_voter(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<DatafixResponse>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();

    info!("Delete voter: {input:?}");

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

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

    services::datafix::api_datafix::disable_datafix_voter(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
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

    info!("Unmark voter as voted {input:?}");

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

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

    services::datafix::api_datafix::unmark_voter_as_voted(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
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

    info!("Mark voter as voted: {input:?}");

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

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

    services::datafix::api_datafix::mark_as_voted_via_channel(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input,
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
    info!("Replace pin: {input:?}");

    let required_perm = vec![Permissions::DATAFIX_ACCOUNT];
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| {
        error!("Error authorizing {e:?}");
        DatafixResponse::new(Status::Unauthorized)
    })?;

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

    let pin = services::datafix::api_datafix::replace_voter_pin(
        &hasura_transaction,
        &keycloak_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
        &input.voter_id,
    )
    .await?;

    Ok(Json(ReplacePinOutput { pin }))
}
