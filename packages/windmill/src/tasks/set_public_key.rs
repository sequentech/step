// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context};
use celery::error::TaskError;
use sequent_core::services::keycloak;
use serde_json::Value;
use std::collections::HashSet;
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::hasura::keys_ceremony::get_keys_ceremony;
use crate::hasura::trustee::get_trustees_by_name;
use crate::services::election_event_board::get_election_event_board;
use crate::services::public_keys;
use crate::types::error::Result;
use sequent_core::types::ceremonies::{CeremonyStatus, ExecutionStatus, Trustee, TrusteeStatus};

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 10)]
pub async fn set_public_key(tenant_id: String, election_event_id: String) -> Result<()> {
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

    let bulletin_board_reference = election_event.bulletin_board_reference.clone();
    let board_name = match get_election_event_board(bulletin_board_reference) {
        Some(board_name) => board_name,
        None => {
            event!(Level::INFO, "Public key not found");
            return Ok(());
        }
    };

    if election_event.public_key.is_some() {
        event!(Level::INFO, "Public key already set");
        return Ok(());
    }

    // set public key in the election event
    let public_key = public_keys::get_public_key(board_name).await?;
    hasura::election_event::update_election_event_public_key(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        public_key.clone(),
    )
    .await?;

    // find the keys ceremony, and then update it
    let keys_ceremonies = get_keys_ceremony(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .with_context(|| "error listing existing keys ceremonies")?
    .sequent_backend_keys_ceremony;

    if keys_ceremonies.len() == 0 {
        event!(Level::INFO, "Strange, no ceremonies!");
        return Ok(());
    }

    if keys_ceremonies.len() > 1 {
        event!(
            Level::ERROR,
            "Strange, too many ceremonies! we'll just update the first one"
        );
    }
    let keys_ceremony = &keys_ceremonies[0];
    if keys_ceremony.execution_status != Some(ExecutionStatus::NOT_STARTED.to_string()) {
        event!(
            Level::ERROR,
            "Strange, keys ceremony in wrong execution_status={:?}",
            keys_ceremony.execution_status
        );
        return Err("keys ceremony in wrong execution_status".into());
    }
    let current_status: CeremonyStatus = serde_json::from_value(
        keys_ceremony
            .status
            .clone()
            .ok_or(anyhow!("Empty keys ceremony status"))?,
    )
    .with_context(|| "error parsing keys ceremony current status")?;

    // verify trustee names and fetch their objects to get their ids
    let trustee_names = current_status
        .trustees
        .clone()
        .into_iter()
        .map(|trustee| trustee.name)
        .collect::<HashSet<String>>();
    let trustees_by_name = get_trustees_by_name(
        auth_headers.clone(),
        tenant_id.clone(),
        trustee_names.clone().into_iter().collect::<Vec<_>>(),
    )
    .await?
    .data
    .with_context(|| "can't find trustees")?
    .sequent_backend_trustee
    .into_iter()
    .filter_map(|trustee| trustee.name)
    .collect::<HashSet<String>>();
    // we should have a list with the same trustees
    if trustee_names != trustees_by_name {
        return Err("trustee_names don't correspond to trustees_by_name".into());
    }

    let new_execution_status: String = ExecutionStatus::IN_PROCESS.to_string();
    let new_status: Value = serde_json::to_value(CeremonyStatus {
        stop_date: None,
        public_key: Some(public_key.clone()),
        logs: vec![],
        trustees: current_status
            .trustees
            .clone()
            .into_iter()
            .map(|trustee| Trustee {
                name: trustee.name,
                status: TrusteeStatus::KEY_GENERATED,
            })
            .collect::<Vec<Trustee>>(),
    })?;

    Ok(())
}
