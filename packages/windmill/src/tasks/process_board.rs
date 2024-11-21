// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result as AnyhowResult};
use celery::error::TaskError;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::types::ceremonies::KeysCeremonyExecutionStatus;
use tracing::{event, instrument, Level};

use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::keys_ceremony::get_keys_ceremonies;
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::tasks::create_keys::create_keys;
use crate::tasks::execute_tally_session::execute_tally_session;
use crate::tasks::set_public_key::set_public_key;
use crate::types::error::Result;

#[instrument(err)]
pub async fn process_board_impl(tenant_id: String, election_event_id: String) -> AnyhowResult<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await?;

    let hasura_transaction = hasura_db_client.transaction().await?;

    let election_event =
        get_election_event_by_id(&hasura_transaction, &tenant_id, &election_event_id).await?;
    let celery_app = get_celery_app().await;

    let keys_ceremonies =
        get_keys_ceremonies(&hasura_transaction, &tenant_id, &election_event_id).await?;

    for keys_ceremony in keys_ceremonies {
        let status = keys_ceremony.status()?;
        let execution_status = keys_ceremony.execution_status()?;
        if execution_status == KeysCeremonyExecutionStatus::STARTED {
            // create the public keys in async task
            let task = celery_app
                .send_task(create_keys::new(
                    tenant_id.clone(),
                    election_event_id.clone(),
                    keys_ceremony.id.clone(),
                ))
                .await?;
            event!(Level::INFO, "Sent create_keys task {}", task.task_id);
        } else if execution_status == KeysCeremonyExecutionStatus::IN_PROGRESS
            && status.public_key.is_none()
        {
            let task = celery_app
                .send_task(set_public_key::new(
                    tenant_id.clone(),
                    election_event_id.clone(),
                    keys_ceremony.id.clone(),
                ))
                .await
                .map_err(|e| anyhow::Error::from(e))?;
            event!(
                Level::INFO,
                "Sent set_public_key task {} for keys ceremony {}",
                task.task_id,
                keys_ceremony.id
            );
        }
    }
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
