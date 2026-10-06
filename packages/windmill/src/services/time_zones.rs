// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Zone arithmetic with the tz database (VOTE-LIFECYCLE). Which zone applies
//! where is `sequent_core::time_zones`; this module turns a wall time in a
//! zone into the instant the scheduler runs, and back.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, NaiveDateTime, Offset, TimeZone as _, Utc};
use chrono_tz::Tz;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{
    ElectionEventPresentation, ElectionEventTimeZones, ElectionPresentation,
};
use sequent_core::time_zones::{effective_time_zone, primary_time_zone};
use sequent_core::types::date_time::TimeZone;
use serde_json::Value;
use tracing::{instrument, warn};
use uuid::Uuid;

/// Old names browsers still report (CLDR) and their tzdata canonical names.
const ALIASES: &[(&str, &str)] = &[
    ("Asia/Calcutta", "Asia/Kolkata"),
    ("Asia/Katmandu", "Asia/Kathmandu"),
    ("Asia/Saigon", "Asia/Ho_Chi_Minh"),
    ("Asia/Rangoon", "Asia/Yangon"),
    ("Asia/Dacca", "Asia/Dhaka"),
    ("Asia/Ulan_Bator", "Asia/Ulaanbaatar"),
    ("Europe/Kiev", "Europe/Kyiv"),
    ("America/Buenos_Aires", "America/Argentina/Buenos_Aires"),
    ("America/Godthab", "America/Nuuk"),
    ("Atlantic/Faeroe", "Atlantic/Faroe"),
    ("Pacific/Enderbury", "Pacific/Kanton"),
    ("Pacific/Truk", "Pacific/Chuuk"),
    ("Pacific/Ponape", "Pacific/Pohnpei"),
    ("Africa/Asmera", "Africa/Asmara"),
    ("America/Catamarca", "America/Argentina/Catamarca"),
    ("America/Cordoba", "America/Argentina/Cordoba"),
    ("America/Jujuy", "America/Argentina/Jujuy"),
    ("America/Mendoza", "America/Argentina/Mendoza"),
    ("America/Coral_Harbour", "America/Atikokan"),
    ("America/Indianapolis", "America/Indiana/Indianapolis"),
    ("America/Louisville", "America/Kentucky/Louisville"),
];

/// The tzdata canonical form of a zone name.
pub fn canonical_zone(name: &str) -> &str {
    let name = name.trim();
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map(|(_, canonical)| *canonical)
        .unwrap_or(name)
}

/// A zone by its IANA name (aliases accepted).
pub fn parse_zone(name: &str) -> Result<Tz> {
    canonical_zone(name)
        .parse::<Tz>()
        .map_err(|_| anyhow!("Unknown timezone: {name}"))
}

/// A wall time as the inputs send it: `YYYY-MM-DDTHH:MM`, seconds optional.
pub fn parse_local(local: &str) -> Result<NaiveDateTime> {
    let local = local.trim();
    NaiveDateTime::parse_from_str(local, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(local, "%Y-%m-%dT%H:%M:%S"))
        .map_err(|_| anyhow!("Invalid local date and time: {local}"))
}

/// A wall time resolved in a zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved {
    /// The wall time happens exactly once.
    Exact(DateTime<Utc>),
    /// The wall time doesn't exist (clocks go forward); it runs at the
    /// same distance after the change, as browsers show it.
    Gap { shifted: DateTime<Utc> },
    /// The wall time happens twice (clocks go back); the first one is used.
    Overlap {
        first: DateTime<Utc>,
        second: DateTime<Utc>,
    },
}

impl Resolved {
    /// The instant the scheduler uses.
    pub fn instant(&self) -> DateTime<Utc> {
        match self {
            Resolved::Exact(instant) => *instant,
            Resolved::Gap { shifted } => *shifted,
            Resolved::Overlap { first, .. } => *first,
        }
    }
}

