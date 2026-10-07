// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Saving a scheduled date (VOTE-LIFECYCLE §5). A date is entered as a wall
//! time in a zone; the server resolves it with the tz database, stores the
//! wall time, the zone and the instant, and answers with the warnings the
//! row deserves (DST, the 30-day voting window, the final testing lead time).
//! Warnings never block a save.

use crate::postgres::election::*;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::*;
use crate::services::election_event_status::get_election_event_status;
use crate::services::signing::log::Actor;
use crate::services::time_zones::{local_in, parse_local, parse_zone, resolve_local, Resolved};
use crate::services::{election_event_dates, enrollment_windows, scheduled_outcome};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, NaiveDateTime, SecondsFormat, Timelike, Utc};
use chrono_tz::Tz;
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::ballot::{
    ElectionEventPresentation, ElectionEventStatus, ElectionPresentation, PeriodDates,
    StringifiedPeriodDates,
};
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use sequent_core::time_zones::{effective_time_zone, DEFAULT_TIME_ZONE};
use sequent_core::types::hasura::core::Election;
use sequent_core::types::scheduled_event::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::str::FromStr;
use tracing::{instrument, warn};

/// How many local calendar days each Post's voting window covers (V.18, 5.1.1).
pub const VOTING_WINDOW_DAYS: i64 = 30;
/// How long before a Post's voting opens its final testing starts, at least (2.8.2).
pub const FINAL_TESTING_LEAD_DAYS: i64 = 7;

pub const WARNING_DST_GAP: &str = "dst-gap";
pub const WARNING_DST_OVERLAP: &str = "dst-overlap";
pub const WARNING_VOTING_WINDOW_DAYS: &str = "voting-window-days";
pub const WARNING_FINAL_TESTING_LEAD_TIME: &str = "final-testing-lead-time";
pub const WARNING_CLOSE_BEFORE_OPEN: &str = "close-before-open";
pub const WARNING_SHORT_LAST_DAY: &str = "short-last-day";
/// A last local voting day shorter than this many hours is noted. How the
/// 30-day rule applies to Posts far west of the primary zone is the
/// election authority's decision (ticket D4); the check only warns.
pub const SHORT_LAST_DAY_HOURS: i64 = 12;

/// The date as the form sends it: a wall time and its zone, or (older
/// clients) an instant with an offset.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduleInput {
    pub local_date_time: Option<String>,
    pub time_zone: Option<String>,
    pub scheduled_date: Option<String>,
}

/// A note on a saved row; it never blocks the save. `message_key` is an
/// i18n key and `params` its placeholders.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScheduleWarning {
    pub code: String,
    pub election_id: Option<String>,
    pub message_key: String,
    pub params: Map<String, Value>,
}

/// What a save answers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SavedSchedule {
    /// The stored instant (RFC 3339, UTC), `None` when the date was removed.
    pub scheduled_date: Option<String>,
    pub warnings: Vec<ScheduleWarning>,
}

/// A date the server refuses (the request is wrong, not the server).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSchedule(pub String);

impl std::fmt::Display for InvalidSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidSchedule {}

fn invalid(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(InvalidSchedule(message.into()))
}

/// An instant as the scheduler and the database CHECK read it.
pub fn instant_text(instant: DateTime<Utc>) -> String {
    instant.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// A wall time as the forms show it: `YYYY-MM-DDTHH:MM` (seconds only when set).
pub fn wall_text(local: NaiveDateTime) -> String {
    if local.second() == 0 {
        local.format("%Y-%m-%dT%H:%M").to_string()
    } else {
        local.format("%Y-%m-%dT%H:%M:%S").to_string()
    }
}

/// An instant with an offset; a date without one is refused, since the
/// scheduler can't tell when it would run.
pub fn parse_instant(date: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date.trim())
        .map(|date| date.with_timezone(&Utc))
        .map_err(|_| {
            invalid(format!(
                "Invalid scheduled date {date:?}: it needs a date, a time and an offset (RFC 3339)"
            ))
        })
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn params(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.clone()))
        .collect()
}

