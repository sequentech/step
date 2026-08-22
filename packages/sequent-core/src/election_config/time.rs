// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! A moment a plan names, and the instant the platform acts on.
//!
//! These are not the same thing, and treating them as the same is how a wizard
//! builds an election whose voting period never opens.
//!
//! The scheduler reads `cron_config.scheduled_date` through
//! [`crate::services::date::ISO8601::to_date`], which is
//! `DateTime::parse_from_rfc3339` — and RFC 3339 **requires an offset**. A plan
//! that says `2027-03-01T09:00` produces a date that does not parse, so
//! `get_datetime` returns `None`, the poller drops the event, and nothing
//! happens on the day. No error is raised anywhere along that path.
//!
//! So a plan carries three things rather than one string:
//!
//! - `local`, the wall clock somebody typed, kept verbatim so a plan reopened
//!   next month reads back as its author wrote it rather than converted into
//!   wherever the reader happens to be;
//! - `zone`, the IANA name, carried for people and never computed on;
//! - `offset_minutes`, which is what turns the first into an instant.
//!
//! **There is no timezone database here, on purpose.** This module compiles to
//! wasm32, and `chrono-tz` is about a megabyte of tables to answer a question
//! the browser can already answer for free — `new Date(local).getTimezoneOffset()`
//! gives the right offset for that date, daylight saving included. Whoever picks
//! the time resolves the offset; this side records it, checks it, and computes
//! with it.

use std::cmp::Ordering;
use std::fmt;

use chrono::{DateTime, FixedOffset, NaiveDateTime};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use super::problem::{Code, Problem};

/// Minutes east of UTC. Real zones run from -12:00 to +14:00.
const MIN_OFFSET: i32 = -12 * 60;
const MAX_OFFSET: i32 = 14 * 60;

/// A wall-clock time, the zone it was written in, and the offset that turns it
/// into an instant.
///
/// See the module docs for why all three are kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Timestamp {
    /// `YYYY-MM-DDTHH:MM`, no offset — what a `datetime-local` input produces.
    pub local: String,

    /// IANA name, `America/Los_Angeles`. Empty when a plan predates zones.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,

    /// Minutes east of UTC **at `local`**, resolved by whoever chose the time.
    #[serde(default)]
    pub offset_minutes: i32,
}

impl Timestamp {
    /// A time in UTC, which is what a plan written before zones existed meant.
    pub fn utc(local: impl Into<String>) -> Self {
        Timestamp {
            local: local.into(),
            zone: String::new(),
            offset_minutes: 0,
        }
    }

    /// A time in a named zone at a known offset.
    pub fn new(
        local: impl Into<String>,
        zone: impl Into<String>,
        offset_minutes: i32,
    ) -> Self {
        Timestamp {
            local: local.into(),
            zone: zone.into(),
            offset_minutes,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.local.trim().is_empty()
    }

    /// The instant this names.
    ///
    /// Everything that compares two times goes through here. Comparing the
    /// `local` strings instead is wrong the moment two of them sit in different
    /// zones: 09:00 in Tokyo is before 08:00 in Los Angeles, and text says
    /// otherwise.
    pub fn instant(&self) -> Result<DateTime<FixedOffset>, Problem> {
        let naive = self.naive()?;
        // `checked_mul` because these fields come out of a saved plan, which is
        // a document people hand-edit. `i32::MAX * 60` panics in a debug build
        // and — worse — wraps to a plausible-looking -00:01 in a release one.
        let offset = self
            .offset_minutes
            .checked_mul(60)
            .and_then(FixedOffset::east_opt)
            .ok_or_else(|| {
                self.problem(format!(
                    "{} is not a usable offset",
                    self.offset_minutes
                ))
            })?;
        // `single()` rather than `earliest()`: a fixed offset has exactly one
        // mapping, so there is no ambiguity to resolve. An earlier version
        // guarded here against a spring-forward gap, which reads sensibly and
        // is unreachable — detecting that needs the zone's rules, and this
        // module deliberately has no timezone database.
        Ok(naive
            .and_local_timezone(offset)
            .single()
            .unwrap_or_else(|| {
                unreachable!(
                    "a fixed offset maps every wall clock exactly once"
                )
            }))
    }

    /// The shape the platform's scheduler parses.
    ///
    /// `DateTime::parse_from_rfc3339` is the only reader of this value, so
    /// anything it rejects is an event that silently never fires.
    pub fn to_rfc3339(&self) -> Result<String, Problem> {
        Ok(self.instant()?.to_rfc3339())
    }

    /// `local` as a date and time, accepting it with or without seconds.
    fn naive(&self) -> Result<NaiveDateTime, Problem> {
        let text = self.local.trim();
        NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S")
            .or_else(|_| NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M"))
            .map_err(|_| {
                self.problem(format!(
                    "'{text}' is not a date and time. Expected YYYY-MM-DDTHH:MM."
                ))
            })
    }

    fn zone_or_utc(&self) -> &str {
        if self.zone.trim().is_empty() {
            "UTC"
        } else {
            &self.zone
        }
    }

    fn problem(&self, message: String) -> Problem {
        Problem::error(Code::InvalidValue, "schedule", message)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} ({})", self.local, self.zone_or_utc())
    }
}

/// Order by the instant named, falling back to text when one will not parse.
///
/// A total order is needed because validation sorts and compares, and refusing
/// to order an unparseable value would hide the very problem being reported.
pub fn compare(left: &Timestamp, right: &Timestamp) -> Ordering {
    match (left.instant(), right.instant()) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.local.cmp(&right.local),
    }
}

