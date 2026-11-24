// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::document::{insert_document, insert_document_with_annotations};
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use deadpool_postgres::Transaction;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use sequent_core::election_config::encrypted_delivery::MAX_SIGNED_DELIVERY_BYTES;
use std::io::Write;
use tokio::io::{AsyncRead, AsyncReadExt};

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

// Protected bytes are never placed in the ordinary document namespace. Even a
// generic consumer which reads by document ID without an egress policy cannot
// construct this key from user-editable document metadata.
fn configuration_package_key(tenant: uuid::Uuid, storage_id: uuid::Uuid) -> String {
    format!("configuration-package-staging/tenant-{tenant}/{storage_id}/delivery.zip")
}

pub fn valid_configuration_package_size(size: usize) -> bool {
    (1..=MAX_SIGNED_DELIVERY_BYTES).contains(&size)
}

pub async fn stage_configuration_package(
    tx: &Transaction<'_>,
    size: usize,
    tenant_id: &str,
) -> AnyhowResult<(Document, String)> {
    if !valid_configuration_package_size(size) {
        return Err(anyhow!("invalid configuration package size"));
    }
    let tenant = uuid::Uuid::parse_str(tenant_id)?;
    let document = insert_document(
        tx,
        tenant_id,
        None,
        "delivery.zip",
        "application/zip",
        size.try_into()?,
        false,
        None,
    )
    .await?;
    let document_id = uuid::Uuid::parse_str(&document.id)?;
    let storage_id: uuid::Uuid = tx
        .query_one(
            "INSERT INTO sequent_backend.configuration_package_staging (tenant_id, document_id)
         VALUES ($1, $2) RETURNING storage_id",
            &[&tenant, &document_id],
        )
        .await?
        .try_get(0)?;
    let url =
        s3::get_upload_url(configuration_package_key(tenant, storage_id), false, false).await?;
    Ok((document, url))
}

async fn configuration_package_storage(
    tx: &Transaction<'_>,
    tenant: uuid::Uuid,
    document_id: &str,
) -> AnyhowResult<Option<uuid::Uuid>> {
    let id = uuid::Uuid::parse_str(document_id)?;
    tx.query_opt(
        "SELECT storage_id FROM sequent_backend.configuration_package_staging
         WHERE tenant_id = $1 AND document_id = $2",
        &[&tenant, &id],
    )
    .await?
    .map(|row| row.try_get::<_, uuid::Uuid>(0).map_err(anyhow::Error::from))
    .transpose()
}

async fn write_bounded_package(
    mut input: impl AsyncRead + Unpin,
    mut output: impl Write,
    limit: usize,
) -> AnyhowResult<()> {
    let mut buffer = [0u8; 16 * 1024];
    let mut remaining = limit;
    loop {
        // At most one byte past the configured limit is consumed, never written.
        let read_limit = buffer.len().min(remaining.saturating_add(1));
        let read = input.read(&mut buffer[..read_limit]).await?;
        if read == 0 {
            if remaining == limit {
                return Err(anyhow!("configuration package is empty"));
            }
            break;
        }
        if read > remaining {
            return Err(anyhow!("configuration package exceeds the size limit"));
        }
        output.write_all(&buffer[..read])?;
        remaining -= read;
    }
    Ok(())
}

async fn download_bounded_package(bucket: String, key: &str) -> AnyhowResult<NamedTempFile> {
    let client = s3::get_shared_s3_client(s3::S3Endpoint::Server).await?;
    let response = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|_| anyhow!("configuration package is not available"))?;
    if response.content_length().is_some_and(|len| {
        usize::try_from(len).map_or(true, |size| !valid_configuration_package_size(size))
    }) {
        return Err(anyhow!("invalid configuration package size"));
    }
    let mut file = NamedTempFile::new()?;
    write_bounded_package(
        response.body.into_async_read(),
        &mut file,
        MAX_SIGNED_DELIVERY_BYTES,
    )
    .await?;
    file.flush()?;
    Ok(file)
}

