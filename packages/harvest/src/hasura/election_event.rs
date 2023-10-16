// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::serde::json::Value;
use std::env;
use tracing::instrument;

use crate::connection;
pub use crate::hasura::types::*;
use crate::services::to_result::ToResult;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_event_board.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionEventBoard;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_event_status.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionEventStatus;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_election_event_public_key.graphql",
    response_derives = "Debug"
)]
pub struct UpdateElectionEventPublicKey;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_election_event.graphql",
    response_derives = "Debug"
)]
pub struct GetElectionEvent;

#[instrument(skip_all)]
pub async fn update_election_event_board(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    board: Value,
) -> Result<Response<update_election_event_board::ResponseData>> {
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
    let response_body: Response<update_election_event_board::ResponseData> =
        res.json().await?;
    response_body.ok()
}

#[instrument(skip_all)]
pub async fn update_election_event_status(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    status: Value,
) -> Result<Response<update_election_event_status::ResponseData>> {
    let variables = update_election_event_status::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        status: status,
    };
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateElectionEventStatus::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_election_event_status::ResponseData> =
        res.json().await?;
    response_body.ok()
}

#[instrument(skip_all)]
pub async fn get_election_event(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
) -> Result<Response<get_election_event::ResponseData>> {
    let variables = get_election_event::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
    };
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetElectionEvent::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_election_event::ResponseData> =
        res.json().await?;
    response_body.ok()
}

#[instrument(skip_all)]
pub async fn update_election_event_public_key(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    public_key: String,
) -> Result<Response<update_election_event_public_key::ResponseData>> {
    let variables = update_election_event_public_key::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        public_key: public_key,
    };
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateElectionEventPublicKey::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<
        update_election_event_public_key::ResponseData,
    > = res.json().await?;
    response_body.ok()
}
