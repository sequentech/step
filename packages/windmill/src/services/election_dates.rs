// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::*;
use crate::postgres::scheduled_event::*;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionPresentation, VotingPeriodDates};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::types::scheduled_event::*;
use tracing::{info, instrument};

#[instrument(skip(hasura_transaction), err)]
pub async fn manage_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    scheduled_date: Option<&str>,
    is_start: bool,
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

    if is_start {
        match scheduled_date {
            Some(date) => {
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
                    let event_processor = EventProcessors::START_VOTING_PERIOD;

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
                        deserialize_value(serde_json::to_value(payload)?)?,
                    )
                    .await?;
                }
            }
            None => {
                //STOP PREVIOUS START TASK
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
    } else {
        match scheduled_date {
            Some(date) => {
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
                    let event_processor = EventProcessors::END_VOTING_PERIOD;

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
                //STOP PREVIOUS END TASK
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
    Ok(())
}
