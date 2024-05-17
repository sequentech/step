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
pub async fn insert_contest(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    for contest in &data.contests {
        contest.data.validate()?;

        let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.contest
                (id, tenant_id, election_event_id, election_id, created_at, last_updated_at, labels, annotations, is_acclaimed, is_active, name, description, presentation, min_votes, max_votes, voting_type, counting_algorithm, is_encrypted, tally_configuration, conditions, winning_candidates_num, alias, image_document_id)
                VALUES
                ($1, $2, $3, $4, NOW(), NOW(), $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21);
            "#,
        )
        .await?;

        let rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &contest.id,
                    &Uuid::parse_str(&contest.data.tenant_id)?,
                    &Uuid::parse_str(&contest.data.election_event_id)?,
                    &Uuid::parse_str(&contest.data.election_id)?,
                    &contest.data.labels,
                    &contest.data.annotations,
                    &contest.data.is_acclaimed,
                    &contest.data.is_active,
                    &contest.data.name,
                    &contest.data.description,
                    &contest.data.presentation,
                    &contest.data.min_votes.and_then(|val| Some(val as i32)),
                    &contest.data.max_votes.and_then(|val| Some(val as i32)),
                    &contest.data.voting_type,
                    &contest.data.counting_algorithm,
                    &contest.data.is_encrypted,
                    &contest.data.tally_configuration,
                    &contest.data.conditions,
                    &contest
                        .data
                        .winning_candidates_num
                        .and_then(|val| Some(val as i32)),
                    &contest.data.alias,
                    &contest.data.image_document_id,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}
