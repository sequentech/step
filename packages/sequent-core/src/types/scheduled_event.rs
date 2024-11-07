// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use crate::ballot::format_date;
use crate::ballot::ScheduledEventDates;
use crate::ballot::VotingPeriodDates;
use anyhow::Result;
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
    ALLOW_INIT_REPORT,
    CREATE_REPORT,
    SEND_TEMPLATE,
    START_VOTING_PERIOD,
    END_VOTING_PERIOD,
    ALLOW_VOTING_PERIOD_END,
    START_ENROLLMENT_PERIOD,
    END_ENROLLMENT_PERIOD,
    START_LOCKDOWN_PERIOD,
    END_LOCKDOWN_PERIOD,
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

pub fn prepare_report_scheduled_dates(
    scheduled_events: Vec<ScheduledEvent>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<HashMap<EventProcessors, ScheduledEventDates>> {
    let payload = ManageElectionDatePayload {
        election_id: election_id.map(|s| s.to_string()),
    };
    let payload_val = serde_json::to_value(&payload)?;

    let events = [
        EventProcessors::ALLOW_INIT_REPORT,
        EventProcessors::ALLOW_VOTING_PERIOD_END        ,
    ];

    let mut scheduled_event_map: HashMap<EventProcessors, ScheduledEventDates> =
        HashMap::new();

    for event in events.iter() {
        let date_name = generate_manage_date_task_name(
            tenant_id,
            election_event_id,
            election_id,
            &event,
        );
        let cloned_events = scheduled_events.clone();
        let event_date = cloned_events
            .iter()
            .find(|scheduled_event| {
                scheduled_event.tenant_id == Some(tenant_id.to_string())
                    && scheduled_event.election_event_id
                        == Some(election_event_id.to_string())
                    && scheduled_event.task_id == Some(date_name.clone())
                    && scheduled_event.event_payload
                        == Some(payload_val.clone())
            });

        if let Some(event_date) = event_date {
            scheduled_event_map.insert(
                event.clone(),
                ScheduledEventDates {
                    scheduled_at: event_date
                        .cron_config.as_ref()
                        .and_then(|cron| cron.scheduled_date.clone()),
                    stopped_at: Some(format_date(&event_date.stopped_at, "-")),
                },
            );
        }
    }

    Ok(scheduled_event_map)
}
