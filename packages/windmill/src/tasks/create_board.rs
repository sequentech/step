// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use celery::error::TaskError;

use sequent_core;
use sequent_core::services::keycloak;
use std::env;
use tracing::{event, instrument, Level};

use crate::types::error::Result;
use crate::hasura;
use crate::services::election_event_board::BoardSerializable;
use crate::services::protocol_manager::get_board_client;
use crate::services::election_event_board::get_election_event_board;

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn create_board(tenant_id: String, election_event_id: String) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    // fetch election_event
    let election_event = &hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .expect("expected data".into())
    .sequent_backend_election_event[0];

    let bulletin_board_reference = get_election_event_board(election_event.bulletin_board_reference.clone());

    if bulletin_board_reference.is_some() {
        event!(Level::INFO, "Board already created");
        return Ok(());
    }

    let board_db: String = election_event_id.chars().filter(|&c| c != '-').collect();

    let index_db = env::var("IMMUDB_INDEX_DB").expect(&format!("IMMUDB_INDEX_DB must be set"));
    let mut board_client = get_board_client().await?;
    let board = board_client
        .create_board(index_db.as_str(), board_db.as_str())
        .await?;
    let board_serializable: BoardSerializable = board.into();
    let board_value = serde_json::to_value(board_serializable.clone())?;

    let _hasura_response = hasura::election_event::update_election_event_board(
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
        board_value,
    )
    .await?;

    let _board_json = serde_json::to_value(board_serializable.clone())?;

    Ok(())
}
