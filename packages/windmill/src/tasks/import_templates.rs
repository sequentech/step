// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::template::insert_templates;
use crate::types::error::{Error, Result};
use crate::{postgres::document::get_document, services::documents::get_document_as_temp_file};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use sequent_core::types::hasura::core::Template;
use sequent_core::util::integrity_check::integrity_check;

use std::io::Seek;
use tracing::{info, instrument};
use uuid::Uuid;

#[instrument(err)]
pub async fn import_templates_task(
    hasura_transaction: &Transaction<'_>,
    tenant_id: String,
    document_id: String,
    sha256: Option<String>,
) -> Result<()> {
    let document = get_document(hasura_transaction, &tenant_id, None, &document_id)
        .await
        .map_err(|e| "Error obtaining the document: {:e?}")?
        .ok_or(Error::String("document not found".to_string()))?;

    let mut temp_file = get_document_as_temp_file(&tenant_id, &document).await?;
    temp_file.rewind()?;

    match sha256 {
        Some(hash) if !hash.is_empty() => match integrity_check(&temp_file, hash) {
            Ok(_) => {
                info!("Hash verified !");
            }
            Err(err) => {
                return Err(err.into());
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

    let mut templates: Vec<Template> = vec![];

    for result in rdr.records() {
        let record = result.map_err(|e| "Error reading CSV record: {:e?}")?;

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

    insert_templates(hasura_transaction, &templates).await?;

    Ok(())
}
