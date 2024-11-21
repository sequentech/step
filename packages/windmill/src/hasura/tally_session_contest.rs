// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use b3::messages::newtypes::BatchNumber;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use sequent_core::services::connection;
use std::env;
use tracing::instrument;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_tally_session_contest.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertTallySessionContest;

#[instrument(skip(auth_headers), err)]
pub async fn insert_tally_session_contest(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    area_id: String,
    contest_id: String,
    session_id: BatchNumber,
    tally_session_id: String,
    election_id: String,
) -> Result<Response<insert_tally_session_contest::ResponseData>> {
    let variables = insert_tally_session_contest::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        area_id: area_id,
        contest_id: contest_id,
        session_id: session_id as i64,
        tally_session_id: tally_session_id,
        election_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertTallySessionContest::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_tally_session_contest::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_tally_session_contest.graphql",
    response_derives = "Debug"
)]
pub struct GetTallySessionContest;

#[instrument(skip(auth_headers), err)]
pub async fn get_tally_session_contest(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    tally_session_contest_id: String,
) -> Result<Response<get_tally_session_contest::ResponseData>> {
    let variables = get_tally_session_contest::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        tally_session_id: tally_session_id,
        tally_session_contest_id: tally_session_contest_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetTallySessionContest::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_tally_session_contest::ResponseData> = res.json().await?;
    response_body.ok()
}
