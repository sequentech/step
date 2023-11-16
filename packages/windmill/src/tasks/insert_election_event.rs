// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use immu_board::util::get_board_name;
use sequent_core;
use sequent_core::services::{connection, keycloak};
use serde_json::Value;
use std::env;
use tracing::{event, Level, instrument};
use uuid::Uuid;

use crate::services::election_event_board::BoardSerializable;
use crate::hasura;
use crate::hasura::election_event::insert_election_event::sequent_backend_election_event_insert_input as InsertElectionEventInput;
use crate::services::protocol_manager::get_board_client;
use crate::types::error::Result;

#[instrument(skip(auth_headers))]
pub async fn create_immu_board(
    auth_headers: &connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Value> {
    let index_db = env::var("IMMUDB_INDEX_DB").expect(&format!("IMMUDB_INDEX_DB must be set"));
    let board_name = get_board_name(tenant_id, election_event_id);
    let mut board_client = get_board_client().await?;
    event!(Level::INFO, "FF Before");
    let board = board_client.create_board(&index_db, &board_name).await?;
    event!(Level::INFO, "FF After");

    let board_serializable: BoardSerializable = board.into();
    let board_value = serde_json::to_value(board_serializable.clone())?;
    event!(Level::INFO, "FF End");
    Ok(board_value)
}

#[instrument(skip(auth_headers))]
pub async fn create_keycloak_realm(
    auth_headers: &connection::AuthHeaders,
    object: &InsertElectionEventInput,
) -> Result<()> {
    todo!()
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
    let auth_headers = keycloak::get_client_credentials().await?;

    let id = object.id.clone().unwrap_or(Uuid::new_v4().to_string());

    let board = create_immu_board(
        &auth_headers,
        &object.tenant_id.as_ref().ok_or("empty-tenant-id")?,
        &id.as_ref(),
    )
    .await?;
    let mut final_object = object.clone();
    final_object.bulletin_board_reference = Some(board);
    final_object.id = Some(id);
    create_keycloak_realm(&auth_headers, &final_object).await?;
    insert_election_event_db(&auth_headers, &final_object).await?;

    Ok(())
}
