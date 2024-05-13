// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::scheduled_event::find_all_active_events;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::tasks::manage_election_date::manage_election_date;
use crate::types::error::Result;
use crate::types::scheduled_event::EventProcessors;
use celery::error::TaskError;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use tracing::instrument;
use tracing::{event, Level};

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn scheduled_events() -> Result<()> {
    let celery_app = get_celery_app().await;
    let now = ISO8601::now();
    let one_minute_later = now + Duration::seconds(60);
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

    for scheduled_event in to_be_run_now {
        let Some(event_processor) = scheduled_event.event_processor.clone() else {
            continue;
        };
        if EventProcessors::START_ELECTION == event_processor
            || EventProcessors::END_ELECTION == event_processor
        {
            // create the public keys in async task
            let task = celery_app
                .send_task(manage_election_date::new(
                    scheduled_event.tenant_id.clone(),
                    scheduled_event.election_event_id.clone(),
                    scheduled_event.id.clone(),
                ))
                .await?;
            event!(
                Level::INFO,
                "Sent manage_election_date task {}",
                task.task_id
            );
        }
    }

    Ok(())
}
