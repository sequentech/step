// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::connection;
use anyhow::Result;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use rocket::serde::json::Value;
use std::env;

type uuid = String;
type jsonb = Value;
type timestamptz = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_document.graphql",
    response_derives = "Debug"
)]
pub struct InsertDocument;

pub async fn perform_insert_document(
    auth_headers: connection::AuthHeaders,
    variables: insert_document::Variables,
) -> Result<Response<insert_document::ResponseData>> {
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertDocument::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_document::ResponseData> =
        res.json().await?;
    Ok(response_body)
}

pub async fn insert_document(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    name: String,
    media_type: String,
    size: i64,
) -> Result<Response<insert_document::ResponseData>> {
    let variables = insert_document::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        name: name,
        media_type: media_type,
        size: size,
    };
    perform_insert_document(auth_headers, variables).await
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_document.graphql",
    response_derives = "Debug"
)]
pub struct GetDocument;

pub async fn find_document(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    document_id: String,
) -> Result<Response<get_document::ResponseData>> {
    let variables = get_document::Variables {
        tenant_id: tenant_id,
        election_event_id: election_event_id,
        document_id: document_id,
    };
    let hasura_endpoint = env::var("HASURA_ENDPOINT")
        .expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetDocument::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_document::ResponseData> =
        res.json().await?;
    Ok(response_body)
}
