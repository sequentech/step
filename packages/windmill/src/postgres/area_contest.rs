// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import_election_event::{AreaContest, ImportElectionEventSchema};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

impl TryFrom<Row> for AreaContest {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(AreaContest {
            id: item.try_get("id")?,
            area_id: item.try_get("area_id")?,
            contest_id: item.try_get("contest_id")?,
        })
    }
}

#[instrument(err, skip_all)]
pub async fn insert_area_contest(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    for area_contest in &data.area_contest_list {
        let statement = hasura_transaction
            .prepare(
                r#"
                INSERT INTO sequent_backend.area_contest
                (id, tenant_id, election_event_id, contest_id, area_id, created_at, last_updated_at)
                VALUES
                ($1, $2, $3, $4, $5, NOW(), NOW());
            "#,
            )
            .await?;

        let rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &area_contest.id,
                    &data.tenant_id,
                    &Uuid::parse_str(&data.election_event.id)?,
                    &area_contest.contest_id,
                    &area_contest.area_id,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn export_area_contests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<AreaContest>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, area_id, contest_id
                FROM
                    sequent_backend.area_contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
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

    let area_contests: Vec<AreaContest> = rows
        .into_iter()
        .map(|row| -> Result<AreaContest> { row.try_into() })
        .collect::<Result<Vec<AreaContest>>>()?;

    Ok(area_contests)
}
