// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::error::TaskError;
use sequent_core::services::keycloak;
use strand::backend::ristretto::RistrettoCtx;
use tracing::{event, instrument, Level};

use crate::hasura;
use crate::services::celery_app::get_celery_app;
use crate::services::election_event_board::get_election_event_board;
use crate::services::protocol_manager;
use crate::tasks::tally_ballots::process_ballots_from_messages;
use crate::types::error::Result;

#[instrument]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn process_board(election_event_id: String, tenant_id: String) -> Result<()> {
    // get credentials
    let auth_headers = keycloak::get_client_credentials().await?;

    // fetch election_event
    let hasura_response = hasura::election_event::get_election_event(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;
    let election_event = &hasura_response
        .data
        .expect("expected data")
        .sequent_backend_election_event[0];

    // fetch tally_session_execution
    let hasura_response = hasura::tally_session_execution::get_tally_session_execution(
        auth_headers.clone(),
        tenant_id.clone(),
        election_event_id.clone(),
    )
    .await?;

    // TODO:Read from tally session
    // Then from tally session execution
    let res = &hasura_response.data.expect("expected data");
    dbg!(&res);

    let bulletin_board_opt =
        get_election_event_board(election_event.bulletin_board_reference.clone());

    if let Some(bulletin_board) = bulletin_board_opt {
        let pm = protocol_manager::gen_protocol_manager::<RistrettoCtx>();

        let celery_app = get_celery_app().await;
        let task = celery_app
            .send_task(process_ballots_from_messages::new(bulletin_board.clone()))
            .await?;
        event!(Level::INFO, "Sent task {}", task.task_id);
    }

    Ok(())
}
