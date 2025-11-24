// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::reports::get_reports_by_election_id;
use crate::postgres::reports::ReportType;
use crate::postgres::tally_session::get_tally_session_by_id;
use crate::services::ceremonies::encrypter::encrypt_file;
use crate::services::ceremonies::velvet_tally::build_ballot_images_pipe_config;
use crate::services::ceremonies::velvet_tally::call_velvet;
use crate::services::ceremonies::velvet_tally::generate_initial_state;
use crate::services::compress::extract_archive_to_temp_dir;
use crate::services::consolidation::create_transmission_package_service::download_tally_tar_gz_to_file;
use crate::services::consolidation::zip::compress_folder_to_zip;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::reports::utils::get_public_assets_path_env_var;
use crate::services::tasks_execution::{update, update_complete, update_fail};
use crate::services::tasks_semaphore::acquire_semaphore;
use crate::types::error::Error;
use crate::types::error::Result;
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use hex;
use sequent_core::ballot::ContestEncryptionPolicy;
use sequent_core::services::{pdf, s3};
use sequent_core::types::ceremonies::TallyExecutionStatus;
use sequent_core::types::hasura::core::TallySession;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::types::hasura::extra::TasksExecutionStatus;
use sequent_core::util::path::get_folder_name;
use sequent_core::util::path::list_subfolders;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use strand::hash::hash_sha256;
use tempfile::tempdir;
use tracing::{info, instrument};
use velvet::config::ballot_images_config::PipeConfigBallotImages;
use velvet::pipes::pipe_name::PipeName;
use velvet::pipes::pipe_name::PipeNameOutputDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum EGenerateTemplate {
    BallotImages {
        election_event_id: String,
        election_id: String,
        tally_session_id: String,
    },
}

#[instrument(err, skip(hasura_transaction, tally_session))]
async fn create_config(
    hasura_transaction: &Transaction<'_>,
    tally_session: &TallySession,
    tally_path: &PathBuf,
    contest_encryption_policy: &ContestEncryptionPolicy,
) -> AnyhowResult<String> {
    let config_path = tally_path.join("velvet-config.json");

    if fs::metadata(&config_path).is_ok() {
        fs::remove_file(&config_path)?;
    }

    let public_asset_path = get_public_assets_path_env_var()?;

    let minio_endpoint_base = s3::get_minio_url()?;

    let ballot_images_pipe_config: PipeConfigBallotImages = build_ballot_images_pipe_config(
        &tally_session,
        &hasura_transaction,
        minio_endpoint_base.clone(),
        public_asset_path.clone(),
    )
    .await?;
    let pipe_config = serde_json::to_value(ballot_images_pipe_config)?;

    let first_pipe_id = "ballot-images";

    let stages_def = {
        let mut map = HashMap::new();
        map.insert(
            "main".to_string(),
            velvet::config::Stage {
                pipeline: vec![velvet::config::PipeConfig {
                    id: first_pipe_id.to_string(),
                    pipe: match contest_encryption_policy {
                        ContestEncryptionPolicy::MULTIPLE_CONTESTS => PipeName::MCBallotImages,
                        ContestEncryptionPolicy::SINGLE_CONTEST => PipeName::BallotImages,
                    },
                    config: Some(pipe_config),
                }],
            },
        );
        map
    };

    let stages = velvet::config::Stages {
        order: vec!["main".to_string()],
        stages_def,
    };

    let velvet_config = velvet::config::Config {
        version: "0.0.0".to_string(),
        stages,
    };
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&config_path)?;

    writeln!(file, "{}", serde_json::to_string(&velvet_config)?)?;
    file.flush()?;

    Ok(first_pipe_id.to_string())
}

