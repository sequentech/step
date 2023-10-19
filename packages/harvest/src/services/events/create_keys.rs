// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{bail, Context, Result};
use rocket::serde::{Deserialize, Serialize};
use serde_json::Value;
use windmill::connection;
extern crate celery as external_celery; 
use external_celery::export::Arc;
use external_celery::Celery;
use windmill::tasks::set_public_key::set_public_key_task;
use windmill::types::scheduled_event::ScheduledEvent;
use tracing::{event, instrument, Level};

use crate::services::celery;
use crate::hasura;
use crate::hasura::election_event::update_election_event_status;
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

#[instrument(skip(auth_headers, celery_app))]
pub async fn create_keys(
    auth_headers: connection::AuthHeaders,
    body: CreateKeysBody,
    event: ScheduledEvent,
    celery_app: Arc<Celery>,
) -> Result<()> {
    // read tenant_id and election_event_id
    let tenant_id = event
        .tenant_id
        .clone()
        .with_context(|| "scheduled event is missing tenant_id")?;
    let election_event_id = event
        .election_event_id
        .clone()
        .with_context(|| "scheduled event is missing election_event_id")?;
    // fetch election_event
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

    // check config is not already created
    let status: Option<election_event_status::ElectionEventStatus> =
        match election_event.status.clone() {
            Some(value) => serde_json::from_value(value)?,
            None => None,
        };
    if election_event_status::is_config_created(&status) {
        bail!("bulletin board config already created");
    }

    let board_name = get_election_event_board(
        election_event.bulletin_board_reference.clone(),
    )
    .with_context(|| "missing bulletin board")?;

    // create config/keys for board
    protocol_manager::create_keys(
        board_name.as_str(),
        body.trustee_pks.clone(),
        body.threshold.clone(),
    )
    .await?;

    // update election event with status: keys created
    let new_status =
        serde_json::to_value(election_event_status::ElectionEventStatus {
            config_created: Some(true),
            stopped: Some(false),
        })?;

    update_election_event_status(
        auth_headers.clone(),
        tenant_id,
        election_event_id,
        new_status,
    )
    .await?;

    let task = celery_app
    .send_task(set_public_key_task::new(
        auth_headers.clone(),
        event.clone(),
    ))
    .await?;
    event!(Level::INFO, "Sent SET_PUBLIC_KEY task {}", task.task_id);

    Ok(())
}
