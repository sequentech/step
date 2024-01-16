// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::anyhow;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_client_credentials;
use sequent_core::types::hasura_types::Document;
use tracing::instrument;

use crate::hasura;
use crate::services::date::ISO8601;
use crate::services::s3;
use crate::types::error::Result;

#[instrument(skip(bytes, auth_headers), err)]
pub async fn upload_and_return_document(
    bytes: Vec<u8>,
    media_type: String,
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    name: String,
) -> Result<Document> {
    let size = bytes.len();

    let new_document = hasura::document::insert_document(
        auth_headers,
        tenant_id.clone(),
        Some(election_event_id.clone()),
        name,
        media_type.clone(),
        size as i64,
        false,
    )
    .await?;

    let document = &new_document
        .data
        .ok_or(anyhow!("expected data"))?
        .insert_sequent_backend_document
        .ok_or(anyhow!("expected document"))?
        .returning[0];

    let document_id = document.id.clone();

    let document_s3_key = s3::get_document_key(tenant_id, election_event_id, document_id);

    s3::upload_to_s3(
        &bytes,
        document_s3_key,
        media_type,
        s3::get_private_bucket()?,
    )
    .await?;

    Ok(Document {
        id: document.id.clone(),
        tenant_id: document.tenant_id.clone(),
        election_event_id: document.election_event_id.clone(),
        name: document.name.clone(),
        media_type: document.media_type.clone(),
        size: document.size.clone(),
        labels: document.labels.clone(),
        annotations: document.annotations.clone(),
        created_at: document
            .created_at
            .clone()
            .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
        last_updated_at: document
            .last_updated_at
            .clone()
            .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
        is_public: document.is_public.clone(),
    })
}

#[instrument(skip(auth_headers), err)]
pub async fn get_upload_url(
    auth_headers: connection::AuthHeaders,
    name: &str,
    media_type: &str,
    size: usize,
    tenant_id: &str,
    is_public: bool,
) -> Result<(Document, String)> {
    let document = &hasura::document::insert_document(
        auth_headers,
        tenant_id.to_string(),
        None,
        name.to_string(),
        media_type.to_string(),
        size as i64,
        is_public,
    )
    .await?
    .data
    .ok_or(anyhow!("expected data"))?
    .insert_sequent_backend_document
    .ok_or(anyhow!("expected document"))?
    .returning[0];
    let path = match is_public {
        true => s3::get_public_document_key(
            tenant_id.to_string(),
            document.id.clone(),
            name.to_string()
        ),
        false => s3::get_document_key(
            tenant_id.to_string(),
            Default::default(),
            document.id.clone(),
        ),
    };
    let url = s3::get_upload_url(path.to_string(), is_public).await?;

    let ret_document = Document {
        id: document.id.clone(),
        tenant_id: document.tenant_id.clone(),
        election_event_id: document.election_event_id.clone(),
        name: document.name.clone(),
        media_type: document.media_type.clone(),
        size: document.size.clone(),
        labels: document.labels.clone(),
        annotations: document.annotations.clone(),
        created_at: document
            .created_at
            .clone()
            .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
        last_updated_at: document
            .last_updated_at
            .clone()
            .map(|value| ISO8601::to_date(value.as_str()).unwrap()),
        is_public: document.is_public.clone(),
    };
    Ok((ret_document, url))
}

#[instrument(err)]
pub async fn fetch_document(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
) -> Result<String> {
    let auth_headers = get_client_credentials().await?;
    let document_result = hasura::document::find_document(
        auth_headers,
        tenant_id.clone(),
        election_event_id.clone(),
        document_id.clone(),
    )
    .await?;

    let document = &document_result
        .data
        .ok_or(anyhow!("expected data"))?
        .sequent_backend_document[0];

    let document_s3_key = s3::get_document_key(tenant_id.clone(), election_event_id, document_id);
    let bucket = if document.is_public.unwrap_or(false) {
        s3::get_public_bucket()?
    } else {
        s3::get_private_bucket()?
    };
    let url = s3::get_document_url(document_s3_key, bucket).await?;
    Ok(url)
}
