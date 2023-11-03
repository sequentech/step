// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use celery::task::TaskResult;
use sequent_core::services::openid;
use strand::backend::ristretto::RistrettoCtx;
use tracing::instrument;

use crate::hasura;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::types::task_error::into_task_error;
use crate::types::error::{Error, Result};

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn process_board(election_event_id: String, tenant_id: String) -> Result<()> {
    // get credentials
    let auth_headers = openid::get_client_credentials()
        .await?;
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

    let bulletin_board_opt =
        get_election_event_board(election_event.bulletin_board_reference.clone());
    if bulletin_board_opt.is_none() {
        return Ok(());
    }
    let bulletin_board = bulletin_board_opt.unwrap();

    let pm = protocol_manager::gen_protocol_manager::<RistrettoCtx>();

    Ok(())
}
