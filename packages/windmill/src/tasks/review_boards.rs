// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election_event::get_batch_election_events;
use crate::postgres::tenant::get_tenant_by_id;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::tasks::insert_tenant::insert_tenant;
use crate::tasks::process_board::process_board;
use crate::types::error::Result;
use anyhow::Context;
use celery::error::TaskError;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak;
use std::env;
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

    // check default tenant exists
    let default_tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .with_context(|| "Missing variable SUPER_ADMIN_TENANT_ID")?;

    if let Err(_) = get_tenant_by_id(&hasura_transaction, &default_tenant_id).await {
        let default_tenant_slug = env::var("ENV_SLUG")
            .with_context(|| "Missing variable ENV_SLUG")?
            .to_lowercase();
        let task = celery_app
            .send_task(
                insert_tenant::new(default_tenant_id.clone(), default_tenant_slug.clone())
                    .with_expires_in(30),
            )
            .await?;
        event!(
            Level::INFO,
            "Sent task {} to create tenant {} named {}",
            task.task_id,
            default_tenant_id,
            default_tenant_slug
        );
    }

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
