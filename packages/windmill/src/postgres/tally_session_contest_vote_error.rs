// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_tally_session_contest_vote_error(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    contest_id: Uuid,
    tally_session_id: Uuid,
    area_id: Uuid,
    tally_session_contest_id: Uuid,
    cast_vote_id: Uuid,
    error: String,
) -> Result<()> {
    let statement = hasura_transaction
        .prepare(r#"
            INSERT INTO
                sequent_backend.tally_session_contest_vote_error
            (tenant_id, election_event_id, contest_id, tally_session_id, area_id, cast_vote_id, error, tally_session_contest_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id
        "#)
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &contest_id,
                &tally_session_id,
                &area_id,
                &cast_vote_id,
                &error,
                &tally_session_contest_id,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting cast vote: {}", err))?;

    Ok(())
}
