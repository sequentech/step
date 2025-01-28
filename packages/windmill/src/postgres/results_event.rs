// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::results::*;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;
pub struct ResultsEventWrapper(pub ResultsEvent);

impl TryFrom<Row> for ResultsEventWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let documents_value: Option<Value> = item.try_get("documents")?;
        let documents: Option<ResultDocuments> = documents_value
            .map(|value| deserialize_value(value))
            .transpose()?;

        Ok(ResultsEventWrapper(ResultsEvent {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            name: item.try_get("name")?,
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            documents,
        }))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn update_results_event_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    results_event_id: &str,
    election_event_id: &str,
    documents: &ResultDocuments,
) -> Result<()> {
    let documents_value = serde_json::to_value(documents.clone())?;
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(&tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let results_event_uuid: uuid::Uuid = Uuid::parse_str(&results_event_id)
        .map_err(|err| anyhow!("Error parsing results_event_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(&election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE
                    sequent_backend.results_event
                SET
                    documents = $1
                WHERE
                    tenant_id = $2 AND
                    id = $3 AND
                    election_event_id = $4
                RETURNING
                    id;
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
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the areas query: {}", err))?;

    if 1 == rows.len() {
        Ok(())
    } else if rows.len() > 1 {
        Err(anyhow!(
            "Too many affected rows in table results_event: {}",
            rows.len()
        ))
    } else {
        Err(anyhow!("Rows not found in table results_event"))
    }
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_results_event_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    results_event_id: &str,
) -> Result<ResultsEvent> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.results_event
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    id = $3;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(results_event_id)?,
            ],
        )
        .await?;

    let results_events: Vec<ResultsEvent> = rows
        .into_iter()
        .map(|row| -> Result<ResultsEvent> {
            row.try_into()
                .map(|res: ResultsEventWrapper| -> ResultsEvent { res.0 })
        })
        .collect::<Result<Vec<ResultsEvent>>>()?;

    results_events
        .get(0)
        .map(|results_event| results_event.clone())
        .ok_or(anyhow!("Results event {results_event_id} not found"))
}

#[instrument(err, skip(hasura_transaction), ret)]
pub async fn insert_results_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<ResultsEvent> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.results_event
                (tenant_id, election_event_id)
                VALUES(
                    $1,
                    $2
                )
                RETURNING
                    *;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting row: {}", err))?;

    let values: Vec<ResultsEvent> = rows
        .into_iter()
        .map(|row| -> Result<ResultsEvent> {
            row.try_into()
                .map(|res: ResultsEventWrapper| -> ResultsEvent { res.0 })
        })
        .collect::<Result<Vec<ResultsEvent>>>()?;

    let Some(value) = values.first() else {
        return Err(anyhow!("Error inserting row"));
    };
    Ok(value.clone())
}