/// The instant of `local` in `zone`.
pub fn resolve_local(local: NaiveDateTime, zone: Tz) -> Resolved {
    match zone.from_local_datetime(&local) {
        chrono::LocalResult::Single(at) => Resolved::Exact(at.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(first, second) => Resolved::Overlap {
            first: first.with_timezone(&Utc),
            second: second.with_timezone(&Utc),
        },
        chrono::LocalResult::None => {
            // Read the wall time with the offset in force just before the
            // change: 02:30 at -05:00 is 07:30Z, shown as 03:30 after it.
            let before = (1..=24)
                .map(|hours| local - Duration::hours(hours))
                .find_map(|earlier| zone.from_local_datetime(&earlier).earliest())
                .map(|at| at.offset().fix().local_minus_utc())
                .unwrap_or(0);
            Resolved::Gap {
                shifted: Utc.from_utc_datetime(&(local - Duration::seconds(before.into()))),
            }
        }
    }
}

/// The offset of `zone` at `instant`, in seconds east of UTC.
pub fn offset_seconds_at(zone: Tz, instant: DateTime<Utc>) -> i32 {
    zone.offset_from_utc_datetime(&instant.naive_utc())
        .fix()
        .local_minus_utc()
}

/// The offset of `zone` at `instant` as the EML/ACM generators take it:
/// whole hours when it is, else minutes (+05:30, +05:45).
pub fn offset_at(zone: Tz, instant: DateTime<Utc>) -> TimeZone {
    let seconds = offset_seconds_at(zone, instant);
    if seconds % 3600 == 0 {
        TimeZone::Offset(seconds / 3600)
    } else {
        TimeZone::OffsetMinutes(seconds / 60)
    }
}

/// `instant` as wall time in `zone`.
pub fn local_in(zone: Tz, instant: DateTime<Utc>) -> NaiveDateTime {
    instant.with_timezone(&zone).naive_local()
}

/// A zone the configuration names, or UTC when the tz database doesn't know
/// it (logged: saving validates zones, so this is a stored mistake).
pub fn zone_or_utc(name: &str) -> Tz {
    parse_zone(name).unwrap_or_else(|_| {
        warn!("The configured timezone {name:?} is unknown; UTC is used");
        Tz::UTC
    })
}

/// The event's primary zone: its default for event-wide schedules, reports,
/// signatures, transmission packages and monitoring.
pub fn primary_zone(event: Option<&ElectionEventPresentation>) -> Tz {
    zone_or_utc(&primary_time_zone(event))
}

/// The zone of an election (a Post): its own when configured, else the
/// event's primary.
pub fn election_zone(
    event: Option<&ElectionEventPresentation>,
    election: Option<&ElectionPresentation>,
) -> Tz {
    zone_or_utc(&effective_time_zone(event, election))
}

/// Only the event's `presentation.timezones`, so a presentation that other
/// code reads differently never stops a zone from being read. Malformed
/// timezones count as none (UTC), logged, like an unknown zone name: a
/// stored mistake mustn't stop signing, transmission or monitoring.
fn event_zones(timezones: Option<Value>) -> ElectionEventPresentation {
    let timezones = timezones
        .filter(|value| !value.is_null())
        .and_then(|value| {
            serde_json::from_value::<ElectionEventTimeZones>(value.clone())
                .map_err(|error| {
                    warn!("The event's presentation.timezones {value} is malformed ({error}); UTC is used");
                })
                .ok()
        });
    ElectionEventPresentation {
        timezones,
        ..Default::default()
    }
}

/// The primary zone of a stored election event.
#[instrument(err, skip(hasura_transaction))]
pub async fn event_time_zone(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
) -> Result<Tz> {
    let row = hasura_transaction
        .query_opt(
            "SELECT presentation->'timezones' FROM sequent_backend.election_event
             WHERE tenant_id = $1 AND id = $2",
            &[&tenant_id, &election_event_id],
        )
        .await
        .context("Failed to read the event's timezones")?
        .ok_or_else(|| anyhow!("Election event {election_event_id} not found"))?;
    Ok(primary_zone(Some(&event_zones(row.get(0)))))
}

/// The zone of a stored election (a Post), by the rule of
/// [`sequent_core::time_zones::effective_time_zone`].
#[instrument(err, skip(hasura_transaction))]
pub async fn election_time_zone(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    election_id: Uuid,
) -> Result<Tz> {
    let row = hasura_transaction
        .query_opt(
            "SELECT ev.presentation->'timezones', el.presentation->>'timezone'
             FROM sequent_backend.election el
             JOIN sequent_backend.election_event ev
               ON ev.tenant_id = el.tenant_id AND ev.id = el.election_event_id
             WHERE el.tenant_id = $1 AND el.election_event_id = $2 AND el.id = $3",
            &[&tenant_id, &election_event_id, &election_id],
        )
        .await
        .context("Failed to read the election's timezone")?
        .ok_or_else(|| anyhow!("Election {election_id} not found"))?;
    let event = event_zones(row.get(0));
    let election = ElectionPresentation {
        timezone: row.get(1),
        ..Default::default()
    };
    Ok(election_zone(Some(&event), Some(&election)))
}

#[cfg(test)]
#[path = "time_zones_tests.rs"]
mod time_zones_tests;
