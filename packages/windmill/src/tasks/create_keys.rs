// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use celery::error::TaskError;
use celery::export::Arc;
use celery::prelude::*;
use celery::Celery;
use serde::{Deserialize, Serialize};
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::services::openid;
use serde_json::Value;
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::hasura::election_event::update_election_event_status;
use crate::services::celery_app::*;
use crate::services::election_event_board::{get_election_event_board, BoardSerializable};
use crate::services::protocol_manager;
use crate::tasks::set_public_key::set_public_key;
use crate::types::scheduled_event::ScheduledEvent;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct CreateKeysBody {
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

#[instrument]
#[celery::task]
pub async fn create_keys(
    body: CreateKeysBody,
    tenant_id: String,
    election_event_id: String,
) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    let celery_app = get_celery_app().await;
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
    let status: Option<ElectionEventStatus> = match election_event.status.clone() {
        Some(value) => serde_json::from_value(value)
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?,
        None => None,
    };
    if status.map(|val| val.is_config_created()).unwrap_or(false) {
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
    let new_status = serde_json::to_value(ElectionEventStatus {
        config_created: Some(true),
        stopped: Some(false),
    })
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    update_election_event_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        new_status,
    )
    .await
    .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;

    let task = celery_app
        .send_task(set_public_key::new(tenant_id, election_event_id))
        .await
        .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?;
    event!(Level::INFO, "Sent SET_PUBLIC_KEY task {}", task.task_id);

    Ok(())
}
