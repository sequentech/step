// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The voting window of a transparent event, its policy's `opens` and
//! `closes`: from the earliest scheduled start to the latest scheduled end,
//! grace periods included, of the event and its elections.

use anyhow::{Context, Result};
use sequent_core::ballot::{EGracePeriodPolicy, ElectionPresentation};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::services::date::ISO8601;
use sequent_core::types::hasura::core::Election;
use sequent_core::types::scheduled_event::{generate_voting_period_dates, ScheduledEvent};

/// The longest quiet period step's policies ask for, before `closes`.
pub const QUIET_SECS: i64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VotingWindow {
    pub opens: i64,
    pub closes: i64,
}

impl VotingWindow {
    /// `quiet 3600`, or the whole window if it is shorter.
    pub fn quiet(&self) -> i64 {
        QUIET_SECS.min(self.closes - self.opens)
    }
}

pub(super) fn timestamp(date: Option<String>) -> Result<Option<i64>> {
    date.map(|date| {
        ISO8601::to_date(&date)
            .map(|date| date.timestamp())
            .with_context(|| format!("Error parsing the scheduled date {date:?}"))
    })
    .transpose()
}

/// How long after its end an election keeps accepting the ballots of voters
/// who logged in before it.
pub(super) fn grace_secs(election: &Election) -> Result<i64> {
    let presentation: ElectionPresentation = election
        .presentation
        .clone()
        .map(deserialize_value)
        .transpose()
        .context("Error parsing the election's presentation")?
        .unwrap_or_default();
    let policy = presentation
        .grace_period_policy
        .unwrap_or(EGracePeriodPolicy::NO_GRACE_PERIOD);
    Ok(match policy {
        EGracePeriodPolicy::NO_GRACE_PERIOD => 0,
        _ => i64::try_from(presentation.grace_period_secs.unwrap_or(0)).unwrap_or(i64::MAX),
    })
}

/// The window, or `None` while an election has no scheduled start or end,
/// its own or the event's.
pub fn voting_window(
    scheduled_events: &[ScheduledEvent],
    tenant_id: &str,
    election_event_id: &str,
    elections: &[Election],
) -> Result<Option<VotingWindow>> {
    let event_dates = generate_voting_period_dates(
        scheduled_events.to_vec(),
        tenant_id,
        election_event_id,
        None,
    )?;
    let event_start = timestamp(event_dates.start_date)?;
    let event_end = timestamp(event_dates.end_date)?;
    let mut window: Option<VotingWindow> = None;
    for election in elections {
        let dates = generate_voting_period_dates(
            scheduled_events.to_vec(),
            tenant_id,
            election_event_id,
            Some(&election.id),
        )?;
        let (Some(start), Some(end)) = (
            timestamp(dates.start_date)?.or(event_start),
            timestamp(dates.end_date)?.or(event_end),
        ) else {
            return Ok(None);
        };
        let end = end.saturating_add(grace_secs(election)?);
        window = Some(match window {
            None => VotingWindow {
                opens: start,
                closes: end,
            },
            Some(w) => VotingWindow {
                opens: w.opens.min(start),
                closes: w.closes.max(end),
            },
        });
    }
    Ok(window.filter(|w| w.opens < w.closes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::scheduled_event::{
        generate_manage_date_task_name, CronConfig, EventProcessors,
    };
    use serde_json::json;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";
    const EVENT: &str = "e7a2b3c4-0000-4000-8000-000000000001";
    const MAYOR: &str = "e7a2b3c4-0000-4000-8000-000000000002";
    const COUNCIL: &str = "e7a2b3c4-0000-4000-8000-000000000003";

    fn schedule(election: Option<&str>, processor: EventProcessors, date: &str) -> ScheduledEvent {
        ScheduledEvent {
            id: format!("{election:?}-{processor}"),
            tenant_id: Some(TENANT.into()),
            election_event_id: Some(EVENT.into()),
            created_at: None,
            stopped_at: None,
            archived_at: None,
            labels: None,
            annotations: None,
            event_processor: Some(processor.clone()),
            cron_config: Some(CronConfig {
                cron: None,
                scheduled_date: Some(date.into()),
            }),
            event_payload: Some(json!({ "election_id": election })),
            task_id: Some(generate_manage_date_task_name(
                TENANT, EVENT, election, &processor,
            )),
        }
    }

    fn election(id: &str, presentation: Option<serde_json::Value>) -> Election {
        serde_json::from_value(json!({
            "id": id,
            "tenant_id": TENANT,
            "election_event_id": EVENT,
            "presentation": presentation,
        }))
        .unwrap()
    }

    #[test]
    fn the_window_spans_every_election() {
        let events = vec![
            schedule(
                Some(MAYOR),
                EventProcessors::START_VOTING_PERIOD,
                "2026-11-01T08:00:00Z",
            ),
            schedule(
                Some(MAYOR),
                EventProcessors::END_VOTING_PERIOD,
                "2026-11-01T20:00:00Z",
            ),
            schedule(
                Some(COUNCIL),
                EventProcessors::START_VOTING_PERIOD,
                "2026-11-01T09:00:00Z",
            ),
            schedule(
                Some(COUNCIL),
                EventProcessors::END_VOTING_PERIOD,
                "2026-11-01T22:00:00+01:00",
            ),
        ];
        let window = voting_window(
            &events,
            TENANT,
            EVENT,
            &[election(MAYOR, None), election(COUNCIL, None)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(window.opens, 1793520000);
        assert_eq!(window.closes, 1793566800);
        assert_eq!(window.quiet(), QUIET_SECS);
    }

    #[test]
    fn elections_take_the_events_dates_and_their_grace_periods() {
        let events = vec![
            schedule(
                None,
                EventProcessors::START_VOTING_PERIOD,
                "2026-11-01T08:00:00Z",
            ),
            schedule(
                None,
                EventProcessors::END_VOTING_PERIOD,
                "2026-11-01T08:30:00Z",
            ),
        ];
        let grace = json!({ "grace_period_policy": "grace-period-without-alert", "grace_period_secs": 600 });
        let window = voting_window(
            &events,
            TENANT,
            EVENT,
            &[election(MAYOR, Some(grace)), election(COUNCIL, None)],
        )
        .unwrap()
        .unwrap();
        assert_eq!(window.closes - window.opens, 2400);
        assert_eq!(window.quiet(), 2400);
    }

    #[test]
    fn an_election_without_an_end_has_no_window() {
        let events = vec![schedule(
            Some(MAYOR),
            EventProcessors::START_VOTING_PERIOD,
            "2026-11-01T08:00:00Z",
        )];
        assert_eq!(
            voting_window(&events, TENANT, EVENT, &[election(MAYOR, None)]).unwrap(),
            None
        );
        assert_eq!(voting_window(&[], TENANT, EVENT, &[]).unwrap(), None);
    }
}
