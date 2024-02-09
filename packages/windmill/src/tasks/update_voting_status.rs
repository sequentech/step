// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use celery::error::TaskError;
use sequent_core::ballot::{ElectionStatus, VotingStatus};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::hasura;
use crate::types::error::{Error, Result};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateVotingStatusPayload {
    pub election_id: String,
    pub status: VotingStatus,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn update_voting_status(
    payload: UpdateVotingStatusPayload,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let election_event_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "can't find election event")?;
    let election_event = &election_event_response.sequent_backend_election_event[0];
    let old_status: ElectionStatus = match election_event.status.clone() {
        Some(status) => deserialize_value(status)?,
        None => ElectionStatus {
            voting_status: payload.status.clone(),
        },
    };
    let new_status = ElectionStatus {
        voting_status: payload.status.clone(),
    };

    if payload.status == VotingStatus::OPEN && election_event.public_key.is_none() {
        return Err(Error::String("Missing public key".into()));
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
