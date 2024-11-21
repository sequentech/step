// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use strum_macros::Display;
use strum_macros::EnumString;
use tracing::instrument;

use crate::hasura::event_execution;
use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use sequent_core::services::connection;
use std::str::FromStr;

#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
pub enum EventExecutionState {
    Started,
    Success,
    Failure,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
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

#[instrument(skip_all, err)]
pub async fn insert_event_execution(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    scheduled_event_id: String,
    execution_state: EventExecutionState,
    execution_payload: Value,
    result_payload: Option<Value>,
) -> Result<Response<insert_event_execution::ResponseData>> {
    let variables = insert_event_execution::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        scheduled_event_id: scheduled_event_id,
        execution_state: Some(execution_state.to_string()),
        execution_payload: execution_payload,
        result_payload: result_payload,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertEventExecution::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_event_execution::ResponseData> = res.json().await?;
    response_body.ok()
}
