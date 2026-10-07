// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The tz database check (VOTE-LIFECYCLE §5). Every scheduled date keeps the
//! wall time and the zone it was entered in. When a tz database update moves
//! the instant of a future wall time (a country changes its offset or its
//! DST rules), a daily check records the new instant on the scheduled event
//! (`annotations.schedule_recompute`) and changes nothing; an administrator
//! applies it (`apply_schedule_recompute`), which is logged.

use crate::postgres::scheduled_event::{
    find_all_active_events, find_scheduled_event_by_election_event_id,
    set_scheduled_event_annotation, update_scheduled_event,
};
use crate::services::election_dates::instant_text;
use crate::services::scheduled_outcome;
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use crate::services::time_zones::{parse_local, parse_zone, resolve_local, Resolved};
use anyhow::Result;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::types::scheduled_event::{CronConfig, ScheduledEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// The annotation key on `scheduled_event.annotations`.
pub const ANNOTATION: &str = "schedule_recompute";

/// A pending change, as the Scheduled Events banner lists it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleRecompute {
    /// The instant the current tz database gives (RFC 3339, UTC).
    pub scheduled_date: String,
    /// The stored instant.
    pub previous: String,
    pub local: String,
    pub timezone: String,
    pub checked_at: String,
}

fn parse(date: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(date)
        .ok()
        .map(|date| date.with_timezone(&Utc))
}

/// The stored instant and the one the current tz database gives for the
/// row's wall time and zone, when they differ. Rows without a wall time
/// and a zone (entered before they were kept) are left alone.
pub fn recomputed(cron: &CronConfig) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let stored = parse(cron.scheduled_date.as_deref()?)?;
    let local = parse_local(cron.local.as_deref()?).ok()?;
    let zone = match parse_zone(cron.timezone.as_deref()?) {
        Ok(zone) => zone,
        Err(err) => {
            warn!("Can't recompute a scheduled date: {err}");
            return None;
        }
    };
    let resolved = resolve_local(local, zone);
    // A wall time that happens twice keeps the occurrence it was stored
    // with: either one is unchanged.
    if let Resolved::Overlap { first, second } = resolved {
        if stored == first || stored == second {
            return None;
        }
    }
    let instant = resolved.instant();
    (instant != stored).then_some((stored, instant))
}

/// The pending change of a future, active row, or `None`.
pub fn pending_change(event: &ScheduledEvent, now: DateTime<Utc>) -> Option<ScheduleRecompute> {
    if event.stopped_at.is_some() || event.archived_at.is_some() {
        return None;
    }
    let cron = event.cron_config.as_ref()?;
    let (stored, instant) = recomputed(cron)?;
    if stored <= now && instant <= now {
        return None;
    }
    Some(ScheduleRecompute {
        scheduled_date: instant_text(instant),
        previous: instant_text(stored),
        local: cron.local.clone()?,
        timezone: cron.timezone.clone()?,
        checked_at: instant_text(now),
    })
}

fn annotation(event: &ScheduledEvent) -> Option<ScheduleRecompute> {
    event
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(ANNOTATION))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}

/// Records (or clears) the pending change of every active future row;
/// answers how many rows have one.
#[instrument(skip(hasura_transaction), err)]
pub async fn check_all(hasura_transaction: &Transaction<'_>, now: DateTime<Utc>) -> Result<usize> {
    let events = find_all_active_events(hasura_transaction).await?;
    let mut pending = 0;
    for event in &events {
        let Some(tenant_id) = event.tenant_id.as_deref() else {
            continue;
        };
        let change = pending_change(event, now);
        let recorded = annotation(event);
        match change {
            Some(change) => {
                pending += 1;
                // Only a new or different instant rewrites the annotation.
                if recorded.map(|recorded| recorded.scheduled_date)
                    != Some(change.scheduled_date.clone())
                {
                    info!(
                        "Scheduled event {} moves from {} to {} with the current tz database",
                        event.id, change.previous, change.scheduled_date
                    );
                    set_scheduled_event_annotation(
                        hasura_transaction,
                        tenant_id,
                        &event.id,
                        ANNOTATION,
                        Some(serde_json::to_value(&change)?),
                    )
                    .await?;
                }
            }
            None if recorded.is_some() => {
                set_scheduled_event_annotation(
                    hasura_transaction,
                    tenant_id,
                    &event.id,
                    ANNOTATION,
                    None,
                )
                .await?;
            }
            None => {}
        }
    }
    Ok(pending)
}

/// Whether applying this batch changes enrollment windows. Caller holds the event lock.
pub async fn enrollment_changes_pending(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    now: DateTime<Utc>,
) -> Result<bool> {
    crate::postgres::scheduled_event::lock_scheduling_event(
        transaction,
        tenant_id,
        election_event_id,
    )
    .await?;
    let events =
        find_scheduled_event_by_election_event_id(transaction, tenant_id, election_event_id)
            .await?;
    Ok(events.iter().any(|event| {
        matches!(
            event.event_processor,
            Some(
                sequent_core::types::scheduled_event::EventProcessors::START_ENROLLMENT_PERIOD
                    | sequent_core::types::scheduled_event::EventProcessors::END_ENROLLMENT_PERIOD
            )
        ) && annotation(event).is_some()
            && pending_change(event, now).is_some()
    }))
}

