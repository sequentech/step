// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub key: String,
    pub value: Vec<u8>,
    pub created_at: Option<DateTime<Local>>,
}

impl TryFrom<Row> for Secret {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(Secret {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item
                .try_get::<_, Option<Uuid>>("election_event_id")?
                .map(|val| val.to_string()),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            key: item.try_get("key")?,
            value: item.try_get::<_, &[u8]>("value")?.into(),
            created_at: item.get("created_at"),
        })
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_secret_by_key(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    key: &str,
) -> Result<Option<Secret>> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;

    let election_event_uuid = election_event_id
        .map(|id| {
            Uuid::parse_str(id)
                .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))
        })
        .transpose()?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                "sequent_backend".secret 
            WHERE
                tenant_id = $1 AND
                key = $2 AND
                (election_event_id = $3 OR $3 IS NULL)
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &key, &election_event_uuid])
        .await
        .map_err(|err| anyhow!("Error reading secret: {err}"))?;

    let secrets = rows
        .into_iter()
        .map(|row| -> Result<Secret> { row.try_into() })
        .collect::<Result<Vec<Secret>>>()
        .with_context(|| "Error converting rows into Secrets")?;

    if 0 == secrets.len() {
        return Ok(None);
    } else if secrets.len() > 1 {
        return Err(anyhow!("Found too many secrets: {}", secrets.len()));
    } else {
        Ok(Some(secrets[0].clone()))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_secret(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<&str>,
    key: &str,
    encrypted_bytes: &Vec<u8>,
) -> Result<Secret> {
    let tenant_uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid = election_event_id
        .map(|id| {
            Uuid::parse_str(id)
                .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))
        })
        .transpose()?;
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    "sequent_backend".secret
                (tenant_id, key, value, election_event_id) 
                VALUES
                    ($1, $2, $3, $4)
                RETURNING
                    *;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing scheduled event statement: {}", err))?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &key, &encrypted_bytes, &election_event_uuid],
        )
        .await
        .map_err(|err| anyhow!("Error inserting scheduled event: {}", err))?;

    let rows: Vec<Secret> = rows
        .into_iter()
        .map(|row| -> Result<Secret> { row.try_into() })
        .collect::<Result<Vec<Secret>>>()
        .map_err(|err| anyhow!("Error deserializing secret: {}", err))?;

    if 1 == rows.len() {
        Ok(rows[0].clone())
    } else {
        Err(anyhow!("Unexpected rows affected {}", rows.len()))
    }
}
