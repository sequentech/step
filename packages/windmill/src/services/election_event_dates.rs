// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::election_event::{get_election_event_by_id, update_election_event_dates};
use crate::postgres::scheduled_event::*;
use crate::tasks::manage_election_event_date::ManageElectionDatePayload;
use crate::types::scheduled_event::CronConfig;
use crate::types::scheduled_event::EventProcessors;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::ElectionEventDates;
use tracing::{info, instrument};

#[instrument]
pub fn generate_manage_date_task_name(
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    is_start: bool,
) -> String {
    let base = format!("tenant_{}_event_{}_", tenant_id, election_event_id,);

    let base_with_election = match election_id {
        Some(id) => format!("{}election_{}_", base, id),
        None => base,
    };

    format!(
        "{}{}",
        base_with_election,
        if is_start { "start" } else { "end" },
    )
}

#[instrument(skip(hasura_transaction), err)]
pub async fn manage_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<()> {
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;

    let current_dates: ElectionEventDates = election_event
        .dates
        .clone()
        .map(|presentation| serde_json::from_value(presentation))
        .transpose()
        .map_err(|err| anyhow!("Error parsing election dates {:?}", err))?
        .unwrap_or(Default::default());

    let mut new_dates = current_dates.clone();
    let start_task_id = generate_manage_date_task_name(tenant_id, election_event_id, None, true);
    let end_task_id = generate_manage_date_task_name(tenant_id, election_event_id, None, false);
    let scheduled_manage_start_date_opt = find_scheduled_event_by_task_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &start_task_id,
    )
    .await?;
    let scheduled_manage_end_date_opt = find_scheduled_event_by_task_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &end_task_id,
    )
    .await?;
    match start_date {
        Some(date) => {
            new_dates.scheduled_opening = Some(true);
            new_dates.start_date = Some(date.to_string());
            //TODO: check if date is smaller than now or bigger than end_date and return error
            let cron_config = CronConfig {
                cron: None,
                scheduled_date: Some(date.to_string()),
            };

            if let Some(scheduled_manage_start_date) = scheduled_manage_start_date_opt {
                update_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    &scheduled_manage_start_date.id,
                    cron_config,
                )
                .await?;
            } else {
                info!("insert_scheduled_event");
                let event_processor = EventProcessors::START_ELECTION;

                let payload = ManageElectionDatePayload { election_id: None };
                insert_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    event_processor,
                    &start_task_id,
                    cron_config,
                    serde_json::to_value(payload)?,
                )
                .await?;
            }
        }
        None => {
            new_dates.scheduled_opening = Some(false);
            new_dates.start_date = None;
            if (current_dates.start_date.is_none()) {
            } else {
                //STOP PREVIOS START TASK
                new_dates.scheduled_opening = Some(false);
                if let Some(scheduled_manage_start_date) = scheduled_manage_start_date_opt {
                    stop_scheduled_event(
                        hasura_transaction,
                        tenant_id,
                        &scheduled_manage_start_date.id,
                    )
                    .await?;
                }
            }
        }
    }

    match end_date {
        Some(date) => {
            info!("end_date is not null${date:?}");
            new_dates.scheduled_closing = Some(true);
            new_dates.end_date = Some(date.to_string());
            //TODO: check if date is smaller than now or bigger than end_date and return error;
            let cron_config = CronConfig {
                cron: None,
                scheduled_date: Some(date.to_string()),
            };
            info!("cron_config={cron_config:?}");
            if let Some(scheduled_manage_end_date) = scheduled_manage_end_date_opt {
                info!("update_scheduled_event");
                update_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    &scheduled_manage_end_date.id,
                    cron_config,
                )
                .await?;
            } else {
                info!("insert_scheduled_event");
                let event_processor = EventProcessors::END_ELECTION;

                let payload = ManageElectionDatePayload { election_id: None };
                insert_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    event_processor,
                    &end_task_id,
                    cron_config,
                    serde_json::to_value(payload)?,
                )
                .await?;
            }
        }
        None => {
            info!("end_date is null");
            new_dates.scheduled_closing = Some(false);
            new_dates.end_date = None;
            info!(
                "current_dates.scheduled_closing={0:?}",
                current_dates.scheduled_closing
            );
            if (current_dates.scheduled_closing.is_none()) {
                info!("cuurent date.schedule_closing is none");
            } else {
                //STOP PREVIOS END TASK
                info!("stopping previouse task");
                if let Some(scheduled_manage_end_date) = scheduled_manage_end_date_opt {
                    stop_scheduled_event(
                        hasura_transaction,
                        tenant_id,
                        &scheduled_manage_end_date.id,
                    )
                    .await?;
                }
            }
        }
    }

    info!("update_election_presentation with new_dates={new_dates:?}");
    let new_election_event_dates = Some(new_dates);
    update_election_event_dates(
        hasura_transaction,
        tenant_id,
        election_event_id,
        serde_json::to_value(new_election_event_dates)?,
    )
    .await?;
    Ok(())
}
