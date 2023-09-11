// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use rocket::serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::connection;
use crate::hasura;
use crate::routes::scheduled_event::ScheduledEvent;
use crate::services::election_event_board::BoardSerializable;
use crate::services::protocol_manager;

#[derive(Deserialize, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateKeysBody {
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

pub async fn create_keys(
    auth_headers: connection::AuthHeaders,
    body: CreateKeysBody,
    event: ScheduledEvent,
) -> Result<()> {
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        event
            .tenant_id
            .clone()
            .with_context(|| "scheduled event is missing tenant_id")?,
        event
            .election_event_id
            .clone()
            .with_context(|| "scheduled event is missing election_event_id")?,
    )
    .await?;
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];
    let status = election_event.status.clone();
    let bulletin_board_reference = election_event
        .bulletin_board_reference
        .clone()
        .with_context(|| "missing bulletin board")?;

    let board_value: Value = bulletin_board_reference.into();

    let board_serializable: BoardSerializable =
        serde_json::from_value(board_value)?;

    protocol_manager::create_keys(
        board_serializable.database_name.as_str(),
        body.trustee_pks.clone(),
        body.threshold.clone(),
    )
    .await
}
