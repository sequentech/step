// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use serde_json::Value;
use std::env;
use tracing::instrument;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use sequent_core::services::connection;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_all_elections_for_event.graphql",
    response_derives = "Debug, Clone, Deserialize"
)]
pub struct GetAllElectionsForEvent;

#[instrument(skip(auth_headers), err)]
pub async fn get_all_elections_for_event(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<Response<get_all_elections_for_event::ResponseData>> {
    let variables = get_all_elections_for_event::Variables {
        tenant_id,
        election_event_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetAllElectionsForEvent::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_all_elections_for_event::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_statistics.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionStatistics;

#[instrument(skip_all, err)]
pub async fn update_election_statistics(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    statistics: Value,
) -> Result<Response<update_election_statistics::ResponseData>> {
    let variables = update_election_statistics::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        election_id: election_id,
        statistics: statistics,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateElectionStatistics::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_election_statistics::ResponseData> = res.json().await?;
    response_body.ok()
}
