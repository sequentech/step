// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::documents::upload_and_return_document;
use crate::services::export::export_verifiable_bulletin_board::export_verifiable_bulletin_board_sqlite_file;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::Result as TaskResult;
use anyhow::Result;
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::TasksExecution;
use std::env;
use std::fs::File;
use tracing::instrument;
use zip::write::FileOptions;

pub async fn process_export_verifiable_bulletin_board(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    document_id: String,
    tally_session_id: String,
    election_event_id: String,
) -> Result<()> {
    let zip_filename = format!("export_verifiable_bulletin_board.zip");
    let zip_path = env::temp_dir().join(&zip_filename);

    let cwd = env::current_dir().map_err(|e| anyhow::anyhow!(e))?;
    println!("Current working directory: {:?}", cwd);

    // Create a new ZIP file
    let zip_file =
        File::create(&zip_path).map_err(|e| anyhow::anyhow!("Error creating ZIP file: {e:?}"))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options: FileOptions<()> =
        FileOptions::default().compression_method(zip::CompressionMethod::DEFLATE);

    let temp_bulletin_boards_file = export_verifiable_bulletin_board_sqlite_file(
        hasura_transaction,
        tenant_id.clone(),
        document_id.clone(),
        tally_session_id,
        election_event_id.clone(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Error exporting verifiable bulletin board: {e:?}"))?;

    zip_writer
        .start_file("verifiable_bulletin_board.db", options.clone())
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut bulletin_boards_file = File::open(temp_bulletin_boards_file)
        .map_err(|e| anyhow::anyhow!("Error opening temporary bulletin boards file: {e:?}"))?;
    std::io::copy(&mut bulletin_boards_file, &mut zip_writer)
        .map_err(|e| anyhow::anyhow!("Error copying bulletin boards file to ZIP: {e:?}"))?;

    zip_writer.finish().map_err(|e| anyhow::anyhow!(e))?;

    let zip_size = std::fs::metadata(&zip_path)
        .map_err(|e| anyhow::anyhow!("Error getting ZIP file metadata: {e:?}"))?
        .len();

    let _document = upload_and_return_document(
        &hasura_transaction,
        zip_path.to_str().unwrap(),
        zip_size,
        "application/zip",
        &tenant_id.to_string(),
        Some(election_event_id.to_string()),
        &zip_filename,
        Some(document_id.to_string()),
        false,
    )
    .await?;

    std::fs::remove_file(&zip_path)
        .map_err(|e| anyhow::anyhow!("Error removing ZIP file: {e:?}"))?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_verifiable_bulletin_board_task(
    tenant_id: String,
    document_id: String,
    task_execution: TasksExecution,
    tally_session_id: String,
    election_event_id: String,
) -> TaskResult<()> {
    let result = provide_hasura_transaction(|hasura_transaction| {
        let document_copy = document_id.clone();
        Box::pin(async move {
            process_export_verifiable_bulletin_board(
                hasura_transaction,
                tenant_id,
                document_copy.clone(),
                tally_session_id,
                election_event_id,
            )
            .await
        })
    })
    .await;

    match result {
        Ok(_) => {
            let _res = update_complete(&task_execution, Some(document_id.clone())).await;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Error exporting verifiable bulletin board: {err:?}");
            let _res = update_fail(&task_execution, &err.to_string()).await;
            Err(err_str.into())
        }
    }
}
