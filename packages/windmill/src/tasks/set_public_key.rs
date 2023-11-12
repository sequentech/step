// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Context;
use celery::error::TaskError;
use sequent_core::services::keycloak;
use tracing::{instrument};

use crate::hasura;
use crate::services::election_event_board::get_election_event_board;
use crate::services::public_keys;
use crate::types::error::Result;

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
    let board_name = get_election_event_board(bulletin_board_reference)
        .with_context(|| "election event is missing bulletin board")?;

    if election_event.public_key.is_some() {
        return Ok(());
    }

    let public_key = public_keys::get_public_key(board_name).await?;
    hasura::election_event::update_election_event_public_key(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
        public_key,
    )
    .await?;
    Ok(())
}
