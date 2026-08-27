// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `get_upload_url` -> PUT, shared by the election-event template (JSON)
//! and the voters CSV — both are just "upload bytes, get a document id
//! back" (`packages/step-cli/src/utils/upload_file.rs:15-117`).

use anyhow::{bail, Context, Result};
use graphql_client::GraphQLQuery;

use crate::hasura::HasuraClient;
use crate::types::hasura::*;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_upload_url.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetUploadUrl;

pub async fn upload_document(
    client: &HasuraClient,
    http: &reqwest::Client,
    name: &str,
    media_type: &str,
    election_event_id: Option<String>,
    bytes: &[u8],
) -> Result<String> {
    let upload_variables = get_upload_url::Variables {
        name: name.to_string(),
        media_type: media_type.to_string(),
        size: bytes.len() as i64,
        is_public: false,
        is_local: Some(false),
        election_event_id,
    };
    let upload_data = client
        .data_or_bail::<GetUploadUrl>(upload_variables)
        .await
        .with_context(|| format!("failed to get an upload URL for `{name}`"))?;
    let upload = upload_data
        .get_upload_url
        .ok_or_else(|| anyhow::anyhow!("get_upload_url returned no data"))?;

    let put_response = http
        .put(&upload.url)
        .header("Content-Type", media_type)
        .body(bytes.to_vec())
        .send()
        .await
        .with_context(|| format!("failed to upload `{name}`"))?;
    if !put_response.status().is_success() {
        let status = put_response.status();
        let body = put_response.text().await.unwrap_or_default();
        bail!("upload of `{name}` failed (HTTP {status}): {body}");
    }

    Ok(upload.document_id)
}
