// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::documents::upload_and_return_document_postgres;
use super::electoral_log::{list_electoral_log, GetElectoralLogBody};
use super::providers::transactions_provider::provide_hasura_transaction;
use super::temp_path::{generate_temp_file, get_file_size};
use crate::postgres::template::get_templates_by_tenant_id;
use crate::services::database::{get_hasura_pool, PgConfig};
use anyhow::Context;
use anyhow::{anyhow, Result};
use csv::WriterBuilder;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::Document;
use tempfile::NamedTempFile;

pub async fn read_export_data(
    tenant_id: &str,
    election_event_id: &str,
    name: &str,
) -> Result<NamedTempFile> {
    let mut offset = 0;
    let limit = PgConfig::from_env()?.default_sql_batch_size as i64;

    // Create a temporary file to write CSV data
    let mut temp_file =
        generate_temp_file(&name, ".csv").with_context(|| "Error creating named temp file")?;
    let mut csv_writer = WriterBuilder::new().from_writer(temp_file.as_file_mut());
    // let templates = get_templates_by_tenant_id(tenant_id).await?;
    // //
    //     loop {
    //         let electoral_logs = list_electoral_log(GetElectoralLogBody {
    //             tenant_id: String::from(tenant_id),
    //             election_event_id: String::from(election_event_id),
    //             limit: Some(limit),
    //             offset: Some(offset),
    //             filter: None,
    //             order_by: None,
    //         })
    //         .await?;

    //         for item in &electoral_logs.items {
    //             csv_writer.serialize(item)?; // Serialize each item to CSV
    //         }

    //         let total = electoral_logs.total.aggregate.count;

    //         if electoral_logs.items.is_empty() || offset >= total {
    //             break;
    //         }

    //         offset += limit;
    //     }

    // Flush and finish writing to the temporary file
    csv_writer.flush()?;
    drop(csv_writer);

    Ok(temp_file)
}

pub async fn write_export_document(
    transaction: &Transaction<'_>,
    temp_file: NamedTempFile,
    name: &str,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Document> {
    let temp_path = temp_file.into_temp_path();
    let temp_path_string = temp_path.to_string_lossy().to_string();
    let file_size =
        get_file_size(temp_path_string.as_str()).with_context(|| "Error obtaining file size")?;

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
    election_event_id: Option<&str>,
    document_id: &str,
) -> Result<()> {
    provide_hasura_transaction(|hasura_transaction| {
        let document_id = document_id.to_string();
        let tenant_id = tenant_id.to_string();
        let election_event_id = election_event_id.map_or_else(String::new, |s| s.to_string());
        Box::pin(async move {
            // Your async code here
            let name = format!("export-template-{}", tenant_id);
            let temp_file = read_export_data(&tenant_id, &election_event_id, &name).await?;

            write_export_document(
                hasura_transaction,
                temp_file,
                &name,
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
