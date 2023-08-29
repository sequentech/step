// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::connection;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::response::Debug;
use rocket::serde::json::{Json, Value};
use serde::Deserialize;
use std::env;
use anyhow::Result;

type uuid = String;
type jsonb = Value;
type timestamptz = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_status.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionStatus;

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
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateElectionStatus::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_election_status::ResponseData> =
        res.json().await?;
    Ok(response_body)
}