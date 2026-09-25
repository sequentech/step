// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use tracing::{event, instrument, Level};

use crate::adapters::keys_ceremony::{CeleryKeysCeremonyTasks, PgKeysCeremonyStore};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::tally_session::get_tally_session_by_election_event_id_pending_post_tally_task;
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::services::celery_app::get_celery_app;
use crate::services::ceremonies::keys_ceremony::dispatch_keys_ceremony_tasks;
use crate::services::database::get_hasura_pool;
use crate::tasks::execute_tally_session::execute_tally_session;
use crate::tasks::post_tally::post_tally_task;
use crate::types::error::Result;

#[instrument(err)]
pub async fn process_board_impl(tenant_id: String, election_event_id: String) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;

    let hasura_transaction = hasura_db_client.transaction().await?;

    let election_event =
        get_election_event_by_id(&hasura_transaction, &tenant_id, &election_event_id).await?;
    let celery_app = get_celery_app().await;

    dispatch_keys_ceremony_tasks(
        &PgKeysCeremonyStore {
            transaction: &hasura_transaction,
        },
        &CeleryKeysCeremonyTasks {
            celery_app: &celery_app,
        },
        &tenant_id,
        &election_event_id,
    )
    .await?;

    // Run tally
    // fetch tally_sessions
    let tally_sessions = get_tally_sessions_by_election_event_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        true,
    )
    .await?;

    for tally_session in tally_sessions {
        let task = celery_app
            .send_task(execute_tally_session::new(
                tenant_id.clone(),
                election_event_id.clone(),
                tally_session.id.clone(),
                tally_session.tally_type.clone(),
                tally_session.election_ids.clone(),
                false, // force_new_results_id: regular run, not a recount/tie-break re-run
            ))
            .await?;
        event!(Level::INFO, "Sent task {}", task.task_id);
    }

    // Run post tally
    // fetch tally_sessions
    let tally_sessions = get_tally_session_by_election_event_id_pending_post_tally_task(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
    )
    .await?;
    for tally_session in tally_sessions {
        let task = celery_app
            .send_task(post_tally_task::new(
                tenant_id.clone(),
                election_event_id.clone(),
                tally_session.id.clone(),
            ))
            .await?;
        event!(Level::INFO, "Sent task {}", task.task_id);
    }

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn process_board(tenant_id: String, election_event_id: String) -> Result<()> {
    process_board_impl(tenant_id, election_event_id).await?;

    Ok(())
}