#[instrument(err, skip(hasura_transaction))]
async fn generate_template_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    document_id: &str,
    input: &EGenerateTemplate,
) -> AnyhowResult<()> {
    let EGenerateTemplate::BallotImages {
        election_event_id,
        election_id,
        tally_session_id,
    } = input.clone();

    let tally_session = get_tally_session_by_id(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;
    let contest_encryption_policy = tally_session
        .configuration
        .clone()
        .unwrap_or_default()
        .get_contest_encryption_policy();

    if !tally_session.is_execution_completed
        || tally_session.execution_status != Some(TallyExecutionStatus::SUCCESS.to_string())
    {
        return Err(anyhow!("Tally session is not completed"));
    }

    let tar_gz_file = download_tally_tar_gz_to_file(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    let tally_path = extract_archive_to_temp_dir(tar_gz_file.path(), false)?;

    let tally_path_path = tally_path.into_path();

    let pipe_name = if contest_encryption_policy == ContestEncryptionPolicy::MULTIPLE_CONTESTS {
        PipeNameOutputDir::MCBallotImages
    } else {
        PipeNameOutputDir::BallotImages
    }
    .as_ref()
    .to_string();

    let report_type = ReportType::BALLOT_IMAGES;

    let step_path = tally_path_path.join("output").join(&pipe_name);

    if fs::metadata(&step_path).is_ok() {
        fs::remove_dir_all(&step_path)?;
    }

    let first_pipe_id = create_config(
        hasura_transaction,
        &tally_session,
        &tally_path_path,
        &contest_encryption_policy,
    )
    .await?;

    call_velvet(tally_path_path.clone(), &first_pipe_id).await?;

    let election_path = step_path.join(format!("election__{election_id}"));

    let output_zip_tempfile = generate_temp_file(&pipe_name, "zip")?;
    let output_zip_path = output_zip_tempfile.path();

    // clear the path
    let subfolders = list_subfolders(&election_path);
    for subfolder in subfolders {
        let entries = fs::read_dir(subfolder)?;
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }
                let Ok(name) = entry.file_name().into_string() else {
                    continue;
                };
                if name.ends_with(".html") {
                    fs::remove_file(&path)?;
                }
            }
        }
    }

    compress_folder_to_zip(&election_path, output_zip_path)?;

    let election_reports =
        get_reports_by_election_id(hasura_transaction, tenant_id, &election_id).await?;

    let report = election_reports
        .iter()
        .find(|report| {
            report.report_type == report_type.to_string()
                && report
                    .election_id
                    .as_ref()
                    .map_or(true, |id| *id == election_id)
        })
        .cloned();

    // Encrypt the file if needed
    let final_zipped_file = encrypt_file(
        hasura_transaction,
        tenant_id,
        &election_event_id,
        &output_zip_path.to_string_lossy(),
        report.as_ref(),
    )
    .await?;

    let (file_extension, mime_type) = if final_zipped_file.ends_with(".enc") {
        ("zip.enc", "application/octet-stream")
    } else {
        ("zip", "application/zip")
    };

    let file_size =
        get_file_size(&final_zipped_file).with_context(|| "Error obtaining file size")?;

    let otuput_doc_name = format!("election-{election_id}-ballot-images.{file_extension}");

    let _document = upload_and_return_document(
        &hasura_transaction,
        &final_zipped_file,
        file_size,
        mime_type,
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
                update_fail(task_exec, "Failed to get Hasura DB pool").await?;
            }
            return Err(anyhow!("Error getting Hasura DB pool: {}", err));
        }
    };

    let hasura_transaction = match db_client.transaction().await {
        Ok(transaction) => {
            if let Some(ref task_exec) = task_execution {
                update(
                    /* tenant_id: */ &tenant_id,
                    /* task_id: */ &task_exec.id,
                    /* status: */ TasksExecutionStatus::IN_PROGRESS,
                    /* logs: */ json!([]),
                    /* annotations: */
                    Some(document_id.clone()),
                )
                .await?;
            }
            transaction
        }
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                update_fail(task_exec, "Failed to get Hasura DB pool").await?;
            };
            return Err(anyhow!("Error starting Hasura transaction: {err}"));
        }
    };

    match generate_template_document(&hasura_transaction, &tenant_id, &document_id, &input).await {
        Ok(transaction) => Ok(transaction),
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                let err_str = format!("{:?}", err);
                update_fail(task_exec, &err_str).await?;
            };
            Err(err)
        }
    }
    .context("Error generating template document")?;

    match hasura_transaction.commit().await {
        Ok(transaction) => {
            if let Some(ref task_exec) = task_execution {
                update_complete(task_exec, Some(document_id.clone())).await?;
            }
            Ok(transaction)
        }
        Err(err) => {
            if let Some(ref task_exec) = task_execution {
                update_fail(task_exec, "Failed to commit Hasura transaction").await?;
            };
            Err(err)
        }
    }?;

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
    let _permit = acquire_semaphore().await?;
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
