// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use crate::postgres::scheduled_event::*;
use anyhow::{anyhow, bail, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::types::scheduled_event::*;

fn payload(event: &ScheduledEvent) -> Result<ManageElectionDatePayload> {
    Ok(serde_json::from_value(
        event
            .event_payload
            .clone()
            .unwrap_or_else(|| serde_json::json!({})),
    )?)
}

fn voting(processor: &EventProcessors) -> bool {
    matches!(
        processor,
        EventProcessors::START_VOTING_PERIOD | EventProcessors::END_VOTING_PERIOD
    )
}

fn scoped_events<'a>(
    events: &'a [ScheduledEvent],
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    processor: &EventProcessors,
) -> Result<Vec<&'a ScheduledEvent>> {
    let mut scoped = Vec::new();
    for event in events {
        if event.tenant_id.as_deref() == Some(tenant_id)
            && event.election_event_id.as_deref() == Some(election_event_id)
            && event.event_processor.as_ref() == Some(processor)
            && event.archived_at.is_none()
            && payload(event)?.election_id.as_deref() == election_id
        {
            scoped.push(event);
        }
    }
    Ok(scoped)
}

fn select_schedule<'a>(
    events: &[&'a ScheduledEvent],
    scheduled_event_id: Option<&str>,
    processor: &EventProcessors,
    channels: Option<&[VotingStatusChannel]>,
) -> Result<Option<&'a ScheduledEvent>> {
    if let Some(id) = scheduled_event_id {
        return events
            .iter()
            .copied()
            .find(|event| event.id == id)
            .map(Some)
            .ok_or_else(|| anyhow!("Scheduled event not found in this scope or already archived"));
    }
    let requested = ManageElectionDatePayload {
        election_id: None,
        voting_channels: channels.map(<[_]>::to_vec),
    }
    .channels();
    let mut matches = Vec::new();
    for event in events {
        if !voting(processor) || channels.is_none() || payload(event)?.channels() == requested {
            matches.push(*event);
        }
    }
    if matches.len() > 1 {
        bail!("Several schedules match; select the scheduled event to edit or delete");
    }
    Ok(matches.into_iter().next())
}

/// Save within an event lock so concurrent requests cannot overwrite or create
/// overlapping channel schedules. A row ID always takes precedence over task names.
pub async fn manage_dates(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    cron_config: Option<CronConfig>,
    processor: &EventProcessors,
    voting_channels: Option<Vec<VotingStatusChannel>>,
    scheduled_event_id: Option<&str>,
) -> Result<()> {
    lock_scheduling_event(transaction, tenant_id, election_event_id).await?;
    let events =
        find_scheduled_event_by_election_event_id(transaction, tenant_id, election_event_id)
            .await?;
    let scoped = scoped_events(
        &events,
        tenant_id,
        election_event_id,
        election_id,
        processor,
    )?;
    let selected = select_schedule(
        &scoped,
        scheduled_event_id,
        processor,
        voting_channels.as_deref(),
    )?;
    let Some(cron_config) = cron_config else {
        if let Some(selected) = selected {
            archive_scheduled_event(transaction, tenant_id, &selected.id).await?;
        }
        return Ok(());
    };
    // Older clients that only change a date preserve the selected channels.
    let channels = match (voting_channels, selected) {
        (None, Some(selected)) => payload(selected)?.voting_channels,
        (channels, _) => channels,
    };
    let next_payload = ManageElectionDatePayload {
        election_id: election_id.map(str::to_string),
        voting_channels: channels,
    };
    validate_scheduled_voting_channels(processor, next_payload.voting_channels.as_deref())?;
    if voting(processor) {
        let requested = next_payload.channels();
        for other in &scoped {
            if selected.is_some_and(|selected| selected.id == other.id) {
                continue;
            }
            if let Some(channel) = payload(other)?
                .channels()
                .iter()
                .find(|channel| requested.contains(channel))
            {
                bail!("Another schedule already targets {channel} for this action. Edit or delete that schedule first.");
            }
        }
    }
    let task_id = generate_channel_date_task_name(
        tenant_id,
        election_event_id,
        election_id,
        processor,
        next_payload.voting_channels.as_deref(),
    );
    // A legacy kiosk-only row may still occupy the online task name. Move only
    // its name, preserving its ID, time, payload and queued execution.
    if voting(processor) {
        let base =
            generate_manage_date_task_name(tenant_id, election_event_id, election_id, processor);
        for other in &scoped {
            if selected.is_some_and(|selected| selected.id == other.id)
                || other.task_id.as_deref() != Some(base.as_str())
            {
                continue;
            }
            let other_task = generate_channel_date_task_name(
                tenant_id,
                election_event_id,
                election_id,
                processor,
                payload(other)?.voting_channels.as_deref(),
            );
            if other_task != base {
                rename_scheduled_event_task(transaction, tenant_id, &other.id, &other_task).await?;
            }
        }
    }
    if let Some(selected) = selected {
        let updated = update_scheduled_event(
            transaction,
            tenant_id,
            &selected.id,
            cron_config,
            next_payload.voting_channels.as_ref(),
            Some(&task_id),
        )
        .await?;
        if updated != 1 {
            bail!("The scheduled event has already run or was archived; refresh the list before editing it");
        }
    } else {
        insert_scheduled_event(
            transaction,
            tenant_id,
            election_event_id,
            processor.clone(),
            &task_id,
            cron_config,
            serde_json::to_value(next_payload)?,
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "scheduled_event_dates_tests.rs"]
mod tests;
