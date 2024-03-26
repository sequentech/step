// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::tasks::insert_election_event::{upsert_immu_board, upsert_keycloak_realm};
use ::keycloak::types::RealmRepresentation;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

use sequent_core::types::hasura::core::{
    Area as AreaData, Candidate as CandidateData, Contest as ContestData, Election as ElectionData,
    ElectionEvent as ElectionEventData,
};

use super::database::get_hasura_pool;

#[derive(Debug, Deserialize)]
pub struct Election {
    id: Uuid,
    election_event_id: Uuid,
    data: ElectionData,
}

#[derive(Debug, Deserialize)]
pub struct Contest {
    id: Uuid,
    election_id: Uuid,
    data: ContestData,
    area_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct Candidate {
    id: Uuid,
    contest_id: Uuid,
    data: CandidateData,
}

#[derive(Debug, Deserialize)]
pub struct AreaContest {
    id: Uuid,
    area_id: Uuid,
    contest_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ImportElectionEventSchema {
    tenant_id: Uuid,
    keycloak_event_realm: RealmRepresentation,
    election_event_data: ElectionEventData,
    elections: Vec<Election>,
    contests: Vec<Contest>,
    candidates: Vec<Candidate>,
    areas: Vec<AreaData>,
    area_contest_list: Vec<AreaContest>,
}

pub async fn process(data: &ImportElectionEventSchema) -> Result<()> {
    let tenant_id = &data.tenant_id.to_string();
    let election_event_id = &data.election_event_data.id;

    // upsert_immu_board(tenant_id.as_str(), &election_event_id).await?;
    // upsert_keycloak_realm(tenant_id.as_str(), &election_event_id).await?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    insert_election_event(&hasura_transaction, &data).await?;
    insert_election(&hasura_transaction, &data).await?;
    insert_contest(&hasura_transaction, &data).await?;
    insert_candidate(&hasura_transaction, &data).await?;
    insert_area(&hasura_transaction, &data).await?;
    insert_area_contest(&hasura_transaction, &data).await?;

    let commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    println!("c'est gagneeeee");

    Ok(())
}

async fn insert_election_event(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    data.election_event_data.validate()?;

    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.election_event
                (id, created_at, updated_at, labels, annotations, tenant_id, name, description, presentation, bulletin_board_reference, is_archived, voting_channels, dates, status, user_boards, encryption_protocol, is_audit, audit_election_event_id, public_key, alias, statistics)
                VALUES
                ($1, NOW(), NOW(), $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19);
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(&data.election_event_data.id)?,
                &data.election_event_data.labels,
                &data.election_event_data.annotations,
                &Uuid::parse_str(&data.election_event_data.tenant_id)?,
                &data.election_event_data.name,
                &data.election_event_data.description,
                &data.election_event_data.presentation,
                &data.election_event_data.bulletin_board_reference,
                &data.election_event_data.is_archived,
                &data.election_event_data.voting_channels,
                &data.election_event_data.dates,
                &data.election_event_data.status,
                &data.election_event_data.user_boards,
                &data.election_event_data.encryption_protocol,
                &data.election_event_data.is_audit,
                &data
                    .election_event_data
                    .audit_election_event_id
                    .as_ref()
                    .and_then(|s| Uuid::parse_str(&s).ok()),
                &data.election_event_data.public_key,
                &data.election_event_data.alias,
                &data.election_event_data.statistics,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the document query: {err}"))?;

    Ok(())
}

async fn insert_election(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    for election in &data.elections {
        election.data.validate()?;

        let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.election
                (id, tenant_id, election_event_id, created_at, last_updated_at, labels, annotations, name, description, presentation, dates, status, eml, num_allowed_revotes, is_consolidated_ballot_encoding, spoil_ballot_option, alias, voting_channels, is_kiosk, image_document_id, statistics, receipts)
                VALUES
                ($1, $2, $3, NOW(), NOW(), $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20);    
            "#,
        )
        .await?;

        let rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &election.id,
                    &Uuid::parse_str(&election.data.tenant_id)?,
                    &Uuid::parse_str(&election.data.election_event_id)?,
                    &election.data.labels,
                    &election.data.annotations,
                    &election.data.name,
                    &election.data.description,
                    &election.data.presentation,
                    &election.data.dates,
                    &election.data.status,
                    &election.data.eml,
                    &election
                        .data
                        .num_allowed_revotes
                        .and_then(|val| Some(val as i32)),
                    &election.data.is_consolidated_ballot_encoding,
                    &election.data.spoil_ballot_option,
                    &election.data.alias,
                    &election.data.voting_channels,
                    &election.data.is_kiosk,
                    &election.data.image_document_id,
                    &election.data.statistics,
                    &election.data.receipts,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

async fn insert_contest(
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

async fn insert_candidate(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    for candidate in &data.candidates {
        let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.candidate
                (id, tenant_id, election_event_id, contest_id, created_at, last_updated_at, labels, annotations, name, description, type, presentation, is_public, alias, image_document_id)
                VALUES
                ($1, $2, $3, $4, NOW(), NOW(), $5, $6, $7, $8, $9, $10, $11, $12, $13);
            "#,
        )
        .await?;

        let rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &candidate.id,
                    &Uuid::parse_str(&candidate.data.tenant_id)?,
                    &Uuid::parse_str(&candidate.data.election_event_id)?,
                    &candidate
                        .data
                        .contest_id
                        .as_ref()
                        .and_then(|id| Uuid::parse_str(&id).ok()),
                    &candidate.data.labels,
                    &candidate.data.annotations,
                    &candidate.data.name,
                    &candidate.data.description,
                    &candidate.data.r#type,
                    &candidate.data.presentation,
                    &candidate.data.is_public,
                    &candidate.data.alias,
                    &candidate.data.image_document_id,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

async fn insert_area(
    hasura_transaction: &Transaction<'_>,
    data: &ImportElectionEventSchema,
) -> Result<()> {
    for area in &data.areas {
        let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO sequent_backend.area
                (id, tenant_id, election_event_id, created_at, last_updated_at, labels, annotations, name, description, type)
                VALUES
                ($1, $2, $3, NOW(), NOW(), $4, $5, $6, $7, $8);
            "#,
        )
        .await?;

        let rows: Vec<Row> = hasura_transaction
            .query(
                &statement,
                &[
                    &Uuid::parse_str(&area.id)?,
                    &Uuid::parse_str(&area.tenant_id)?,
                    &Uuid::parse_str(&area.election_event_id)?,
                    &area.labels,
                    &area.annotations,
                    &area.name,
                    &area.description,
                    &area.r#type,
                ],
            )
            .await
            .map_err(|err| anyhow!("Error running the document query: {err}"))?;
    }

    Ok(())
}

async fn insert_area_contest(
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

/*

-- Delete area_contest
DELETE FROM sequent_backend.area_contest WHERE election_event_id = '1f8b9a17-d3e4-4fc4-aeeb-182f8b9a17d3';

-- Delete candidates
DELETE FROM sequent_backend.candidate WHERE election_event_id = '1f8b9a17-d3e4-4fc4-aeeb-182f8b9a17d3';

-- Delete contests
DELETE FROM sequent_backend.contest WHERE election_event_id = '1f8b9a17-d3e4-4fc4-aeeb-182f8b9a17d3';

-- Delete elections
DELETE FROM sequent_backend.election WHERE election_event_id = '1f8b9a17-d3e4-4fc4-aeeb-182f8b9a17d3';

-- Delete areas
DELETE FROM sequent_backend.area WHERE election_event_id = '1f8b9a17-d3e4-4fc4-aeeb-182f8b9a17d3';

-- Delete the election event itself
DELETE FROM sequent_backend.election_event WHERE id = '1f8b9a17-d3e4-4fc4-aeeb-182f8b9a17d3';

*/
