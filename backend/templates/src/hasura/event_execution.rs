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

type uuid = String;
type jsonb = Value;
type timestamptz = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_event_execution.graphql",
    response_derives = "Debug"
)]
pub struct InsertEventExecution;

pub async fn insert_event_execution(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    execution_state: String,
    execution_payload: Value,
    result_payload: Option<Value>,
) -> Result<Response<insert_event_execution::ResponseData>, reqwest::Error> {
    let variables = insert_event_execution::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        scheduled_event_id: scheduled_event_id,
        execution_state: Some(execution_state),
        execution_payload: execution_payload,
        result_payload: result_payload,
    };
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertEventExecution::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_event_execution::ResponseData> =
        res.json().await?;
    Ok(response_body)
}
