// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use serde::Deserialize;
use serde_json::Value;
use tracing::instrument;
use windmill::services::plugins::plugin_manager;

#[derive(Deserialize, Debug)]
pub struct PluginsRouteInput {
    path: String,
    payload: Value,
}

#[instrument(skip(claims))]
#[post("/plugin", format = "json", data = "<body>")]
pub async fn plugin_routes(
    claims: jwt::JwtClaims,
    body: Json<PluginsRouteInput>,
) -> Result<Json<Value>, (Status, String)> {
    let input = body.into_inner();
    let plugin_manager = plugin_manager::get_plugin_manager();

    let mut full_payload = input.payload;
    full_payload["claims"] = serde_json::to_value(&claims).unwrap_or_default();

    plugin_manager
        .call_route(&input.path, &full_payload.to_string())
        .await
        .map(Json)
        .map_err(|e| (Status::InternalServerError, e.to_string()))
}
