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
use sequent_core::types::hasura::core::{TasksExecution, Template};
use sequent_core::util::integrity_check::{integrity_check, HashFileVerifyError};

use sequent_core::services::reports::bundle;
use std::io::{Read, Seek};
use tracing::{info, instrument};

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

    temp_file.rewind()?;
    let mut bytes = Vec::new();
    temp_file
        .take(bundle::MAX_ZIP_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    let templates: Vec<Template> = bundle::decode(&bytes)
        .map_err(|e| anyhow!(e))?
        .into_iter()
        .map(|row| Template {
            alias: row.alias,
            // The selected tenant is authoritative, including for archives
            // exported from another tenant. Never trust an archive tenant ID.
            tenant_id: tenant_id.clone(),
            template: row.template,
            created_by: row.created_by,
            labels: row.labels,
            annotations: row.annotations,
            created_at: row.created_at,
            updated_at: row.updated_at,
            communication_method: row.communication_method,
            r#type: row.r#type,
        })
        .collect();

    let mut aliases = std::collections::HashSet::new();
    if templates
        .iter()
        .any(|template| !aliases.insert(&template.alias))
    {
        return Err(anyhow!(
            "The bundle contains duplicate aliases for the selected tenant"
        ));
    }
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
