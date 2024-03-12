// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::results::ResultDocuments;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[instrument(skip(hasura_transaction, content, cast_ballot_signature), err)]
pub async fn insert_cast_vote(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
    content: &str,
    voter_id_string: &str,
    ballot_id: &str,
    cast_ballot_signature: &[u8],
) -> Result<CastVote> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.cast_vote
                (tenant_id, election_event_id, election_id, area_id, voter_id_string, ballot_id, content, cast_ballot_signature)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8
                )
                RETURNING
                    id,
                    ballot_id,
                    election_id,
                    election_event_id,
                    tenant_id,
                    election_id,
                    area_id,
                    created_at,
                    last_updated_at,
                    labels,
                    annotations,
                    content,
                    cast_ballot_signature,
                    voter_id_string,
                    election_event_id;
            "#,
        )
        .await?;
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

    let cast_votes: Vec<CastVote> = rows
        .into_iter()
        .map(|row| -> Result<CastVote> { row.try_into() })
        .collect::<Result<Vec<CastVote>>>()?;

    if 1 == cast_votes.len() {
        Ok(cast_votes[0].clone())
    } else {
        Err(anyhow!("Unexpected rows affected {}", cast_votes.len()))
    }
}