/// The cron config to store for `input`, and its DST warnings. `None` means
/// the date was cleared. `default_zone` is the row's zone (§3), used when
/// the input names none.
pub fn resolve_schedule(
    input: &ScheduleInput,
    default_zone: &str,
    election_id: Option<&str>,
) -> Result<Option<(CronConfig, Vec<ScheduleWarning>)>> {
    let zone_name = non_empty(&input.time_zone).unwrap_or(default_zone);
    let zone: Tz =
        parse_zone(zone_name).map_err(|_| invalid(format!("Unknown timezone: {zone_name}")))?;
    let mut warnings = vec![];
    let (instant, local) = if let Some(local_text) = non_empty(&input.local_date_time) {
        let local = parse_local(local_text).map_err(|err| invalid(err.to_string()))?;
        let resolved = resolve_local(local, zone);
        match resolved {
            Resolved::Exact(_) => {}
            Resolved::Gap { shifted } => warnings.push(ScheduleWarning {
                code: WARNING_DST_GAP.into(),
                election_id: election_id.map(str::to_owned),
                message_key: "timezones.gap".into(),
                params: params(&[
                    ("local", json!(wall_text(local))),
                    ("shifted_local", json!(wall_text(local_in(zone, shifted)))),
                    ("time_zone", json!(zone.name())),
                    ("instant", json!(instant_text(shifted))),
                ]),
            }),
            Resolved::Overlap { first, second } => warnings.push(ScheduleWarning {
                code: WARNING_DST_OVERLAP.into(),
                election_id: election_id.map(str::to_owned),
                message_key: "timezones.overlap".into(),
                params: params(&[
                    ("local", json!(wall_text(local))),
                    ("time_zone", json!(zone.name())),
                    ("instant", json!(instant_text(first))),
                    ("second_instant", json!(instant_text(second))),
                ]),
            }),
        }
        // A wall time in a gap is saved as the shifted time the preview
        // shows, so it round-trips (design §1).
        let saved_local = match resolved {
            Resolved::Gap { shifted } => local_in(zone, shifted),
            _ => local,
        };
        (resolved.instant(), saved_local)
    } else if let Some(date) = non_empty(&input.scheduled_date) {
        // Older clients send the instant only; keep its wall time in the
        // row's zone so a tz database update can recompute it.
        let instant = parse_instant(date)?;
        (instant, local_in(zone, instant))
    } else {
        return Ok(None);
    };
    Ok(Some((
        CronConfig {
            cron: None,
            scheduled_date: Some(instant_text(instant)),
            local: Some(wall_text(local)),
            timezone: Some(zone.name().to_owned()),
        },
        warnings,
    )))
}

/// A Post and the zone its schedule is in.
#[derive(Debug, Clone, PartialEq)]
pub struct PostZone {
    pub election_id: String,
    pub zone: Tz,
}

fn zone_or_utc(name: &str) -> Tz {
    parse_zone(name).unwrap_or_else(|_| {
        warn!("Unknown timezone {name:?}; using {DEFAULT_TIME_ZONE}");
        Tz::UTC
    })
}

/// The zone of each election, by the §3 rule.
pub fn post_zones(
    event: Option<&ElectionEventPresentation>,
    elections: &[Election],
) -> Vec<PostZone> {
    elections
        .iter()
        .map(|election| PostZone {
            election_id: election.id.clone(),
            zone: zone_or_utc(&effective_time_zone(
                event,
                election.get_presentation().as_ref(),
            )),
        })
        .collect()
}

fn scheduled_instant(
    events: &[ScheduledEvent],
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    processor: &EventProcessors,
) -> Option<DateTime<Utc>> {
    let task_id =
        generate_manage_date_task_name(tenant_id, election_event_id, Some(election_id), processor);
    events
        .iter()
        .find(|event| {
            event.archived_at.is_none() && event.task_id.as_deref() == Some(task_id.as_str())
        })
        .and_then(|event| event.cron_config.as_ref())
        .and_then(|cron| cron.scheduled_date.as_deref())
        .and_then(|date| DateTime::parse_from_rfc3339(date).ok())
        .map(|date| date.with_timezone(&Utc))
}

/// The local calendar dates a voting window `[start, end)` touches.
pub fn voting_window_days(zone: Tz, start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    let first = local_in(zone, start).date();
    let last = local_in(zone, end - Duration::seconds(1)).date();
    (last - first).num_days() + 1
}

/// The whole hours of voting on the last local voting day of `[start, end)`.
pub fn last_day_hours(zone: Tz, start: DateTime<Utc>, end: DateTime<Utc>) -> i64 {
    let last_day = local_in(zone, end - Duration::seconds(1)).date();
    let day_start = local_in(zone, start).max(last_day.and_hms_opt(0, 0, 0).unwrap_or_default());
    (local_in(zone, end) - day_start).num_hours()
}