/// Applies the pending changes of the event: each row gets the instant the
/// current tz database gives for its wall time and zone. Answers how many
/// rows changed; logs them as one entry.
#[instrument(skip(hasura_transaction), err)]
pub async fn apply(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    actor: &Actor,
    now: DateTime<Utc>,
) -> Result<usize> {
    crate::postgres::scheduled_event::lock_scheduling_event(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?;
    let events =
        find_scheduled_event_by_election_event_id(hasura_transaction, tenant_id, election_event_id)
            .await?;
    let mut changes = vec![];
    for event in events.iter().filter(|event| annotation(event).is_some()) {
        // Recomputed again: the row or the tz database may have changed
        // since the check.
        match pending_change(event, now) {
            Some(change) => {
                let mut cron = event.cron_config.clone().unwrap_or_default();
                cron.scheduled_date = Some(change.scheduled_date.clone());
                update_scheduled_event(hasura_transaction, tenant_id, &event.id, cron, None, None)
                    .await?;
                changes.push(json!({
                    "scheduled_event_id": event.id,
                    "event_processor": event.event_processor,
                    "election_id": event
                        .event_payload
                        .as_ref()
                        .and_then(|payload| payload.get("election_id"))
                        .cloned(),
                    "local": change.local,
                    "timezone": change.timezone,
                    "previous": change.previous,
                    "scheduled_date": change.scheduled_date,
                }));
            }
            None => {
                set_scheduled_event_annotation(
                    hasura_transaction,
                    tenant_id,
                    &event.id,
                    ANNOTATION,
                    None,
                )
                .await?;
            }
        }
    }
    if changes.is_empty() {
        return Ok(0);
    }
    stage(
        hasura_transaction,
        &LogStep {
            kind: SigningStatementKind::ScheduleRecomputeApplied,
            user: actor.clone(),
            system: SystemOutcome::Info,
            scope: LogScope {
                tenant_id: Uuid::parse_str(tenant_id)?,
                election_event_id: Uuid::parse_str(election_event_id)?,
                election_id: None,
                area_id: None,
            },
            description: format!(
                "Applied {} scheduled date(s) recomputed with the current tz database",
                changes.len()
            ),
            details: json!({ "changes": changes }),
        },
    )
    .await?;
    scheduled_outcome::recompute_predictions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        actor,
    )
    .await?;
    Ok(changes.len())
}

#[cfg(test)]
mod schedule_recompute_tests {
    use super::*;

    fn event(local: Option<&str>, zone: Option<&str>, stored: &str) -> ScheduledEvent {
        serde_json::from_value(json!({
            "id": "row", "tenant_id": "tenant", "election_event_id": "event",
            "cron_config": {"scheduled_date": stored, "local": local, "timezone": zone},
        }))
        .unwrap()
    }

    fn now() -> DateTime<Utc> {
        parse("2028-01-01T00:00:00Z").unwrap()
    }

    #[test]
    fn a_row_whose_wall_time_now_resolves_elsewhere_has_a_pending_change() {
        // Stored when the zone was thought to be +04:00; tzdata says +04:00
        // for Dubai, so 21:00Z is a moved instant.
        let moved = event(
            Some("2028-04-09T00:00"),
            Some("Asia/Dubai"),
            "2028-04-08T21:00:00Z",
        );
        let change = pending_change(&moved, now()).expect("a change");
        assert_eq!(change.scheduled_date, "2028-04-08T20:00:00Z");
        assert_eq!(change.previous, "2028-04-08T21:00:00Z");
        assert_eq!(change.timezone, "Asia/Dubai");
        assert_eq!(change.local, "2028-04-09T00:00");
    }

    #[test]
    fn either_occurrence_of_a_repeated_wall_time_is_unchanged() {
        // 01:30 on 5 November 2028 happens twice in New York.
        for stored in ["2028-11-05T05:30:00Z", "2028-11-05T06:30:00Z"] {
            let row = event(Some("2028-11-05T01:30"), Some("America/New_York"), stored);
            assert_eq!(pending_change(&row, now()), None, "{stored}");
        }
        let moved = event(
            Some("2028-11-05T01:30"),
            Some("America/New_York"),
            "2028-11-05T07:30:00Z",
        );
        assert_eq!(
            pending_change(&moved, now()).map(|change| change.scheduled_date),
            Some("2028-11-05T05:30:00Z".to_owned())
        );
    }

    #[test]
    fn a_legacy_instant_in_the_second_occurrence_is_saved_and_kept() {
        use crate::services::election_dates::{resolve_schedule, ScheduleInput};
        let input = ScheduleInput {
            scheduled_date: Some("2028-11-05T06:30:00Z".into()),
            ..Default::default()
        };
        let (cron, _) = resolve_schedule(&input, "America/New_York", None)
            .unwrap()
            .unwrap();
        assert_eq!(cron.local.as_deref(), Some("2028-11-05T01:30"));
        assert_eq!(cron.scheduled_date.as_deref(), Some("2028-11-05T06:30:00Z"));
        assert_eq!(recomputed(&cron), None);
    }

    #[test]
    fn unchanged_past_stopped_and_legacy_rows_have_none() {
        let same = event(
            Some("2028-04-09T00:00"),
            Some("Asia/Dubai"),
            "2028-04-08T20:00:00Z",
        );
        assert_eq!(pending_change(&same, now()), None);
        let past = event(
            Some("2027-04-09T00:00"),
            Some("Asia/Dubai"),
            "2027-04-08T21:00:00Z",
        );
        assert_eq!(pending_change(&past, now()), None);
        let mut stopped = event(
            Some("2028-04-09T00:00"),
            Some("Asia/Dubai"),
            "2028-04-08T21:00:00Z",
        );
        stopped.stopped_at = Some(now());
        assert_eq!(pending_change(&stopped, now()), None);
        let legacy = event(None, None, "2028-04-08T21:00:00Z");
        assert_eq!(pending_change(&legacy, now()), None);
        let unknown = event(
            Some("2028-04-09T00:00"),
            Some("Mars/Olympus"),
            "2028-04-08T21:00:00Z",
        );
        assert_eq!(pending_change(&unknown, now()), None);
    }
}
