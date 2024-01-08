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
    query_path = "src/graphql/insert_results_contest_candidate.graphql",
    response_derives = "Debug"
)]
pub struct InsertResultsContestCandidate;

#[instrument(skip_all, err)]
pub async fn insert_results_contest_candidate(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    candidate_id: &str,
    results_event_id: &str,
    cast_votes: Option<i64>,
    cast_votes_percent: Option<f64>,
    winning_position: Option<i64>,
    points: Option<i64>,
) -> Result<Response<insert_results_contest_candidate::ResponseData>> {
    let variables = insert_results_contest_candidate::Variables {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: Some(election_id.to_string()),
        contest_id: Some(contest_id.to_string()),
        candidate_id: Some(candidate_id.to_string()),
        results_event_id: results_event_id.to_string(),
        cast_votes,
        cast_votes_percent,
        winning_position,
        points,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertResultsContestCandidate::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key.clone(), auth_headers.value.clone())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_results_contest_candidate::ResponseData> =
        res.json().await?;
    response_body.ok()
}
