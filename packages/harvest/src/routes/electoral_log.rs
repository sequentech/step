// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::routes::immudb_log_audit::{create_named_param, get_immudb_client};
use crate::services::authorization::authorize;
use crate::services::electoral_log;
use crate::types::resources::{
    Aggregate, DataList, OrderDirection, TotalAggregate,
};
use anyhow::{anyhow, Context, Result};
use immu_board::assign_value;
use immudb_rs::{sql_value::Value, Client, NamedParam, Row, SqlValue};
use rocket::serde::json::Json;
use rocket::{http::Status, response::Debug};
use sequent_core::services::connection;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use strum_macros::{Display, EnumString, ToString};
use tracing::instrument;
use windmill::services::database::PgConfig;

#[instrument]
#[post("/immudb/electoral-log", format = "json", data = "<body>")]
pub async fn list_electoral_log(
    body: Json<electoral_log::GetElectoralLogBody>,
    claims: JwtClaims,
) -> Result<Json<DataList<electoral_log::ElectoralLogRow>>, (Status, String)> {
    let input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::LOGS_READ],
    )?;
    let ret_val = electoral_log::list_electoral_log(input)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(ret_val))
}
