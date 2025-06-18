// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Context, Result};
use electoral_log::client::types::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use tracing::instrument;
use windmill::services::electoral_log::{
    count_electoral_log, list_electoral_log as windmill_list_electoral_log,
    ElectoralLogRow,
};
use windmill::types::resources::DataList;

#[instrument(skip(claims))]
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

    let (data_res, count_res) = tokio::join!(
        windmill_list_electoral_log(input.clone()),
        count_electoral_log(input)
    );

    let mut data = data_res.map_err(|e| {
        (
            Status::InternalServerError,
            format!("Eror listing electoral log: {e:?}"),
        )
    })?;
    data.total.aggregate.count = count_res.map_err(|e| {
        (
            Status::InternalServerError,
            format!("Error counting electoral log: {e:?}"),
        )
    })?;

    Ok(Json(data))
}
