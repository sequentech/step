// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::response::Debug;
use rocket::serde::json::Json;
use tracing::instrument;
use windmill::services::jwks::{JWKKey, get_jwks, JwksOutput};

#[instrument]
#[get("/jwks.json")]
pub async fn get_jwks_json() -> Result<Json<JwksOutput>, Debug<anyhow::Error>> {
    let keys: Vec<JWKKey> = get_jwks().await.map_err(|e| anyhow::Error::from(e))?;
    Ok(Json(JwksOutput {
        keys,
    }))
}