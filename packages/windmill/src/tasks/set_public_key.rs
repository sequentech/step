// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use celery::error::TaskError;
use celery::prelude::*;
use sequent_core::services::openid;
use tracing::instrument;

use crate::hasura;
use crate::hasura::event_execution::insert_event_execution_with_result;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::types::scheduled_event::ScheduledEvent;

#[instrument]
#[celery::task(max_retries = 10)]
pub async fn set_public_key(event: ScheduledEvent) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let tenant_id = event
        .tenant_id
        .clone()
        .with_context(|| "missing tenant id")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let election_event_id = event
        .election_event_id
        .clone()
        .with_context(|| "missing election event id")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
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

    let bulletin_board_reference = election_event.bulletin_board_reference.clone();
    let board_name = get_election_event_board(bulletin_board_reference)
        .with_context(|| "election event is missing bulletin board")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    if election_event.public_key.is_some() {
        return Ok(());
    }

    let public_key = protocol_manager::get_public_key(board_name)
        .await
        .map_err(|err| TaskError::ExpectedError(format!("{:?}", err)))?;
    hasura::election_event::update_election_event_public_key(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        public_key,
    )
    .await
    .map_err(|err| TaskError::ExpectedError(format!("{:?}", err)))?;

    insert_event_execution_with_result(auth_headers, event, None)
        .await
        .map_err(|err| TaskError::ExpectedError(format!("{:?}", err)))?;
    Ok(())
}
