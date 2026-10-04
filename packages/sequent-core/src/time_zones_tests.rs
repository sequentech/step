// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::ballot::{ElectionEventTimeZones, LogTimeZonePolicy};

fn event(
    configured: &[&str],
    primary: &str,
    logs: LogTimeZonePolicy,
) -> ElectionEventPresentation {
    ElectionEventPresentation {
        timezones: Some(ElectionEventTimeZones {
            configured: configured
                .iter()
                .map(|zone| zone.to_string())
                .collect(),
            primary: primary.to_owned(),
            logs,
        }),
        ..Default::default()
    }
}

fn election(zone: Option<&str>) -> ElectionPresentation {
    ElectionPresentation {
        timezone: zone.map(str::to_owned),
        ..Default::default()
    }
}

#[test]
fn an_election_uses_its_configured_zone() {
    let event = event(
        &["Asia/Manila", "Asia/Dubai"],
        "Asia/Manila",
        LogTimeZonePolicy::ELECTION,
    );
    assert_eq!(
        effective_time_zone(Some(&event), Some(&election(Some("Asia/Dubai")))),
        "Asia/Dubai"
    );
}

#[test]
fn an_election_without_a_zone_or_with_an_unconfigured_one_uses_the_primary() {
    let event = event(
        &["Europe/Madrid", "Atlantic/Canary"],
        "Europe/Madrid",
        LogTimeZonePolicy::ELECTION,
    );
    assert_eq!(
        effective_time_zone(Some(&event), Some(&election(None))),
        "Europe/Madrid"
    );
    assert_eq!(
        effective_time_zone(Some(&event), Some(&election(Some("Asia/Tokyo")))),
        "Europe/Madrid"
    );
    assert_eq!(effective_time_zone(Some(&event), None), "Europe/Madrid");
}

#[test]
fn an_event_without_timezones_uses_utc() {
    assert_eq!(effective_time_zone(None, None), DEFAULT_TIME_ZONE);
    let unset = ElectionEventPresentation::default();
    assert_eq!(
        effective_time_zone(Some(&unset), Some(&election(Some("Asia/Dubai")))),
        "UTC"
    );
}

#[test]
fn logs_follow_the_event_policy() {
    let primary = event(
        &["Asia/Manila", "Asia/Dubai"],
        "Asia/Manila",
        LogTimeZonePolicy::PRIMARY,
    );
    let per_election = event(
        &["Asia/Manila", "Asia/Dubai"],
        "Asia/Manila",
        LogTimeZonePolicy::ELECTION,
    );
    let dubai = election(Some("Asia/Dubai"));
    assert_eq!(log_time_zone(Some(&primary), Some(&dubai)), "Asia/Manila");
    assert_eq!(
        log_time_zone(Some(&per_election), Some(&dubai)),
        "Asia/Dubai"
    );
}
