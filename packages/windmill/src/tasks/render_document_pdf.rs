// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::document::get_document;
use crate::services::ceremonies::velvet_tally::generate_initial_state;
use crate::services::compress::extract_archive_to_temp_dir;
use crate::services::consolidation::create_transmission_package_service::download_tally_tar_gz_to_file;
use crate::services::database::get_hasura_pool;
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error as WrapError, Result as WrapResult};
use anyhow::{anyhow, Context, Result};
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::pdf::{PdfRenderer, PrintToPdfOptions};
use sequent_core::temp_path::write_into_named_temp_file;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::path::change_file_extension;
use std::io::{Read, Seek};
use tracing::instrument;
use velvet::config::generate_reports::PipeConfigGenerateReports;
use velvet::pipes::pipe_name::PipeName;

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

    let tar_gz_file = download_tally_tar_gz_to_file(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    let tally_path = extract_archive_to_temp_dir(tar_gz_file.path(), false)?;

    let tally_path_path = tally_path.into_path();

    let state = generate_initial_state(&tally_path_path, "decode-ballots")?;

    let pipe = state
        .stages
        .iter()
        .find_map(|stage| {
            stage
                .pipeline
                .iter()
                .find(|pipeline| pipeline.pipe == PipeName::GenerateReports)
        })
        .ok_or(anyhow!("Can't find pipe"))?;

    let config: PipeConfigGenerateReports =
        deserialize_value(pipe.config.clone().ok_or(anyhow!("Missing pipe config"))?)?;

    Ok(config
        .pdf_options
        .map(|option| option.to_print_to_pdf_options()))
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

    let document_name = change_file_extension(
        &document.name.ok_or(anyhow!("Missing document name"))?,
        "pdf",
    )
    .ok_or(anyhow!("Error changing file extension"))?;

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
    tally_session_id: Option<String>,
) -> Result<()> {
    match render_document_pdf_wrap(
        tenant_id,
        document_id,
        election_event_id,
        executer_username,
        output_document_id.clone(),
        tally_session_id,
    )
    .await
    {
        Ok(_) => {
            update_complete(&task_execution, Some(output_document_id.clone())).await?;
        }
        Err(err) => {
            update_fail(&task_execution, format!("{:?}", err).as_str()).await?;
            return Err(err);
        }
    };

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000, max_retries = 2)]
pub async fn render_document_pdf(
    tenant_id: String,
    document_id: String,
    election_event_id: Option<String>,
    task_execution: TasksExecution,
    executer_username: Option<String>,
    output_document_id: String,
    tally_session_id: Option<String>,
) -> WrapResult<()> {
    // Note, put this in a thread?
    render_document_pdf_task_wrap(
        tenant_id,
        document_id,
        election_event_id,
        task_execution,
        executer_username,
        output_document_id,
        tally_session_id,
    )
    .await
    .map_err(|err| WrapError::from(anyhow!("Task panicked: {}", err)))
}

#[instrument(err)]
pub async fn render_documents_pdf_task_wrap(
    tenant_id: String,
    document_ids: Vec<String>,
    election_event_id: Option<String>,
    task_execution: TasksExecution,
    executer_username: Option<String>,
    output_document_id: String,
    tally_session_id: Option<String>,
) -> Result<()> {
    let mut successful_renders = Vec::new();

    for document_id in document_ids.iter() {
        let render_output_id = format!("{}-output", document_id);

        match render_document_pdf_wrap(
            tenant_id.clone(),
            document_id.clone(),
            election_event_id.clone(),
            executer_username.clone(),
            render_output_id.clone(),
            tally_session_id.clone(),
        )
        .await
        {
            Ok(_) => {
                successful_renders.push(render_output_id);
            }
            Err(err) => {
                update_fail(&task_execution, format!("Render failed for document {}: {:?}", document_id, err).as_str()).await?;
                return Err(err);
            }
        };
    }

    // TODO pack all documents into a tar gz file and update results_event

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 60000, max_retries = 2)]
pub async fn render_many_document_pdf(
    tenant_id: String,
    document_ids: Vec<String>,
    election_event_id: Option<String>,
    task_execution: TasksExecution,
    executer_username: Option<String>,
    output_document_id: String,
    tally_session_id: Option<String>,
) -> WrapResult<()> {
    render_documents_pdf_task_wrap(
        tenant_id,
        document_ids,
        election_event_id,
        task_execution,
        executer_username,
        output_document_id,
        tally_session_id,
    )
    .await
    .map_err(|err| WrapError::from(anyhow!("Task panicked: {}", err)))
}
