// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::certificate_authority::get_certificate_authorities_pem_by_ids;
use crate::postgres::document::insert_document;
use crate::services::database::get_hasura_pool;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::Result;
use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::s3;
use sequent_core::types::hasura::core::TasksExecution;
use std::io::Write;
use tempfile::NamedTempFile;
use tracing::instrument;
use uuid::Uuid;

async fn export_certificate_authority_impl(
    tenant_id: String,
    election_event_id: Uuid,
    ids: Vec<Uuid>,
    document_id: String,
) -> AnyhowResult<()> {
    let mut db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error getting DB pool")?;

    let hasura_transaction = db_client
        .transaction()
        .await
        .with_context(|| "Error starting transaction")?;

    let pems = get_certificate_authorities_pem_by_ids(&hasura_transaction, election_event_id, &ids)
        .await
        .with_context(|| "Error fetching certificate authority PEMs")?;

    let pem_bundle = pems.join("\n");

    let mut temp_file = NamedTempFile::new().with_context(|| "Error creating temporary file")?;
    temp_file
        .write_all(pem_bundle.as_bytes())
        .with_context(|| "Error writing PEM content to temporary file")?;

    let file_size = temp_file
        .as_file()
        .metadata()
        .with_context(|| "Error reading temp file metadata")?
        .len();

    let name = format!(
        "{}.pem",
        crate::types::documents::EDocuments::CERTIFICATES.to_file_name()
    );
    let election_event_id_str = election_event_id.to_string();
    let key = s3::get_document_key(
        &tenant_id,
        Some(&election_event_id_str),
        &document_id,
        &name,
    );

    s3::upload_file_to_s3(
        key,
        false,
        s3::get_private_bucket()?,
        "application/x-pem-file".to_string(),
        temp_file.path().to_string_lossy().to_string(),
        None,
        Some(name.clone()),
    )
    .await
    .with_context(|| "Error uploading PEM file to S3")?;

    temp_file
        .close()
        .with_context(|| "Error closing temporary file")?;

    insert_document(
        &hasura_transaction,
        &tenant_id,
        Some(election_event_id_str.clone()),
        &name,
        "application/x-pem-file",
        file_size.try_into().with_context(|| "File size overflow")?,
        false,
        Some(document_id.clone()),
    )
    .await
    .with_context(|| "Error inserting document record")?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit transaction")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_certificate_authority(
    tenant_id: String,
    election_event_id: Uuid,
    ids: Vec<Uuid>,
    document_id: String,
    task_execution: TasksExecution,
) -> Result<()> {
    match export_certificate_authority_impl(tenant_id, election_event_id, ids, document_id.clone())
        .await
    {
        Ok(()) => {
            update_complete(&task_execution, Some(document_id))
                .await
                .context("Failed to update task execution status to COMPLETED")?;
            Ok(())
        }
        Err(err) => {
            if let Err(update_err) = update_fail(&task_execution, &format!("{err:?}")).await {
                tracing::error!(
                    "Failed to update task execution status to FAILED: {:?}",
                    update_err
                );
            }
            Err(err.into())
        }
    }
}
