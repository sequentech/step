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
use tracing::instrument;
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
    // WIP
    let status = Status::Unauthorized;
    Err(DatafixErrorResponse::new(status))
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
