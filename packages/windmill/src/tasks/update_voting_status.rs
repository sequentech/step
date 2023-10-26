// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use celery::error::TaskError;
use celery::prelude::*;
use serde::{Deserialize, Serialize};
use sequent_core::ballot::{ElectionStatus, VotingStatus};
use tracing::instrument;

use crate::connection;
use crate::hasura;
use crate::hasura::event_execution::insert_event_execution_with_result;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::types::scheduled_event::ScheduledEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateVotingStatusPayload {
    pub election_id: String,
    pub status: VotingStatus,
}

#[instrument(skip(auth_headers))]
#[celery::task]
pub async fn update_voting_status(
    auth_headers: connection::AuthHeaders,
    event: ScheduledEvent,
    payload: UpdateVotingStatusPayload,
) -> TaskResult<()> {
    let tenant_id: String = event.tenant_id.clone().unwrap();
    let election_event_id: String = event.election_event_id.clone().unwrap();
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

    insert_event_execution_with_result(auth_headers, event, None)
        .await
        .map_err(|err| TaskError::ExpectedError(format!("{:?}", err)))?;

    Ok(())
}
