// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::application::insert_applications;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::{
    postgres::document::get_document,
    services::documents::get_document_as_temp_file,
    services::tasks_execution::{update_complete, update_fail},
    types::error::Result,
};
use anyhow::{anyhow, Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::hasura::core::TasksExecution;
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};
use std::io::Seek;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 2)]
pub async fn import_applications(
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
    task_execution: TasksExecution,
) -> Result<()> {
    let result = provide_hasura_transaction(|hasura_transaction| {
        let document_copy = document_id.clone();
        Box::pin(async move {
            import_applications_task(
                hasura_transaction,
                tenant_id,
                election_event_id,
                document_copy.clone(),
                sha256,
            )
            .await
        })
    })
    .await;

    match result {
        Ok(_) => {
            let _res = update_complete(&task_execution, Some(document_id.clone())).await;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Error importing applications: {err:?}");
            let _res = update_fail(&task_execution, &err.to_string()).await;
            Err(err_str.into())
        }
    }
}

#[instrument(err)]
pub async fn import_applications_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    election_event_id: String,
    document_id: String,
    sha256: Option<String>,
) -> AnyhowResult<()> {
    let document = get_document(hasura_transaction, &tenant_id, None, &document_id)
        .await
        .with_context(|| "Error obtaining the document")?
        .ok_or(anyhow!("document not found"))?;

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;

    match sha256 {
        Some(hash) if !hash.is_empty() => match integrity_check(&temp_file, hash) {
            Ok(_) => {
                info!("Hash verified !");
            }
            Err(HashFileVerifyError::HashMismatch(input_hash, gen_hash)) => {
                let err_str = format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}");
                return Err(anyhow!(err_str));
            }
            Err(err) => {
                let err_str = format!("Failed to verify the integrity: {err:?}");
                return Err(anyhow!(err_str));
            }
        },
        _ => {
            info!("No hash provided, skipping integrity check");
        }
    }

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(temp_file);

    let mut applications: Vec<Application> = vec![];

    for result in rdr.records() {
        let record = result.with_context(|| "Error reading CSV record")?;

        let _id = record.get(0).unwrap_or("");
        let created_at = record.get(1).unwrap_or("");
        let updated_at = record.get(2).unwrap_or("");
        let tenant_id = record.get(3).unwrap_or("");
        let election_event_id = record.get(4).unwrap_or("");
        let area_id = record.get(5).unwrap_or("");
        let applicant_id = record.get(6).unwrap_or("");
        let applicant_data = record.get(7).unwrap_or("");
        let labels = record.get(8).unwrap_or("");
        let annotations = record.get(9).unwrap_or("");
        let verification_type = record.get(10).unwrap_or("");
        let status = record.get(11).unwrap_or("");

        let new_template_id = Uuid::new_v4();

        let tenant_id_parsed = match Uuid::parse_str(tenant_id) {
            Ok(uuid) => uuid.to_string(),
            Err(_) => {
                tracing::warn!("Invalid UUID for tenant_id: {}", tenant_id);
                continue;
            }
        };

        applications.push(Application {
            id: new_template_id.to_string(),
            created_at: Some(created_at.parse().unwrap_or_default()),
            updated_at: Some(updated_at.parse().unwrap_or_default()),
            tenant_id: tenant_id_parsed,
            election_event_id: election_event_id.to_string(),
            area_id: Some(area_id.to_string()),
            applicant_id: applicant_id.to_string(),
            applicant_data: serde_json::from_str(applicant_data).unwrap_or_default(),
            labels: Some(serde_json::Value::String(labels.to_string())),
            annotations: Some(serde_json::Value::String(annotations.to_string())),
            verification_type: verification_type.to_string(),
            status: status.to_string(),
        });
    }

    insert_applications(hasura_transaction, &applications).await?;

    Ok(())
}
