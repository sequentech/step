// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::{keys_ceremony, trustee};

use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use b3::messages::message::Message;
use b3::messages::statement::StatementType;
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as DbTransaction};
use sequent_core::services::date::{get_now_utc_unix_ms, ISO8601};
use sequent_core::services::keycloak;
use sequent_core::sqlite::results_event;
use sequent_core::temp_path::get_file_size;
use sequent_core::types::ceremonies::{
    KeysCeremonyExecutionStatus, KeysCeremonyStatus, TallyCeremonyStatus, Trustee as BasicTrustee,
    TrusteeStatus,
};
use sequent_core::types::hasura::core::{Document, TallySessionExecution, Trustee};
use serde_json::Value;
use std::collections::HashSet;
use strand::signature::StrandSignaturePk;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::postgres::tally_session_execution::insert_tally_session_execution;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::ceremonies::serialize_logs::generate_logs;
use crate::services::ceremonies::serialize_logs::sort_logs;
use crate::services::compress::create_archive_from_folder;
use crate::services::compress::extract_archive_to_temp_dir;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::services::public_keys;
use crate::tasks::render_document_pdf::get_tally_pdf_config;
use crate::tasks::render_document_pdf::render_document_pdf_wrap;
use crate::{
    postgres::{
        area::get_area_by_id, document::get_document, election::get_election_by_id,
        election_event::get_election_event_by_election_area,
        tally_session::get_tally_session_by_id,
    },
    services::documents::{get_document_as_temp_file, upload_and_return_document},
    types::miru_plugin::{
        MiruCcsServer, MiruDocument, MiruServerDocument, MiruServerDocumentStatus,
        MiruTallySessionData, MiruTransmissionPackageData,
    },
};
use rusqlite::Connection as SqliteConnection;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::services::pdf::sync::PdfRenderer;
use sequent_core::sqlite::results_event::find_results_event_sqlite;
use sequent_core::sqlite::results_event::update_results_event_documents_sqlite;
use sequent_core::types::ceremonies::TallySessionDocuments;
use sequent_core::types::results::ResultsEvent;
use sequent_core::types::templates::PrintToPdfOptionsLocal;
use sequent_core::util::path::change_file_extension;
use sequent_core::util::temp_path::write_into_named_temp_file;
use std::fs;
use std::path::Path;
use tempfile::NamedTempFile;
use tokio::task;
use walkdir::WalkDir;

use crate::services::pg_lock::PgLock;
use crate::services::tasks_semaphore::acquire_semaphore;
use chrono::{DateTime, Duration, Utc};

use crate::postgres::results_event::update_results_event_documents;
use crate::postgres::tally_session::set_post_tally_task_completed;
use tokio::time::Duration as ChronoDuration;

#[instrument(skip(hasura_transaction), err)]
pub async fn download_sqlite_database(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    hasura_transaction: &DbTransaction<'_>,
    tally_session_execution: &TallySessionExecution,
) -> AnyhowResult<NamedTempFile> {
    // Recover sqlite database
    let documents: TallySessionDocuments =
        if let Some(documents) = &tally_session_execution.documents {
            let documents = serde_json::to_string(documents)?;
            deserialize_str::<TallySessionDocuments>(&documents)?
        } else {
            return Err(anyhow!(
            "Could not recover documents from tally session execution with id {tally_session_id}"
        )
            .into());
        };

    let sqlite_database_document_id = if let Some(id) = documents.sqlite {
        id
    } else {
        return Err(anyhow!(
            "Could not recover sqlite database from tally session execution with id {tally_session_id}"
        )
        .into());
    };

    let document = get_document(
        &hasura_transaction,
        &tenant_id,
        Some(election_event_id.to_string()),
        &sqlite_database_document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Can't find document {}", sqlite_database_document_id))?;

    let sqlite_database = get_document_as_temp_file(&tenant_id, &document).await?;

    Ok(sqlite_database)
}

pub fn find_and_process_html_reports(
    root_path: &Path,
    pdf_options: PrintToPdfOptionsLocal,
) -> Result<()> {
    for entry in WalkDir::new(root_path) {
        let entry = entry.map_err(|e| anyhow!("Error reading entry: {e}"))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension.to_str() == Some("html") {
                    let render = fs::read_to_string(path)?;

                    let bytes = PdfRenderer::render_pdf(
                        render,
                        Some(pdf_options.to_print_to_pdf_options()),
                    )
                    .with_context(|| "Error converting html to pdf format")?;
                    let (temp_path, _, _) = write_into_named_temp_file(&bytes, "reports-", ".html")
                        .with_context(|| "Error writing to file")?;

                    let filename: String = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .ok_or(anyhow!("Could not get filename"))?
                        .to_string();

                    let document_name = change_file_extension(&filename, "pdf")
                        .ok_or(anyhow!("Error changing file extension"))?;

                    let parent_dir = path.parent().ok_or_else(|| {
                        anyhow!("Could not get parent directory for '{}'", path.display())
                    })?;

                    let dest_path = parent_dir.join(&document_name);

                    fs::copy(&temp_path, &dest_path).with_context(|| {
                        format!("Failed to copy PDF to '{}'", dest_path.display())
                    })?;
                }
            }
        }
    }

    Ok(())
}