/// The 30-day rule and the final testing lead time for `posts`, from the
/// event's active schedule (`events`). The voting window of a Post is its
/// opening and its own close, or the event-wide close.
pub fn rule_warnings(
    tenant_id: &str,
    election_event_id: &str,
    events: &[ScheduledEvent],
    posts: &[PostZone],
) -> Vec<ScheduleWarning> {
    let parse = |date: Option<String>| {
        date.and_then(|date| DateTime::parse_from_rfc3339(&date).ok())
            .map(|date| date.with_timezone(&Utc))
    };
    let active: Vec<ScheduledEvent> = events
        .iter()
        .filter(|event| event.archived_at.is_none())
        .cloned()
        .collect();
    let mut warnings = vec![];
    for post in posts {
        let zone = post.zone;
        let dates = generate_voting_period_dates(
            active.clone(),
            tenant_id,
            election_event_id,
            Some(&post.election_id),
        )
        .unwrap_or_default();
        let start = parse(dates.start_date);
        let end = parse(dates.end_date);
        if let (Some(start), Some(end)) = (start, end) {
            if end <= start {
                warnings.push(ScheduleWarning {
                    code: WARNING_CLOSE_BEFORE_OPEN.into(),
                    election_id: Some(post.election_id.clone()),
                    message_key: "eventsScreen.warning.closeBeforeOpen".into(),
                    params: params(&[
                        ("start_local", json!(wall_text(local_in(zone, start)))),
                        ("end_local", json!(wall_text(local_in(zone, end)))),
                        ("time_zone", json!(zone.name())),
                    ]),
                });
            } else {
                let days = voting_window_days(zone, start, end);
                if days != VOTING_WINDOW_DAYS {
                    warnings.push(ScheduleWarning {
                        code: WARNING_VOTING_WINDOW_DAYS.into(),
                        election_id: Some(post.election_id.clone()),
                        message_key: "eventsScreen.warning.votingWindowDays".into(),
                        params: params(&[
                            ("days", json!(days)),
                            ("expected", json!(VOTING_WINDOW_DAYS)),
                            ("start_local", json!(wall_text(local_in(zone, start)))),
                            ("end_local", json!(wall_text(local_in(zone, end)))),
                            ("time_zone", json!(zone.name())),
                        ]),
                    });
                }
                let hours = last_day_hours(zone, start, end);
                if hours < SHORT_LAST_DAY_HOURS {
                    warnings.push(ScheduleWarning {
                        code: WARNING_SHORT_LAST_DAY.into(),
                        election_id: Some(post.election_id.clone()),
                        message_key: "eventsScreen.warning.shortLastDay".into(),
                        params: params(&[
                            ("hours", json!(hours)),
                            ("minimum_hours", json!(SHORT_LAST_DAY_HOURS)),
                            ("end_local", json!(wall_text(local_in(zone, end)))),
                            ("time_zone", json!(zone.name())),
                        ]),
                    });
                }
            }
        }
        let final_testing = scheduled_instant(
            &active,
            tenant_id,
            election_event_id,
            &post.election_id,
            &EventProcessors::START_FINAL_TESTING,
        );
        if let (Some(final_testing), Some(start)) = (final_testing, start) {
            // Both wall times in the Post's zone, as the rule reads them.
            let lead = local_in(zone, start) - local_in(zone, final_testing);
            if lead < Duration::days(FINAL_TESTING_LEAD_DAYS) {
                warnings.push(ScheduleWarning {
                    code: WARNING_FINAL_TESTING_LEAD_TIME.into(),
                    election_id: Some(post.election_id.clone()),
                    message_key: "eventsScreen.warning.finalTestingLeadTime".into(),
                    params: params(&[
                        ("minimum_days", json!(FINAL_TESTING_LEAD_DAYS)),
                        ("lead_hours", json!(lead.num_hours())),
                        (
                            "final_testing_local",
                            json!(wall_text(local_in(zone, final_testing))),
                        ),
                        (
                            "voting_start_local",
                            json!(wall_text(local_in(zone, start))),
                        ),
                        ("time_zone", json!(zone.name())),
                    ]),
                });
            }
        }
    }
    warnings
}

/// The processors that only make sense for one election.
pub fn is_election_scoped_only(processor: &EventProcessors) -> bool {
    matches!(
        processor,
        EventProcessors::START_READINESS_TEST
            | EventProcessors::END_READINESS_TEST
            | EventProcessors::START_FINAL_TESTING
            | EventProcessors::END_FINAL_TESTING
            | EventProcessors::START_TEST_VOTING
            | EventProcessors::END_TEST_VOTING
    )
}

