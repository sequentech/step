// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::hasura;
use crate::postgres::scheduled_event::find_all_active_events;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::tasks::process_board::process_board;
use crate::types::error::Result;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use sequent_core::services::keycloak;
use tracing::instrument;
use tracing::{event, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn scheduled_events() -> Result<()> {
    let now = ISO8601::now();
    let one_minute_later = now + Duration::seconds(65);
    let mut hasura_db_client: DbClient = get_hasura_pool().await.get().await.unwrap();
    let hasura_transaction = hasura_db_client.transaction().await?;

    let scheduled_events = find_all_active_events(&hasura_transaction).await?;

    let to_be_run_now = scheduled_events
        .iter()
        .filter(|event| {
            let Some(cron_config) = event.cron_config.clone() else {
                return false;
            };
            let Some(scheduled_date) = cron_config.scheduled_date else {
                return false;
            };
            let Ok(formatted_date) = ISO8601::to_date(&scheduled_date) else {
                return false;
            };
            formatted_date > now && formatted_date < one_minute_later
        })
        .collect::<Vec<_>>();

    Ok(())
}
