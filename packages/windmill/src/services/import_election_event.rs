// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ::keycloak::types::RealmRepresentation;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local};
use deadpool_postgres::{Client as DbClient, Transaction};
use immu_board::util::get_event_board;
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::hasura::election_event::get_election_event;
use crate::hasura::election_event::insert_election_event as insert_election_event_hasura;
use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::services::election_event_board::BoardSerializable;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::protocol_manager::{create_protocol_manager_keys, get_board_client};

use sequent_core::types::hasura::core::{
    Area as AreaData, Candidate as CandidateData, Contest as ContestData, Election as ElectionData,
    ElectionEvent as ElectionEventData,
};

use super::database::get_hasura_pool;

#[derive(Debug, Deserialize, Clone)]
pub struct Election {
    pub id: Uuid,
    pub election_event_id: Uuid,
    pub data: ElectionData,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Contest {
    pub id: Uuid,
    pub election_id: Uuid,
    pub data: ContestData,
    pub created_at: Option<DateTime<Local>>,
    pub area_id: Uuid,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Candidate {
    pub id: Uuid,
    pub contest_id: Uuid,
    pub data: CandidateData,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AreaContest {
    pub id: Uuid,
    pub area_id: Uuid,
    pub contest_id: Uuid,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImportElectionEventSchema {
    pub tenant_id: Uuid,
    pub keycloak_event_realm: Option<RealmRepresentation>,
    pub election_event_data: ElectionEventData,
    pub elections: Vec<Election>,
    pub contests: Vec<Contest>,
    pub candidates: Vec<Candidate>,
    pub areas: Vec<AreaData>,
    pub area_contest_list: Vec<AreaContest>,
}

#[instrument(err)]
pub async fn upsert_immu_board(tenant_id: &str, election_event_id: &str) -> Result<Value> {
    let index_db = env::var("IMMUDB_INDEX_DB").expect(&format!("IMMUDB_INDEX_DB must be set"));
    let board_name = get_event_board(tenant_id, election_event_id);
    let mut board_client = get_board_client().await?;
    let has_board = board_client.has_database(board_name.as_str()).await?;
    let board = if has_board {
        board_client.get_board(&index_db, &board_name).await?
    } else {
        board_client.create_board(&index_db, &board_name).await?
    };

    if !has_board {
        event!(
            Level::INFO,
            "creating protocol manager keys for Election event {}",
            election_event_id
        );
        create_protocol_manager_keys(&board_name).await?;
    }

    let board_serializable: BoardSerializable = board.into();
    let board_value = serde_json::to_value(board_serializable.clone())?;
    Ok(board_value)
}

pub fn read_default_election_event_realm() -> Result<RealmRepresentation> {
    let realm_config_path = env::var("KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH").expect(&format!(
        "KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH must be set"
    ));
    let realm_config = fs::read_to_string(&realm_config_path)
        .expect(&format!("Should have been able to read the configuration file in KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH={realm_config_path}"));

    serde_json::from_str(&realm_config)
        .map_err(|err| anyhow!("Error parsing KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH into RealmRepresentation: {err}"))
}

#[instrument(err)]
pub async fn upsert_keycloak_realm(
    tenant_id: &str,
    election_event_id: &str,
    keycloak_event_realm: Option<RealmRepresentation>,
) -> Result<()> {
    let realm = if let Some(realm) = keycloak_event_realm {
        realm
    } else {
        let realm = read_default_election_event_realm()?;
        realm
    };
    let realm_config = serde_json::to_string(&realm)?;
    let client = KeycloakAdminClient::new().await?;
    let realm_name = get_event_realm(tenant_id, election_event_id);
    client
        .upsert_realm(realm_name.as_str(), &realm_config, tenant_id)
        .await?;
    upsert_realm_jwks(realm_name.as_str()).await?;
    Ok(())
}

#[instrument(skip(auth_headers), err)]
pub async fn insert_election_event_db(
    auth_headers: &connection::AuthHeaders,
    object: &InsertElectionEventInput,
) -> Result<()> {
    let election_event_id = object.id.clone().unwrap();
    let tenant_id = object.tenant_id.clone().unwrap();
    // fetch election_event
    let found_election_event = get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .expect("expected data".into())
    .sequent_backend_election_event;

    if found_election_event.len() > 0 {
        event!(
            Level::INFO,
            "Election event {} for tenant {} already exists",
            election_event_id,
            tenant_id
        );
        return Ok(());
    }

    let new_election_input = InsertElectionEventInput {
        statistics: Some(json!({
            "num_emails_sent": 0,
            "num_sms_sent": 0
        })),
        ..object.clone()
    };

    let _hasura_response =
        insert_election_event_hasura(auth_headers.clone(), new_election_input).await?;

    Ok(())
}

#[instrument(err)]
pub async fn process(data_init: &ImportElectionEventSchema) -> Result<()> {
    let mut data = data_init.clone();
    let tenant_id = &data.tenant_id.to_string();
    let election_event_id = &data.election_event_data.id;

    let board = upsert_immu_board(tenant_id.as_str(), &election_event_id).await?;
    data.election_event_data.bulletin_board_reference = Some(board);
    upsert_keycloak_realm(
        tenant_id.as_str(),
        &election_event_id,
        data.keycloak_event_realm.clone(),
    )
    .await?;

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

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

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
        candidate.data.validate()?;

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
