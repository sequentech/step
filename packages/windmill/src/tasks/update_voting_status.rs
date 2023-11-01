// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use celery::error::TaskError;
use celery::prelude::*;
use sequent_core::ballot::{ElectionStatus, VotingStatus};
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::hasura;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateVotingStatusPayload {
    pub election_id: String,
    pub status: VotingStatus,
}

#[instrument]
#[celery::task]
pub async fn update_voting_status(
    payload: UpdateVotingStatusPayload,
    tenant_id: String,
    election_event_id: String,
) -> TaskResult<()> {
    let auth_headers = keycloak::get_client_credentials()
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let new_status = ElectionStatus {
        voting_status: payload.status.clone(),
    };
    let election_event_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?
    .data
    .with_context(|| "can't find election event")
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    let election_event = &election_event_response.sequent_backend_election_event[0];
    if payload.status == VotingStatus::OPEN && election_event.public_key.is_none() {
        return Err(TaskError::UnexpectedError("Missing public key".into()));
    }
    let new_status_value = serde_json::to_value(new_status)
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let hasura_response = hasura::election::update_election_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        payload.election_id.clone(),
        new_status_value,
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    let _election_response_id = &hasura_response
        .data
        .with_context(|| "can't find election")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?
        .update_sequent_backend_election
        .unwrap()
        .returning[0];

    Ok(())
}
