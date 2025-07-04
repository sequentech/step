// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Document;
use tokio_postgres::row::Row;
use tracing::{info, instrument};
use uuid::Uuid;

pub struct DocumentWrapper(pub Document);

impl TryFrom<Row> for DocumentWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(DocumentWrapper(Document {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item
                .try_get::<_, Option<Uuid>>("tenant_id")?
                .map(|val| val.to_string()),
            election_event_id: item
                .try_get::<_, Option<Uuid>>("election_event_id")?
                .map(|val| val.to_string()),
            name: item.try_get("name")?,
            media_type: item.try_get("media_type")?,
            size: item.try_get("size")?,
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            is_public: item.try_get("is_public")?,
        }))
    }
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<String>,
    document_id: &str,
) -> Result<Option<Document>> {
    let tenant_uuid: uuid::Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: Option<uuid::Uuid> = match election_event_id {
        Some(ref id) if !id.is_empty() => {
            Some(Uuid::parse_str(id).with_context(|| "Error parsing election_event_id as UUID")?)
        }
        _ => None,
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

    let documents: Vec<Document> = rows
        .into_iter()
        .map(|row| -> Result<Document> {
            row.try_into()
                .map(|res: DocumentWrapper| -> Document { res.0 })
        })
        .collect::<Result<Vec<Document>>>()
        .with_context(|| "Error converting rows into documents")?;

    Ok(documents.get(0).cloned())
}

#[instrument(err, skip(hasura_transaction))]
pub async fn insert_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: Option<String>,
    name: &str,
    media_type: &str,
    size: i64,
    is_public: bool,
    document_id: Option<String>,
) -> Result<Document> {
    let document_uuid: uuid::Uuid = document_id
        .map(|id| Uuid::parse_str(&id))
        .unwrap_or(Ok(Uuid::new_v4()))?;
    let election_event_uuid: Option<uuid::Uuid> = election_event_id
        .map(|id| Uuid::parse_str(&id))
        .transpose()?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.document
                (
                    id,
                    tenant_id,
                    election_event_id,
                    name,
                    media_type,
                    size,
                    is_public,
                    created_at
                )
                VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    NOW()
                )
                RETURNING
                    id,
                    tenant_id,
                    election_event_id,
                    name,
                    media_type,
                    size,
                    labels,
                    annotations,
                    created_at,
                    last_updated_at,
                    is_public;
            "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &document_uuid,
                &Uuid::parse_str(tenant_id)?,
                &election_event_uuid,
                &name,
                &media_type,
                &size,
                &is_public,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting document: {}", err))?;

    let documents: Vec<Document> = rows
        .into_iter()
        .map(|row| -> Result<Document> {
            row.try_into()
                .map(|res: DocumentWrapper| -> Document { res.0 })
        })
        .collect::<Result<Vec<Document>>>()
        .with_context(|| "Error converting rows into documents")?;

    documents
        .get(0)
        .map(|val| val.clone())
        .ok_or(anyhow!("Row not inserted"))
}
