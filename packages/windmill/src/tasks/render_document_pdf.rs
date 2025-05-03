// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::document::get_document;
use crate::postgres::results_event::get_results_event_by_id;
use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::services::database::get_hasura_pool;
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error as WrapError, Result as WrapResult};
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::services::pdf::{PdfRenderer, PrintToPdfOptions};
use sequent_core::temp_path::write_into_named_temp_file;
use sequent_core::types::hasura::core::TasksExecution;
use std::io::{Read, Seek};
use tracing::instrument;

#[instrument(err, skip(hasura_transaction))]
pub async fn get_tally_pdf_config(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id_opt: Option<String>,
    tally_id_opt: Option<String>,
) -> Result<Option<PrintToPdfOptions>> {
    let (Some(election_event_id), Some(tally_session_id)) = (election_event_id_opt, tally_id_opt)
    else {
        return Ok(None);
    };

    let Some(tally_session_execution) = get_last_tally_session_execution(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?
    else {
        return Err(anyhow!("No tally session execution found"));
    };
    let results_event_id = tally_session_execution
        .results_event_id
        .ok_or(anyhow!("Missing results id"))?;

    let results_event = get_results_event_by_id(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        &results_event_id,
    )
    .await?;

    let tar_gz_document_id = results_event
        .documents
        .ok_or(anyhow!("Result event with no document"))?
        .tar_gz_original
        .ok_or(anyhow!("Tally with no tar gz"))?;

    let Some(document) = get_document(
        &hasura_transaction,
        &tenant_id,
        Some(election_event_id.clone()),
        &tar_gz_document_id,
    )
    .await?
    else {
        return Err(anyhow!("Document not found: {}", tar_gz_document_id));
    };

    let mut temp_document = get_document_as_temp_file(&tenant_id, &document).await?;

    Ok(None)
}

#[instrument(err)]
pub async fn render_document_pdf_wrap(
    tenant_id: String,
    document_id: String,
    election_event_id: Option<String>,
    executer_username: Option<String>,
    output_document_id: String,
    tally_id: Option<String>,
) -> Result<()> {
    let mut db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("{:?}", err))?;
    let hasura_transaction = db_client.transaction().await?;

    let pdf_options = get_tally_pdf_config(
        &hasura_transaction,
        &tenant_id,
        election_event_id.clone(),
        tally_id,
    )
    .await?;

    let Some(document) = get_document(
        &hasura_transaction,
        &tenant_id,
        election_event_id.clone(),
        &document_id,
    )
    .await?
    else {
        return Err(anyhow!("Document not found: {}", document_id));
    };

    let mut temp_document = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_document.rewind()?;
    let mut render = String::new();
    temp_document.read_to_string(&mut render)?;

    let bytes = PdfRenderer::render_pdf(render, pdf_options)
        .await
        .with_context(|| "Error converting html to pdf format")?;
    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&bytes, "reports-", ".html")
            .with_context(|| "Error writing to file")?;

    let document_name = document.name.ok_or(anyhow!("Missing document name"))?;

    let _document = upload_and_return_document(
        &hasura_transaction,
        &temp_path_string,
        file_size,
        "application/pdf",
        &tenant_id,
        election_event_id.clone(),
        &document_name,
        Some(output_document_id),
        false,
    )
    .await?;

    hasura_transaction.commit().await?;
    Ok(())
}

#[instrument(err)]
pub async fn render_document_pdf_task_wrap(
    tenant_id: String,
    document_id: String,
    election_event_id: Option<String>,
    task_execution: TasksExecution,
    executer_username: Option<String>,
    output_document_id: String,
    tally_id: Option<String>,
) -> Result<()> {
    match render_document_pdf_wrap(
        tenant_id,
        document_id,
        election_event_id,
        executer_username,
        output_document_id.clone(),
        tally_id,
    )
    .await
    {
        Ok(_) => {
            update_complete(&task_execution, Some(output_document_id.clone())).await?;
        }
        Err(err) => {
            update_fail(&task_execution, format!("{:?}", err).as_str()).await?;
        }
    };

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000)]
pub async fn render_document_pdf(
    tenant_id: String,
    document_id: String,
    election_event_id: Option<String>,
    task_execution: TasksExecution,
    executer_username: Option<String>,
    output_document_id: String,
    tally_id: Option<String>,
) -> WrapResult<()> {
    // Note, put this in a thread?
    render_document_pdf_task_wrap(
        tenant_id,
        document_id,
        election_event_id,
        task_execution,
        executer_username,
        output_document_id,
        tally_id,
    )
    .await
    .map_err(|err| WrapError::from(anyhow!("Task panicked: {}", err)))
}
