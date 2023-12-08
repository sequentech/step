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
    query_path = "src/graphql/insert_ballot_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertBallotPublication;

#[instrument(skip_all)]
pub async fn insert_ballot_publication(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
    user_id: String,
) -> Result<Response<insert_ballot_publication::ResponseData>> {
    let variables = insert_ballot_publication::Variables {
        tenant_id,
        election_event_id,
        election_ids,
        user_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertBallotPublication::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_ballot_publication::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_ballot_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetBallotPublication;

#[instrument(skip_all)]
pub async fn get_ballot_publication(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<Response<get_ballot_publication::ResponseData>> {
    let variables = get_ballot_publication::Variables {
        tenant_id,
        election_event_id,
        ballot_publication_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetBallotPublication::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_ballot_publication::ResponseData> = res.json().await?;
    response_body.ok()
}


#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_ballot_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpdateBallotPublication;

#[instrument(skip_all)]
pub async fn update_ballot_publication_d(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    is_published: bool,
) -> Result<Response<update_ballot_publication::ResponseData>> {
    let variables = update_ballot_publication::Variables {
        ballot_publication_id: ballot_publication_id,
        election_event_id: election_event_id,
        tenant_id: tenant_id,
        is_published: is_published,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateBallotPublication::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_ballot_publication::ResponseData> = res.json().await?;
    response_body.ok()
}