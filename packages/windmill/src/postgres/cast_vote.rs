// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::cast_votes::CastVote;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use serde_json::value::Value;
use serde_json::{json, Map};
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
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
) -> Result<CastVote> {
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.cast_vote
                (tenant_id, election_event_id, election_id, area_id, voter_id_string, ballot_id, content, cast_ballot_signature, annotations)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    COALESCE($9::jsonb, '{}')
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

    let annotations: Value = json!({
        "ip": voter_ip,
        "country": voter_country,
    });

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
                &annotations,
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

#[instrument(skip(hasura_transaction), err)]
pub async fn get_cast_votes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    voter_id_string: &str,
) -> Result<Vec<CastVote>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT 
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
                    election_event_id
                FROM
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

    let cast_votes: Vec<CastVote> = rows
        .into_iter()
        .map(|row| -> Result<CastVote> { row.try_into() })
        .collect::<Result<Vec<CastVote>>>()?;

    Ok(cast_votes)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_cast_votes_by_election_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Vec<CastVote>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT 
                    *
                FROM
                    sequent_backend.cast_vote
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3;
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
        .await
        .map_err(|err| anyhow!("Error getting cast votes: {}", err))?;

    let cast_votes: Vec<CastVote> = rows
        .into_iter()
        .map(|row| -> Result<CastVote> { row.try_into() })
        .collect::<Result<Vec<CastVote>>>()?;

    Ok(cast_votes)
}
