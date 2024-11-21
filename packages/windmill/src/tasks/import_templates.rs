// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::contest::export_contests;
use crate::postgres::template::insert_templates;
use crate::{
    postgres::document::get_document,
    services::{database::get_hasura_pool, documents::get_document_as_temp_file},
};
use anyhow::{anyhow, Context, Result};
use csv::StringRecord;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path;
use sequent_core::types::hasura::core::AreaContest;
use sequent_core::types::hasura::core::{Area, Template};
use std::io::Seek;
use tracing::instrument;
use uuid::Uuid;

#[instrument(err)]
pub async fn import_templates_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    document_id: String,
) -> Result<()> {
    let document = get_document(&hasura_transaction, &tenant_id, None, &document_id)
        .await
        .with_context(|| "Error obtaining the document")?
        .ok_or(anyhow!("document not found"))?;

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(false)
        .from_reader(temp_file);

    let mut templates: Vec<Template> = vec![];

    for result in rdr.records() {
        let record = result.with_context(|| "Error reading CSV record")?;

        let template_alias = record.get(0).unwrap_or("");
        let tenant_id = record.get(1).unwrap_or("");
        let template_content = record.get(2).unwrap_or("");
        let created_by = record.get(3).unwrap_or("");
        let labels = record.get(4).unwrap_or("");
        let annotations = record.get(5).unwrap_or("");
        let created_at = record.get(6).unwrap_or("");
        let updated_at = record.get(7).unwrap_or("");
        let communication_method = record.get(8).unwrap_or("");
        let template_type = record.get(9).unwrap_or("");

        let tenant_id_parsed = match Uuid::parse_str(tenant_id) {
            Ok(uuid) => uuid.to_string(),
            Err(_) => {
                tracing::warn!("Invalid UUID for tenant_id: {}", tenant_id);
                continue;
            }
        };
        templates.push(Template {
            alias: template_alias.to_string(),
            tenant_id: tenant_id_parsed,
            template: serde_json::from_str(template_content).unwrap_or_default(),
            created_by: created_by.to_string(),
            labels: Some(serde_json::Value::String(labels.to_string())),
            annotations: Some(serde_json::Value::String(annotations.to_string())),
            created_at: Some(created_at.parse().unwrap_or_default()),
            updated_at: Some(updated_at.parse().unwrap_or_default()),
            communication_method: communication_method.to_string(),
            r#type: template_type.to_string(),
        });
    }

    insert_templates(&hasura_transaction, &templates).await?;

    Ok(())
}
