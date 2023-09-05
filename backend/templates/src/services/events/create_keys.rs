// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use immu_board::{Board, BoardClient, BoardMessage};
use rocket::serde::{Deserialize, Serialize};
use std::env;

use crate::connection;
use crate::services::protocol_manager::ProtocolManagerClient;

#[derive(Deserialize, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateKeysBody {
    pub board_name: String,
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
    pub tenant_id: String,
    pub election_event_id: String,
}

pub async fn create_keys(
    auth_headers: connection::AuthHeaders,
    body: CreateKeysBody,
) -> Result<()> {
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        input.tenant_id.clone(),
        input.election_event_id.clone(),
    )
    .await?;
    let status = hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0]
        .status
        .clone();

    let mut client = ProtocolManagerClient::new()?;
    client.create_keys(body).await
}
