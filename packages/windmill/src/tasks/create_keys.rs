// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use celery::error::TaskError;
use celery::export::Arc;
use celery::prelude::*;
use celery::Celery;
use rocket::serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{event, instrument, Level};

use crate::connection;
use crate::hasura;
use crate::hasura::election_event::update_election_event_status;
use crate::services::celery_app::*;
use crate::services::election_event_board::{get_election_event_board, BoardSerializable};
use crate::services::election_event_status;
use crate::services::protocol_manager;
use crate::tasks::set_public_key::set_public_key;
use crate::types::scheduled_event::ScheduledEvent;

#[derive(Deserialize, Debug, Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct CreateKeysBody {
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

#[instrument(skip(auth_headers))]
#[celery::task]
pub async fn create_keys(
    auth_headers: connection::AuthHeaders,
    body: CreateKeysBody,
    event: ScheduledEvent,
) -> TaskResult<()> {
    let celery_app = get_celery_app().await;
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

    // check config is not already created
    let status: Option<election_event_status::ElectionEventStatus> =
        match election_event.status.clone() {
            Some(value) => serde_json::from_value(value)
                .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?,
            None => None,
        };
    if election_event_status::is_config_created(&status) {
        return Err(TaskError::UnexpectedError(
            "bulletin board config already created".into(),
        ));
    }

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    // create config/keys for board
    protocol_manager::create_keys(
        board_name.as_str(),
        body.trustee_pks.clone(),
        body.threshold.clone(),
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    // update election event with status: keys created
    let new_status = serde_json::to_value(election_event_status::ElectionEventStatus {
        config_created: Some(true),
        stopped: Some(false),
    })
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    update_election_event_status(
        auth_headers.clone(),
        tenant_id,
        election_event_id,
        new_status,
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    let task = celery_app
        .send_task(set_public_key::new(auth_headers.clone(), event.clone()))
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    event!(Level::INFO, "Sent SET_PUBLIC_KEY task {}", task.task_id);

    Ok(())
}
