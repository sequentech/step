// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use rocket::serde::{Deserialize, Serialize};

use crate::connection;
use crate::hasura;
use crate::services::protocol_manager;

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
        body.tenant_id.clone(),
        body.election_event_id.clone(),
    )
    .await?;
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];
    let status = election_event.status.clone();
    let bulletin_board_reference =
        election_event.bulletin_board_reference.clone();

    protocol_manager::create_keys(body).await
}
