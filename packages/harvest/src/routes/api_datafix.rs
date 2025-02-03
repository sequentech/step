// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::HashMap;
use std::iter::Map;
use std::str::FromStr;

use crate::services::authorization::authorize;
use crate::types::optional::OptionalId;
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use reqwest::StatusCode;
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::Json;
use sequent_core::services::connection::DatafixClaims;
use sequent_core::services::keycloak::{
    get_event_realm, get_tenant_realm, GroupInfo, KeycloakAdminClient,
};
use sequent_core::types::keycloak::User;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tracing::{error, info, instrument};
use windmill::postgres::election_event::get_all_tenant_election_events;
use windmill::services::api_datafix::*;
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};

#[derive(Serialize)]
pub struct DatafixErrorResponse {
    pub code: u16,
    pub message: String,
}

type JsonErrorResponse = Json<DatafixErrorResponse>;

impl DatafixErrorResponse {
    #[instrument]
    pub fn new(status: Status) -> JsonErrorResponse {
        Json(DatafixErrorResponse {
            code: status.code,
            message: status.reason().unwrap_or_default().to_string(),
        })
    }
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
) -> Result<Json<String>, JsonErrorResponse> {
    let input: VoterIdBody = body.into_inner();

    info!("Delete voter: {input:?}");

    let required_perm = vec![
        Permissions::DATAFIX_ACCOUNT,
        Permissions::VOTER_READ,
        Permissions::VOTER_WRITE,
    ]; // TODO: Set up the permissions in Keycloak
    info!("{claims:?}");
    authorize(
        &claims.jwt_claims,
        true,
        Some(claims.tenant_id.clone()),
        required_perm,
    )
    .map_err(|e| DatafixErrorResponse::new(Status::Unauthorized))?;

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|e| {
            error!("Error getting hasura client {}", e);
            (DatafixErrorResponse::new(Status::InternalServerError))
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|e| {
            error!("Error starting hasura transaction {}", e);
            (DatafixErrorResponse::new(Status::InternalServerError))
        })?;

    let election_event_id = get_datafix_election_event_id(
        &hasura_transaction,
        &claims.tenant_id,
        &claims.datafix_event_id,
    )
    .await
    .map_err(|_e| DatafixErrorResponse::new(Status::BadRequest))?;

    let realm = get_event_realm(&claims.tenant_id, &election_event_id);
    let client = KeycloakAdminClient::new().await.map_err(|e| {
        (DatafixErrorResponse::new(Status::InternalServerError))
    })?;

    let _user = client
        .edit_user(
            &realm,
            &input.voter_id,
            Some(false), /* Disable the voter, datafix users are not
                          * actually deleted but just disabled */
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .map_err(|e| {
            (DatafixErrorResponse::new(Status::InternalServerError))
        })?;
    Err(DatafixErrorResponse::new(Status::Ok))
}

#[instrument(skip(claims))]
#[post("/unmark-voted", format = "json", data = "<body>")]
pub async fn unmark_voted(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<String>, JsonErrorResponse> {
    // WIP
    let status = Status::Unauthorized;
    Err(DatafixErrorResponse::new(status))
}

#[derive(Deserialize, Debug)]
pub struct MarkVotedBody {
    voter_id: String,
    channel: String,
}

#[instrument(skip(claims))]
#[post("/mark-voted", format = "json", data = "<body>")]
pub async fn mark_voted(
    claims: DatafixClaims,
    body: Json<MarkVotedBody>,
) -> Result<Json<String>, JsonErrorResponse> {
    // WIP
    let status = Status::Unauthorized;
    Err(DatafixErrorResponse::new(status))
}

#[derive(Serialize, Debug)]
pub struct ReplacePinOutput {
    pin: String,
}

#[instrument]
#[post("/replace-pin", format = "json", data = "<body>")]
pub async fn replace_pin(
    claims: DatafixClaims,
    body: Json<VoterIdBody>,
) -> Result<Json<ReplacePinOutput>, JsonErrorResponse> {
    // WIP
    let pin = "684400123987".to_string();
    Ok(Json(ReplacePinOutput { pin }))
}
