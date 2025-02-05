// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::results::ResultDocuments;
use serde_json::Value;
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(hasura_transaction, documents), err)]
pub async fn insert_results_election_area_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    area_name: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    let documents_value = serde_json::to_value(documents.clone())?;
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let results_event_uuid: uuid::Uuid = Uuid::parse_str(&results_event_id)
        .map_err(|err| anyhow!("Error parsing results_event_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(&election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid = Uuid::parse_str(&area_id)
        .map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.results_election_area (
                    documents, 
                    tenant_id, 
                    results_event_id, 
                    election_event_id, 
                    election_id, 
                    area_id,
                    name
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING id;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &documents_value,
                &tenant_uuid,
                &results_event_uuid,
                &election_event_uuid,
                &election_uuid,
                &area_uuid,
                &area_name,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error at inser into results_election_area {} ", err))?;
    Ok(())
}
