// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use deadpool_postgres::Transaction;
use immu_board::util::get_event_board;
use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::{get_client_credentials, KeycloakAdminClient};
use serde_json::{json, Value};
use std::env;
use std::fs;
use tokio_postgres::row::Row;
use tracing::{event, instrument, Level};

use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::hasura::election_event::{get_election_event, insert_election_event};
use crate::services::election_event_board::BoardSerializable;
use crate::services::jwks::upsert_realm_jwks;
use crate::services::protocol_manager::{create_protocol_manager_keys, get_board_client};
use crate::types::error::Result;

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
        create_protocol_manager_keys(&board_name).await?;
    }

    let board_serializable: BoardSerializable = board.into();
    let board_value = serde_json::to_value(board_serializable.clone())?;
    Ok(board_value)
}

#[instrument(err)]
pub async fn upsert_keycloak_realm(tenant_id: &str, election_event_id: &str) -> Result<()> {
    let realm_config_path = env::var("KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH").expect(&format!(
        "KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH must be set"
    ));
    let realm_config = fs::read_to_string(&realm_config_path)
        .expect(&format!("Should have been able to read the configuration file in KEYCLOAK_ELECTION_EVENT_REALM_CONFIG_PATH={realm_config_path}"));
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

    let _hasura_response = insert_election_event(auth_headers.clone(), new_election_input).await?;

    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 4)]
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
