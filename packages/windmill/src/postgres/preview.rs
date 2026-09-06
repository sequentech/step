// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Preview;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct PreviewWrapper(pub Preview);

impl TryFrom<Row> for PreviewWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(PreviewWrapper(Preview {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            document_id: item.try_get::<_, Uuid>("document_id")?.to_string(),
            url: item.try_get("url")?,
            requested_by: item.try_get("requested_by")?,
            annotations: item.try_get("annotations")?,
            created_at: item.get("created_at"),
            updated_at: item.get("updated_at"),
        }))
    }
}

#[instrument(err, skip(hasura_transaction))]
pub async fn insert_preview(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    document_id: &str,
    url: String,
    requested_by: &str,
) -> Result<()> {
    let document_uuid =
        Uuid::parse_str(document_id).with_context(|| "Error parsing tenant_id as UUID")?;

    let tenant_uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.preview
                (
                    tenant_id,
                    document_id,
                    url,
                    requested_by,
                    created_at,
                    updated_at
                )
                VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    NOW(),
                    NOW()
                );
            "#,
        )
        .await?;

    let _rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&tenant_uuid, &document_uuid, &url, &requested_by],
        )
        .await
        .map_err(|err| anyhow!("Error inserting preview: {}", err))?;

    Ok(())
}
