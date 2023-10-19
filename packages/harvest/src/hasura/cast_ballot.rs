// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::serde::json::Value;
use rocket::serde::{Deserialize, Serialize};
use std::env;
use tracing::{event, instrument, Level};
use windmill::connection;

pub use crate::hasura::types::*;
use crate::services::to_result::ToResult;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_cast_ballots.graphql",
    response_derives = "Debug"
)]
pub struct GetCastBallots;

#[instrument(skip_all)]
pub async fn find_ballots(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<Response<get_cast_ballots::ResponseData>> {
    let variables = get_cast_ballots::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
    };
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetCastBallots::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_cast_ballots::ResponseData> =
        res.json().await?;
    response_body.ok()
}
