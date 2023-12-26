// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use anyhow::{anyhow, Result};
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tokio_postgres::row::Row;
use tokio_postgres::types::{BorrowToSql, ToSql, Type as SqlType};
use tracing::{event, instrument, Level};
use windmill::services::database::{
    get_hasura_pool, get_keycloak_pool, PgConfig,
};
use windmill::services::election_event_statistics::get_count_distinct_voters;

use crate::types::resources::{
    Aggregate, DataList, OrderDirection, TotalAggregate,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventStatsInput {
    election_event_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventStatsOutput {
    //total_eligible_voters: i64,
    total_distinct_voters: i64,
    //total_areas: i64,
    //total_elections: i64,
}

#[instrument(skip(claims))]
#[post("/election-event/stats", format = "json", data = "<body>")]
pub async fn get_election_event_stats(
    body: Json<ElectionEventStatsInput>,
    claims: JwtClaims,
) -> Result<Json<ElectionEventStatsOutput>, (Status, String)> {
    authorize(&claims, true, None, vec![Permissions::ADMIN_DASHBOARD_VIEW])?;
    let input = body.into_inner();
    let tenant_id: String = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error loading hasura db client: {err}"),
            )
        })?;
    let mut hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error creating a transaction: {err}"),
            )
        })?;

    let total_distinct_voters: i64 = get_count_distinct_voters(
        &hasura_transaction,
        &tenant_id.as_str(),
        &input.election_event_id.as_str(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error retrieving total_distinct_voters: {err}"),
        )
    })?;

    Ok(Json(ElectionEventStatsOutput {
        total_distinct_voters,
    }))
}
