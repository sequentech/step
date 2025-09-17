// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::{keys_ceremony, trustee};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use b3::messages::message::Message;
use b3::messages::statement::StatementType;
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction as DbTransaction};
use sequent_core::services::date::{get_now_utc_unix_ms, ISO8601};
use sequent_core::services::keycloak;
use sequent_core::types::ceremonies::{
    KeysCeremonyExecutionStatus, KeysCeremonyStatus, Trustee as BasicTrustee, TrusteeStatus,
};
use sequent_core::types::hasura::core::Trustee;
use serde_json::Value;
use std::collections::HashSet;
use strand::signature::StrandSignaturePk;
use tracing::{event, info, instrument, Level};

use crate::postgres::tally_session_execution::get_last_tally_session_execution;
use crate::services::ceremonies::keys_ceremony::get_keys_ceremony_board;
use crate::services::ceremonies::serialize_logs::generate_logs;
use crate::services::ceremonies::serialize_logs::sort_logs;
use crate::services::database::get_hasura_pool;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::services::public_keys;
use crate::types::error::Result;
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
use rusqlite::Transaction as SqliteTransaction;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::types::ceremonies::TallySessionDocuments;
use tempfile::NamedTempFile;

#[instrument(skip(hasura_transaction), err)]
pub async fn download_sqlite_database(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    hasura_transaction: &DbTransaction<'_>,
) -> AnyhowResult<NamedTempFile> {
    // Recover sqlite database
    let tally_session_execution = get_last_tally_session_execution(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await?;

    let tally_session_execution = if let Some(tally_session_execution) = tally_session_execution {
        tally_session_execution
    } else {
        return Err(anyhow!(
            "Could not find last tally session execution with id {tally_session_id}"
        )
        .into());
    };

    let documents: TallySessionDocuments =
        if let Some(documents) = tally_session_execution.documents {
            let documents = serde_json::to_string(&documents)?;
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
        Some(election_event_id),
        &sqlite_database_document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Can't find document {}", sqlite_database_document_id))?;

    let mut sqlite_database = get_document_as_temp_file(&tenant_id, &document).await?;

    Ok(sqlite_database)
}

pub async fn post_tally_task(
    tenant_id: String,
    election_event_id: String,
    tally_session_id: String,
    hasura_transaction: &DbTransaction<'_>,
    sqlite_transaction: &SqliteTransaction<'_>,
) -> AnyhowResult<()> {
    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn post_tally_task(
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

    let sqlite_file: NamedTempFile = download_sqlite_database(
        tenant_id,
        election_event_id,
        tally_session_id,
        &hasura_transaction,
    )
    .await?;

    let database_path = sqlite_file.path();

    let mut sqlite_connection = SqliteConnection::open(&database_path)
        .map_err(|error| anyhow!("Error opening sqlite database: {error}"))?;
    let sqlite_transaction = sqlite_connection
        .transaction()
        .map_err(|error| anyhow!("Error starting sqlite database transaction: {error}"))?;

    sqlite_transaction
        .commit()
        .map_err(|error| anyhow!("Error commiting sqlite database transaction: {error}"))?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;

    Ok(())
}
