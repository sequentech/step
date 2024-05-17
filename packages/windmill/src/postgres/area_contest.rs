// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

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
                    &Uuid::parse_str(&data.election_event_data.id)?,
                    &area_contest.contest_id,
                    &area_contest.area_id,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}
