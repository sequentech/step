// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! How often monitoring figures are counted: one setting read by Windmill's
//! beat, its workers and Harvest, so the dashboards ask for new figures as
//! often as they are made.
//!
//! - [`SNAPSHOT_INTERVAL_ENV`]: seconds between two snapshot passes of each
//!   configured event, the freshest a dashboard can be. Harvest reports it
//!   to the Admin Portal, which polls at the same cadence.
//! - [`VOTER_FULL_PASS_ENV`]: seconds between two full reads of the event's
//!   voters from Keycloak, the expensive part of a pass. Passes in between
//!   read only the voters that changed.
//!
//! A value outside its bounds is clamped to the nearest bound, however many
//! digits it has; a value that is not a whole number of seconds is replaced
//! by the default. Either way
//! the service keeps running and [`Setting::warning`] says what was used
//! instead, for the caller to log. This module only parses: reading the
//! environment is the caller's.

use std::fmt;

/// Seconds between two snapshot passes; beat's `-m` flag is the same
/// setting.
pub const SNAPSHOT_INTERVAL_ENV: &str = "MONITORING_SNAPSHOT_INTERVAL_SECONDS";
/// Seconds between two full reads of the voters from Keycloak.
pub const VOTER_FULL_PASS_ENV: &str = "MONITORING_VOTER_FULL_PASS_SECONDS";

pub const DEFAULT_SNAPSHOT_INTERVAL_SECONDS: u64 = 30;
/// Below this a pass would barely finish before the next is due.
pub const MIN_SNAPSHOT_INTERVAL_SECONDS: u64 = 5;
/// Above this the dashboards would no longer be live.
pub const MAX_SNAPSHOT_INTERVAL_SECONDS: u64 = 3600;

pub const DEFAULT_VOTER_FULL_PASS_SECONDS: u64 = 300;
/// A full pass at least once a day, so a missed change is not kept longer.
pub const MAX_VOTER_FULL_PASS_SECONDS: u64 = 86_400;

/// Where a setting's value came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingSource {
    /// Unset or empty: the default.
    Default,
    /// As configured.
    Configured,
    /// Outside the bounds: the nearest bound is used instead. `requested`
    /// is the value as written, which may be too large for any integer.
    Clamped { requested: String },
    /// Not a whole number of seconds: the default is used instead.
    Unparseable { raw: String },
}

/// One cadence setting as it is used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setting {
    /// The environment variable it is read from.
    pub name: &'static str,
    pub seconds: u64,
    pub source: SettingSource,
}

impl Setting {
    /// What to log when the value used is not the one configured.
    pub fn warning(&self) -> Option<String> {
        match &self.source {
            SettingSource::Default | SettingSource::Configured => None,
            SettingSource::Clamped { requested } => Some(format!(
                "{} = {requested} is outside its bounds; using {} seconds",
                self.name, self.seconds
            )),
            SettingSource::Unparseable { raw } => Some(format!(
                "{} = {raw:?} is not a whole number of seconds; using {} \
                 seconds",
                self.name, self.seconds
            )),
        }
    }
}

impl fmt::Display for Setting {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} = {} s", self.name, self.seconds)
    }
}

/// Both cadence settings, as used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cadence {
    pub snapshot_interval: Setting,
    pub voter_full_pass: Setting,
}

impl Cadence {
    /// The settings from their raw values, as read from the environment.
    pub fn parse(
        snapshot_interval: Option<&str>,
        voter_full_pass: Option<&str>,
    ) -> Self {
        let snapshot_interval = parse_snapshot_interval(snapshot_interval);
        let voter_full_pass =
            parse_voter_full_pass(voter_full_pass, snapshot_interval.seconds);
        Self {
            snapshot_interval,
            voter_full_pass,
        }
    }

    /// One line per setting whose value is not the one configured.
    pub fn warnings(&self) -> Vec<String> {
        [&self.snapshot_interval, &self.voter_full_pass]
            .into_iter()
            .filter_map(Setting::warning)
            .collect()
    }
}

impl Default for Cadence {
    fn default() -> Self {
        Self::parse(None, None)
    }
}

/// Seconds between two snapshot passes, within
/// [`MIN_SNAPSHOT_INTERVAL_SECONDS`]..=[`MAX_SNAPSHOT_INTERVAL_SECONDS`].
pub fn parse_snapshot_interval(raw: Option<&str>) -> Setting {
    parse_bounded(
        SNAPSHOT_INTERVAL_ENV,
        raw,
        DEFAULT_SNAPSHOT_INTERVAL_SECONDS,
        MIN_SNAPSHOT_INTERVAL_SECONDS,
        MAX_SNAPSHOT_INTERVAL_SECONDS,
    )
}

/// Seconds between two full voter passes: never more often than a pass
/// runs, so at least `snapshot_interval_seconds`, and at most
/// [`MAX_VOTER_FULL_PASS_SECONDS`].
pub fn parse_voter_full_pass(
    raw: Option<&str>,
    snapshot_interval_seconds: u64,
) -> Setting {
    let min = snapshot_interval_seconds.min(MAX_VOTER_FULL_PASS_SECONDS);
    parse_bounded(
        VOTER_FULL_PASS_ENV,
        raw,
        DEFAULT_VOTER_FULL_PASS_SECONDS.max(min),
        min,
        MAX_VOTER_FULL_PASS_SECONDS,
    )
}

fn parse_bounded(
    name: &'static str,
    raw: Option<&str>,
    default: u64,
    min: u64,
    max: u64,
) -> Setting {
    let setting = |seconds, source| Setting {
        name,
        seconds,
        source,
    };
    let Some(trimmed) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return setting(default, SettingSource::Default);
    };
    let (negative, digits) = match trimmed.as_bytes()[0] {
        b'-' => (true, &trimmed[1..]),
        b'+' => (false, &trimmed[1..]),
        _ => (false, trimmed),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return setting(
            default,
            SettingSource::Unparseable {
                raw: trimmed.to_string(),
            },
        );
    }
    let clamped = |seconds| {
        setting(
            seconds,
            SettingSource::Clamped {
                requested: trimmed.to_string(),
            },
        )
    };
    // A negative number is below every bound, and one with too many digits
    // for a u64 above every one.
    let seconds = match (negative, digits.parse::<u64>()) {
        (false, Ok(seconds)) | (true, Ok(seconds @ 0)) => seconds,
        (true, _) => return clamped(min),
        (false, Err(_)) => return clamped(max),
    };
    if (min..=max).contains(&seconds) {
        setting(seconds, SettingSource::Configured)
    } else if seconds > max {
        clamped(max)
    } else {
        clamped(min)
    }
}

#[cfg(test)]
#[path = "cadence_tests.rs"]
mod cadence_tests;
