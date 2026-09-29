// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_camel_case_types)]

use crate::ballot::ScheduledEventDates;
use crate::ballot::VotingPeriodDates;
use crate::ballot::{format_date, VotingStatusChannel};
use crate::types::hasura::core::VotingChannels;
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ManageElectionDatePayload {
    pub election_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voting_channels: Option<Vec<VotingStatusChannel>>,
}

/// Order in which scheduled channel changes are applied, so the saved order of
/// a selection never changes the outcome.
const SCHEDULED_CHANNEL_ORDER: [VotingStatusChannel; 4] = [
    VotingStatusChannel::ONLINE,
    VotingStatusChannel::KIOSK,
    VotingStatusChannel::EARLY_VOTING,
    VotingStatusChannel::TELEPHONE,
];

pub const ONLINE_WITH_EARLY_VOTING_START_ERROR: &str =
    "A start voting period schedule cannot open ONLINE and EARLY_VOTING \
     together: early voting has to start before online voting.";

impl ManageElectionDatePayload {
    pub fn channels(&self) -> Vec<VotingStatusChannel> {
        let selected = self
            .voting_channels
            .clone()
            .filter(|channels| !channels.is_empty())
            .unwrap_or_else(|| {
                vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK]
            });
        SCHEDULED_CHANNEL_ORDER
            .into_iter()
            .filter(|channel| selected.contains(channel))
            .collect()
    }

    pub fn enabled_channels(
        &self,
        configured: &VotingChannels,
    ) -> Vec<VotingStatusChannel> {
        let mut channels = self.channels();
        channels
            .retain(|channel| channel.channel_from(configured) == Some(true));
        channels
    }
}

/// Early voting cannot start once online voting has started, so a single
/// start schedule cannot open both channels.
pub fn validate_scheduled_voting_channels(
    event_processor: &EventProcessors,
    voting_channels: Option<&[VotingStatusChannel]>,
) -> Result<()> {
    let channels = voting_channels.unwrap_or_default();
    if *event_processor == EventProcessors::START_VOTING_PERIOD
        && channels.contains(&VotingStatusChannel::ONLINE)
        && channels.contains(&VotingStatusChannel::EARLY_VOTING)
    {
        return Err(anyhow!(ONLINE_WITH_EARLY_VOTING_START_ERROR));
    }
    Ok(())
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
    let matches_payload = |scheduled: &ScheduledEvent| {
        scheduled
            .event_payload
            .clone()
            .and_then(|value| {
                serde_json::from_value::<ManageElectionDatePayload>(value).ok()
            })
            .map(|payload| {
                payload.election_id.as_deref() == election_id
                    && payload.channels().contains(&VotingStatusChannel::ONLINE)
            })
            .unwrap_or(false)
    };

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
                    && matches_payload(scheduled_event)
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
            && matches_payload(scheduled_event)
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

#[cfg(test)]
mod voting_channel_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn legacy_missing_null_and_empty_channels_use_online_and_kiosk() {
        for value in [
            json!({}),
            json!({"voting_channels": null}),
            json!({"voting_channels": []}),
        ] {
            let payload: ManageElectionDatePayload =
                serde_json::from_value(value).unwrap();
            assert_eq!(
                payload.channels(),
                vec![VotingStatusChannel::ONLINE, VotingStatusChannel::KIOSK]
            );
        }
        assert_eq!(
            serde_json::to_value(ManageElectionDatePayload::default()).unwrap(),
            json!({"election_id": null})
        );
    }
    #[test]
    fn explicit_channels_roundtrip_and_filter_disabled_election_channels() {
        let payload: ManageElectionDatePayload = serde_json::from_value(json!({"election_id": "el1", "voting_channels": ["KIOSK", "TELEPHONE", "EARLY_VOTING", "ONLINE"]})).unwrap();
        let roundtrip: ManageElectionDatePayload =
            serde_json::from_value(serde_json::to_value(&payload).unwrap())
                .unwrap();
        assert_eq!(roundtrip.channels(), payload.channels());
        let config = VotingChannels {
            online: Some(true),
            kiosk: None,
            telephone: Some(true),
            early_voting: Some(false),
            paper: None,
        };
        assert_eq!(
            payload.enabled_channels(&config),
            vec![VotingStatusChannel::ONLINE, VotingStatusChannel::TELEPHONE]
        );
        assert!(serde_json::from_value::<ManageElectionDatePayload>(
            json!({"voting_channels": ["INVALID"]})
        )
        .is_err());
    }
    #[test]
    fn channels_follow_a_fixed_order_regardless_of_the_saved_order() {
        let payload: ManageElectionDatePayload = serde_json::from_value(
            json!({"voting_channels": ["TELEPHONE", "EARLY_VOTING", "KIOSK", "ONLINE", "KIOSK"]}),
        )
        .unwrap();
        assert_eq!(
            payload.channels(),
            vec![
                VotingStatusChannel::ONLINE,
                VotingStatusChannel::KIOSK,
                VotingStatusChannel::EARLY_VOTING,
                VotingStatusChannel::TELEPHONE,
            ]
        );
    }
    #[test]
    fn start_schedules_cannot_open_online_and_early_voting_together() {
        use VotingStatusChannel::*;
        let both = [EARLY_VOTING, ONLINE];
        assert!(validate_scheduled_voting_channels(
            &EventProcessors::START_VOTING_PERIOD,
            Some(&both)
        )
        .is_err());
        for (processor, channels) in [
            (EventProcessors::END_VOTING_PERIOD, Some(&both[..])),
            (
                EventProcessors::START_VOTING_PERIOD,
                Some(&[EARLY_VOTING][..]),
            ),
            (
                EventProcessors::START_VOTING_PERIOD,
                Some(&[ONLINE, KIOSK, TELEPHONE][..]),
            ),
            (EventProcessors::START_VOTING_PERIOD, Some(&[][..])),
            (EventProcessors::START_VOTING_PERIOD, None),
        ] {
            assert!(
                validate_scheduled_voting_channels(&processor, channels)
                    .is_ok(),
                "{processor:?} {channels:?}"
            );
        }
    }
    #[test]
    fn online_dates_accept_extended_payload_but_ignore_kiosk_only_schedules() {
        for channels in [
            json!(null),
            json!([]),
            json!(["ONLINE", "KIOSK"]),
            json!(["KIOSK"]),
        ] {
            let schedule: ScheduledEvent = serde_json::from_value(json!({
                "id": "schedule", "tenant_id": "tenant", "election_event_id": "event",
                "task_id": generate_manage_date_task_name("tenant", "event", Some("el1"), &EventProcessors::END_VOTING_PERIOD),
                "event_payload": {"election_id": "el1", "voting_channels": channels},
                "cron_config": {"scheduled_date": "2027-01-01T12:00:00Z"}
            })).unwrap();
            let dates = generate_voting_period_dates(
                vec![schedule],
                "tenant",
                "event",
                Some("el1"),
            )
            .unwrap();
            assert_eq!(dates.end_date.is_some(), channels != json!(["KIOSK"]));
        }
    }
}
