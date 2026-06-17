// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Scheduled automation tasks for election lifecycle transitions.

#![allow(non_camel_case_types)]

use crate::ballot::format_date;
use crate::ballot::ScheduledEventDates;
use crate::ballot::VotingPeriodDates;
use anyhow::{anyhow, Result};
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum_macros::Display;
use strum_macros::EnumString;

/// Worker action triggered by a scheduled event.
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Hash,
)]
pub enum EventProcessors {
    /// Enable generation of the initialization report.
    #[strum(serialize = "ALLOW_INIT_REPORT")]
    ALLOW_INIT_REPORT,
    /// Generate a report at the scheduled time.
    #[strum(serialize = "CREATE_REPORT")]
    CREATE_REPORT,
    /// Send a communication template to voters or admins.
    #[strum(serialize = "SEND_TEMPLATE")]
    SEND_TEMPLATE,
    /// Open the voting period.
    #[strum(serialize = "START_VOTING_PERIOD")]
    START_VOTING_PERIOD,
    /// Close the voting period.
    #[strum(serialize = "END_VOTING_PERIOD")]
    END_VOTING_PERIOD,
    /// Allow the voting period to end (guard before automatic close).
    #[strum(serialize = "ALLOW_VOTING_PERIOD_END")]
    ALLOW_VOTING_PERIOD_END,
    /// Open the voter enrollment period.
    #[strum(serialize = "START_ENROLLMENT_PERIOD")]
    START_ENROLLMENT_PERIOD,
    /// Close the voter enrollment period.
    #[strum(serialize = "END_ENROLLMENT_PERIOD")]
    END_ENROLLMENT_PERIOD,
    /// Begin the pre-election lockdown period.
    #[strum(serialize = "START_LOCKDOWN_PERIOD")]
    START_LOCKDOWN_PERIOD,
    /// End the pre-election lockdown period.
    #[strum(serialize = "END_LOCKDOWN_PERIOD")]
    END_LOCKDOWN_PERIOD,
    /// Enable tally execution for the election.
    #[strum(serialize = "ALLOW_TALLY")]
    ALLOW_TALLY,
}

/// When a scheduled event should run (cron expression or fixed date).
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct CronConfig {
    /// Cron expression for recurring schedules.
    pub cron: Option<String>,
    /// ISO 8601 date/time for one-shot schedules.
    pub scheduled_date: Option<String>,
}

/// Payload identifying which election a date-management task applies to.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageElectionDatePayload {
    /// Target election identifier, or none for event-wide dates.
    pub election_id: Option<String>,
}

/// Payload for enabling or disabling initialization report generation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageAllowInitPayload {
    /// Target election identifier, or none for event-wide scope.
    pub election_id: Option<String>,
    /// When true, initialization reports may be generated.
    #[serde(default = "default_allow_init")]
    pub allow_init: Option<bool>,
}

fn default_allow_init() -> Option<bool> {
    Some(true)
}

/// Payload for enabling automatic voting-period end.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageAllowVotingPeriodEndPayload {
    /// Target election identifier, or none for event-wide scope.
    pub election_id: Option<String>,
    /// When true, the voting period may close at the scheduled end date.
    pub allow_voting_period_end: Option<bool>,
}

/// Payload for enabling tally execution on a scheduled date.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageAllowTallyPayload {
    /// Target election identifier, or none for event-wide scope.
    pub election_id: Option<String>,
}

/// A scheduled task stored in Hasura and executed by the worker.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ScheduledEvent {
    /// Unique scheduled-event identifier.
    pub id: String,
    /// Owning tenant identifier.
    pub tenant_id: Option<String>,
    /// Parent election event identifier.
    pub election_event_id: Option<String>,
    /// Record creation timestamp.
    pub created_at: Option<DateTime<Utc>>,
    /// When the scheduler stopped firing this event.
    pub stopped_at: Option<DateTime<Utc>>,
    /// Soft-archive timestamp.
    pub archived_at: Option<DateTime<Utc>>,
    /// Labels
    pub labels: Option<Value>,
    /// Metadata.
    pub annotations: Option<Value>,
    /// Worker action to execute.
    pub event_processor: Option<EventProcessors>,
    /// Schedule configuration (cron or fixed date).
    pub cron_config: Option<CronConfig>,
    /// Processor-specific JSON payload.
    pub event_payload: Option<Value>,
    /// Stable task name used to match related scheduled events.
    pub task_id: Option<String>,
}

