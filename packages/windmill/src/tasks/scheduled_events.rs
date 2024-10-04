use crate::hasura::scheduled_event;
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::scheduled_event::find_all_active_events;
use crate::postgres::scheduled_event::PostgresScheduledEvent;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::date::ISO8601;
use crate::tasks::manage_election_dates::manage_election_date;
use crate::tasks::manage_election_event_date::manage_election_event_date;
use crate::tasks::manage_election_event_date::ManageElectionDatePayload;
use crate::types::error::Result;
use crate::types::scheduled_event::EventProcessors;
use anyhow::anyhow;
use celery::error::TaskError;
use chrono::prelude::*;
use chrono::Duration;
use deadpool_postgres::Client as DbClient;
use sequent_core::serialization::deserialize_with_path::deserialize_value;
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
    ISO8601::to_date(&scheduled_date).ok()
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn scheduled_events() -> Result<()> {
    let celery_app = get_celery_app().await;
    let now = ISO8601::now();
    let one_minute_later = now + Duration::seconds(60);
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
    let hasura_transaction = hasura_db_client.transaction().await?;

    let scheduled_events = find_all_active_events(&hasura_transaction).await?;
    info!("Found {} scheduled events", scheduled_events.len());
    let to_be_run_now = scheduled_events
        .iter()
        .filter(|event| {
            let Some(formatted_date) = get_datetime(&event) else {
                return false;
            };
            formatted_date < one_minute_later
        })
        .collect::<Vec<_>>();
    info!("Found {} events to be run now", to_be_run_now.len());
    for scheduled_event in to_be_run_now {
        let Some(event_processor) = scheduled_event.event_processor.clone() else {
            continue;
        };
        if EventProcessors::START_VOTING_PERIOD == event_processor
            || EventProcessors::END_VOTING_PERIOD == event_processor
        {
            let Some(datetime) = get_datetime(scheduled_event) else {
                continue;
            };
            let Some(tenant_id) = scheduled_event.tenant_id.clone() else {
                continue;
            };
            let Some(election_event_id) = scheduled_event.election_event_id.clone() else {
                continue;
            };
            let Some(event_payload) = scheduled_event.event_payload.clone() else {
                event!(Level::WARN, "Missing election_event_id");
                return Ok(());
            };
            let payload: ManageElectionDatePayload = deserialize_value(event_payload)?;
            // create the public keys in async task
            match payload.election_id.clone() {
                Some(election_id) => {
                    let task = celery_app
                        .send_task(
                            manage_election_date::new(
                                tenant_id.clone(),
                                election_event_id.clone(),
                                scheduled_event.id.clone(),
                                election_id,
                            )
                            .with_eta(datetime.with_timezone(&Utc))
                            .with_expires_in(120),
                        )
                        .await?;
                    event!(
                        Level::INFO,
                        "Sent manage_election_date task {}",
                        task.task_id
                    );
                }
                None => {
                    let task = celery_app
                        .send_task(
                            manage_election_event_date::new(
                                tenant_id.clone(),
                                election_event_id.clone(),
                                scheduled_event.id.clone(),
                            )
                            .with_eta(datetime.with_timezone(&Utc))
                            .with_expires_in(120),
                        )
                        .await?;
                    event!(
                        Level::INFO,
                        "Sent manage_election_event_date task {}",
                        task.task_id
                    );
                }
            }
        }
    }

    Ok(())
}
