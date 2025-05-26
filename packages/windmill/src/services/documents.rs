// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::insert_document;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use deadpool_postgres::Transaction;
use deadpool_postgres::{Client as DbClient, Transaction as _};

use sequent_core::services::connection;
use sequent_core::types::hasura::core::Document;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

use crate::postgres;
use crate::types::error::Result;
use sequent_core::services::date::ISO8601;
use sequent_core::services::s3;

#[instrument(skip(hasura_transaction), err)]
pub async fn upload_and_return_document(
    hasura_transaction: &Transaction<'_>,
    file_path: &str,
    file_size: u64,
    media_type: &str,
    tenant_id: &str,
    election_event_id: Option<String>,
    name: &str,
    document_id: Option<String>,
    is_public: bool,
) -> AnyhowResult<Document> {
    let document = insert_document(
        hasura_transaction,
        tenant_id,
        election_event_id.clone(),
        name,
        media_type,
        file_size.try_into()?,
        is_public,
        document_id,
    )
    .await?;

    info!("Document inserted {document:?}");

    let (document_s3_key, bucket) = match is_public {
        true => {
            let document_s3_key = s3::get_public_document_key(tenant_id, &document.id, name);
            let bucket = s3::get_public_bucket()?;

            (document_s3_key, bucket)
        }
        false => {
            let document_s3_key =
                s3::get_document_key(tenant_id, election_event_id.as_deref(), &document.id, name);
            let bucket = s3::get_private_bucket()?;

            (document_s3_key, bucket)
        }
    };

    s3::upload_file_to_s3(
        /* key */ document_s3_key,
        /* is_public: always false because it's windmill that uploads the file */ false,
        /* s3_bucket */ bucket,
        /* media_type */ media_type.to_string(),
        /* file_path */ file_path.to_string(),
        /* cache_control_policy */ None,
        Some(name.to_string()),
    )
    .await
    .with_context(|| "Failed uploading file to s3")?;

    Ok(document)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_upload_url(
    hasura_transaction: &Transaction<'_>,
    name: &str,
    media_type: &str,
    size: usize,
    tenant_id: &str,
    is_public: bool,
    is_local: Option<bool>,
    election_event_id: Option<String>,
) -> Result<(Document, String)> {
    let document = insert_document(
        &hasura_transaction,
        &tenant_id,
        election_event_id.clone(),
        &name,
        &media_type,
        size.try_into()?,
        is_public,
        None,
    )
    .await
    .map_err(|err| format!("Error inserting document: {:?}", err))?;

    let path = match is_public {
        true => s3::get_public_document_key(&tenant_id, &document.id, &name),
        false => s3::get_document_key(
            &tenant_id.to_string(),
            election_event_id.clone().as_deref(),
            &document.id,
            &name.to_string(),
        ),
    };
    let url = s3::get_upload_url(path.to_string(), is_public, is_local.unwrap_or(false)).await?;

    Ok((document, url))
}

#[instrument(err)]
pub async fn get_document_url(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    document_id: &str,
) -> anyhow::Result<Option<String>> {
    let document = postgres::document::get_document(
        hasura_transaction,
        tenant_id,
        election_event_id.map(|id| id.to_string()),
        document_id,
    )
    .await?;

    let Some(document) = document else {
        info!("document is None");
        return Ok(None);
    };

    let document_s3_key = if document.is_public.unwrap_or(false) {
        s3::get_public_document_key(
            tenant_id,
            document_id,
            &document.name.clone().unwrap_or_default(),
        )
    } else {
        s3::get_document_key(
            tenant_id,
            election_event_id,
            document_id,
            &document.name.clone().unwrap_or_default(),
        )
    };

    let bucket = if document.is_public.unwrap_or(false) {
        s3::get_public_bucket()?
    } else {
        s3::get_private_bucket()?
    };

    let url = s3::get_document_url(document_s3_key, bucket).await?;

    Ok(Some(url))
}

#[instrument(err)]
pub async fn get_document_as_temp_file(
    tenant_id: &str,
    document: &Document,
) -> anyhow::Result<NamedTempFile> {
    let s3_bucket = s3::get_private_bucket()?;
    let document_name = document.name.clone().unwrap_or_default();
    let election_event_id = document.election_event_id.clone();

    // Obtain the key for the document in S3
    let document_s3_key = s3::get_document_key(
        tenant_id,
        election_event_id.as_deref(),
        &document.id,
        &document_name,
    );

    let file = s3::get_object_into_temp_file(
        s3_bucket.as_str(),
        document_s3_key.as_str(),
        &document_name,
        ".tmp",
    )
    .await
    .with_context(|| "Failed to get S3 object into temporary file")?;

    // Return the temporary file and the separator as a tuple
    Ok(file)
}
