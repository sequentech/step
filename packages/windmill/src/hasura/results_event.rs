// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use std::env;
use tracing::instrument;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use sequent_core::services::connection;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_results_event.graphql",
    response_derives = "Debug"
)]
pub struct InsertResultsEvent;

#[instrument(skip_all, err)]
pub async fn insert_results_event(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Response<insert_results_event::ResponseData>> {
    let variables = insert_results_event::Variables {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertResultsEvent::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key.clone(), auth_headers.value.clone())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_results_event::ResponseData> = res.json().await?;
    response_body.ok()
}
