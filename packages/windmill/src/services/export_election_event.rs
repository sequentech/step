// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::area::export_areas;
use crate::postgres::area_contest::export_area_contests;
use crate::postgres::candidate::export_candidates;
use crate::postgres::contest::export_contests;
use crate::postgres::election::export_elections;
use crate::postgres::election_event::export_election_event;
use crate::services::database::get_hasura_pool;
use crate::services::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use futures::executor::block_on;
use futures::try_join;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::{services::keycloak::get_event_realm, types::hasura::core::Document};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::{
    documents::upload_and_return_document_postgres, temp_path::write_into_named_temp_file,
};

pub async fn read_export_data(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ImportElectionEventSchema> {
    let client = KeycloakAdminClient::new().await?;
    let board_name = get_event_realm(tenant_id, election_event_id);
    let realm = client.get_realm(&board_name).await?;
    let (election_event, elections, contests, candidates, areas, area_contests) = try_join!(
        export_election_event(&transaction, tenant_id, election_event_id),
        export_elections(&transaction, tenant_id, election_event_id),
        export_contests(&transaction, tenant_id, election_event_id),
        export_candidates(&transaction, tenant_id, election_event_id),
        export_areas(&transaction, tenant_id, election_event_id),
        export_area_contests(&transaction, tenant_id, election_event_id),
    )?;

    Ok(ImportElectionEventSchema {
        tenant_id: Uuid::parse_str(&tenant_id)?,
        keycloak_event_realm: Some(realm),
        election_event: election_event,
        elections: elections,
        contests: contests,
        candidates: candidates,
        areas: areas,
        area_contests: area_contests,
    })
}

pub async fn write_export_document(
    transaction: &Transaction<'_>,
    data: ImportElectionEventSchema,
    document_id: &str,
) -> Result<Document> {
    let data_str = serde_json::to_string(&data)?;
    let data_bytes = data_str.into_bytes();

    let name = format!("export-election-event-{}", &data.election_event.id);

    let (temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&data_bytes, &name, ".json")?;

    upload_and_return_document_postgres(
        transaction,
        &temp_path_string,
        file_size,
        "application/json",
        &data.tenant_id.to_string(),
        &data.election_event.id,
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

    let export_data = read_export_data(&hasura_transaction, tenant_id, election_event_id).await?;

    write_export_document(&hasura_transaction, export_data, document_id).await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
