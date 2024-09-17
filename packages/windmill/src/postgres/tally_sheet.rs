// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::anyhow;
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::hasura::core::TallySheet;
use sequent_core::types::tally_sheets::AreaContestResults;
use serde_json::Value;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

pub struct TallySheetWrapper(pub TallySheet);

impl TryFrom<Row> for TallySheetWrapper {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        let content_val: Option<Value> = item.try_get("content")?;
        let content: Option<AreaContestResults> =
            content_val.map(|val| deserialize_value(val)).transpose()?;
        Ok(TallySheetWrapper(TallySheet {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            contest_id: item.try_get::<_, Uuid>("contest_id")?.to_string(),
            area_id: item.try_get::<_, Uuid>("area_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            published_at: item.get("published_at"),
            published_by_user_id: item.try_get("published_by_user_id")?,
            content: content,
            channel: item.try_get("channel")?,
            deleted_at: item.get("deleted_at"),
            created_by_user_id: item.try_get("created_by_user_id")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn get_published_tally_sheets_by_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<TallySheet>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    *
                FROM
                    sequent_backend.tally_sheet
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    published_at IS NOT NULL AND
                    deleted_at IS NULL;
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
        .await?;

    let election_events: Vec<TallySheet> = rows
        .into_iter()
        .map(|row| -> Result<TallySheet> {
            row.try_into()
                .map(|res: TallySheetWrapper| -> TallySheet { res.0 })
        })
        .collect::<Result<Vec<TallySheet>>>()?;

    Ok(election_events)
}
