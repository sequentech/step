// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use sequent_core::services::connection;
use tracing::{event, instrument, Level};
use windmill::services::jwks::{JWKKey, get_jwks, JwksOutput};
use windmill::tasks::insert_election_event;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use uuid::Uuid;

#[instrument]
#[get("/jwks.json", format = "json")]
pub async fn get_jwks_json() -> Result<Json<GetJwksOutput>, Debug<anyhow::Error>> {
    let keys: Vec<JWKKey> = get_jwks().await.map_err(|e| anyhow::Error::from(e))?;
    Ok(Json(JwksOutput {
        keys,
    }))
}