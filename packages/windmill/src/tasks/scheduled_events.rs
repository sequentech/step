// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::{
    election::{get_election_by_id, update_election_presentation},
    scheduled_event::find_all_active_events,
};
use crate::services::{
    celery_app::get_celery_app,
    database::{get_hasura_pool, get_keycloak_pool},
    tasks_execution::update_fail,
};
use crate::tasks::{
    manage_election_allow_tally::manage_election_allow_tally,
    manage_election_dates::manage_election_date,
    manage_election_event_date::manage_election_event_date,
    manage_election_event_enrollment::manage_election_event_enrollment,
    manage_election_event_lockdown::manage_election_event_lockdown,
    manage_election_init_report::manage_election_init_report,
    manage_election_voting_period_end::manage_election_voting_period_end,
};
use crate::types::error::{Error, Result};
use anyhow::anyhow;
use celery::{error::TaskError, Celery};
use chrono::prelude::*;
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::ballot::{ElectionPresentation, InitReport, VotingPeriodEnd};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::date::ISO8601;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm, KeycloakAdminClient};
use sequent_core::types::scheduled_event::*;
use std::sync::Arc;
use tracing::instrument;
use tracing::{event, info, Level};

#[instrument]
pub fn get_datetime(event: &ScheduledEvent) -> Option<DateTime<Local>> {
    let Some(cron_config) = event.cron_config.clone() else {
        return None;
    };
    let Some(scheduled_date) = cron_config.scheduled_date else {
        return None;
    };
    ISO8601::to_date(&scheduled_date).ok()
}

#[instrument(skip(celery_app), err)]
pub async fn handle_allow_init_report(
    celery_app: Arc<Celery>,
    scheduled_event: &ScheduledEvent,
) -> Result<()> {
    let Some(datetime) = get_datetime(scheduled_event) else {
        return Ok(());
    };
    let Some(tenant_id) = scheduled_event.tenant_id.clone() else {
        return Ok(());
    };
    let Some(election_event_id) = scheduled_event.election_event_id.clone() else {
        return Ok(());
    };
    let Some(event_payload) = scheduled_event.event_payload.clone() else {
        event!(Level::WARN, "Missing election_event_id");
        return Ok(());
    };
    let payload: ManageAllowInitPayload = deserialize_value(event_payload)
        .map_err(|e| anyhow!("Error deserializing manage election date payload {}", e))?;
    // run the actual task in a different async task
    match payload.election_id.clone() {
        Some(election_id) => {
            let task = celery_app
                .send_task(
                    manage_election_init_report::new(
                        tenant_id.clone(),
                        election_event_id.clone(),
                        scheduled_event.id.clone(),
                        election_id,
                    )
                    .with_eta(datetime.with_timezone(&Utc))
                    .with_expires_in(120),
                )
                .await
                .map_err(|e| anyhow!("Error sending task to celery {}", e))?;
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
                .await
                .map_err(|e| anyhow!("Error sending task to celery {}", e))?;
            event!(
                Level::INFO,
                "Sent manage_election_event_date task {}",
                task.task_id
            );
        }
    }
    Ok(())
}

