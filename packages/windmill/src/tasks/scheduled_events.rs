// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::scheduled_event::find_all_active_events;
use crate::postgres::scheduled_event::PostgresScheduledEvent;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::tasks::manage_election_date::manage_election_date;
use crate::types::error::Result;
use crate::types::scheduled_event::EventProcessors;
use anyhow::anyhow;
use celery::error::TaskError;
use chrono::prelude::*;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use tracing::instrument;
use tracing::{event, info, Level};

#[instrument]
pub fn get_datetime(event: &PostgresScheduledEvent) -> Option<DateTime<Local>> {
    let Some(cron_config) = event.cron_config.clone() else {
        return None;
    };
    let Some(scheduled_date) = cron_config.scheduled_date else {
        return None;
    };
    info!("scheduled_date={scheduled_date}");
    ISO8601::to_date(&scheduled_date).ok()
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task]
pub async fn scheduled_events() -> Result<()> {
    let celery_app = get_celery_app().await;
    let now = ISO8601::now();
    info!("now={now}");
    let one_minute_later = now + Duration::seconds(60);
    info!("one_minute_later={one_minute_later}");
    info!("Running between {now} and {one_minute_later}");
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
    let hasura_transaction = hasura_db_client.transaction().await?;

    let scheduled_events = find_all_active_events(&hasura_transaction).await?;
    let to_be_run_now = scheduled_events
        .iter()
        .filter(|event| {
            let Some(formatted_date) = get_datetime(&event) else {
                info!("Error parsing date");
                return false;
            };
            formatted_date < one_minute_later
        })
        .collect::<Vec<_>>();
        info!("to_be_run_now length={:?}", to_be_run_now.len());
    for scheduled_event in to_be_run_now {
        let Some(event_processor) = scheduled_event.event_processor.clone() else {
            continue;
        };
        info!("event_processor={event_processor}");
        if EventProcessors::START_ELECTION == event_processor
            || EventProcessors::END_ELECTION == event_processor
        {
            let Some(datetime) = get_datetime(scheduled_event) else {
                info!("Error parsing date omri");
                continue;
            };
            info!("datetime={datetime}");
            // create the public keys in async task
            let task = celery_app
                .send_task(
                    manage_election_date::new(
                        scheduled_event.tenant_id.clone(),
                        scheduled_event.election_event_id.clone(),
                        scheduled_event.id.clone(),
                    )
                    .with_eta(datetime.with_timezone(&Utc)),
                )
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
