// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ::uuid::Uuid as UuidType;
use anyhow::{anyhow, Context, Result};
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use std::env;
use tracing::instrument;

use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use sequent_core::services::connection;
use tokio_postgres::row::Row;

use deadpool_postgres::Transaction;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_document.graphql",
    response_derives = "Debug"
)]
pub struct InsertDocument;

#[instrument(skip(auth_headers), err)]
pub async fn insert_document(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: Option<String>,
    name: String,
    media_type: String,
    size: i64,
    is_public: bool,
    document_id: Option<String>,
) -> Result<Response<insert_document::ResponseData>> {
    let variables = insert_document::Variables {
        tenant_id,
        document_id: document_id.unwrap_or(UuidType::new_v4().to_string()),
        election_event_id,
        name,
        media_type,
        size,
        is_public,
    };

    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertDocument::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_document::ResponseData> = res.json().await?;

    response_body.ok()
}
