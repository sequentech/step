// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use crate::types::resources::{Aggregate, DataList, TotalAggregate};
use anyhow::Result;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use windmill::services::cast_votes::{
    get_count_votes_per_day, get_top_count_votes_by_ip, CastVoteCountByIp,
    CastVotesPerDay, ListCastVotesByIpFilter,
};
use windmill::services::database::{get_hasura_pool, get_keycloak_pool};
use windmill::services::election_event_statistics::{
    get_count_areas, get_count_distinct_voters, get_count_elections,
};
use windmill::services::users::count_keycloak_enabled_users;

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventStatsInput {
    election_event_id: String,
    start_date: String,
    end_date: String,
    user_timezone: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ElectionEventStatsOutput {
    total_eligible_voters: i64,
    total_distinct_voters: i64,
    total_areas: i64,
    total_elections: i64,
    votes_per_day: Vec<CastVotesPerDay>,
}

#[instrument(skip(claims))]
#[post("/election-event/stats", format = "json", data = "<body>")]
pub async fn get_election_event_stats(
    body: Json<ElectionEventStatsInput>,
    claims: JwtClaims,
) -> Result<Json<ElectionEventStatsOutput>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_DASHBOARD_VIEW],
    )?;
    let input = body.into_inner();
    let tenant_id: String = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error loading hasura db client: {err}"),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error creating a transaction: {err}"),
            )
        })?;
    let mut keycloak_db_client: DbClient =
        get_keycloak_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error loading keycloak db client: {err}"),
            )
        })?;
    let keycloak_transaction =
        keycloak_db_client.transaction().await.map_err(|err| {
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
    let total_elections: i64 = get_count_elections(
        &hasura_transaction,
        &tenant_id.as_str(),
        &input.election_event_id.as_str(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error retrieving total_elections: {err}"),
        )
    })?;

    let total_areas: i64 = get_count_areas(
        &hasura_transaction,
        &tenant_id.as_str(),
        &input.election_event_id.as_str(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error retrieving total_areas: {err}"),
        )
    })?;

    let votes_per_day: Vec<CastVotesPerDay> = get_count_votes_per_day(
        &hasura_transaction,
        &tenant_id.as_str(),
        &input.election_event_id.as_str(),
        &input.start_date.as_str(),
        &input.end_date.as_str(),
        None,
        &input.user_timezone.as_str(),
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error retrieving votes_per_day: {err}"),
        )
    })?;

    let realm_name =
        get_event_realm(tenant_id.as_str(), input.election_event_id.as_str());

    let total_eligible_voters: i64 =
        count_keycloak_enabled_users(&keycloak_transaction, &realm_name)
            .await
            .map_err(|e| (Status::InternalServerError, format!("{:?}", e)))?;

    Ok(Json(ElectionEventStatsOutput {
        total_distinct_voters,
        total_areas,
        total_eligible_voters: total_eligible_voters.into(),
        total_elections: total_elections.into(),
        votes_per_day,
    }))
}

#[derive(Deserialize, Debug)]
pub struct GetTopCastVotesByIp {
    election_event_id: String,
    limit: Option<i32>,
    offset: Option<i32>,
    ip: Option<String>,
    country: Option<String>,
    election_id: Option<String>,
}

#[instrument(skip(claims))]
#[post("/election-event/top-votes-by-ip", format = "json", data = "<body>")]
pub async fn get_election_event_top_votes_by_ip(
    claims: JwtClaims,
    body: Json<GetTopCastVotesByIp>,
) -> Result<Json<DataList<CastVoteCountByIp>>, (Status, String)> {
    authorize(
        &claims,
        true,
        Some(claims.hasura_claims.tenant_id.clone()),
        vec![Permissions::ADMIN_DASHBOARD_VIEW],
    )?;

    let input = body.into_inner();
    let tenant_id: String = claims.hasura_claims.tenant_id.clone();

    let mut hasura_db_client: DbClient =
        get_hasura_pool().await.get().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error loading hasura db client: {err}"),
            )
        })?;
    let hasura_transaction =
        hasura_db_client.transaction().await.map_err(|err| {
            (
                Status::InternalServerError,
                format!("Error creating a transaction: {err}"),
            )
        })?;
    let filter = ListCastVotesByIpFilter {
        election_id: input.election_id.clone(),
        limit: input.limit,
        offset: input.offset,
        ip: input.ip,
        country: input.country,
    };

    let (cast_votes_by_ip, count) = get_top_count_votes_by_ip(
        &hasura_transaction,
        &tenant_id,
        &input.election_event_id,
        filter,
    )
    .await
    .map_err(|err| {
        (
            Status::InternalServerError,
            format!("Error getting top cast votes by ip: {err}"),
        )
    })?;

    Ok(Json(DataList {
        items: cast_votes_by_ip,
        total: TotalAggregate {
            aggregate: Aggregate {
                count: count as i64,
            },
        },
    }))
}
