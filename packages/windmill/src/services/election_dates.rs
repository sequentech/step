// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::scheduled_event::*;
use crate::services::election_event_dates::generate_manage_date_task_name;
use crate::tasks::manage_election_event_date::ManageElectionDatePayload;
use crate::types::scheduled_event::EventProcessors;
use crate::{postgres::election::*, types::scheduled_event::CronConfig};
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::ElectionPresentation;
use tracing::{info, instrument};

#[instrument(skip(hasura_transaction), err)]
pub async fn manage_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<()> {
    let found_election = get_election_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await?;
    let Some(election) = found_election else {
        return Err(anyhow!("Election not found"));
    };
    let election_presentation: ElectionPresentation = election
        .presentation
        .clone()
        .map(|presentation| serde_json::from_value(presentation))
        .transpose()
        .map_err(|err| anyhow!("Error parsing election presentation {:?}", err))?
        .unwrap_or(Default::default());
    let current_dates = election_presentation
        .dates
        .clone()
        .unwrap_or(Default::default());
    let mut new_dates = current_dates.clone();
    let start_task_id =
        generate_manage_date_task_name(tenant_id, election_event_id, Some(election_id), true);
    let end_task_id =
        generate_manage_date_task_name(tenant_id, election_event_id, Some(election_id), false);

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
                let event_processor = EventProcessors::START_ELECTION;

                let payload = ManageElectionDatePayload {
                    election_id: Some(election_id.to_string()),
                };
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
            new_dates.scheduled_closing = Some(true);
            new_dates.end_date = Some(date.to_string());
            //TODO: check if date is smaller than now or bigger than end_date and return error;
            let cron_config = CronConfig {
                cron: None,
                scheduled_date: Some(date.to_string()),
            };
            if let Some(scheduled_manage_end_date) = scheduled_manage_end_date_opt {
                update_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    &scheduled_manage_end_date.id,
                    cron_config,
                )
                .await?;
            } else {
                let event_processor = EventProcessors::END_ELECTION;

                let payload = ManageElectionDatePayload {
                    election_id: Some(election_id.to_string()),
                };
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
            new_dates.scheduled_closing = Some(false);
            new_dates.end_date = None;
            if (current_dates.scheduled_closing.is_none()) {
            } else {
                //STOP PREVIOS END TASK
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

    let mut new_election_presentation: ElectionPresentation = election_presentation.clone();
    new_election_presentation.dates = Some(new_dates);
    update_election_presentation(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
        serde_json::to_value(new_election_presentation)?,
    )
    .await?;
    Ok(())
}
