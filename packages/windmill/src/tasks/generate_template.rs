// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tally_session::get_tally_session_by_id;
use crate::services::ceremonies::velvet_tally::build_ballot_images_pipe_config;
use crate::services::ceremonies::velvet_tally::build_vote_receipe_pipe_config;
use crate::services::compress::decompress_file;
use crate::services::consolidation::create_transmission_package_service::download_tally_tar_gz_to_file;
use crate::services::consolidation::zip::compress_folder_to_zip;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document_postgres;
use crate::services::reports::utils::get_public_assets_path_env_var;
use crate::services::s3;
use crate::services::tasks_execution::update_fail;
use crate::services::temp_path::generate_temp_file;
use crate::services::temp_path::get_file_size;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use hex;
use sequent_core::services::pdf;
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::path::get_folder_name;
use sequent_core::util::path::list_subfolders;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use strand::hash::hash_sha256;
use tempfile::tempdir;
use tracing::{info, instrument};
use velvet::config::vote_receipt::PipeConfigVoteReceipts;
use velvet::pipes::pipe_name::PipeNameOutputDir;
use velvet::pipes::vote_receipts::mcballot_receipts::{
    BALLOT_IMAGES_OUTPUT_FILE_HTML, OUTPUT_FILE_HTML,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum EGenerateTemplate {
    BallotImages {
        election_event_id: String,
        election_id: String,
        tally_session_id: String,
    },
    VoteReceipts {
        election_event_id: String,
        election_id: String,
        tally_session_id: String,
    },
}

async fn generate_template_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    document_id: &str,
    input: &EGenerateTemplate,
) -> AnyhowResult<()> {
    let (is_ballot_images, election_event_id, election_id, tally_session_id) = match input.clone() {
        EGenerateTemplate::BallotImages {
            election_event_id,
            election_id,
            tally_session_id,
        } => (true, election_event_id, election_id, tally_session_id),
        EGenerateTemplate::VoteReceipts {
            election_event_id,
            election_id,
            tally_session_id,
        } => (false, election_event_id, election_id, tally_session_id),
    };

    let tally_session = get_tally_session_by_id(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    if !tally_session.is_execution_completed
        || tally_session.execution_status != Some(TallyExecutionStatus::SUCCESS.to_string())
    {
        return Err(anyhow!("Tally session is not completed"));
    }
    let public_asset_path = get_public_assets_path_env_var()?;

    let minio_endpoint_base = s3::get_minio_url()?;

    let pipe_config_pdf_options = if is_ballot_images {
        let pipe_config = build_ballot_images_pipe_config(
            &tally_session,
            &hasura_transaction,
            minio_endpoint_base.clone(),
            public_asset_path.clone(),
        )
        .await?;
        pipe_config.pdf_options.clone()
    } else {
        let pipe_config = build_vote_receipe_pipe_config(
            &tally_session,
            &hasura_transaction,
            minio_endpoint_base.clone(),
            public_asset_path.clone(),
        )
        .await?;
        pipe_config.pdf_options.clone()
    };

    let tar_gz_file = download_tally_tar_gz_to_file(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    let tally_path = decompress_file(tar_gz_file.path())?;

    let tally_path_path = tally_path.into_path();

    let pipe_name = if is_ballot_images {
        PipeNameOutputDir::MCBallotImages.as_ref().to_string()
    } else {
        PipeNameOutputDir::MCBallotReceipts.as_ref().to_string()
    };

    let election_path =
        tally_path_path.join(format!("output/{pipe_name}/election__{election_id}",));

    if !election_path.exists() {
        return Err(anyhow!("Error reading election path in zip"));
    }

    let area_folders = list_subfolders(election_path.as_path());

    if area_folders.len() == 0 {
        return Err(anyhow!("No areas for election"));
    }
    let out_temp_dir = tempdir().with_context(|| "Error generating temp directory")?;
    let out_temp_dir_path = out_temp_dir.path();

    for area_folder in area_folders {
        let html_name = if is_ballot_images {
            BALLOT_IMAGES_OUTPUT_FILE_HTML
        } else {
            OUTPUT_FILE_HTML
        };
        let html_path = area_folder.join(html_name);
        if !html_path.exists() {
            return Err(anyhow!(
                "Error reading html ballot image in zip: {}",
                html_path.display()
            ));
        }

        let bytes_html = fs::read_to_string(html_path.as_path())?;

        let pdf_options = match pipe_config_pdf_options.clone() {
            Some(options) => Some(options.to_print_to_pdf_options()),
            None => None,
        };
        let bytes_pdf = pdf::html_to_pdf(bytes_html, pdf_options)?;

        let area_name =
            get_folder_name(html_path.as_path()).ok_or(anyhow!("Can't read folder name"))?;
        let out_area_path = out_temp_dir_path.join(area_name);

        fs::create_dir_all(&out_area_path)?;

        let hash_bytes = hash_sha256(bytes_pdf.as_slice())?;
        let hash_hex = hex::encode(hash_bytes);
        let pdf_filename = if is_ballot_images {
            format!("mcballot_images_{hash_hex}.pdf")
        } else {
            "mcballots_receipts.pdf".to_string()
        };

        let out_file_path = out_area_path.join(&pdf_filename);

        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(out_file_path)?;
        file.write_all(&bytes_pdf)?;
        file.flush()?;
    }

    let output_zip_tempfile = generate_temp_file(&pipe_name, "zip")?;
    let output_zip_path = output_zip_tempfile.path();
    let output_zip_str = output_zip_path.to_string_lossy();

    compress_folder_to_zip(out_temp_dir_path, output_zip_path)?;
    let file_size = get_file_size(&output_zip_str).with_context(|| "Error obtaining file size")?;

    let otuput_doc_name = if is_ballot_images {
        format!("election-{election_id}-ballot-images.zip")
    } else {
        format!("election-{election_id}-vote-receipts.zip")
    };

    let document = upload_and_return_document_postgres(
        &hasura_transaction,
        &output_zip_str,
        file_size,
        "applization/zip",
        tenant_id,
        Some(election_event_id.to_string()),
        &otuput_doc_name,
        Some(document_id.to_string()),
        false,
    )
    .await?;

    Ok(())
}

#[instrument(err)]
async fn generate_template_block(
    tenant_id: String,
    document_id: String,
    input: EGenerateTemplate,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
) -> AnyhowResult<()> {
    let mut db_client: DbClient = match get_hasura_pool().await.get().await {
        Ok(client) => client,
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, "Failed to get Hasura DB pool").await;
            }
            return Err(anyhow!("Error getting Hasura DB pool: {}", err));
        }
    };

    let hasura_transaction = match db_client.transaction().await {
        Ok(transaction) => transaction,
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let _ = update_fail(task_exec, "Failed to get Hasura DB pool").await;
            };
            return Err(anyhow!("Error starting Hasura transaction: {err}"));
        }
    };

    generate_template_document(&hasura_transaction, &tenant_id, &document_id, &input).await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "Failed to commit Hasura transaction")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn generate_template(
    tenant_id: String,
    document_id: String,
    input: EGenerateTemplate,
    task_execution: Option<TasksExecution>,
    executer_username: Option<String>,
) -> Result<()> {
    // Spawn the task using an async block
    let handle = tokio::task::spawn_blocking({
        move || {
            tokio::runtime::Handle::current().block_on(async move {
                generate_template_block(
                    tenant_id,
                    document_id,
                    input,
                    task_execution,
                    executer_username,
                )
                .await
                .map_err(|err| anyhow!("generate_report error: {:?}", err))
            })
        }
    });

    // Await the result and handle JoinError explicitly
    match handle.await {
        Ok(inner_result) => inner_result.map_err(|err| Error::from(err.context("Task failed"))),
        Err(join_error) => Err(Error::from(anyhow!("Task panicked: {}", join_error))),
    }?;

    Ok(())
}