/// Builds the canonical task name for a date-management scheduled event.
pub fn generate_manage_date_task_name(
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    event_processor: &EventProcessors,
) -> String {
    let base = format!("tenant_{}_event_{}_", tenant_id, election_event_id,);

    let base_with_election = match election_id {
        Some(id) => format!("{}election_{}_", base, id),
        None => base,
    };

    format!("{}{}", base_with_election, event_processor,)
}

/// Extracts voting-period start and end dates from matching scheduled events.
pub fn generate_voting_period_dates(
    scheduled_events: Vec<ScheduledEvent>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<VotingPeriodDates> {
    let payload = ManageElectionDatePayload {
        election_id: election_id.map(|s| s.to_string()),
    };
    let payload_val = serde_json::to_value(&payload)?;

    let start_date_name = generate_manage_date_task_name(
        tenant_id,
        election_event_id,
        election_id,
        &EventProcessors::START_VOTING_PERIOD,
    );
    let start_date =
        scheduled_events
            .clone()
            .into_iter()
            .find(|scheduled_event| {
                scheduled_event.tenant_id == Some(tenant_id.to_string())
                    && scheduled_event.election_event_id
                        == Some(election_event_id.to_string())
                    && scheduled_event.task_id == Some(start_date_name.clone())
                    && scheduled_event.event_payload
                        == Some(payload_val.clone())
            });

    let end_date_name = generate_manage_date_task_name(
        tenant_id,
        election_event_id,
        election_id,
        &EventProcessors::END_VOTING_PERIOD,
    );
    let end_date = scheduled_events.into_iter().find(|scheduled_event| {
        scheduled_event.tenant_id == Some(tenant_id.to_string())
            && scheduled_event.election_event_id
                == Some(election_event_id.to_string())
            && scheduled_event.task_id == Some(end_date_name.clone())
            && scheduled_event.event_payload == Some(payload_val.clone())
    });

    Ok(VotingPeriodDates {
        start_date: start_date
            .map(|val| val.cron_config.map(|val| val.scheduled_date))
            .flatten()
            .flatten(),
        end_date: end_date
            .map(|val| val.cron_config.map(|val| val.scheduled_date))
            .flatten()
            .flatten(),
    })
}

/// Converts a list of schedule events to a map of date names and
/// ScheduledEventDates.
///
/// If election_id is None, it will contain only dates schedule for the election
/// event.
/// If the election_id is Some(_), it will contain also dates scheduled for this
/// specific election.
pub fn prepare_scheduled_dates(
    scheduled_events: Vec<ScheduledEvent>,
    election_id: Option<&str>,
) -> Result<HashMap<String, ScheduledEventDates>> {
    // List of event processors related to scheduled event dates
    let date_event_processors = [
        EventProcessors::ALLOW_INIT_REPORT,
        EventProcessors::ALLOW_VOTING_PERIOD_END,
        EventProcessors::START_VOTING_PERIOD,
        EventProcessors::END_VOTING_PERIOD,
        EventProcessors::START_ENROLLMENT_PERIOD,
        EventProcessors::END_ENROLLMENT_PERIOD,
        EventProcessors::START_LOCKDOWN_PERIOD,
        EventProcessors::END_LOCKDOWN_PERIOD,
    ];

    Ok(scheduled_events
        .iter()
        .filter_map(|scheduled_event| {
            let Some(ref event_payload) = scheduled_event.event_payload else {
                return None;
            };
            let Ok(ManageElectionDatePayload {
                election_id: se_election_id,
                ..
            }) = serde_json::from_value(event_payload.clone())
            else {
                return None;
            };
            let Some(ref event_processor) = scheduled_event.event_processor
            else {
                return None;
            };
            if !date_event_processors.contains(&event_processor)
                || (se_election_id.is_some()
                    && election_id.is_some()
                    && se_election_id.as_deref() != election_id)
            {
                return None;
            }
            return Some((
                event_processor.to_string(),
                ScheduledEventDates {
                    scheduled_at: scheduled_event
                        .cron_config
                        .as_ref()
                        .and_then(|cron| cron.scheduled_date.clone()),
                    stopped_at: Some(format_date(
                        &scheduled_event.stopped_at,
                        "-",
                    )),
                },
            ));
        })
        .collect())
}
