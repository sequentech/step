// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::documents::upload_and_return_document;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use anyhow::Context;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::election_config::emit::{json_csv, JsonField, SCHEDULED_EVENT_COLUMNS};
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

/// One row of `export_scheduled_events-<id>.csv`, in the order the importer reads.
///
/// Named field by field on purpose. The order is [`SCHEDULED_EVENT_COLUMNS`], and
/// stating each one here is what makes a mismatch a compile-time or test failure
/// instead of a payload silently read as a task id.
fn scheduled_event_row(event: &ScheduledEvent) -> Result<Vec<JsonField>> {
    /// An `Option<T>` as a field: absent is a SQL NULL, written bare.
    fn optional<T: serde::Serialize>(value: &Option<T>) -> Result<JsonField> {
        match value {
            None => Ok(JsonField::Null),
            Some(value) => JsonField::json(value)
                .map_err(|e| anyhow!("Error serializing scheduled event field: {e:?}")),
        }
    }

    Ok(vec![
        JsonField::string(event.id.clone()),
        optional(&event.tenant_id)?,
        optional(&event.election_event_id)?,
        optional(&event.created_at)?,
        optional(&event.stopped_at)?,
        optional(&event.archived_at)?,
        optional(&event.labels)?,
        optional(&event.annotations)?,
        optional(&event.event_processor)?,
        optional(&event.cron_config)?,
        optional(&event.event_payload)?,
        optional(&event.task_id)?,
    ])
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
    let name = format!("scheduled_events-{}", election_event_id);

    // Written through the shared emitter, which is also what `step-cli` and the
    // browser-side tools use, so an export and a generated bundle are the same
    // shape rather than two implementations that happen to agree.
    //
    // This used to derive both the header and each row from
    // `serde_json::to_value(event).as_object()`, taking `.keys()` and `.values()`.
    // That worked, but only because three unstated things lined up: the
    // `preserve_order` feature is enabled somewhere in the dependency graph, so a
    // `serde_json::Map` iterates in insertion order rather than alphabetically;
    // insertion order is `ScheduledEvent`'s field order; and that order happens to
    // match what the importer reads. `import_scheduled_events.rs` takes the
    // payload from `record.get(10)` — under alphabetical ordering index 10 is
    // `task_id` and the payload is at 5, so every exported event would import with
    // its payload read as a task name. Reordering the struct, or losing
    // `preserve_order` from the graph, would have done that silently.
    let rows: Vec<Vec<JsonField>> = data
        .iter()
        .map(scheduled_event_row)
        .collect::<Result<Vec<_>>>()?;

    let data_bytes = json_csv(SCHEDULED_EVENT_COLUMNS, &rows).into_bytes();

    // Write the serialized data into a temporary file
    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_bytes, &name, ".csv")
            .with_context(|| "Failed to write scheduled events into temp file")?;

    if to_upload {
        upload_and_return_document(
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
