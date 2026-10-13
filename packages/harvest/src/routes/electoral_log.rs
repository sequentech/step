// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::{authorize, permitted_event_election_ids};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::database::get_hasura_pool;
use windmill::services::electoral_log::{
    list_electoral_log as get_logs, list_tenant_log as get_tenant_logs,
    ElectoralLogRow, GetElectoralLogBody,
};
use windmill::types::resources::DataList;

#[instrument]
#[post("/electoral-log", format = "json", data = "<body>")]
pub async fn list_electoral_log(
    body: Json<GetElectoralLogBody>,
    claims: JwtClaims,
) -> Result<Json<DataList<ElectoralLogRow>>, (Status, String)> {
    let mut input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::LOGS_READ],
    )?;
    input.permitted_election_ids = {
        let mut hasura_db_client: DbClient = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
        let hasura_transaction = hasura_db_client
            .transaction()
            .await
            .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
        permitted_event_election_ids(
            &hasura_transaction,
            &claims,
            &input.tenant_id,
            &input.election_event_id,
        )
        .await?
    };
    let ret_val = get_logs(input)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(ret_val))
}

/// The tenant's log: the activity that belongs to none of its election events.
#[instrument]
#[post("/tenant-log", format = "json", data = "<body>")]
pub async fn list_tenant_log(
    body: Json<GetElectoralLogBody>,
    claims: JwtClaims,
) -> Result<Json<DataList<ElectoralLogRow>>, (Status, String)> {
    let mut input = body.into_inner();
    authorize(
        &claims,
        true,
        Some(input.tenant_id.clone()),
        vec![Permissions::TENANT_LOGS_READ],
    )?;
    // The tenant's log has no election event, election or area to select by.
    input.election_event_id = String::new();
    input.election_id = None;
    input.area_ids = None;
    input.communications = None;
    let ret_val = get_tenant_logs(input)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(ret_val))
}