#[instrument(skip(celery_app), err)]
pub async fn handle_allow_voting_period_end(
    celery_app: Arc<Celery>,
    scheduled_event: &ScheduledEvent,
) -> Result<()> {
    let Some(datetime) = get_datetime(scheduled_event) else {
        return Ok(());
    };
    let Some(tenant_id) = scheduled_event.tenant_id.clone() else {
        return Ok(());
    };
    let Some(election_event_id) = scheduled_event.election_event_id.clone() else {
        return Ok(());
    };
    let Some(event_payload) = scheduled_event.event_payload.clone() else {
        event!(Level::WARN, "Missing election_event_id");
        return Ok(());
    };
    let payload: ManageAllowVotingPeriodEndPayload = deserialize_value(event_payload)
        .map_err(|e| anyhow!("Error deserializing manage election date payload {}", e))?;
    // run the actual task in a different async task
    match payload.election_id.clone() {
        Some(election_id) => {
            let task = celery_app
                .send_task(
                    manage_election_voting_period_end::new(
                        tenant_id.clone(),
                        election_event_id.clone(),
                        scheduled_event.id.clone(),
                        election_id,
                    )
                    .with_eta(datetime.with_timezone(&Utc))
                    .with_expires_in(120),
                )
                .await
                .map_err(|e| anyhow!("Error sending task to celery {}", e))?;
            event!(
                Level::INFO,
                "Sent manage_election_voting_period_end task {}",
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
                .await
                .map_err(|e| anyhow!("Error sending task to celery {}", e))?;
            event!(
                Level::INFO,
                "Sent manage_election_voting_period_end task {}",
                task.task_id
            );
        }
    }
    Ok(())
}

#[instrument(skip(celery_app), err)]
pub async fn handle_voting_event(
    celery_app: Arc<Celery>,
    scheduled_event: &ScheduledEvent,
) -> Result<()> {
    let Some(datetime) = get_datetime(scheduled_event) else {
        return Ok(());
    };
    let Some(tenant_id) = scheduled_event.tenant_id.clone() else {
        return Ok(());
    };
    let Some(election_event_id) = scheduled_event.election_event_id.clone() else {
        return Ok(());
    };
    let Some(event_payload) = scheduled_event.event_payload.clone() else {
        event!(Level::WARN, "Missing election_event_id");
        return Ok(());
    };
    let payload: ManageElectionDatePayload = deserialize_value(event_payload)
        .map_err(|e| anyhow!("Error deserializing manage election date payload {}", e))?;
    // run the actual task in a different async task
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
                .await
                .map_err(|e| anyhow!("Error sending task to celery {}", e))?;
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
                .await
                .map_err(|e| anyhow!("Error sending task to celery {}", e))?;
            event!(
                Level::INFO,
                "Sent manage_election_event_date task {}",
                task.task_id
            );
        }
    }
    Ok(())
}

#[instrument(skip(celery_app), err)]
pub async fn handle_election_event_enrollment(
    celery_app: Arc<Celery>,
    scheduled_event: &ScheduledEvent,
) -> Result<()> {
    let Some(datetime) = get_datetime(scheduled_event) else {
        return Ok(());
    };
    let Some(tenant_id) = scheduled_event.tenant_id.clone() else {
        return Ok(());
    };
    let Some(election_event_id) = scheduled_event.election_event_id.clone() else {
        return Ok(());
    };

    // run the actual task in a different async task
    let task = celery_app
        .send_task(
            manage_election_event_enrollment::new(
                tenant_id.clone(),
                election_event_id.clone(),
                scheduled_event.id.clone(),
            )
            .with_eta(datetime.with_timezone(&Utc))
            .with_expires_in(120),
        )
        .await
        .map_err(|e| anyhow!("Error sending task to celery {}", e))?;

    event!(
        Level::INFO,
        "Sent manage_election_voting_period_end task {}",
        task.task_id
    );

    Ok(())
}

pub async fn handle_election_lockdown(
    celery_app: Arc<Celery>,
    scheduled_event: &ScheduledEvent,
) -> Result<()> {
    let Some(datetime) = get_datetime(scheduled_event) else {
        return Ok(());
    };
    let Some(tenant_id) = scheduled_event.tenant_id.clone() else {
        return Ok(());
    };
    let Some(election_event_id) = scheduled_event.election_event_id.clone() else {
        return Ok(());
    };
    // run the actual task in a different async task
    let task = celery_app
        .send_task(
            manage_election_event_lockdown::new(
                tenant_id.clone(),
                election_event_id.clone(),
                scheduled_event.id.clone(),
            )
            .with_eta(datetime.with_timezone(&Utc))
            .with_expires_in(120),
        )
        .await
        .map_err(|e| anyhow!("Error sending task to celery {}", e))?;
    event!(
        Level::INFO,
        "Sent manage_election_date task {}",
        task.task_id
    );
    Ok(())
}

#[instrument(skip(celery_app), err)]
pub async fn handle_election_allow_tally(
    celery_app: Arc<Celery>,
    scheduled_event: &ScheduledEvent,
) -> Result<()> {
    let Some(datetime) = get_datetime(scheduled_event) else {
        return Ok(());
    };
    let Some(tenant_id) = scheduled_event.tenant_id.clone() else {
        return Ok(());
    };
    let Some(election_event_id) = scheduled_event.election_event_id.clone() else {
        return Ok(());
    };
    let Some(event_payload) = scheduled_event.event_payload.clone() else {
        event!(Level::WARN, "Missing election_event_id");
        return Ok(());
    };
    let payload: ManageAllowTallyPayload = deserialize_value(event_payload)
        .map_err(|e| anyhow!("Error deserializing manage election date payload {}", e))?;
    // run the actual task in a different async task
    match payload.election_id.clone() {
        Some(election_id) => {
            let task = celery_app
                .send_task(
                    manage_election_allow_tally::new(
                        tenant_id.clone(),
                        election_event_id.clone(),
                        scheduled_event.id.clone(),
                        election_id,
                    )
                    .with_eta(datetime.with_timezone(&Utc))
                    .with_expires_in(120),
                )
                .await
                .map_err(|e| anyhow!("Error sending task to celery {}", e))?;
            event!(
                Level::INFO,
                "Sent manage_election_allow_tally task {}",
                task.task_id
            );
        }
        None => {}
    }
    Ok(())
}

