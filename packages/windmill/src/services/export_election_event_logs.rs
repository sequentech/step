// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::electoral_log::{list_electoral_log, GetElectoralLogBody};
use super::{
    documents::upload_and_return_document_postgres, temp_path::write_into_named_temp_file,
};
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::{services::keycloak::get_event_realm, types::hasura::core::Document};

pub async fn read_export_data(tenant_id: &str, election_event_id: &str) -> Result<String> {
    let mut offset = 0;
    let limit = 500;
    let mut all_items = Vec::new();

    loop {
        let electoral_logs = list_electoral_log(GetElectoralLogBody {
            tenant_id: String::from(tenant_id),
            election_event_id: String::from(election_event_id),
            limit: Some(limit),
            offset: Some(offset),
            filter: None,
            order_by: None,
        })
        .await?;

        let items = electoral_logs.items;
        let total = electoral_logs.total.aggregate.count as usize;
        all_items.extend(items);

        if all_items.len() >= total {
            break;
        }

        offset += limit;
    }
    let data = serde_json::to_string(&all_items)?;
    Ok(data)
}

pub async fn write_export_document(
    transaction: &Transaction<'_>,
    data: &str,
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Document> {
    let data_str = data.to_string();
    let data_bytes = data_str.into_bytes();

    let name = format!("export-election-event-logs-{}", election_event_id);

    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_bytes, &name, ".json")?;

    upload_and_return_document_postgres(
        transaction,
        &temp_path_string,
        file_size,
        "application/json",
        tenant_id,
        election_event_id,
        &name,
        Some(document_id.to_string()),
        false, // is_public: bool,
    )
    .await
}

pub async fn process_export(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let data = read_export_data(tenant_id, election_event_id).await?;

    write_export_document(
        &hasura_transaction,
        data.as_str(),
        document_id,
        tenant_id,
        election_event_id,
    )
    .await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
