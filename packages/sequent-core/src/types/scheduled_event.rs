// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
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
    #[strum(serialize = "ALLOW_INIT_REPORT")]
    ALLOW_INIT_REPORT,
    #[strum(serialize = "CREATE_REPORT")]
    CREATE_REPORT,
    #[strum(serialize = "SEND_TEMPLATE")]
    SEND_TEMPLATE,
    #[strum(serialize = "START_VOTING_PERIOD")]
    START_VOTING_PERIOD,
    #[strum(serialize = "END_VOTING_PERIOD")]
    END_VOTING_PERIOD,
    #[strum(serialize = "ALLOW_VOTING_PERIOD_END")]
    ALLOW_VOTING_PERIOD_END,
    #[strum(serialize = "START_ENROLLMENT_PERIOD")]
    START_ENROLLMENT_PERIOD,
    #[strum(serialize = "END_ENROLLMENT_PERIOD")]
    END_ENROLLMENT_PERIOD,
    #[strum(serialize = "START_LOCKDOWN_PERIOD")]
    START_LOCKDOWN_PERIOD,
    #[strum(serialize = "END_LOCKDOWN_PERIOD")]
    END_LOCKDOWN_PERIOD,
    #[strum(serialize = "ALLOW_TALLY")]
    ALLOW_TALLY,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct CronConfig {
    pub cron: Option<String>,
    pub scheduled_date: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageElectionDatePayload {
    pub election_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageAllowInitPayload {
    pub election_id: Option<String>,
    #[serde(default = "default_allow_init")]
    pub allow_init: Option<bool>,
}

fn default_allow_init() -> Option<bool> {
    Some(true)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageAllowVotingPeriodEndPayload {
    pub election_id: Option<String>,
    pub allow_voting_period_end: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManageAllowTallyPayload {
    pub election_id: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ScheduledEvent {
    pub id: String,
    pub tenant_id: Option<String>,
    pub election_event_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub stopped_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub labels: Option<Value>,
    pub annotations: Option<Value>,
    pub event_processor: Option<EventProcessors>,
    pub cron_config: Option<CronConfig>,
    pub event_payload: Option<Value>,
    pub task_id: Option<String>,
}

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
