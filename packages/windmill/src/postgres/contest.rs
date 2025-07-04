// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::import::import_election_event::ImportElectionEventSchema;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::hasura::core::Contest;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub struct ContestWrapper(pub Contest);

impl TryFrom<Row> for ContestWrapper {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        let min_votes: Option<i32> = item.try_get("min_votes")?;
        let max_votes: Option<i32> = item.try_get("max_votes")?;
        let winning_candidates_num: Option<i32> = item.try_get("winning_candidates_num")?;

        Ok(ContestWrapper(Contest {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            labels: item.try_get("labels")?,
            annotations: item.try_get("annotations")?,
            is_acclaimed: item.try_get("is_acclaimed")?,
            is_active: item.try_get("is_active")?,
            name: item.try_get("name")?,
            alias: item.try_get("alias")?,
            description: item.try_get("description")?,
            presentation: item.try_get("presentation")?,
            min_votes: min_votes.map(|val| val as i64),
            max_votes: max_votes.map(|val| val as i64),
            winning_candidates_num: winning_candidates_num.map(|val| val as i64),
            voting_type: item.try_get("voting_type")?,
            counting_algorithm: item.try_get("counting_algorithm")?,
            is_encrypted: item.try_get("is_encrypted")?,
            tally_configuration: item.try_get("tally_configuration")?,
            image_document_id: item.try_get("image_document_id")?,
            conditions: item.try_get("conditions")?,
        }))
    }
}

#[instrument(err, skip_all)]
pub async fn insert_contest(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    for contest in &data.contests {
        contest.validate()?;

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
                    &Uuid::parse_str(&contest.id)?,
                    &Uuid::parse_str(&contest.tenant_id)?,
                    &Uuid::parse_str(&contest.election_event_id)?,
                    &Uuid::parse_str(&contest.election_id)?,
                    &contest.labels,
                    &contest.annotations,
                    &contest.is_acclaimed,
                    &contest.is_active,
                    &contest.name,
                    &contest.description,
                    &contest.presentation,
                    &contest.min_votes.and_then(|val| Some(val as i32)),
                    &contest.max_votes.and_then(|val| Some(val as i32)),
                    &contest.voting_type,
                    &contest.counting_algorithm,
                    &contest.is_encrypted,
                    &contest.tally_configuration,
                    &contest.conditions,
                    &contest
                        .winning_candidates_num
                        .and_then(|val| Some(val as i32)),
                    &contest.alias,
                    &contest.image_document_id,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub async fn export_contests(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<Contest>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id, tenant_id, election_event_id, election_id, created_at, last_updated_at, labels, annotations, is_acclaimed, is_active, name, description, presentation, min_votes, max_votes, voting_type, counting_algorithm, is_encrypted, tally_configuration, conditions, winning_candidates_num, alias, image_document_id
                FROM
                    sequent_backend.contest
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

    let election_events: Vec<Contest> = rows
        .into_iter()
        .map(|row| -> Result<Contest> {
            row.try_into()
                .map(|res: ContestWrapper| -> Contest { res.0 })
        })
        .collect::<Result<Vec<Contest>>>()?;

    Ok(election_events)
}

#[instrument(err, skip_all)]
pub async fn get_contest_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    contest_id: &str,
) -> Result<Contest> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT *
                FROM
                    sequent_backend.contest
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2;
                    contest_id = $3;
            "#,
        )
        .await?;

    let row: Option<Row> = hasura_transaction
        .query_opt(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &Uuid::parse_str(contest_id)?,
            ],
        )
        .await?;

    if let Some(row) = row {
        let contest: Contest = row
            .try_into()
            .map(|res: ContestWrapper| -> Contest { res.0 })?;
        Ok(contest as Contest)
    } else {
        Err(anyhow::anyhow!("No contest found with the provided id"))
    }
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_contest_by_election_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Vec<Contest>> {
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.contest
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
        .await?;

    let contests: Vec<Contest> = rows
        .into_iter()
        .map(|row| -> Result<Contest> {
            row.try_into()
                .map(|res: ContestWrapper| -> Contest { res.0 })
        })
        .collect::<Result<Vec<Contest>>>()?;

    Ok(contests)
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_contest_by_election_ids(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &Vec<String>,
) -> Result<Vec<Contest>> {
    let uuid_tenant_id = Uuid::parse_str(tenant_id)?;
    let uuid_election_event_id = Uuid::parse_str(election_event_id)?;

    let uuid_election_ids: Vec<Uuid> = election_ids
        .iter()
        .map(|id| Uuid::parse_str(id))
        .collect::<Result<_, _>>()?;

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                *
            FROM
                sequent_backend.contest
            WHERE
                tenant_id = $1 AND
                election_event_id = $2 AND
                election_id = ANY($3);
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[&uuid_tenant_id, &uuid_election_event_id, &uuid_election_ids],
        )
        .await?;

    let contests: Vec<Contest> = rows
        .into_iter()
        .map(|row| -> Result<Contest> { row.try_into().map(|res: ContestWrapper| res.0) })
        .collect::<Result<Vec<Contest>>>()?;

    Ok(contests)
}
