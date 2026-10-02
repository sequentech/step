// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::template::get_templates_by_tenant_id;
use crate::services::database::get_hasura_pool;
use crate::services::documents::upload_and_return_document;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::reports::{bundle, platform_csv::ImportedTemplate};
use sequent_core::types::hasura::core::Document;
use sequent_core::types::hasura::core::Template;
use sequent_core::util::temp_path::write_into_named_temp_file;
use tracing::instrument;

#[instrument(err, skip(transaction))]
pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Vec<Template>> {
    let templates = get_templates_by_tenant_id(transaction, tenant_id).await?;

    let transformed_templates: Vec<Template> = templates
        .into_iter()
        .map(|template| Template {
            alias: template.alias.to_string(),
            tenant_id: template.tenant_id.to_string(),
            template: template.template,
            created_by: template.created_by,
            labels: template.labels,
            annotations: template.annotations,
            created_at: template.created_at,
            updated_at: template.updated_at,
            communication_method: template.communication_method,
            r#type: template.r#type,
        })
        .collect();
    Ok(transformed_templates)
}

#[instrument(err, skip(transaction, data))]
pub async fn write_export_document(
    transaction: &Transaction<'_>,
    data: Vec<Template>,
    document_id: &str,
) -> Result<Document> {
    let name = format!("template-{}", document_id);
    let full_name = format!("{}.zip", name);
    let rows: Vec<ImportedTemplate> = data
        .iter()
        .map(|template| ImportedTemplate {
            alias: template.alias.clone(),
            tenant_id: template.tenant_id.clone(),
            template: template.template.clone(),
            created_by: template.created_by.clone(),
            labels: template.labels.clone(),
            annotations: template.annotations.clone(),
            created_at: template.created_at,
            updated_at: template.updated_at,
            communication_method: template.communication_method.clone(),
            r#type: template.r#type.clone(),
        })
        .collect();
    let data_bytes = bundle::encode(rows).map_err(|e| anyhow!(e))?;
    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_bytes, &name, ".zip")
            .map_err(|e| anyhow!("Error writing template bundle: {e:?}"))?;

    if let Some(first_template) = data.first() {
        upload_and_return_document(
            transaction,
            &temp_path_string,
            file_size,
            "application/zip",
            &first_template.tenant_id.to_string(),
            None,
            &full_name,
            Some(document_id.to_string()),
            false,
        )
        .await
    } else {
        Err(anyhow::anyhow!("No templates available to write"))
    }
}

#[instrument(err)]
pub async fn process_export(tenant_id: &str, document_id: &str) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let export_data = read_export_data(&hasura_transaction, tenant_id).await?;
    write_export_document(&hasura_transaction, export_data.clone(), document_id).await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {e}"));

    Ok(())
}
