// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use immu_board::BoardClient;
use rocket::serde::{Deserialize, Serialize};
use std::env;

use crate::connection;
use crate::hasura;
use crate::services::election_event_board::BoardSerializable;

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct CreateBoardPayload {
    pub board_name: String,
}

async fn get_client() -> Result<BoardClient> {
    let username =
        env::var("IMMUDB_USER").expect(&format!("IMMUDB_USER must be set"));
    let password = env::var("IMMUDB_PASSWORD")
        .expect(&format!("IMMUDB_PASSWORD must be set"));
    let server_url = env::var("IMMUDB_SERVER_URL")
        .expect(&format!("IMMUDB_SERVER_URL must be set"));
    let mut client = BoardClient::new(
        server_url.as_str(),
        username.as_str(),
        password.as_str(),
    )
    .await?;
    client.login(&username, &password).await?;
    Ok(client)
}

pub async fn create_board(
    auth_headers: connection::AuthHeaders,
    tenant_id: &str,
    election_event_id: &str,
    board_db: &str,
) -> Result<BoardSerializable> {
    let index_db = env::var("IMMUDB_INDEX_DB")
        .expect(&format!("IMMUDB_INDEX_DB must be set"));
    let mut board_client = get_client().await?;
    let board = board_client
        .create_board(index_db.as_str(), board_db)
        .await?;
    let board_serializable: BoardSerializable = board.into();
    let board_value = serde_json::to_value(board_serializable.clone())?;

    let hasura_response = hasura::election_event::update_election_event_board(
        auth_headers.clone(),
        tenant_id.to_string(),
        election_event_id.to_string(),
        board_value,
    )
    .await?;
    Ok(board_serializable)
}
