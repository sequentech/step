// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Which timezone applies where (VOTE-LIFECYCLE). Zone names only: the tz
//! database stays out of this crate (and out of the WASM build); windmill's
//! `services::time_zones` does the zone arithmetic.
//!
//! The one rule: a row's zone is its election's `timezone` when that zone is
//! configured on the event, else the event's primary, else `UTC`.

use crate::ballot::{
    ElectionEventPresentation, ElectionPresentation, InitializationScope,
    LifecyclePolicies, LogTimeZonePolicy, UnsignedScheduledClosePolicy,
};

/// The zone used when an event configures none.
pub const DEFAULT_TIME_ZONE: &str = "UTC";

/// The event's primary zone, or [`DEFAULT_TIME_ZONE`].
pub fn primary_time_zone(event: Option<&ElectionEventPresentation>) -> String {
    event
        .and_then(|event| event.timezones.as_ref())
        .map(|zones| zones.primary.trim())
        .filter(|primary| !primary.is_empty())
        .unwrap_or(DEFAULT_TIME_ZONE)
        .to_owned()
}

/// The zone of an election (or of an event-wide row when `election` is
/// `None`): the election's own zone when it is one of the configured zones,
/// else the primary.
pub fn effective_time_zone(
    event: Option<&ElectionEventPresentation>,
    election: Option<&ElectionPresentation>,
) -> String {
    let configured = event
        .and_then(|event| event.timezones.as_ref())
        .map(|zones| zones.configured.as_slice())
        .unwrap_or(&[]);
    election
        .and_then(|election| election.timezone.as_deref())
        .map(str::trim)
        .filter(|zone| configured.iter().any(|known| known == zone))
        .map(str::to_owned)
        .unwrap_or_else(|| primary_time_zone(event))
}

/// The zone the Logs tab and log exports show for a row of `election`.
pub fn log_time_zone(
    event: Option<&ElectionEventPresentation>,
    election: Option<&ElectionPresentation>,
) -> String {
    let policy = event
        .and_then(|event| event.timezones.as_ref())
        .map(|zones| zones.logs.clone())
        .unwrap_or_default();
    match policy {
        LogTimeZonePolicy::PRIMARY => primary_time_zone(event),
        LogTimeZonePolicy::ELECTION => effective_time_zone(event, election),
    }
}

/// The event's lifecycle policies, with the defaults for anything unset.
pub fn lifecycle_policies(
    event: Option<&ElectionEventPresentation>,
) -> LifecyclePolicies {
    event
        .and_then(|event| event.lifecycle_policies.clone())
        .unwrap_or_default()
}

/// The defaults that apply before anything is published: the most
/// restrictive values.
pub fn restrictive_lifecycle_policies() -> LifecyclePolicies {
    LifecyclePolicies {
        initialization_scope: InitializationScope::POST,
        unsigned_scheduled_close: UnsignedScheduledClosePolicy::REFUSE,
    }
}

#[cfg(test)]
#[path = "time_zones_tests.rs"]
mod time_zones_tests;
