// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::tenant::get_tenant_by_id;
use crate::services::google_meet::take_service_account_key;
use crate::types::documents::EDocuments;
use anyhow::Context;
use anyhow::{anyhow, Result};
use csv::Writer;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::Tenant;
use sequent_core::util::temp_path::write_into_named_temp_file;
use tempfile::{NamedTempFile, TempPath};
use tracing::{event, info, instrument, Level};

#[instrument(err, skip(transaction))]
pub async fn read_tenant_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
) -> Result<Tenant> {
    let tenant: Tenant = get_tenant_by_id(transaction, tenant_id).await?;

    Ok(tenant)
}

#[instrument(err, skip(transaction))]
pub async fn write_export_document(
    data: Tenant,
    transaction: &Transaction<'_>,
    document_id: &str,
    tenant_id: &str,
) -> Result<(TempPath)> {
    let headers = vec![
        "id".to_string(),
        "slug".to_string(),
        "created_at".to_string(),
        "updated_at".to_string(),
        "labels".to_string(),
        "annotations".to_string(),
        "is_active".to_string(),
        "voting_channels".to_string(),
        "settings".to_string(),
        "test".to_string(),
    ];

    let name = format!("{}-{}", EDocuments::TENANT_CONFIG, tenant_id);

    let mut writer = Writer::from_writer(vec![]);
    writer.write_record(&headers)?;

    writer.write_record(&tenant_export_values(data)?)?;

    let data_bytes = writer
        .into_inner()
        .map_err(|e| anyhow!("Error converting writer into inner: {e:?}"))?;

    // Write the serialized data into a temporary file
    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_bytes, &name, ".csv")
            .with_context(|| "Failed to write tenant into temp file")?;

    Ok(temp_path)
}

fn tenant_export_values(mut data: Tenant) -> Result<Vec<String>> {
    if let Some(settings) = data.settings.as_mut() {
        take_service_account_key(settings);
    }

    Ok(serde_json::to_value(data)?
        .as_object()
        .ok_or_else(|| anyhow!("Failed to convert tenant to JSON object"))?
        .values()
        .map(|value| value.to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tenant_export_values_omit_service_account_key() {
        let tenant = Tenant {
            id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5".to_string(),
            slug: "tenant".to_string(),
            created_at: None,
            updated_at: None,
            labels: None,
            annotations: None,
            is_active: true,
            voting_channels: None,
            settings: Some(json!({
                "gapi_key": {"private_key": "test-private-key"},
                "gapi_email": "organizer@example.org",
            })),
            test: None,
        };

        let values = tenant_export_values(tenant).expect("tenant export values");

        let settings = values.join(",");
        assert!(!settings.contains("gapi_key"));
        assert!(!settings.contains("test-private-key"));
        assert!(settings.contains("organizer@example.org"));
    }
}
