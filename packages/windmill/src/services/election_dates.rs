// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::*;
use crate::postgres::scheduled_event::*;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{
    EInitializeReportPolicy, ElectionEventStatus, PeriodDates, StringifiedPeriodDates,
};
use sequent_core::types::hasura::core::Election;
use sequent_core::types::scheduled_event::*;
use std::str::FromStr;
use tracing::instrument;

#[instrument(skip(hasura_transaction), err)]
pub async fn manage_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    scheduled_date: Option<&str>,
    event_processor: &str,
) -> Result<()> {
    let found_election = get_election_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .map_err(|e| anyhow!("election not found: {e:?}"))?;

    let Some(election) = found_election else {
        return Err(anyhow!("Election not found"));
    };

    let event_processor_val: EventProcessors = EventProcessors::from_str(&event_processor)
        .map_err(|err| {
            anyhow!("Error mapping {event_processor:?} into an EventProcessor: {err:?}")
        })?;

    let task_id = generate_manage_date_task_name(
        tenant_id,
        election_event_id,
        Some(election_id),
        &event_processor_val,
    );

    let old_scheduled_event_opt =
        find_scheduled_event_by_task_id(hasura_transaction, tenant_id, election_event_id, &task_id)
            .await
            .map_err(|e| anyhow!("scheduled event by task id not found: {e:?}"))?;

    // if there's an schedule date, we have to either insert or create this
    if let Some(date) = scheduled_date {
        let cron_config = CronConfig {
            cron: None,
            scheduled_date: Some(date.to_string()),
        };

        match old_scheduled_event_opt {
            Some(old_scheduled_event) if old_scheduled_event.archived_at.is_none() => {
                update_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    &old_scheduled_event.id,
                    cron_config,
                )
                .await
                .map_err(|e| anyhow!("error updating scheduled event: {e:?}"))?;
            }
            _ => {
                let payload = ManageElectionDatePayload {
                    election_id: Some(election_id.to_string()),
                };

                insert_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    event_processor_val,
                    &task_id,
                    cron_config,
                    serde_json::to_value(payload)
                        .map_err(|e| anyhow!("error deserializing payload: {e:?}"))?,
                )
                .await
                .map_err(|e| anyhow!("error inserting scheduled event: {e:?}"))?;
            }
        };
    } else {
        // Archive previous task if the date is set to null and we found some
        // task
        if let Some(old_scheduled_event) = old_scheduled_event_opt {
            archive_scheduled_event(hasura_transaction, tenant_id, &old_scheduled_event.id)
                .await
                .map_err(|e| anyhow!("error archiving scheduled event: {e:?}"))?;
        }
    }

    Ok(())
}

#[instrument(err, skip_all)]
pub fn get_election_dates(
    election: &Election,
    scheduled_events: Vec<ScheduledEvent>,
) -> Result<StringifiedPeriodDates> {
    let status = election.status.clone().unwrap_or_default();
    let period_dates: PeriodDates = status.voting_period_dates;
    let mut dates = period_dates.to_string_fields();

    if let Ok(scheduled_event_dates) = prepare_scheduled_dates(scheduled_events, Some(&election.id))
    {
        dates.scheduled_event_dates = Some(scheduled_event_dates);
    }

    Ok(dates)
}
