// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use serde_json::Value;
use std::env;
use tracing::instrument;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use sequent_core::services::connection;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_results_area_contest.graphql",
    response_derives = "Debug"
)]
pub struct InsertResultsAreaContest;

#[instrument(skip(auth_headers), err)]
pub async fn insert_results_area_contest(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    contest_id: &str,
    area_id: &str,
    results_event_id: &str,
    elegible_census: Option<i64>,
    total_votes: Option<i64>,
    total_votes_percent: Option<f64>,
    total_auditable_votes: Option<i64>,
    total_auditable_votes_percent: Option<f64>,
    total_valid_votes: Option<i64>,
    total_valid_votes_percent: Option<f64>,
    total_invalid_votes: Option<i64>,
    total_invalid_votes_percent: Option<f64>,
    explicit_invalid_votes: Option<i64>,
    explicit_invalid_votes_percent: Option<f64>,
    implicit_invalid_votes: Option<i64>,
    implicit_invalid_votes_percent: Option<f64>,
    blank_votes: Option<i64>,
    blank_votes_percent: Option<f64>,
    annotations: Option<Value>,
) -> Result<Response<insert_results_area_contest::ResponseData>> {
    let variables = insert_results_area_contest::Variables {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
        election_id: Some(election_id.to_string()),
        contest_id: Some(contest_id.to_string()),
        area_id: Some(area_id.to_string()),
        results_event_id: results_event_id.to_string(),
        elegible_census,
        total_votes,
        total_votes_percent,
        total_auditable_votes,
        total_auditable_votes_percent,
        total_valid_votes,
        total_valid_votes_percent,
        total_invalid_votes,
        total_invalid_votes_percent,
        explicit_invalid_votes,
        explicit_invalid_votes_percent,
        implicit_invalid_votes,
        implicit_invalid_votes_percent,
        blank_votes,
        blank_votes_percent,
        annotations,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertResultsAreaContest::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key.clone(), auth_headers.value.clone())
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_results_area_contest::ResponseData> = res.json().await?;
    response_body.ok()
}
