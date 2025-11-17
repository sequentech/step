// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
use tokio_postgres::types::ToSql;
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

#[instrument(skip(hasura_transaction), err)]
pub async fn publish_tally_sheet(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_sheet_id: &str,
    user_id: &str,
    publish: bool,
) -> Result<Option<()>> {
    let set_published_at = if publish { "now()" } else { "NULL" };
    let filter_published_at = if publish { "NULL" } else { "NOT NULL" };
    let publish_statement = hasura_transaction
        .prepare(
            format!(
                r#"
        UPDATE sequent_backend.tally_sheet tally_sheet
        SET
            published_at = {set_published_at},
            published_by_user_id = $4
        WHERE
            tally_sheet.tenant_id = $1 AND
            tally_sheet.election_event_id = $2 AND
            tally_sheet.id = $3 AND
            tally_sheet.deleted_at IS NULL AND
            tally_sheet.published_at IS {filter_published_at}
        RETURNING *
    "#
            )
            .as_str(),
        )
        .await?;

    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let tally_sheet_uuid: uuid::Uuid = Uuid::parse_str(tally_sheet_id)
        .map_err(|err| anyhow!("Error parsing tally_sheet_id as UUID: {}", err))?;
    let publish_params: Vec<&(dyn ToSql + Sync)> = vec![
        &tenant_uuid,
        &election_event_uuid,
        &tally_sheet_uuid,
        &user_id,
    ];
    let publish_rows: Vec<Row> = hasura_transaction
        .query(&publish_statement, &publish_params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;
    if publish_rows.len() != 1 {
        return Ok(None);
    }
    Ok(Some(()))
}
