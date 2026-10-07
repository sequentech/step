// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::*;
use crate::services::election_event_status::get_election_event_status;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::ballot::{ElectionEventStatus, PeriodDates, StringifiedPeriodDates};
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
    voting_channels: Option<Vec<VotingStatusChannel>>,
    scheduled_event_id: Option<&str>,
) -> Result<()> {
    let found_election = get_election_by_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .map_err(|e| anyhow!("election not found: {e:?}"))?;

    let Some(_election) = found_election else {
        return Err(anyhow!("Election not found"));
    };

    let event_processor_val: EventProcessors = EventProcessors::from_str(&event_processor)
        .map_err(|err| {
            anyhow!("Error mapping {event_processor:?} into an EventProcessor: {err:?}")
        })?;

    let cron_config = scheduled_date.map(|date| CronConfig {
        cron: None,
        scheduled_date: Some(date.to_string()),
    });
    crate::services::scheduled_event_dates::manage_dates(
        hasura_transaction,
        tenant_id,
        election_event_id,
        Some(election_id),
        cron_config,
        &event_processor_val,
        voting_channels,
        scheduled_event_id,
    )
    .await
}

#[instrument(err, skip_all)]
pub fn get_election_dates(
    election: &Election,
    scheduled_events: Vec<ScheduledEvent>,
) -> Result<StringifiedPeriodDates> {
    let status: ElectionEventStatus =
        get_election_event_status(election.status.clone()).unwrap_or_default();
    let period_dates: PeriodDates = status.voting_period_dates;
    let mut dates = period_dates.to_string_fields();

    if let Ok(scheduled_event_dates) = prepare_scheduled_dates(scheduled_events, Some(&election.id))
    {
        dates.scheduled_event_dates = Some(scheduled_event_dates);
    }

    Ok(dates)
}
