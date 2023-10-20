// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{bail, Context, Result};
use celery::error::TaskError;
use celery::prelude::*;
use rocket::serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;

use crate::connection;
use crate::hasura;
use crate::hasura::cast_ballot;
use crate::hasura::election_event::update_election_event_status;
use crate::hasura::event_execution::insert_event_execution_with_result;
use crate::services::election_event_board::{get_election_event_board, BoardSerializable};
use crate::services::election_event_status;
use crate::services::protocol_manager;
use crate::types::scheduled_event::ScheduledEvent;

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct InsertBallotsPayload {
    pub trustee_pks: Vec<String>,
}

#[instrument(skip(auth_headers))]
#[celery::task]
pub async fn insert_ballots(
    auth_headers: connection::AuthHeaders,
    event: ScheduledEvent,
    body: InsertBallotsPayload,
) -> TaskResult<()> {
    // read tenant_id and election_event_id
    let tenant_id = event
        .tenant_id
        .clone()
        .with_context(|| "scheduled event is missing tenant_id")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let election_event_id = event
        .election_event_id
        .clone()
        .with_context(|| "scheduled event is missing election_event_id")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    // fetch election_event
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let election_event = &hasura_response
        .data
        .expect("expected data".into())
        .sequent_backend_election_event[0];

    // check config is already created
    let status: Option<election_event_status::ElectionEventStatus> =
        match election_event.status.clone() {
            Some(value) => serde_json::from_value(value)
                .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?,
            None => None,
        };
    if !election_event_status::is_config_created(&status) {
        return Err(TaskError::UnexpectedError(
            "bulletin board config missing".into(),
        ));
    }
    if !election_event_status::is_stopped(&status) {
        return Err(TaskError::UnexpectedError(
            "election event is not stopped".into(),
        ));
    }

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    let cast_ballots_response = hasura::cast_ballot::find_ballots(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    insert_event_execution_with_result(auth_headers, event, None)
        .await
        .map_err(|err| TaskError::ExpectedError(format!("{:?}", err)))?;

    Ok(())
}
