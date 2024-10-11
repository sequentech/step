use crate::hasura::scheduled_event;
// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::{get_election_by_id, update_election_presentation};
use crate::postgres::keycloak_realm;
use crate::postgres::scheduled_event::find_all_active_events;
use crate::services::celery_app::get_celery_app;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::date::ISO8601;
use crate::services::tasks_execution::update_fail;
use crate::tasks::manage_election_dates::manage_election_date;
use crate::tasks::manage_election_event_date::manage_election_event_date;
use crate::tasks::manage_election_init_report::manage_election_init_report;
use crate::tasks::manage_election_voting_period_end::manage_election_voting_period_end;
use crate::types::error::{Error, Result};
use anyhow::anyhow;
use celery::{error::TaskError, Celery};
use chrono::prelude::*;
use chrono::Duration;
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::ballot::{ElectionPresentation, InitReport, VotingPeriodEnd};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
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

pub async fn handle_allow_init_report(
    celery_app: Arc<Celery>,
    transaction: &Transaction<'_>,
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
    let payload: ManageElectionDatePayload = deserialize_value(event_payload)?;
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
    Ok(())
}

pub async fn handle_allow_voting_period_end(
    celery_app: Arc<Celery>,
    transaction: &Transaction<'_>,
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
    let payload: ManageElectionDatePayload = deserialize_value(event_payload)?;
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
                .await?;
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
                .await?;
            event!(
                Level::INFO,
                "Sent manage_election_voting_period_end task {}",
                task.task_id
            );
        }
    }
    Ok(())
}

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
    let payload: ManageElectionDatePayload = deserialize_value(event_payload)?;
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
    Ok(())
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
        match event_processor {
            EventProcessors::ALLOW_INIT_REPORT => {
                handle_allow_init_report(celery_app.clone(), &hasura_transaction, scheduled_event);
            }
            EventProcessors::ALLOW_VOTING_PERIOD_END => {
                handle_allow_voting_period_end(
                    celery_app.clone(),
                    &hasura_transaction,
                    scheduled_event,
                );
            }
            EventProcessors::START_VOTING_PERIOD | EventProcessors::END_VOTING_PERIOD => {
                if let Err(err) = handle_voting_event(celery_app.clone(), &scheduled_event).await {
                    event!(
                        Level::ERROR,
                        "Event {} failed with error {}",
                        scheduled_event.id,
                        err,
                    );
                } else {
                    event!(
                        Level::INFO,
                        "Event {} executed successfully",
                        scheduled_event.id,
                    );
                }
            }
            EventProcessors::START_ENROLLMENT_PERIOD | EventProcessors::END_ENROLLMENT_PERIOD => {
                let realm_name = match scheduled_event.election_event_id {
                    Some(ref event_id) => get_event_realm(
                        scheduled_event
                            .tenant_id
                            .as_ref()
                            .ok_or("scheduled event missing tenant_id")?
                            .as_str(),
                        scheduled_event
                            .election_event_id
                            .as_ref()
                            .ok_or("scheduled event missing election_event_id")?
                            .as_str(),
                    ),
                    None => get_tenant_realm(
                        &scheduled_event
                            .tenant_id
                            .as_ref()
                            .ok_or("scheduled event missing tenant_id")?
                            .as_str(),
                    ),
                };

                let mut keycloak_db_client: DbClient = match get_keycloak_pool().await.get().await {
                    Ok(client) => client,
                    Err(err) => {
                        return Err(Error::String(format!(
                            "Error getting Keycloak DB pool: {err}"
                        )));
                    }
                };

                let keycloak_transaction = match keycloak_db_client.transaction().await {
                    Ok(transaction) => transaction,
                    Err(err) => {
                        return Err(Error::String(format!(
                            "Error starting Keycloak transaction: {err}"
                        )));
                    }
                };

                let realm_id = match keycloak_realm::get_realm_id(
                    &keycloak_transaction,
                    realm_name.to_string(),
                )
                .await
                {
                    Ok(id) => id,
                    Err(err) => {
                        return Err(Error::String(format!("Error obtaining realm id: {err}")));
                    }
                };
                let keycloak_client = KeycloakAdminClient::new().await?;
                let other_client = KeycloakAdminClient::pub_new().await?;
                let mut realm = keycloak_client
                    .get_realm(&other_client, &realm_name)
                    .await?;
                realm.registration_allowed =
                    Some(event_processor == EventProcessors::START_ENROLLMENT_PERIOD);
                let keycloak_client = KeycloakAdminClient::new().await?;
                keycloak_client
                    .upsert_realm(
                        &realm_name,
                        &serde_json::to_string(&realm)?,
                        scheduled_event
                            .tenant_id
                            .as_ref()
                            .ok_or("scheduled event missing tenant_id")?
                            .as_str(),
                        false,
                        None,
                    )
                    .await?
            }
            EventProcessors::START_LOCKDOWN_PERIOD => {}
            EventProcessors::END_LOCKDOWN_PERIOD => {}
            EventProcessors::CREATE_REPORT | EventProcessors::SEND_TEMPLATE => {
                // Nothing to do for these event processors.  Avoid a
                // catch all to ignore unknown events, this way when
                // new variants are added to `EventProcessors`, a
                // compile time error will happen notifying about the
                // missing logic for handling that new variant.
            }
        }
    }

    Ok(())
}
