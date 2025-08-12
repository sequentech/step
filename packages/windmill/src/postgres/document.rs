// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::{Document, SupportMaterial};
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

pub struct SupportMaterialDocumentWrapper {
    pub support_material: SupportMaterial,
    pub document: Document,
}

impl TryFrom<Row> for SupportMaterialDocumentWrapper {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        Ok(SupportMaterialDocumentWrapper {
            support_material: SupportMaterial {
                id: row.try_get::<_, Uuid>("support_material_id")?.to_string(),
                created_at: row.get("sm_created_at"),
                last_updated_at: row.get("sm_last_updated_at"),
                kind: row.try_get("kind")?,
                data: row.try_get("data")?,
                tenant_id: row.try_get::<_, Uuid>("tenant_id")?.to_string(),
                election_event_id: row.try_get::<_, Uuid>("election_event_id")?.to_string(),
                labels: row.try_get("sm_labels")?,
                annotations: row.try_get("sm_annotations")?,
                document_id: Some(row.try_get::<_, Uuid>("document_id")?.to_string()),
                is_hidden: row.try_get("is_hidden")?,
            },
            document: Document {
                id: row.try_get::<_, Uuid>("document_id")?.to_string(),
                tenant_id: Some(row.try_get::<_, Uuid>("tenant_id")?.to_string()),
                election_event_id: Some(row.try_get::<_, Uuid>("election_event_id")?.to_string()),
                name: row.try_get("name")?,
                media_type: row.try_get("media_type")?,
                size: row.try_get("size")?,
                labels: row.try_get("doc_labels")?,
                annotations: row.try_get("doc_annotations")?,
                created_at: row.try_get("doc_created_at")?,
                last_updated_at: row.try_get("doc_last_updated_at")?,
                is_public: row.try_get("is_public")?,
            },
        })
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

/// Returns a vector of tuples of the (SupportMaterial, Document)s
/// associated with a given election event.
#[instrument(err, skip(hasura_transaction))]
pub async fn get_support_material_documents(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<Vec<(SupportMaterial, Document)>>> {
    let tenant_uuid: uuid::Uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;

    let document_statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                sm.id AS support_material_id,
                sm.kind,
                sm.data,
                sm.labels AS sm_labels,
                sm.annotations AS sm_annotations,
                sm.is_hidden,
                sm.created_at AS sm_created_at,
                sm.last_updated_at AS sm_last_updated_at,
                d.id AS document_id,
                d.tenant_id AS tenant_id,
                d.election_event_id AS election_event_id,
                d.name,
                d.media_type,
                d.size,
                d.labels AS doc_labels,
                d.annotations AS doc_annotations,
                d.is_public,
                d.created_at AS doc_created_at,
                d.last_updated_at AS doc_last_updated_at
            FROM
                "sequent_backend".support_material sm
            JOIN
                "sequent_backend".document d
            ON
                sm.document_id::uuid = d.id
            WHERE
                sm.tenant_id = $1
                AND sm.election_event_id = $2
                AND sm.is_hidden = false
                AND d.tenant_id = sm.tenant_id
                AND d.election_event_id = sm.election_event_id;
            "#,
        )
        .await
        .with_context(|| "Error preparing the get_support_material_documents query")?;

    let rows: Vec<Row> = hasura_transaction
        .query(&document_statement, &[&tenant_uuid, &election_event_uuid])
        .await
        .map_err(|err| anyhow!("Error running the get_support_material_documents query: {err}"))?;
    let documents: Vec<(SupportMaterial, Document)> = rows
        .into_iter()
        .map(|row| -> Result<(SupportMaterial, Document)> {
            row.try_into()
                .map(|res: SupportMaterialDocumentWrapper| (res.support_material, res.document))
        })
        .collect::<Result<Vec<(SupportMaterial, Document)>>>()
        .with_context(|| "Error converting rows into (SupportMaterial, Document)")?;

    Ok(Some(documents))
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
