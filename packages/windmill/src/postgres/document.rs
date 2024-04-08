// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::{User, VotesInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Document {
    pub id: String,
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub name: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_updated_at: Option<DateTime<Utc>>,
    pub is_public: Option<bool>,
}

impl TryFrom<Row> for Document {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(Document {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item
                .try_get::<_, Option<Uuid>>("election_event_id")?
                .map(|val| val.to_string()),
            name: item.get("name"),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            is_public: item.get("is_public"),
        })
    }
}

#[instrument(err)]
pub async fn get_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<String>,
    document_id: &str,
) -> Result<Option<Document>> {
    let tenant_uuid: uuid::Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: Option<uuid::Uuid> = match election_event_id {
        Some(ref election_event_id) => Some(
            Uuid::parse_str(election_event_id)
                .with_context(|| "Error parsing election_event_id as UUID")?,
        ),
        None => None,
    };
    let document_uuid: uuid::Uuid =
        Uuid::parse_str(document_id).with_context(|| "Error parsing document_id as UUID")?;

    let document_statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM "sequent_backend".document
            WHERE
                tenant_id = $1
                AND (election_event_id = $2 OR $2 IS NULL)
                AND id = $3
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &document_statement,
            &[&tenant_uuid, &election_event_uuid, &document_uuid],
        )
        .await
        .map_err(|err| anyhow!("Error running the document query: {err}"))?;

    let documents = rows
        .into_iter()
        .map(|row| -> Result<Document> { row.try_into() })
        .collect::<Result<Vec<Document>>>()
        .with_context(|| "Error converting rows into documents")?;

    Ok(documents.get(0).cloned())
}
