// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::str::FromStr;

use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::types::scheduled_event::*;
use tracing::instrument;

#[instrument(skip(hasura_transaction), err)]
pub async fn manage_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    scheduled_date: Option<&str>,
    event_processor: &str,
    voting_channels: Option<Vec<VotingStatusChannel>>,
    scheduled_event_id: Option<&str>,
) -> Result<()> {
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
        None,
        cron_config,
        &event_processor_val,
        voting_channels,
        scheduled_event_id,
    )
    .await
}
