// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use crate::ballot::format_date;
use crate::ballot::ScheduledEventDates;
use crate::ballot::VotingPeriodDates;
use anyhow::{anyhow, Result};
use chrono::DateTime;
use chrono::Utc;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum_macros::Display;
use strum_macros::EnumString;

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
/// Enum representing different types of event processors for scheduled events.
pub enum EventProcessors {
    #[strum(serialize = "ALLOW_INIT_REPORT")]
    /// Allow Initialization report to be generated.
    ALLOW_INIT_REPORT,
    #[strum(serialize = "CREATE_REPORT")]
    /// Scheduled event to create a report.
    CREATE_REPORT,
    #[strum(serialize = "SEND_TEMPLATE")]
    /// Scheduled event to send a template.
    SEND_TEMPLATE,
    #[strum(serialize = "START_VOTING_PERIOD")]
    /// Start of the voting period.
    START_VOTING_PERIOD,
    #[strum(serialize = "END_VOTING_PERIOD")]
    /// End of the voting period.
    END_VOTING_PERIOD,
    #[strum(serialize = "ALLOW_VOTING_PERIOD_END")]
    /// Allow the voting period to end.
    ALLOW_VOTING_PERIOD_END,
    #[strum(serialize = "START_ENROLLMENT_PERIOD")]
    /// Start of the enrollment period.
    START_ENROLLMENT_PERIOD,
    #[strum(serialize = "END_ENROLLMENT_PERIOD")]
    /// End of the enrollment period.
    END_ENROLLMENT_PERIOD,
    #[strum(serialize = "START_LOCKDOWN_PERIOD")]
    /// Start of the lockdown period.
    START_LOCKDOWN_PERIOD,
    #[strum(serialize = "END_LOCKDOWN_PERIOD")]
    /// End of the lockdown period.
    END_LOCKDOWN_PERIOD,
    #[strum(serialize = "ALLOW_TALLY")]
    /// Allow the tally to be performed.
    ALLOW_TALLY,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
/// Configuration for a cron job, including the cron expression and the scheduled date.
pub struct CronConfig {
    /// Cron expression defining the schedule for the event.
    pub cron: Option<String>,
    /// Scheduled date for the event.
    pub scheduled_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Payload for managing election dates, containing an optional election ID.
pub struct ManageElectionDatePayload {
    /// Election ID associated with the election date management.
    pub election_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Payload for managing the allowance of initialization report.
pub struct ManageAllowInitPayload {
    /// Election ID associated with the initialization report.
    pub election_id: Option<String>,
    #[serde(
        default = "default_allow_init",
        deserialize_with = "deserialize_allow_init"
    )]
    /// Flag indicating whether the initialization report is allowed. Defaults to true.
    ///
    /// Absent field and JSON `null` deserialize as `true` for compatibility with older payloads.
    pub allow_init: bool,
}

/// Default value for `allow_init` field in `ManageAllowInitPayload`.
///
/// Always returns true.
const fn default_allow_init() -> bool {
    true
}

/// Deserialize the `allow_init` field in `ManageAllowInitPayload`.
fn deserialize_allow_init<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<bool>::deserialize(deserializer)?;
    Ok(opt.unwrap_or(true))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Payload for managing the allowance of voting period end.
pub struct ManageAllowVotingPeriodEndPayload {
    /// Election ID associated with the voting period end.
    pub election_id: Option<String>,
    /// Flag indicating whether the voting period end is allowed.
    pub allow_voting_period_end: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Payload for managing the allowance of tally.
pub struct ManageAllowTallyPayload {
    /// Election ID associated with the tally.
    pub election_id: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
/// Represents a scheduled event in the system.
pub struct ScheduledEvent {
    /// Unique identifier for the scheduled event.
    pub id: String,
    /// Optional tenant ID associated with the event.
    pub tenant_id: Option<String>,
    /// Optional election event ID associated with the event.
    pub election_event_id: Option<String>,
    /// Scheduled creation date for the event, if applicable.
    pub created_at: Option<DateTime<Utc>>,
    /// Scheduled stop date for the event, if applicable.
    pub stopped_at: Option<DateTime<Utc>>,
    /// Scheduled archive date for the event, if applicable.
    pub archived_at: Option<DateTime<Utc>>,
    /// Labels associated with the event.
    pub labels: Option<Value>,
    /// Annotations associated with the event.
    pub annotations: Option<Value>,
    /// Event processor (type).
    pub event_processor: Option<EventProcessors>,
    /// Cron configuration for the event.
    pub cron_config: Option<CronConfig>,
    /// Event payload.
    pub event_payload: Option<Value>,
    /// Task ID associated with the event.
    pub task_id: Option<String>,
}

#[must_use]
/// Generates a task name for managing scheduled dates
pub fn generate_manage_date_task_name(
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    event_processor: &EventProcessors,
) -> String {
    let base = format!("tenant_{tenant_id}_event_{election_event_id}_");

    let base_with_election = match election_id {
        Some(id) => format!("{base}election_{id}_"),
        None => base,
    };

    format!("{base_with_election}{event_processor}")
}

/// Generate voting period dates from scheduled events.
///
/// # Errors
/// Returns an error if payload serialization or date extraction fails.
pub fn generate_voting_period_dates(
    scheduled_events: Vec<ScheduledEvent>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<VotingPeriodDates> {
    let payload = ManageElectionDatePayload {
        election_id: election_id.map(std::string::ToString::to_string),
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
            .and_then(|val| val.cron_config.and_then(|val| val.scheduled_date)),
        end_date: end_date
            .and_then(|val| val.cron_config.and_then(|val| val.scheduled_date)),
    })
}

/// Converts a list of schedule events to a map of date names and
/// `ScheduledEventDates`.
///
/// If `election_id` is None, it will contain only dates schedule for the election event.
/// If the `election_id` is Some(_), it will contain also dates scheduled for this specific election.
///
/// # Errors
/// Returns an error if deserialization or parsing fails.
pub fn prepare_scheduled_dates(
    scheduled_events: &[ScheduledEvent],
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
            let event_payload = scheduled_event.event_payload.as_ref()?;
            let Ok(ManageElectionDatePayload {
                election_id: se_election_id,
                ..
            }) = serde_json::from_value(event_payload.clone())
            else {
                return None;
            };
            let event_processor = scheduled_event.event_processor.as_ref()?;
            if !date_event_processors.contains(event_processor)
                || (se_election_id.is_some()
                    && election_id.is_some()
                    && se_election_id.as_deref() != election_id)
            {
                return None;
            }
            Some((
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
            ))
        })
        .collect())
}
