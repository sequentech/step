// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura;
use crate::hasura::election_event::update_election_event_status;
use crate::services::celery_app::*;
use crate::services::election_event_board::get_election_event_board;
use crate::services::public_keys;
use crate::types::error::{Error, Result};
use anyhow::Context;
use celery::error::TaskError;
use sequent_core::ballot::ElectionEventStatus;
use sequent_core::ballot::VotingStatus;
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::keycloak;
use serde::{Deserialize, Serialize};
use std::default::Default;
use tracing::instrument;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct CreateKeysBody {
    pub trustee_pks: Vec<String>,
    pub threshold: usize,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn create_keys(
    body: CreateKeysBody,
    tenant_id: String,
    election_event_id: String,
) -> Result<()> {
    let auth_headers = keycloak::get_client_credentials().await?;
    let _celery_app = get_celery_app().await;
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
    let status: Option<ElectionEventStatus> = match election_event.status.clone() {
        Some(value) => deserialize_value(value)?,
        None => None,
    };
    if status.map(|val| val.is_config_created()).unwrap_or(false) {
        return Err(Error::String(
            "bulletin board config already created".into(),
        ));
    }

    let board_name = get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")?;

    // create config/keys for board
    public_keys::create_keys(
        board_name.as_str(),
        body.trustee_pks.clone(),
        body.threshold.clone(),
    )
    .await?;

    // update election event with status: keys created
    let mut new_status: ElectionEventStatus = Default::default();
    new_status.config_created = Some(true);
    let new_status_js = serde_json::to_value(new_status)?;

    update_election_event_status(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        new_status_js,
    )
    .await?;

    Ok(())
}
