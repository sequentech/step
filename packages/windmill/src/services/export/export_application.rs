// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::application::get_applications_by_election;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use anyhow::Context;
use anyhow::{anyhow, Result};
use base64::write;
use csv::Writer;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::{Application, Document};
use sequent_core::util::temp_path::{
    generate_temp_file, get_file_size, write_into_named_temp_file,
};
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};

#[instrument(err, skip(transaction))]
pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &Option<String>,
) -> Result<Vec<Application>> {
    // Fetch the application from the database
    let applications: Vec<Application> = get_applications_by_election(
        transaction,
        tenant_id,
        election_event_id,
        election_id.as_deref(),
    )
    .await?;

    Ok(applications)
}

#[instrument(err, skip(transaction))]
pub async fn write_export_document(
    transaction: &Transaction<'_>,
    temp_file_path: Vec<Application>,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Document> {
    let headers = vec![
        "id",
        "created_at",
        "updated_at",
        "tenant_id",
        "election_event_id",
        "area_id",
        "applicant_id",
        "applicant_data",
        "labels",
        "annotations",
        "verification_type",
        "status",
    ];
    let name = format!("applications-{}", document_id);

    let mut writer = Writer::from_writer(vec![]);
    writer.write_record(&headers)?;
    for application in temp_file_path.clone() {
        writer.write_record(&[
            &application.id,
            &application
                .created_at
                .map(|dt| dt.to_string())
                .unwrap_or_default(),
            &application
                .updated_at
                .map(|dt| dt.to_string())
                .unwrap_or_default(),
            &application.tenant_id,
            &application.election_event_id,
            &application.area_id.unwrap_or_default(),
            &application.applicant_id,
            &application.applicant_data.to_string(),
            &application
                .labels
                .map(|v| v.to_string())
                .unwrap_or_default(),
            &application
                .annotations
                .map(|v| v.to_string())
                .unwrap_or_default(),
            &application.verification_type,
            &application.status,
        ])?;
    }
    let data_bytes = writer
        .into_inner()
        .map_err(|e| anyhow!("Error converting writer into inner: {e:?}"))?;
    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_bytes, &name, ".csv")
            .map_err(|e| anyhow!("Error writing the applications into temp file: {e:?}"))?;

    let first_task = temp_file_path.first();
    if let Some(first_task) = first_task {
        upload_and_return_document(
            transaction,
            &temp_path_string,
            file_size,
            "text/csv",
            &first_task.tenant_id.to_string(),
            Some(first_task.election_event_id.clone()),
            &name,
            Some(document_id.to_string()),
            false,
        )
        .await
    } else {
        Err(anyhow!("No tasks available to write"))
    }
}

#[instrument(err)]
pub async fn process_export(
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    document_id: &str,
) -> Result<()> {
    provide_hasura_transaction(|hasura_transaction| {
        let document_id = document_id.to_string();
        let tenant_id = tenant_id.to_string();
        let election_event_id = election_event_id.to_string();
        info!(
            "Processing export for tenant_id: {}, election_event_id: {}",
            tenant_id, election_event_id
        );
        Box::pin(async move {
            // Fetch the data into a temp file instead of a vector
            let temp_file = read_export_data(
                &hasura_transaction,
                &tenant_id,
                &election_event_id,
                &election_id,
            )
            .await?;

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
