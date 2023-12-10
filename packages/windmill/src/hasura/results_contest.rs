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
    query_path = "src/graphql/insert_results_contest.graphql",
    response_derives = "Debug"
)]
pub struct InsertResultsContest;

#[instrument(skip_all)]
pub async fn insert_results_contest(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    results_event_id: &str,
    elegible_census: &Option<i64>,
    total_valid_votes: &Option<i64>,
    explicit_invalid_votes: &Option<i64>,
    implicit_invalid_votes: &Option<i64>,
    blank_votes: &Option<i64>,
    voting_type: &Option<String>,
    counting_algorithm: &Option<String>,
    name: &Option<String>,
) -> Result<Response<insert_results_contest::ResponseData>> {
    let variables = insert_results_contest::Variables {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: Some(election_id.to_string()),
        contest_id: Some(contest_id.to_string()),
        results_event_id: results_event_id.to_string(),
        elegible_census: elegible_census.clone(),
        total_valid_votes: total_valid_votes.clone(),
        explicit_invalid_votes: explicit_invalid_votes.clone(),
        implicit_invalid_votes: implicit_invalid_votes.clone(),
        blank_votes: blank_votes.clone(),
        voting_type: voting_type.clone(),
        counting_algorithm: counting_algorithm.clone(),
        name: name.clone(),
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertResultsContest::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key.clone(), auth_headers.value.clone())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_results_contest::ResponseData> = res.json().await?;
    response_body.ok()
}
