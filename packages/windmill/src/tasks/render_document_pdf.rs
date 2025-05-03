// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::{Error as WrapError, Result as WrapResult};
use crate::postgres::document::get_document;
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Result, Context};
use deadpool_postgres::{Client as DbClient, Transaction};
use celery::error::TaskError;
use sequent_core::temp_path::write_into_named_temp_file;
use sequent_core::types::hasura::core::TasksExecution;
use tracing::instrument;
use sequent_core::services::pdf::PdfRenderer;
use std::io::BufReader;
use std::io::{Read, Seek};

#[instrument(err)]
pub async fn render_document_pdf_wrap(
    tenant_id: String,
    document_id: String,
    election_event_id: Option<String>,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
) -> Result<()> {
    let mut db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("{:?}", err))?;
    let hasura_transaction = db_client.transaction().await?;

    let Some(document) = get_document(
        &hasura_transaction,
        &tenant_id,
        election_event_id.clone(),
        &document_id,
    )
    .await? else {
        return Err(anyhow!("Document not found: {}", document_id));
    };

    let mut temp_document = get_document_as_temp_file(
        &tenant_id,
        &document,
    ).await?;
    temp_document.rewind()?;
    let mut render = String::new();
    temp_document.read_to_string(&mut render)?;

    let bytes = PdfRenderer::render_pdf(render, None)
    .await
    .with_context(|| "Error converting html to pdf format")?;
    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes, "reports-", ".html")
            .with_context(|| "Error writing to file")?;

    // document.name

    let _document = upload_and_return_document(
        &hasura_transaction,
        &temp_path_string,
        file_size,
        "application/pdf",
        &tenant_id,
        election_event_id.clone(),
        "test.pdf",
        None,
        false,
    )
    .await?;

    hasura_transaction.commit().await?;
    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000)]
pub async fn render_document_pdf(
    tenant_id: String,
    document_id: String,
    election_event_id: Option<String>,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
) -> WrapResult<()> {
    render_document_pdf_wrap(
        tenant_id,
        document_id,
        election_event_id,
        task_execution,
        executer_username,
    ).await
        .map_err(|err| WrapError::from(anyhow!("Task panicked: {}", err)) )
}
