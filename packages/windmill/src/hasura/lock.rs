// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use sequent_core::services::connection;
use std::env;
use tracing::instrument;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/upsert_lock.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpsertLock;

#[instrument(skip(auth_headers))]
pub async fn upsert_lock(
    auth_headers: connection::AuthHeaders,
    key: String,
    value: String,
    expiry_date: Option<String>,
) -> Result<Response<upsert_lock::ResponseData>> {
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));

    let variables = upsert_lock::Variables {
        key,
        value,
        expiry_date,
    };

    let request_body = UpsertLock::build_query(variables);

    let client = reqwest::Client::new();

    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<upsert_lock::ResponseData> = res.json().await?;

    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/delete_lock.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct DeleteLock;

#[instrument(skip(auth_headers))]
pub async fn delete_lock(
    auth_headers: connection::AuthHeaders,
    key: String,
    value: String,
) -> Result<Response<delete_lock::ResponseData>> {
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));

    let variables = delete_lock::Variables {
        key,
        value,
    };

    let request_body = DeleteLock::build_query(variables);

    let client = reqwest::Client::new();

    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;

    let response_body: Response<delete_lock::ResponseData> = res.json().await?;

    response_body.ok()
}
