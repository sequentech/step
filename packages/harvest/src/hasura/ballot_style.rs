// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::connection;
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::serde::json::Value;
use std::env;
use tracing::instrument;

type uuid = String;
type jsonb = Value;
type timestamptz = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_ballot_style_area.graphql",
    response_derives = "Debug"
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
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetBallotStyleArea::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_ballot_style_area::ResponseData> =
        res.json().await?;
    Ok(response_body)
}
