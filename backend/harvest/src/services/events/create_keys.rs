// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use rocket::serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::connection;
use crate::hasura;
use crate::hasura::election_event::update_election_event_status;
use crate::routes::scheduled_event::ScheduledEvent;
use crate::services::election_event_board::{
    get_election_event_board, BoardSerializable,
};
use crate::services::election_event_status;
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
    let tenant_id = event
        .tenant_id
        .clone()
        .with_context(|| "scheduled event is missing tenant_id")?;
    let election_event_id = event
        .election_event_id
        .clone()
        .with_context(|| "scheduled event is missing election_event_id")?;
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];

    let status: Option<election_event_status::ElectionEventStatus> =
        match election_event.status.clone() {
            Some(value) => serde_json::from_value(value)?,
            None => None,
        };
    if election_event_status::is_config_created(status) {
        bail!("bulletin board config already created");
    }

    let board_name = get_election_event_board(
        election_event.bulletin_board_reference.clone(),
    )
    .with_context(|| "missing bulletin board")?;

    protocol_manager::create_keys(
        board_name.as_str(),
        body.trustee_pks.clone(),
        body.threshold.clone(),
    )
    .await?;

    let new_status =
        serde_json::to_value(election_event_status::ElectionEventStatus {
            config_created: Some(true),
        })?;

    update_election_event_status(
        auth_headers,
        tenant_id,
        election_event_id,
        new_status,
    )
    .await
}
