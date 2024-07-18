// SPDX-FileCopyrightText: 2024 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ::keycloak::types::RealmRepresentation;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use immu_board::util::get_event_board;
use sequent_core::ballot::ElectionEventStatistics;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::ElectionStatistics;
use sequent_core::ballot::ElectionStatus;
use sequent_core::services::connection;
use sequent_core::services::keycloak;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use sequent_core::services::replace_uuids::replace_uuids;
use sequent_core::types::hasura::core::AreaContest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use super::database::get_hasura_pool;
use super::documents;
use crate::hasura::election_event::get_election_event;
use crate::hasura::election_event::insert_election_event as insert_election_event_hasura;
use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::postgres;
use crate::postgres::area::insert_areas;
use crate::postgres::area_contest::insert_area_contests;
use crate::postgres::candidate::insert_candidates;
use crate::postgres::contest::insert_contest;
use crate::postgres::election::insert_election;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::election_event::insert_election_event;
use crate::services::election_event_board::BoardSerializable;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::protocol_manager::{create_protocol_manager_keys, get_board_client};
use crate::tasks::import_election_event::ImportElectionEventBody;
use sequent_core::types::hasura::core::{Area, Candidate, Contest, Election, ElectionEvent};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportElectionEventSchema {
    pub tenant_id: Uuid,
    pub keycloak_event_realm: Option<RealmRepresentation>,
    pub election_event: ElectionEvent,
    pub elections: Vec<Election>,
    pub contests: Vec<Contest>,
    pub candidates: Vec<Candidate>,
    pub areas: Vec<Area>,
    pub area_contests: Vec<AreaContest>,
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

#[instrument(err, skip(keycloak_event_realm))]
pub async fn upsert_keycloak_realm(
    tenant_id: &str,
    election_event_id: &str,
    keycloak_event_realm: Option<RealmRepresentation>,
) -> Result<()> {
    let realm = if let Some(realm) = keycloak_event_realm.clone() {
        realm
    } else {
        let realm = read_default_election_event_realm()?;
        realm
    };
    let realm_config = serde_json::to_string(&realm)?;
    let client = KeycloakAdminClient::new().await?;
    let realm_name = get_event_realm(tenant_id, election_event_id);
    client
        .upsert_realm(
            realm_name.as_str(),
            &realm_config,
            tenant_id,
            keycloak_event_realm.is_none(),
        )
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

#[instrument(err, skip(data_str, original_data))]
pub fn replace_ids(
    data_str: &str,
    original_data: &ImportElectionEventSchema,
    id_opt: Option<String>,
    tenant_id: String,
) -> Result<ImportElectionEventSchema> {
    let keep: Vec<String> = if id_opt.is_some() {
        vec![
            original_data.election_event.id.clone(),
            original_data.tenant_id.clone().to_string(),
        ]
    } else {
        vec![original_data.tenant_id.clone().to_string()]
    };
    let mut new_data = replace_uuids(data_str, keep);

    if let Some(id) = id_opt {
        new_data = new_data.replace(&original_data.election_event.id, &id);
    }
    if original_data.tenant_id.to_string() != tenant_id {
        new_data = new_data.replace(&original_data.tenant_id.to_string(), &tenant_id);
    }

    let data: ImportElectionEventSchema = serde_json::from_str(&new_data)?;
    Ok(data.clone())
}

#[instrument(err)]
pub async fn get_document(
    hasura_transaction: &Transaction<'_>,
    object: ImportElectionEventBody,
    id: Option<String>,
    tenant_id: String,
) -> Result<ImportElectionEventSchema> {
    let document = postgres::document::get_document(
        hasura_transaction,
        &object.tenant_id,
        None,
        &object.document_id,
    )
    .await?
    .ok_or(anyhow!(
        "Error trying to get document id {}: not found",
        &object.document_id
    ))?;

    let temp_file_path = documents::get_document_as_temp_file(&object.tenant_id, &document)
        .await
        .map_err(|err| anyhow!("Error trying to get document as temporary file {err}"))?;

    let mut file = File::open(temp_file_path)?;

    let mut data_str = String::new();
    file.read_to_string(&mut data_str)?;

    let original_data: ImportElectionEventSchema = serde_json::from_str(&data_str)?;

    let data = replace_ids(&data_str, &original_data, id, tenant_id)?;

    Ok(data)
}

#[instrument(err, skip_all)]
pub async fn process(
    object: ImportElectionEventBody,
    election_event_id: String,
    tenant_id: String,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| anyhow!("Error starting hasura transaction: {err}"))?;

    let mut data: ImportElectionEventSchema = get_document(
        &hasura_transaction,
        object,
        Some(election_event_id),
        tenant_id.clone(),
    )
    .await?;
    let tenant_id = &data.tenant_id.to_string();
    let election_event_id = &data.election_event.id;

    let board = upsert_immu_board(tenant_id.as_str(), &election_event_id).await?;
    data.election_event.bulletin_board_reference = Some(board);
    data.election_event.public_key = None;
    data.election_event.statistics =
        Some(serde_json::to_value(ElectionEventStatistics::default())?);
    data.election_event.status = Some(serde_json::to_value(ElectionEventStatus::default())?);
    data.elections = data
        .elections
        .into_iter()
        .map(|election| -> Result<Election> {
            let mut clone = election.clone();
            clone.statistics = Some(serde_json::to_value(ElectionStatistics::default())?);
            clone.status = Some(serde_json::to_value(ElectionStatus::default())?);
            Ok(clone)
        })
        .collect::<Result<Vec<Election>>>()?;

    upsert_keycloak_realm(
        tenant_id.as_str(),
        &election_event_id,
        data.keycloak_event_realm.clone(),
    )
    .await?;

    insert_election_event(&hasura_transaction, &data).await?;
    insert_election(&hasura_transaction, &data).await?;
    insert_contest(&hasura_transaction, &data).await?;
    insert_candidates(
        &hasura_transaction,
        &data.tenant_id.to_string(),
        &data.election_event.id,
        &data.candidates,
    )
    .await?;
    insert_areas(&hasura_transaction, &data.areas).await?;
    insert_area_contests(
        &hasura_transaction,
        &data.tenant_id.to_string(),
        &data.election_event.id,
        &data.area_contests,
    )
    .await?;

    let _commit = hasura_transaction
        .commit()
        .await
        .map_err(|e| anyhow!("Commit failed: {}", e));

    Ok(())
}
