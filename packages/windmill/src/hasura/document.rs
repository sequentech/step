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

#[instrument(skip_all, err)]
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

// #[derive(GraphQLQuery)]
// #[graphql(
//     schema_path = "src/graphql/schema.json",
//     query_path = "src/graphql/get_document.graphql",
//     response_derives = "Debug"
// )]
// pub struct GetDocument;

#[instrument(skip_all, err)]
pub async fn find_document(
    hasura_transaction: &Transaction<'_>,
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    // election_event_id: String,
    document_id: String,
    // ) -> Result<Response<get_document::ResponseData>> {
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                id,
                election_event_id,
                tenant_id,
                name,
                media_type,
                size,
                labels,
                annotations,
                created_at,
                last_updated_at,
                is_public
            FROM
                sequent_backend.document
            WHERE
                id = $1
                -- AND election_event_id = $2
                AND tenant_id = $2;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&document_id, /* &election_event_id */ &tenant_id],
        )
        .await
        .map_err(|err| anyhow!("Error running the find_document query: {}", err))?;

    dbg!(&rows);

    let document_ids: Vec<String> = rows
        .into_iter()
        .map(|row| -> Result<String> {
            Ok(row
                .try_get::<&str, String>("id")
                .map_err(|err| anyhow!("Error getting the document id of a row: {}", err))?)
        })
        .collect::<Result<Vec<String>>>()
        .map_err(|err| anyhow!("Error getting the documents ids: {}", err))?;

    dbg!(&document_ids);

    // let variables = get_document::Variables {
    //     tenant_id: tenant_id.to_string(),
    //     election_event_id: election_event_id.to_string(),
    //     document_id: document_id.to_string(),
    // };
    //
    // let hasura_endpoint =
    //     env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));

    // let request_body = GetDocument::build_query(variables);

    // let client = reqwest::Client::new();
    // let res = client
    //     .post(hasura_endpoint)
    //     .header(auth_headers.key, auth_headers.value)
    //     .json(&request_body)
    //     .send()
    //     .await?;
    //
    // let response_body: Response<get_document::ResponseData> = res.json().await?;
    //
    // response_body.ok()

    Ok(())
}
