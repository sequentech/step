// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use celery::error::TaskError;
use celery::prelude::*;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::services::openid;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::hasura;
use crate::services::election_event_board::get_election_event_board;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct InsertBallotsPayload {
    pub trustee_pks: Vec<String>,
}

#[instrument]
#[celery::task]
pub async fn insert_ballots(
    body: InsertBallotsPayload,
    tenant_id: String,
    election_event_id: String,
) -> TaskResult<()> {
    let auth_headers = openid::get_client_credentials()
        .await
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
    let status: Option<ElectionEventStatus> = match election_event.status.clone() {
        Some(value) => serde_json::from_value(value)
            .map_err(|err| TaskError::UnexpectedError(format!("{:?}", err)))?,
        None => None,
    };
    if !status
        .clone()
        .map(|val| val.is_config_created())
        .unwrap_or(false)
    {
        return Err(TaskError::UnexpectedError(
            "bulletin board config missing".into(),
        ));
    }
    if !status.map(|val| val.is_stopped()).unwrap_or(false) {
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

    Ok(())
}
