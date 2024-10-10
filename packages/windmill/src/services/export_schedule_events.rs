// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::database::get_hasura_pool;
use super::providers::transactions_provider::provide_hasura_transaction;
use super::temp_path::{generate_temp_file, get_file_size};
use super::{
    documents::upload_and_return_document_postgres, temp_path::write_into_named_temp_file,
};
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use anyhow::Context;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::Document;
use sequent_core::types::scheduled_event::ScheduledEvent;
use tempfile::{NamedTempFile, TempPath};

pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<TempPath> {
    // Fetch the scheduled events from the database
    let scheduled_events =
        find_scheduled_event_by_election_event_id(transaction, tenant_id, election_event_id)
            .await?;

    // Serialize the scheduled events to a JSON string
    let data_str = serde_json::to_string(&scheduled_events)
        .with_context(|| "Failed to serialize scheduled events to JSON")?;

    // Write the serialized data into a temporary file
    let name = format!("scheduled_events-{}", election_event_id);
    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_str.into_bytes(), &name, ".json")
            .with_context(|| "Failed to write scheduled events into temp file")?;

    Ok(temp_path)
}

pub async fn write_export_document(
    transaction: &Transaction<'_>,
    temp_file_path: TempPath,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Document> {
    let temp_path_string = temp_file_path.to_str().unwrap().to_string();

    let file_size =
        get_file_size(temp_path_string.as_str()).with_context(|| "Error obtaining file size")?;

    let name = format!("tasks_execution-{}", election_event_id);

    upload_and_return_document_postgres(
        transaction,
        &temp_path_string,
        file_size,
        "text/csv",
        tenant_id,
        election_event_id,
        &name,
        Some(document_id.to_string()),
        false, // is_public: bool,
    )
    .await
}

pub async fn process_export(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
) -> Result<()> {
    provide_hasura_transaction(|hasura_transaction| {
        let document_id = document_id.to_string();
        let tenant_id = tenant_id.to_string();
        let election_event_id = election_event_id.to_string();

        Box::pin(async move {
            // Fetch the data into a temp file instead of a vector
            let temp_file =
                read_export_data(&hasura_transaction, &tenant_id, &election_event_id).await?;

            // Pass the temp file to the write_export_document function
            write_export_document(
                &hasura_transaction,
                temp_file,
                &document_id,
                &tenant_id,
                &election_event_id,
            )
            .await?;

            Ok(())
        })
    })
    .await
}
