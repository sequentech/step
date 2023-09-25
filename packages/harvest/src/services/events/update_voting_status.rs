// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result, bail};
use rocket::serde::{Deserialize, Serialize};
use strum_macros::Display;
use strum_macros::EnumString;
use tracing::instrument;

use crate::connection;
use crate::hasura;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;

#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
#[serde(crate = "rocket::serde")]
pub enum VotingStatus {
    NOT_STARTED,
    OPEN,
    PAUSED,
    CLOSED,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct UpdateVotingStatusPayload {
    pub election_id: String,
    pub status: VotingStatus,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct ElectionStatus {
    pub voting_status: VotingStatus,
}

#[instrument(skip(auth_headers))]
pub async fn update_voting_status(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    payload: UpdateVotingStatusPayload,
) -> Result<()> {
    let new_status = ElectionStatus {
        voting_status: payload.status.clone(),
    };
    let election_event_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find election event")?;

    let election_event = &election_event_response.sequent_backend_election_event[0];
    if payload.status == VotingStatus::OPEN && election_event.public_key.is_none() {
        bail!("Missing public key");
    }
    let new_status_value = serde_json::to_value(new_status)?;
    let hasura_response = hasura::election::update_election_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        payload.election_id.clone(),
        new_status_value,
    )
    .await?;

    let _election_response_id = &hasura_response
        .data
        .with_context(|| "can't find election")?
        .update_sequent_backend_election
        .unwrap()
        .returning[0];

    Ok(())
}