fn checks_voting_rules(processor: &EventProcessors) -> bool {
    matches!(
        processor,
        EventProcessors::START_VOTING_PERIOD
            | EventProcessors::END_VOTING_PERIOD
            | EventProcessors::START_FINAL_TESTING
    )
}

/// Saves (or, with an empty date, archives) the scheduled date of
/// `event_processor` for the election (or the whole event), and answers with
/// the stored instant and the warnings. Refusals are [`InvalidSchedule`].
#[instrument(skip(hasura_transaction), err)]
pub async fn save_schedule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    event_processor: &EventProcessors,
    input: &ScheduleInput,
    voting_channels: Option<Vec<VotingStatusChannel>>,
    actor: &Actor,
    scheduled_event_id: Option<&str>,
) -> Result<SavedSchedule> {
    if election_id.is_none() && is_election_scoped_only(event_processor) {
        return Err(invalid(format!("{event_processor} needs an election")));
    }
    crate::postgres::scheduled_event::lock_scheduling_event(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?;
    let event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    let event_presentation: Option<ElectionEventPresentation> = event
        .presentation
        .clone()
        .and_then(|value| match deserialize_value(value) {
            Ok(presentation) => Some(presentation),
            Err(err) => {
                warn!("Unreadable presentation of election event {election_event_id}: {err}");
                None
            }
        });
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id).await?;
    let election = match election_id {
        Some(id) => Some(
            elections
                .iter()
                .find(|election| election.id == id)
                .ok_or_else(|| invalid(format!("Election {id} not found")))?,
        ),
        None => None,
    };
    let election_presentation: Option<ElectionPresentation> =
        election.and_then(|election| election.get_presentation());
    let default_zone =
        effective_time_zone(event_presentation.as_ref(), election_presentation.as_ref());

    let (cron_config, mut warnings) = match resolve_schedule(input, &default_zone, election_id)? {
        Some((cron_config, warnings)) => (Some(cron_config), warnings),
        None => (None, vec![]),
    };
    let scheduled_date = cron_config
        .as_ref()
        .and_then(|cron| cron.scheduled_date.clone());

    match election_id {
        Some(id) => {
            manage_dates(
                hasura_transaction,
                tenant_id,
                election_event_id,
                id,
                cron_config,
                &event_processor.to_string(),
                voting_channels,
                scheduled_event_id,
            )
            .await?
        }
        None => {
            election_event_dates::manage_dates(
                hasura_transaction,
                tenant_id,
                election_event_id,
                cron_config,
                &event_processor.to_string(),
                voting_channels,
                scheduled_event_id,
            )
            .await?
        }
    }

    if checks_voting_rules(event_processor) {
        let events = find_scheduled_event_by_election_event_id(
            hasura_transaction,
            tenant_id,
            election_event_id,
        )
        .await?;
        let posts: Vec<PostZone> = post_zones(event_presentation.as_ref(), &elections)
            .into_iter()
            .filter(|post| election_id.map_or(true, |id| post.election_id == id))
            .collect();
        warnings.extend(rule_warnings(tenant_id, election_event_id, &events, &posts));
    }

    scheduled_outcome::recompute_predictions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        actor,
    )
    .await?;

    Ok(SavedSchedule {
        scheduled_date,
        warnings,
    })
}

#[instrument(skip(hasura_transaction), err)]
pub async fn manage_dates(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    cron_config: Option<CronConfig>,
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
    .map_err(|err| invalid(err.to_string()))
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

/// Apply authenticated current deadline metadata to report/notification data.
/// Never call this while constructing an artifact whose bytes were signed.
/// A known null removes a live deadline that the lifecycle policy refuses.
pub fn apply_display_voting_close(
    dates: &mut StringifiedPeriodDates,
    close: Option<&DisplayVotingClose>,
) {
    if let Some(close) = close {
        let entry = dates
            .scheduled_event_dates
            .get_or_insert_with(Default::default)
            .entry(EventProcessors::END_VOTING_PERIOD.to_string())
            .or_default();
        entry.scheduled_at = close.scheduled_at.clone();
        entry.timezone = close.timezone.clone();
    }
}

#[cfg(test)]
#[path = "election_dates_tests.rs"]
mod election_dates_tests;
