// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::template::insert_templates;
use crate::services::providers::transactions_provider::provide_hasura_transaction;
use crate::services::tasks_execution::{update_complete, update_fail};
use crate::types::error::{Error, Result};
use crate::{postgres::document::get_document, services::documents::get_document_as_temp_file};
use anyhow::{anyhow, Error as AnyhowError, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::types::hasura::core::{TasksExecution, Template};
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};

use sequent_core::services::uuid_validation::parse_uuid_v4;
use std::io::{Read, Seek};
use tracing::{info, instrument};
use uuid::Uuid;

fn parse_templates_csv<R: Read>(reader: R, tenant_id: &str) -> AnyhowResult<Vec<Template>> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(reader);

    let mut templates: Vec<Template> = vec![];

    for result in rdr.records() {
        let record = result.map_err(|e| anyhow!("Error reading CSV record: {:?}", e))?;

        let template_alias = record.get(0).unwrap_or("");
        let row_tenant_id = record.get(1).unwrap_or("");
        let template_content = record.get(2).unwrap_or("");
        let created_by = record.get(3).unwrap_or("");
        let labels = record.get(4).unwrap_or("");
        let annotations = record.get(5).unwrap_or("");
        let created_at = record.get(6).unwrap_or("");
        let updated_at = record.get(7).unwrap_or("");
        let communication_method = record.get(8).unwrap_or("");
        let template_type = record.get(9).unwrap_or("");

        if parse_uuid_v4(row_tenant_id).is_err() {
            tracing::warn!("Invalid UUID for tenant_id: {}", row_tenant_id);
            continue;
        }
        templates.push(Template {
            alias: template_alias.to_string(),
            tenant_id: tenant_id.to_string(),
            template: deserialize_str(template_content).unwrap_or_default(),
            created_by: created_by.to_string(),
            labels: Some(serde_json::Value::String(labels.to_string())),
            annotations: Some(serde_json::Value::String(annotations.to_string())),
            created_at: Some(created_at.parse().unwrap_or_default()),
            updated_at: Some(updated_at.parse().unwrap_or_default()),
            communication_method: communication_method.to_string(),
            r#type: template_type.to_string(),
        });
    }

    Ok(templates)
}

#[instrument(err)]
pub async fn import_templates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    document_id: String,
    sha256: Option<String>,
) -> AnyhowResult<()> {
    let document = get_document(hasura_transaction, &tenant_id, None, &document_id)
        .await
        .map_err(|e| anyhow!("Error obtaining the document: {:?}", e))?
        .ok_or(Error::String("document not found".to_string()))?;

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;

    match sha256 {
        Some(hash) if !hash.is_empty() => match integrity_check(&temp_file, hash) {
            Ok(_) => {
                info!("Hash verified !");
            }
            Err(HashFileVerifyError::HashMismatch(input_hash, gen_hash)) => {
                let err_str = format!("Failed to verify the integrity: Hash of voters file: {gen_hash} does not match with the input hash: {input_hash}");
                return Err(AnyhowError::new(Error::String(err_str)));
            }
            Err(err) => {
                let err_str = format!("Failed to verify the integrity: {err:?}");
                return Err(AnyhowError::new(Error::String(err_str)));
            }
        },
        _ => {
            info!("No hash provided, skipping integrity check");
        }
    }

    let templates = parse_templates_csv(temp_file, &tenant_id)?;

    insert_templates(hasura_transaction, &templates).await?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn import_templates_task(
    tenant_id: String,
    document_id: String,
    sha256: Option<String>,
    task_execution: TasksExecution,
) -> Result<()> {
    let result = provide_hasura_transaction(|hasura_transaction| {
        let document_copy = document_id.clone();
        Box::pin(async move {
            import_templates(hasura_transaction, tenant_id, document_copy, sha256).await
        })
    })
    .await;
    match result {
        Ok(_) => {
            let _res = update_complete(&task_execution, Some(document_id.clone())).await;
            Ok(())
        }
        Err(err) => {
            let err_str = format!("Error importing templates: {err:?}");
            let _res = update_fail(&task_execution, &err.to_string()).await;
            Err(err_str.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TASK_TENANT_ID: &str = "4f1c6b38-9f5e-4a59-8d3b-2a7f0c1e5d61";
    const ROW_TENANT_ID: &str = "b7e2d915-3c4a-4e8f-9a61-5d0f2c8b7e43";

    fn csv_with_row_tenant(row_tenant_id: &str) -> String {
        format!(
            "alias,tenant_id,template,created_by,labels,annotations,created_at,updated_at,communication_method,type\n\
             welcome,{row_tenant_id},{{}},admin,,,,,EMAIL,BALLOT_RECEIPT\n"
        )
    }

    #[test]
    fn imported_templates_belong_to_task_tenant() {
        let csv = csv_with_row_tenant(ROW_TENANT_ID);
        let templates = parse_templates_csv(csv.as_bytes(), TASK_TENANT_ID).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].tenant_id, TASK_TENANT_ID);
        assert_eq!(templates[0].alias, "welcome");
    }

    #[test]
    fn header_row_is_not_imported() {
        let csv = csv_with_row_tenant(TASK_TENANT_ID);
        let templates = parse_templates_csv(csv.as_bytes(), TASK_TENANT_ID).unwrap();
        assert_eq!(templates.len(), 1);
    }
}
