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
use reqwest::StatusCode;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use sequent_core::services::keycloak::{
    get_event_realm, get_tenant_realm, GroupInfo, KeycloakAdminClient,
};
use sequent_core::types::keycloak::User;
use sequent_core::types::permissions::Permissions;
use serde::Deserialize;
use serde_json::Value;
use tracing::instrument;
use windmill::services::application::{
    confirm_application, get_group_names, reject_application,
    verify_application, ApplicationAnnotations, ApplicationVerificationResult,
};
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};

#[derive(Deserialize, Debug)]
pub struct DatafixDeleteVoterBody {
    voter_id: String,
}

#[instrument(skip(claims))]
#[post("/delete-voter", format = "json", data = "<body>")]
pub async fn delete_voter(
    claims: jwt::JwtClaims,
    body: Json<DatafixDeleteVoterBody>,
) -> Result<Json<String>, JsonError> {
    // WIP
    Ok(Json("Success".to_string()))
}
