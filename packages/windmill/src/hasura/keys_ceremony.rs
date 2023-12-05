// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use braid_messages::newtypes::BatchNumber;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use sequent_core::services::connection;
use serde_json::Value;
use std::env;
use tracing::instrument;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_keys_ceremony.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertKeysCeremony;

#[instrument(skip(auth_headers))]
pub async fn insert_keys_ceremony(
    auth_headers: connection::AuthHeaders,
    id: String,
    tenant_id: String,
    election_event_id: String,
    trustee_ids: Vec<String>,
    status: Option<Value>,
    execution_status: Option<String>,
) -> Result<Response<insert_keys_ceremony::ResponseData>> {
    let variables = insert_keys_ceremony::Variables {
        id: id,
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        trustee_ids: Some(trustee_ids),
        status: status,
        execution_status: execution_status,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertKeysCeremony::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_keys_ceremony::ResponseData> = res.json().await?;
    response_body.ok()
}
