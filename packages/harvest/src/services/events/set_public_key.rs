// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use rocket::serde::{Deserialize, Serialize};
use strum_macros::Display;
use strum_macros::EnumString;
use tracing::instrument;

use crate::connection;
use crate::hasura;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::routes::scheduled_event::ScheduledEvent;


#[instrument(skip(auth_headers))]
pub async fn set_public_key(
    auth_headers: connection::AuthHeaders,
    event: ScheduledEvent,
) -> Result<()> {
    let tenant_id = event.tenant_id.clone().with_context(|| "missing tenant id")?;
    let election_event_id = event.election_event_id.clone().with_context(|| "missing election event id")?;
    let election_event_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find election event")?;

    let election_event = &election_event_response.sequent_backend_election_event[0];

    if election_event.public_key.is_some() {
        return Ok(())
    }

    let bulletin_board_reference = election_event
        .bulletin_board_reference
        .clone();
    let board_name = get_election_event_board(bulletin_board_reference)
        .with_context(|| "election event is missing bulletin board")?;
    let public_key = protocol_manager::get_public_key(board_name).await?;
    hasura::election_event::update_election_event_public_key(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        public_key
    ).await?;
    Ok(())
}