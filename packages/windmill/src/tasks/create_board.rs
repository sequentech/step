// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::types::error::Result;
use celery::error::TaskError;
use immu_board::BoardClient;
use sequent_core;
use sequent_core::services::keycloak;
use std::env;
use tracing::instrument;

use crate::hasura;
use crate::services::election_event_board::BoardSerializable;

async fn get_client() -> Result<BoardClient> {
    let username = env::var("IMMUDB_USER").expect(&format!("IMMUDB_USER must be set"));
    let password = env::var("IMMUDB_PASSWORD").expect(&format!("IMMUDB_PASSWORD must be set"));
    let server_url =
        env::var("IMMUDB_SERVER_URL").expect(&format!("IMMUDB_SERVER_URL must be set"));
    let mut client =
        BoardClient::new(server_url.as_str(), username.as_str(), password.as_str()).await?;
    client.login(&username, &password).await?;
    Ok(client)
}

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn create_board(tenant_id: String, election_event_id: String) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let board_db: String = election_event_id.chars().filter(|&c| c != '-').collect();

    let index_db = env::var("IMMUDB_INDEX_DB").expect(&format!("IMMUDB_INDEX_DB must be set"));
    let mut board_client = get_client().await?;
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
