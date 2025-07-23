// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;
use windmill::services::plugins_manager::plugin_manager::{
    self, PluginManager,
};

#[derive(Deserialize, Debug)]
pub struct PluginsRouteInput {
    path: String,
    data: Value,
}

#[derive(Deserialize, Debug, Serialize)]
pub struct PluginsRouteOutput {
    data: Value,
}

#[instrument(skip(claims))]
#[post("/plugin", format = "json", data = "<body>")]
pub async fn plugin_routes(
    claims: jwt::JwtClaims,
    body: Json<PluginsRouteInput>,
) -> Result<Json<PluginsRouteOutput>, (Status, String)> {
    let input = body.into_inner();
    let plugin_manager = plugin_manager::get_plugin_manager()
        .await
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;

    let mut route_data = input.data;

    let claims_json_string: String = serde_json::to_string(&claims)
    .expect("Failed to serialize JwtClaims to string");

    route_data["claims"] = serde_json::Value::String(claims_json_string);

    let res: Value = plugin_manager
        .call_route(&input.path, route_data.to_string())
        .await
        .and_then(|result_str| {
            serde_json::from_str::<Value>(&result_str)
                .map_err(|e| anyhow::anyhow!(e))
        })
        .map_err(|e| (Status::InternalServerError, e.to_string()))?;

    Ok(Json(PluginsRouteOutput { data: res }))
}
