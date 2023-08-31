// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::connection;
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::response::Debug;
use rocket::serde::json::{Json, Value};
use serde::Deserialize;
use std::env;

type uuid = String;
type jsonb = Value;
type timestamptz = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_event_board.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionEventBoard;

pub async fn update_election_event_board(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    board: Value,
) -> Result<()> {
    let variables = update_election_event_board::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        board: board,
    };
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateElectionEventBoard::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let _response_body: Response<update_election_event_board::ResponseData> =
        res.json().await?;
    Ok(())
}