/// Accept either the object or a bare string.
///
/// A plan saved before zones existed said `"2027-03-01T09:00"`, and both this
/// implementation and the TypeScript one treated that as UTC. Reading it as UTC
/// keeps those plans compiling to what they always compiled to; validation then
/// says out loud that no zone was given, because a schedule handed to a client
/// with no zone on it is how two people arrive an hour apart.
impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Either;

        impl<'de> Visitor<'de> for Either {
            type Value = Timestamp;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a date and time, or {local, zone, offset_minutes}")
            }

            fn visit_str<E: de::Error>(
                self,
                value: &str,
            ) -> Result<Timestamp, E> {
                Ok(Timestamp::utc(value))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Timestamp, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut local = None;
                let mut zone = None;
                let mut offset_minutes = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "local" => local = Some(map.next_value::<String>()?),
                        "zone" => zone = Some(map.next_value::<String>()?),
                        "offset_minutes" => {
                            offset_minutes = Some(map.next_value::<i32>()?)
                        }
                        // Ignored rather than refused: a plan from a newer build
                        // is caught by its version, not by one unknown key here.
                        _ => {
                            let _ =
                                map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                Ok(Timestamp {
                    local: local
                        .ok_or_else(|| de::Error::missing_field("local"))?,
                    zone: zone.unwrap_or_default(),
                    offset_minutes: offset_minutes.unwrap_or(0),
                })
            }
        }

        deserializer.deserialize_any(Either)
    }
}

/// Everything wrong with one timestamp, in the plan's own vocabulary.
///
/// `at` is the path the wizard routes the message by, so it names the field
/// somebody can actually go and fix.
pub fn check(stamp: &Timestamp, at: &str, problems: &mut Vec<Problem>) {
    if stamp.is_empty() {
        return;
    }

    if let Err(problem) = stamp.naive() {
        problems.push(Problem::error(Code::InvalidValue, at, problem.message));
        return;
    }

    if stamp.offset_minutes < MIN_OFFSET || stamp.offset_minutes > MAX_OFFSET {
        problems.push(Problem::error(
            Code::InvalidValue,
            at,
            format!(
                "{} minutes is not a real UTC offset; they run from {MIN_OFFSET} to {MAX_OFFSET}",
                stamp.offset_minutes
            ),
        ));
    } else if stamp.offset_minutes % 15 != 0 {
        // Every zone in use is a whole number of quarter-hours. A value that is
        // not is in range and looks almost right, which is exactly why it needs
        // saying — an offset out by minutes survives a reading that an offset
        // out by hours would not.
        problems.push(Problem::error(
            Code::InvalidValue,
            at,
            format!(
                "an offset of {} minutes is not a multiple of 15, and every real \
                 timezone is",
                stamp.offset_minutes
            ),
        ));
    }

    if stamp.zone.trim().is_empty() {
        problems.push(Problem::warning(
            Code::MissingField,
            at,
            "no timezone was named, so this is read as UTC. A schedule handed to \
             a client without a zone on it is how two people arrive an hour apart.",
        ));
    }
}

#[cfg(test)]
#[path = "time_tests.rs"]
mod time_tests;
