// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::serde::json::Value;
use std::env;
use tracing::instrument;

use crate::connection;
use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_status.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionStatus;

#[instrument(skip_all)]
pub async fn update_election_status(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    status: Value,
) -> Result<Response<update_election_status::ResponseData>> {
    let variables = update_election_status::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        election_id: election_id,
        status: status,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateElectionStatus::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_election_status::ResponseData> = res.json().await?;
    response_body.ok()
}