pub async fn dispatch(celery_app: Arc<Celery>, scheduled_event: &ScheduledEvent) -> Result<()> {
    let Some(event_processor) = scheduled_event.event_processor.clone() else {
        return Ok(());
    };
    match event_processor {
        EventProcessors::ALLOW_INIT_REPORT => {
            handle_allow_init_report(celery_app, scheduled_event).await
        }
        EventProcessors::ALLOW_VOTING_PERIOD_END => {
            handle_allow_voting_period_end(celery_app, scheduled_event).await
        }
        EventProcessors::START_VOTING_PERIOD | EventProcessors::END_VOTING_PERIOD => {
            handle_voting_event(celery_app, scheduled_event).await
        }
        EventProcessors::START_ENROLLMENT_PERIOD | EventProcessors::END_ENROLLMENT_PERIOD => {
            handle_election_event_enrollment(celery_app, scheduled_event).await
        }
        EventProcessors::START_LOCKDOWN_PERIOD | EventProcessors::END_LOCKDOWN_PERIOD => {
            handle_election_lockdown(celery_app, scheduled_event).await
        }
        EventProcessors::ALLOW_TALLY => {
            handle_election_allow_tally(celery_app, scheduled_event).await
        }
        EventProcessors::CREATE_REPORT | EventProcessors::SEND_TEMPLATE => {
            // Nothing to do for these event processors.  Avoid a
            // catch all to ignore unknown events, this way when
            // new variants are added to `EventProcessors`, a
            // compile time error will happen notifying about the
            // missing logic for handling that new variant.
            Ok(())
        }
    }
}

/// Dispatches every due event and returns how many failed.
pub async fn run_due<'a, I, F, Fut>(events: I, mut dispatch: F) -> usize
where
    I: IntoIterator<Item = &'a ScheduledEvent>,
    F: FnMut(&'a ScheduledEvent) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let mut failed = 0;
    for scheduled_event in events {
        match dispatch(scheduled_event).await {
            Ok(()) => event!(
                Level::INFO,
                "Event {} dispatched successfully",
                scheduled_event.id,
            ),
            Err(err) => {
                failed += 1;
                event!(
                    Level::ERROR,
                    "Event {} ({:?}) failed with error {}",
                    scheduled_event.id,
                    scheduled_event.event_processor,
                    err,
                );
            }
        }
    }
    failed
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(time_limit = 10, max_retries = 0, expires = 30)]
pub async fn scheduled_events(rate_seconds: u64) -> Result<()> {
    let celery_app = get_celery_app().await;
    let now = ISO8601::now();
    let nsecs_later = now + Duration::seconds(rate_seconds as i64);
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| anyhow!("Error getting hasura client {}", e))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| anyhow!("Error creating a hasura transaction {}", e))?;

    let scheduled_events = find_all_active_events(&hasura_transaction)
        .await
        .map_err(|e| anyhow!("Error finding all active events {}", e))?;
    info!("Found {} scheduled events", scheduled_events.len());
    let to_be_run_now = scheduled_events
        .iter()
        .filter(|event| {
            let Some(formatted_date) = get_datetime(&event) else {
                return false;
            };
            formatted_date < nsecs_later
        })
        .collect::<Vec<_>>();
    info!("Found {} events to be run now", to_be_run_now.len());
    let failed = run_due(to_be_run_now, |scheduled_event| {
        dispatch(celery_app.clone(), scheduled_event)
    })
    .await;
    if failed > 0 {
        event!(Level::WARN, "{failed} scheduled events failed this tick");
    }

    Ok(())
}

#[cfg(test)]
#[path = "scheduled_events_tests.rs"]
mod scheduled_events_tests;
