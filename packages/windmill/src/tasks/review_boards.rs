// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::get_batch_election_events;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::tasks::process_board::process_board;
use crate::types::error::Result;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak;
use tracing::instrument;
use tracing::{event, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(expires = 30)]
pub async fn review_boards() -> Result<()> {
    let limit: i64 = 100;
    let mut offset: i64 = 0;
    let mut last_length = limit;
    let celery_app = get_celery_app().await;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|err| format!("Error getting hasura db pool: {err}"))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|err| format!("Error starting hasura transaction: {err}"))?;

    while last_length == limit {
        let election_events = get_batch_election_events(&hasura_transaction, limit, offset).await?;

        last_length = election_events.len() as i64;
        offset += last_length;

        for election_event in election_events {
            let task2 = celery_app
                .send_task(
                    process_board::new(election_event.tenant_id.clone(), election_event.id.clone())
                        .with_expires_in(30),
                )
                .await?;
            event!(Level::INFO, "Sent task {}", task2.task_id);
        }
    }

    Ok(())
}
