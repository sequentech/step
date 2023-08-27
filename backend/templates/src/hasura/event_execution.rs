// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::connection;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::response::Debug;
use rocket::serde::json::{Json, Value};
use rocket::serde::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;
use strum_macros::Display;
use strum_macros::EnumString;

type uuid = String;
type jsonb = Value;
type timestamptz = String;

#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
#[serde(crate = "rocket::serde")]
pub enum EventExecutionState {
    Started,
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(crate = "rocket::serde")]
pub struct EventExecution {
    pub id: String,
    pub tenant_id: Option<String>,
    pub election_event_id: Option<String>,
    pub scheduled_event_id: String,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub execution_state: Option<EventExecutionState>,
    pub execution_payload: Option<Value>,
    pub result_payload: Option<Value>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
}

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
    execution_state: EventExecutionState,
    execution_payload: Value,
    result_payload: Option<Value>,
) -> Result<Response<insert_event_execution::ResponseData>, reqwest::Error> {
    let variables = insert_event_execution::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        scheduled_event_id: scheduled_event_id,
        execution_state: Some(execution_state.to_string()),
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
