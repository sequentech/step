// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::electoral_log::{list_electoral_log, GetElectoralLogBody};
use super::{
    documents::upload_and_return_document_postgres, temp_path::write_into_named_temp_file,
};
use crate::services::database::{get_hasura_pool, PgConfig};
use anyhow::{anyhow, Result};
use csv::WriterBuilder;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::Document;
use tempfile::NamedTempFile;
use tokio::fs::read;

pub async fn read_export_data(tenant_id: &str, election_event_id: &str) -> Result<Vec<u8>> {
    let mut offset = 0;
    let limit = PgConfig::from_env()?.default_sql_batch_size as i64;

    // Create a temporary file to write CSV data
    let mut temp_file = NamedTempFile::new()?;
    let mut csv_writer = WriterBuilder::new().from_writer(temp_file.as_file_mut());

    loop {
        let electoral_logs = list_electoral_log(GetElectoralLogBody {
            tenant_id: String::from(tenant_id),
            election_event_id: String::from(election_event_id),
            limit: Some(limit),
            offset: Some(offset),
            filter: None,
            order_by: None,
        })
        .await?;

        for item in &electoral_logs.items {
            csv_writer.serialize(item)?; // Serialize each item to CSV
        }

        let total = electoral_logs.total.aggregate.count;

        if electoral_logs.items.is_empty() || offset >= total {
            break;
        }

        offset += limit;
    }

    // Flush and finish writing to the temporary file
    csv_writer.flush()?;
    drop(csv_writer);

    let contents = read(temp_file).await?;

    Ok(contents)
}

pub async fn write_export_document(
    transaction: &Transaction<'_>,
    contents: Vec<u8>,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Document> {
    let name = format!("export-election-event-logs-{}", election_event_id);

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&contents, &name, ".csv")?;

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
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let data = read_export_data(tenant_id, election_event_id).await?;

    write_export_document(
        &hasura_transaction,
        data,
        document_id,
        tenant_id,
        election_event_id,
    )
    .await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
