// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use immu_board::util::get_board_name;
use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use serde_json::Value;
use std::env;
use tracing::{event, instrument, Level};

use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::hasura::election_event::{get_election_event, insert_election_event};
use crate::services::election_event_board::BoardSerializable;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::protocol_manager::get_board_client;
use crate::types::error::Result;

#[instrument]
pub async fn upsert_immu_board(tenant_id: &str, election_event_id: &str) -> Result<Value> {
    let index_db = env::var("IMMUDB_INDEX_DB").expect(&format!("IMMUDB_INDEX_DB must be set"));
    let board_name = get_board_name(tenant_id, election_event_id);
    let mut board_client = get_board_client().await?;
    let has_board = board_client.has_database(board_name.as_str()).await?;
    let board = if has_board {
        board_client.get_board(&index_db, &board_name).await?
    } else {
        board_client.create_board(&index_db, &board_name).await?
    };

    let board_serializable: BoardSerializable = board.into();
    let board_value = serde_json::to_value(board_serializable.clone())?;
    Ok(board_value)
}

#[instrument]
pub async fn upsert_keycloak_realm(tenant_id: &str, election_event_id: &str) -> Result<()> {
    let json_realm_config =
        env::var("KEYCLOAK_ELECTION_EVENT_REALM_CONFIG").expect(&format!("KEYCLOAK_ELECTION_EVENT_REALM_CONFIG must be set"));
    let client = KeycloakAdminClient::new().await?;
    let board_name = get_board_name(tenant_id, election_event_id);
    client.upsert_realm(board_name.as_str(), &json_realm_config).await?;
    upsert_realm_jwks(board_name.as_str()).await?;
    Ok(())
}

#[instrument(skip(auth_headers))]
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

    let _hasura_response = insert_election_event(auth_headers.clone(), object.clone()).await?;

    Ok(())
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_election_event_t(object: InsertElectionEventInput, id: String) -> Result<()> {
    let mut final_object = object.clone();
    final_object.id = Some(id.clone());
    let tenant_id = object.tenant_id.clone().unwrap();

    let board = upsert_immu_board(tenant_id.as_str(), &id.as_ref()).await?;
    final_object.bulletin_board_reference = Some(board);
    final_object.id = Some(id.clone());
    upsert_keycloak_realm(tenant_id.as_str(), &id.as_ref()).await?;
    let auth_headers = get_client_credentials().await?;
    insert_election_event_db(&auth_headers, &final_object).await?;

    Ok(())
}