#[instrument(err)]
pub async fn post_tally_task_impl(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|error| anyhow!("Error getting client: {error}"))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting hasura transaction: {err}"))?;

    let tally_session_execution = get_last_tally_session_execution(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?
    .ok_or(anyhow!("Could not find last tally session execution."))?;

    let sqlite_file: NamedTempFile = download_sqlite_database(
        &tenant_id,
        &election_event_id,
        &tally_session_id,
        &hasura_transaction,
        &tally_session_execution,
    )
    .await?;

    let database_path_clone = sqlite_file.path().to_path_buf();

    let election_event_id_clone = election_event_id.clone();
    let tenant_id_clone = tenant_id.clone();
    let results_event: ResultsEvent = task::spawn_blocking(move || -> Result<ResultsEvent> {
        let mut sqlite_connection = SqliteConnection::open(&database_path_clone)
            .map_err(|error| anyhow!("Error opening sqlite database: {error}"))?;
        let sqlite_transaction = sqlite_connection
            .transaction()
            .map_err(|error| anyhow!("Error starting sqlite database transaction: {error}"))?;

        // Get the tar.gz from the latest results
        let results_event = find_results_event_sqlite(
            &sqlite_transaction,
            &tenant_id_clone,
            &election_event_id_clone,
        )?;
        Ok(results_event)
    })
    .await
    .map_err(|e| anyhow!("{e}"))?
    .map_err(|e| anyhow!("{e}"))?;

    let results_event_documents = results_event
        .documents
        .ok_or(anyhow!("Could not find results event id"))?;
    let tar_gz_document_id = results_event_documents
        .tar_gz
        .clone()
        .ok_or(anyhow!("No tar gz in results event"))?;

    let tar_gz_document = get_document(
        &hasura_transaction,
        &tenant_id.clone(),
        Some(election_event_id.to_string()),
        &tar_gz_document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Can't find document {}", tar_gz_document_id))?;

    let mut tar_gz_file = get_document_as_temp_file(&tenant_id, &tar_gz_document).await?;

    // Unpack targz

    let tally_path: tempfile::TempDir = extract_archive_to_temp_dir(tar_gz_file.path(), false)?;

    let pdf_options = get_tally_pdf_config(
        &hasura_transaction,
        &tenant_id,
        Some(election_event_id.clone()),
        Some(tally_session_execution.tally_session_id.clone()),
    )
    .await?
    .ok_or(anyhow!("Could not find options."))?;

    let pdf_options = PrintToPdfOptionsLocal::from_pdf_options(pdf_options);

    // Search for all html reports that do not have pdf and generate it
    find_and_process_html_reports(tally_path.path(), pdf_options)?;

    // Create the archive again
    let (tar_file_temp_path, tar_file_str, file_size) =
        create_archive_from_folder(tally_path.path(), false)?;

    let tar_document_id = Uuid::new_v4().to_string();
    let updated_targz_document = upload_and_return_document(
        &hasura_transaction,
        &tar_file_str,
        file_size,
        "application/gzip",
        &tenant_id,
        Some(election_event_id.to_string()),
        "tally.tar.gz",
        Some(tar_document_id.to_string()),
        false,
    )
    .await?;

    let results_event_id = tally_session_execution.results_event_id.ok_or(anyhow!(
        "No results event id set in tally session execution"
    ))?;

    let election_event_id_clone = election_event_id.clone();
    let tenant_id_clone = tenant_id.clone();
    let results_event_id_clone = results_event_id.clone();
    let database_path_clone = sqlite_file.path().to_path_buf();

    let mut documents = results_event_documents.clone();
    documents.tar_gz_pdfs = Some(updated_targz_document.id.clone());

    // Update the documents in hasura database
    update_results_event_documents(
        &hasura_transaction,
        &tenant_id_clone,
        &results_event_id_clone,
        &election_event_id_clone,
        &documents,
    )
    .await?;

    // Update the documents in sqlite database
    let db_result = task::spawn_blocking(move || -> Result<()> {
        let mut sqlite_connection = SqliteConnection::open(&database_path_clone)
            .map_err(|error| anyhow!("Error opening sqlite database: {error}"))?;
        let sqlite_transaction = sqlite_connection
            .transaction()
            .map_err(|error| anyhow!("Error starting sqlite database transaction: {error}"))?;

        update_results_event_documents_sqlite(
            &sqlite_transaction,
            &tenant_id_clone,
            &results_event_id_clone,
            &election_event_id_clone,
            &documents,
        )?;

        sqlite_transaction
            .commit()
            .map_err(|e| anyhow!("Error while commiting to sqlite database: {e}"))?;

        Ok(())
    })
    .await
    .map_err(|e| anyhow!("Error while updating sqlite database: {e}"))?;

    // Upload updated sqlite database
    let database_document_id = Uuid::new_v4().to_string();
    let file_name = format!("results-{}.db", results_event_id);
    let file_path = sqlite_file.path().to_string_lossy().to_string();
    let file_size = get_file_size(&file_path)?;

    let _document = upload_and_return_document(
        &hasura_transaction,
        &file_path,
        file_size,
        "application/vnd.sqlite3",
        &tenant_id,
        Some(election_event_id.to_string()),
        &file_name,
        Some(database_document_id.to_string()),
        false,
    )
    .await?;

    let previous_tally_session_documents: TallySessionDocuments = serde_json::from_value(
        tally_session_execution
            .documents
            .ok_or(anyhow!("No documents in tally session execution"))?,
    )?;

    let updated_documents = TallySessionDocuments {
        sqlite: Some(database_document_id.to_string()),
        xlsx: previous_tally_session_documents.xlsx.clone(),
    };

    let updated_status = serde_json::from_value(
        tally_session_execution
            .status
            .ok_or(anyhow!("No documents in tally session execution"))?,
    )?;

    // Add a new tally session execution
    insert_tally_session_execution(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        tally_session_execution.current_message_id,
        &tally_session_id,
        Some(updated_status),
        Some(results_event_id),
        tally_session_execution.session_ids,
        Some(updated_documents),
    )
    .await?;

    set_post_tally_task_completed(
        &hasura_transaction, //: &Transaction<'_>,
        &tenant_id,          //: &str,
        &election_event_id,  //: &str,
        &tally_session_id,   //: &str,
    )
    .await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 1200000, max_retries = 0, expires = 15)]
pub async fn post_tally_task(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
) -> Result<()> {
    let _permit = acquire_semaphore().await?;
    let Ok(lock) = PgLock::acquire(
        format!(
            "post-tally-task-{}-{}-{}",
            tenant_id, election_event_id, tally_session_id
        ),
        Uuid::new_v4().to_string(),
        ISO8601::now() + Duration::seconds(120),
    )
    .await
    else {
        info!(
            "Skipping: post tally in progress for event {} and session id {}",
            election_event_id, tally_session_id
        );
        return Ok(());
    };
    let mut interval = tokio::time::interval(ChronoDuration::from_secs(30));
    let mut current_task = tokio::spawn(post_tally_task_impl(
        tenant_id,
        election_event_id,
        tally_session_id,
    ));
    let res = loop {
        tokio::select! {
            _ = interval.tick() => {
                // Execute the callback function here
                lock.update_expiry().await?;
            }
            res = &mut current_task => {

                break res.map_err(|err| Error::String(format!("Error executing loop: {:?}", err))).flatten();
            }
        }
    };
    lock.release().await?;

    Ok(())
}
