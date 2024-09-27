// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Pool, PoolError, Runtime, Transaction};
use std::convert::From;
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[instrument(skip(transaction), err)]
pub async fn publish_tally_sheet(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    tally_sheet_id: &str,
    user_id: &str,
    publish: bool,
) -> Result<Option<()>> {
    let set_published_at = if publish { "now()" } else { "NULL" };
    let filter_published_at = if publish { "NULL" } else { "NOT NULL" };
    let publish_statement = transaction
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
    let publish_rows: Vec<Row> = transaction
        .query(&publish_statement, &publish_params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;
    if publish_rows.len() != 1 {
        return Ok(None);
    }
    Ok(Some(()))
}
