// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use tracing::{event, instrument, Level};
use windmill::services::vault;
use windmill::tasks::insert_election_event;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct JWKKey {
    alg: String,
    kty: String,
    r#use: String,
    n: String,
    e: String,
    kid: String,
    x5t: String,
    x5c: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GetJwksOutput {
    keys: Vec<JWKKey>,
}

#[instrument]
#[get("/jwks.json", format = "json")]
pub async fn get_jwks_json() -> Result<Json<GetJwksOutput>, Debug<anyhow::Error>> {
    let jwks_json = vault::read_secret("keycloak/jwks.json".to_string()).await?;
    let keys: Vec<JWKKey> = from_str(&jwks_json).map_err(|e| anyhow::Error::from(e))?;
    Ok(Json(GetJwksOutput {
        keys,
    }))
}