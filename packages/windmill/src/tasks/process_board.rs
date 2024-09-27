// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use sequent_core::services::keycloak;

use tracing::{event, instrument, Level};

use crate::hasura;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::election_event_status::has_config_created;
use crate::tasks::execute_tally_session::execute_tally_session;
use crate::tasks::set_public_key::set_public_key;
use crate::types::error::Result;

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_board(tenant_id: String, election_event_id: String) -> Result<()> {
    // get credentials
    let auth_headers = keycloak::get_client_credentials().await?;

    // fetch election_event
    let election_events = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .expect("expected data")
    .sequent_backend_election_event;

    if 0 == election_events.len() {
        event!(
            Level::INFO,
            "Election Event not found {}",
            election_event_id.clone()
        );
        return Ok(());
    }

    let election_event = &election_events[0];

    let bulletin_board_opt =
        get_election_event_board(election_event.bulletin_board_reference.clone());

    let celery_app = get_celery_app().await;
    // if there's no bulletin board, create it
    if bulletin_board_opt.is_none() {
        event!(
            Level::INFO,
            "election event {} with no board, skipping",
            election_event_id
        );
        return Ok(());
    }

    // if there's bulletin board and the config is created but there's no
    // public key, try to create it (by reading it from the bulletin board)
    if has_config_created(election_event.status.clone()) && election_event.public_key.is_none() {
        let task = celery_app
            .send_task(set_public_key::new(
                tenant_id.clone(),
                election_event_id.clone(),
            ))
            .await
            .map_err(|e| anyhow::Error::from(e))?;
        event!(Level::INFO, "Sent set_public_key task {}", task.task_id);
        return Ok(());
    }

    // Run tally
    // fetch tally_sessions
    let tally_sessions = hasura::tally_session::get_tally_sessions(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?
    .data
    .expect("expected data")
    .sequent_backend_tally_session;

    for tally_session in tally_sessions {
        let task = celery_app
            .send_task(execute_tally_session::new(
                tenant_id.clone(),
                election_event_id.clone(),
                tally_session.id.clone(),
            ))
            .await?;
        event!(Level::INFO, "Sent task {}", task.task_id);
    }

    Ok(())
}
