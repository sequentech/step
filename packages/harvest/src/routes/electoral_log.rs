// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::electoral_log::{
    filter_all_electoral_log, list_electoral_log as get_logs, ElectoralLogRow,
    GetElectoralLogBody,
};
use windmill::types::resources::DataList;

#[instrument]
#[post("/immudb/electoral-log", format = "json", data = "<body>")]
pub async fn list_electoral_log(
    body: Json<GetElectoralLogBody>,
    claims: JwtClaims,
) -> Result<Json<DataList<ElectoralLogRow>>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::LOGS_READ],
    )?;

    let ret_val = match &input.filter {
        Some(filter) if !filter.is_empty() => filter_all_electoral_log(input)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?,
        _ => get_logs(input)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?,
    };

    Ok(Json(ret_val))
}
