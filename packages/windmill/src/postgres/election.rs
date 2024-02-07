// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

/**
 * Returns a vector of areas per election event, with the posibility of
 * filtering by area_id
 */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_election_max_revotes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<usize> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                id, num_allowed_revotes
            FROM
                sequent_backend.election
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
                &Uuid::parse_str(election_id)?,
            ],
        )
        .await?;

    event!(Level::INFO, "rows: {:?}", rows);

    let revotes: Vec<usize> = rows
        .iter()
        .map(|row| {
            let num_allowed_revotes: Option<i32> = row.try_get("num_allowed_revotes")?;

            Ok(num_allowed_revotes.unwrap_or(1) as usize)
        })
        .collect::<Result<Vec<usize>>>()?;

    let data = revotes.get(0).unwrap_or(&1).clone();

    Ok(data)
}
