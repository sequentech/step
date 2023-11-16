// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use immu_board::util::get_board_name;
use keycloak::types::RealmRepresentation;
use sequent_core;
use sequent_core::services::connection;
use sequent_core::services::keycloak::{get_client_credentials, get_keycloak_client};
use serde_json::Value;
use std::env;
use tracing::instrument;
use uuid::Uuid;

use crate::hasura;
use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::services::election_event_board::BoardSerializable;
use crate::services::protocol_manager::get_board_client;
use crate::types::error::Result;

#[instrument]
pub async fn create_immu_board(
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Value> {
    let index_db = env::var("IMMUDB_INDEX_DB").expect(&format!("IMMUDB_INDEX_DB must be set"));
    let board_name = get_board_name(tenant_id, election_event_id);
    let mut board_client = get_board_client().await?;
    let board = board_client.create_board(&index_db, &board_name).await?;

    let board_serializable: BoardSerializable = board.into();
    let board_value = serde_json::to_value(board_serializable.clone())?;
    Ok(board_value)
}

#[instrument]
pub async fn create_keycloak_realm(tenant_id: &str, election_event_id: &str) -> Result<()> {
    let admin = get_keycloak_client().await?;
    let board_name = get_board_name(tenant_id, election_event_id);
    admin
        .post(RealmRepresentation {
            realm: Some(board_name.into()),
            ..Default::default()
        })
        .await?;

    Ok(())
}

#[instrument(skip(auth_headers))]
pub async fn insert_election_event_db(
    auth_headers: &connection::AuthHeaders,
    object: &InsertElectionEventInput,
) -> Result<()> {
    let _hasura_response =
        hasura::election_event::insert_election_event(auth_headers.clone(), object.clone()).await?;

    Ok(())
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn insert_election_event_t(object: InsertElectionEventInput) -> Result<()> {
    let id = object.id.clone().unwrap_or(Uuid::new_v4().to_string());
    let tenant_id = object.tenant_id.clone().unwrap();

    let board = create_immu_board(tenant_id.as_str(), &id.as_ref()).await?;
    let mut final_object = object.clone();
    final_object.bulletin_board_reference = Some(board);
    final_object.id = Some(id.clone());
    create_keycloak_realm(tenant_id.as_str(), &id.as_ref()).await?;
    let auth_headers = get_client_credentials().await?;
    insert_election_event_db(&auth_headers, &final_object).await?;

    Ok(())
}
