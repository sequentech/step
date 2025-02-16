// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::documents::upload_and_return_document_postgres;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use anyhow::Context;
use anyhow::{anyhow, Result};
use csv::Writer;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::scheduled_event::ScheduledEvent;
use sequent_core::util::temp_path::write_into_named_temp_file;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};

#[instrument(err, skip(transaction))]
pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<ScheduledEvent>> {
    // Fetch the scheduled events from the database
    let scheduled_events: Vec<ScheduledEvent> =
        find_scheduled_event_by_election_event_id(transaction, tenant_id, election_event_id)
            .await?;

    Ok(scheduled_events)
}

#[instrument(err, skip(transaction))]
pub async fn write_export_document(
    data: Vec<ScheduledEvent>,
    transaction: &Transaction<'_>,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    to_upload: bool,
) -> Result<(TempPath)> {
    let headers = if let Some(example_event) = data.get(0) {
        serde_json::to_value(example_event)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert ScheduledEvent to JSON object for headers"))?
            .keys()
            .cloned()
            .collect::<Vec<String>>()
    } else {
        vec![
            "id".to_string(),
            "tenant_id".to_string(),
            "election_event_id".to_string(),
            "created_at".to_string(),
            "stopped_at".to_string(),
            "archived_at".to_string(),
            "labels".to_string(),
            "annotations".to_string(),
            "event_processor".to_string(),
            "cron_config".to_string(),
            "event_payload".to_string(),
            "task_id".to_string(),
        ]
    };

    let name = format!("scheduled_events-{}", election_event_id);

    let mut writer = Writer::from_writer(vec![]);
    writer.write_record(&headers)?;

    for scheduled_event in data.clone() {
        let values: Vec<String> = serde_json::to_value(scheduled_event)?
            .as_object()
            .ok_or_else(|| anyhow!("Failed to convert ScheduledEvent to JSON object"))?
            .values()
            .map(|value| value.to_string())
            .collect();

        writer.write_record(&values)?;
    }

    let data_bytes = writer
        .into_inner()
        .map_err(|e| anyhow!("Error converting writer into inner: {e:?}"))?;

    // Write the serialized data into a temporary file
    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_bytes, &name, ".csv")
            .with_context(|| "Failed to write scheduled events into temp file")?;

    if to_upload {
        upload_and_return_document_postgres(
            transaction,
            &temp_path_string,
            file_size,
            "text/csv",
            tenant_id,
            Some(election_event_id.to_string()),
            &name,
            Some(document_id.to_string()),
            false, // is_public: bool,
        )
        .await?;
    }

    Ok(temp_path)
}

#[instrument(err)]
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
            // Fetch the data and reformat it
            let data =
                read_export_data(&hasura_transaction, &tenant_id, &election_event_id).await?;

            // Pass the temp file to the write_export_document function
            write_export_document(
                data,
                &hasura_transaction,
                &document_id,
                &tenant_id,
                &election_event_id,
                true,
            )
            .await?;

            Ok(())
        })
    })
    .await
}