/// Only the package admission worker uses the protected namespace.
pub(crate) async fn read_configuration_package(
    tx: &Transaction<'_>,
    tenant_id: &str,
    document: &Document,
) -> AnyhowResult<(NamedTempFile, bool)> {
    let tenant = uuid::Uuid::parse_str(tenant_id)?;
    if let Some(storage_id) = configuration_package_storage(tx, tenant, &document.id).await? {
        if document.tenant_id.as_deref() != Some(tenant.to_string().as_str())
            || document.election_event_id.is_some()
            || document.is_public != Some(false)
        {
            return Err(anyhow!(
                "invalid protected configuration package staging document"
            ));
        }
        let file = download_bounded_package(
            s3::get_private_bucket()?,
            &configuration_package_key(tenant, storage_id),
        )
        .await?;
        return Ok((file, true));
    }
    // Previously uploaded ordinary deliveries remain compatible, but document
    // metadata must never grant an exemption from the same streamed byte cap.
    let bucket = if document.is_public == Some(true) {
        s3::get_public_bucket()?
    } else {
        s3::get_private_bucket()?
    };
    let key = s3::get_document_key(
        tenant_id,
        document.election_event_id.as_deref(),
        &document.id,
        document.name.as_deref().unwrap_or_default(),
    );
    Ok((download_bounded_package(bucket, &key).await?, false))
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

/// Application-wide negative egress gate for the actual stored document.
/// Signed MIRU releases use the dedicated exact-file route instead.
pub async fn ordinary_document_url_allowed(
    tx: &Transaction<'_>,
    tenant_id: &str,
    document: &Document,
) -> anyhow::Result<bool> {
    use crate::services::signing::operational_policy::{
        approval_mode_for_event, signing_mode,
    };
    let tenant = uuid::Uuid::parse_str(tenant_id)?;
    if configuration_package_storage(tx, tenant, &document.id)
        .await?
        .is_some()
    {
        return Ok(false);
    }
    let event = document
        .election_event_id
        .as_deref()
        .filter(|id| !id.is_empty())
        .map(uuid::Uuid::parse_str)
        .transpose()?;
    match event {
        Some(event_id) => approval_mode_for_event(tx, tenant, event_id).await?,
        None => signing_mode(tx, tenant).await?,
    };
    let document_id = uuid::Uuid::parse_str(&document.id)?;
    let withheld: Option<bool> = tx
        .query_one(
            "SELECT sequent_backend.operational_miru_generic_withhold($1,$2,$3)",
            &[&tenant, &event, &document_id],
        )
        .await?
        .try_get(0)?;
    Ok(!withheld
        .ok_or_else(|| anyhow::anyhow!("Operational document inventory not yet verified"))?)
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

    if !ordinary_document_url_allowed(hasura_transaction, tenant_id, &document).await? {
        return Ok(None);
    }
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
    use super::{
        configuration_package_key, stored_document_key, valid_configuration_package_size,
        write_bounded_package, MAX_SIGNED_DELIVERY_BYTES,
    };
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

    #[tokio::test]
    async fn private_stage_size_is_enforced_on_actual_bytes_without_writing_past_limit() {
        assert!(!valid_configuration_package_size(0));
        assert!(valid_configuration_package_size(MAX_SIGNED_DELIVERY_BYTES));
        assert!(!valid_configuration_package_size(
            MAX_SIGNED_DELIVERY_BYTES + 1
        ));
        let mut output = Vec::new();
        write_bounded_package(&b"PK12345"[..], &mut output, 7)
            .await
            .unwrap();
        assert_eq!(output, b"PK12345");
        output.clear();
        assert!(write_bounded_package(&b"PK123456"[..], &mut output, 7)
            .await
            .is_err());
        assert!(output.len() <= 7);
        assert!(write_bounded_package(&b""[..], &mut output, 7)
            .await
            .is_err());
    }

    #[test]
    fn private_staging_storage_is_not_derived_from_document_metadata() {
        let tenant = uuid::Uuid::parse_str(TENANT).unwrap();
        let storage_id = uuid::Uuid::new_v4();
        let protected = configuration_package_key(tenant, storage_id);
        assert!(protected.starts_with("configuration-package-staging/"));
        assert!(protected.contains(&storage_id.to_string()));
        assert!(!protected.contains(DOCUMENT));
        for public in [true, false] {
            let mut doc = document(None, Some(public));
            doc.name = Some("delivery.zip".into());
            assert_ne!(protected, stored_document_key(TENANT, &doc));
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
