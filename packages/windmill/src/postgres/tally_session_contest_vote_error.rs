// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::results::ResultDocuments;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(hasura_transaction, ballot_errors), err)]
pub async fn insert_tally_session_contest_vote_error(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    contest_id: Uuid,
    tally_session_id: Uuid,
    area_id: Uuid,
    tally_session_contest_id: Uuid,
    ballot_errors: Vec<(Uuid, String)>,  // Vec<(cast_vote_id, error)>
) -> Result<()> {
    let values: String = ballot_errors
        .into_iter()
        .enumerate()
        .map(|(i, ballot_error)| {
            let delta = 8 * i;
            format!("(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})", delta + 1, delta + 2, delta + 3, delta + 4, delta + 5, delta + 6, delta + 7, delta + 8)
        })
        .collect::<Vec<String>>()
        .join(",\n");
    let statement_string = format!(r#"
        INSERT INTO
            sequent_backend.tally_session_contest_vote_error
        (tenant_id, election_event_id, contest_id, tally_session_id, area_id, cast_vote_id, error, tally_session_contest_id)
        VALUES {}
        RETURNING
            id
    "#,
    values);
    let statement = hasura_transaction
        .prepare(&statement_string)
        .await?;

    let values_row: Vec<&(dyn ToSql + Sync)> = ballot_errors
        .iter()
        .map(|(cast_vote_id, error)| {
            vec![
                &tenant_id,
                &election_event_id,
                &contest_id,
                &tally_session_id,
                &area_id,
                cast_vote_id,
                error,
                &tally_session_contest_id
            ]
        })
        .collect()
        .flatten();
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &election_id,
                &area_id,
                &voter_id_string,
                &ballot_id,
                &content,
                &cast_ballot_signature,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting cast vote: {}", err))?;

    Ok(())
}
