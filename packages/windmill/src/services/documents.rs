// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::{insert_document, insert_document_with_annotations};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use deadpool_postgres::Transaction;
use deadpool_postgres::{Client as DbClient, Transaction as _};

use sequent_core::services::connection;
use sequent_core::types::hasura::core::{Document, DocumentAnnotations};
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
    upload_and_return_document_inner(
        hasura_transaction,
        file_path,
        file_size,
        media_type,
        tenant_id,
        election_event_id,
        name,
        document_id,
        is_public,
        None,
    )
    .await
}

#[instrument(skip(hasura_transaction, annotations), err)]
pub async fn upload_and_return_document_with_annotations(
    hasura_transaction: &Transaction<'_>,
    file_path: &str,
    file_size: u64,
    media_type: &str,
    tenant_id: &str,
    election_event_id: Option<String>,
    name: &str,
    document_id: Option<String>,
    is_public: bool,
    annotations: &DocumentAnnotations,
) -> AnyhowResult<Document> {
    upload_and_return_document_inner(
        hasura_transaction,
        file_path,
        file_size,
        media_type,
        tenant_id,
        election_event_id,
        name,
        document_id,
        is_public,
        Some(annotations),
    )
    .await
}

async fn upload_and_return_document_inner(
    hasura_transaction: &Transaction<'_>,
    file_path: &str,
    file_size: u64,
    media_type: &str,
    tenant_id: &str,
    election_event_id: Option<String>,
    name: &str,
    document_id: Option<String>,
    is_public: bool,
    annotations: Option<&DocumentAnnotations>,
) -> AnyhowResult<Document> {
    let document = insert_document_with_annotations(
        hasura_transaction,
        tenant_id,
        election_event_id.clone(),
        name,
        media_type,
        file_size.try_into()?,
        is_public,
        document_id,
        annotations,
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

/// Uploads a document to S3 public bucket and returns the created Document record.
/// The document is associated with the given election event ID and tenant ID.
/// The Document path does not include the document ID and will be used
/// for when the UI does not have access to the document ID.
#[instrument(skip(hasura_transaction), err)]
pub async fn upload_and_return_public_event_document(
    hasura_transaction: &Transaction<'_>,
    file_path: &str,
    file_size: u64,
    media_type: &str,
    tenant_id: &str,
    election_event_id: &str,
    name: &str,
    document_id: Option<String>,
) -> AnyhowResult<Document> {
    let document = insert_document(
        hasura_transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        name,
        media_type,
        file_size.try_into()?,
        true,
        document_id,
    )
    .await?;

    info!("Document inserted {document:?}");
    let document_s3_key =
        s3::get_public_election_event_document_name_key(tenant_id, election_event_id, name);
    let bucket = s3::get_public_bucket()?;

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

/// Where a document's file is in S3. A private file is under the election
/// event the document belongs to, whatever event the caller named: a tenant
/// log export is asked for with an empty event and stored with none.
pub fn stored_document_key(tenant_id: &str, document: &Document) -> String {
    let name = document.name.clone().unwrap_or_default();
    if document.is_public.unwrap_or(false) {
        return s3::get_public_document_key(tenant_id, &document.id, &name);
    }
    let election_event_id = document
        .election_event_id
        .as_deref()
        .filter(|id| !id.is_empty());
    s3::get_document_key(tenant_id, election_event_id, &document.id, &name)
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

    let document_s3_key = stored_document_key(tenant_id, &document);

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
    let s3_bucket = match document.is_public {
        Some(true) => s3::get_public_bucket()?,
        _ => s3::get_private_bucket()?,
    };

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

#[cfg(test)]
mod tests {
    use super::stored_document_key;
    use sequent_core::types::hasura::core::Document;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const EVENT: &str = "e003472c-4b31-4585-ae8c-73c2f216e887";
    const DOCUMENT: &str = "068b6853-3aa2-4cc7-9ec5-8dc8294bc1e4";

    fn document(election_event_id: Option<&str>, is_public: Option<bool>) -> Document {
        Document {
            id: DOCUMENT.to_string(),
            tenant_id: Some(TENANT.to_string()),
            election_event_id: election_event_id.map(str::to_string),
            name: Some("tenant-logs.csv".to_string()),
            media_type: None,
            size: None,
            labels: None,
            annotations: None,
            created_at: None,
            last_updated_at: None,
            is_public,
        }
    }

    #[test]
    fn a_tenant_document_is_stored_without_an_event() {
        assert_eq!(
            stored_document_key(TENANT, &document(None, None)),
            format!("tenant-{TENANT}/document-{DOCUMENT}/tenant-logs.csv")
        );
        assert_eq!(
            stored_document_key(TENANT, &document(Some(""), Some(false))),
            format!("tenant-{TENANT}/document-{DOCUMENT}/tenant-logs.csv")
        );
    }

    #[test]
    fn an_event_document_is_stored_under_its_event() {
        assert_eq!(
            stored_document_key(TENANT, &document(Some(EVENT), None)),
            format!("tenant-{TENANT}/event-{EVENT}/document-{DOCUMENT}/tenant-logs.csv")
        );
    }

    #[test]
    fn a_public_document_is_stored_by_its_tenant() {
        assert_eq!(
            stored_document_key(TENANT, &document(Some(EVENT), Some(true))),
            sequent_core::services::s3::get_public_document_key(
                TENANT,
                DOCUMENT,
                "tenant-logs.csv"
            )
        );
    }
}
