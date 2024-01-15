// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use tokio_postgres::row::Row;
use tracing::instrument;
use tracing::{event, Level};
use uuid::Uuid;

#[instrument(skip(hasura_transaction), err)]
pub async fn insert_cast_vote(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
    content: &str,
    voter_id_string: &str,
    ballot_id: &str,
    cast_ballot_signature: &Vec<u8>,
) -> Result<(Uuid, DateTime<Utc>)> {
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
                RETURNING id, created_at;
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

    if 1 == rows.len() {
        let id: Uuid = rows[0].get(0);
        let created_at: DateTime<Utc> = rows[0].get(1);
        Ok((id, created_at))
    } else {
        Err(anyhow!("Unexpected rows affected {}", rows.len()))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_cast_votes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
    voter_id_string: &str,
) -> Result<Vec<(Uuid, DateTime<Utc>, Uuid)>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT id, created_at, area_id FROM
                    sequent_backend.cast_vote
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    voter_id_string = $4
                ;
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
                &voter_id_string,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error getting cast votes: {}", err))?;

    let ret: Vec<(Uuid, DateTime<Utc>, Uuid)> = rows
        .iter()
        .map(|row| {
            let id: Uuid = rows[0].get(0);
            let created_at: DateTime<Utc> = rows[0].get(1);
            let area_id: Uuid = rows[0].get(2);

            (id, created_at, area_id)
        })
        .collect();

    Ok(ret)
}
