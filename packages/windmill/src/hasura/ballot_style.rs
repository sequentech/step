// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::serde::json::Value;
use rocket::serde::{Deserialize, Serialize};
use std::env;
use tracing::{event, instrument, Level};
use windmill::connection;

pub use crate::hasura::types::*;
use crate::services::to_result::ToResult;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_ballot_style_area.graphql",
    response_derives = "Debug,Clone"
)]
pub struct GetBallotStyleArea;

#[instrument(skip_all)]
pub async fn get_ballot_style_area(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    area_id: String,
) -> Result<Response<get_ballot_style_area::ResponseData>> {
    let variables = get_ballot_style_area::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        area_id: area_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetBallotStyleArea::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_ballot_style_area::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_ballot_style.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertBallotStyle;

#[instrument(skip_all)]
pub async fn insert_ballot_style(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    election_id: String,
    area_id: String,
    ballot_eml: Option<String>,
    ballot_signature: Option<String>,
    status: Option<String>,
) -> Result<Response<insert_ballot_style::ResponseData>> {
    let variables = insert_ballot_style::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        election_id: election_id,
        area_id: area_id,
        ballot_eml: ballot_eml,
        ballot_signature: ballot_signature,
        status: status,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertBallotStyle::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_ballot_style::ResponseData> = res.json().await?;
    response_body.ok()
}
