// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;

#[test]
fn unset_or_empty_settings_are_the_defaults() {
    for raw in [None, Some(""), Some("  ")] {
        let cadence = Cadence::parse(raw, raw);
        assert_eq!(cadence.snapshot_interval.seconds, 30);
        assert_eq!(cadence.snapshot_interval.source, SettingSource::Default);
        assert_eq!(cadence.voter_full_pass.seconds, 300);
        assert_eq!(cadence.voter_full_pass.source, SettingSource::Default);
        assert!(cadence.warnings().is_empty());
    }
    assert_eq!(Cadence::default(), Cadence::parse(None, None));
}

#[test]
fn values_within_the_bounds_are_used_as_configured() {
    let cadence = Cadence::parse(Some(" 60 "), Some("900"));
    assert_eq!(cadence.snapshot_interval.seconds, 60);
    assert_eq!(cadence.snapshot_interval.source, SettingSource::Configured);
    assert_eq!(cadence.voter_full_pass.seconds, 900);
    assert_eq!(cadence.voter_full_pass.source, SettingSource::Configured);
    assert!(cadence.warnings().is_empty());

    let edges = Cadence::parse(Some("5"), Some("5"));
    assert_eq!(edges.snapshot_interval.seconds, 5);
    assert_eq!(edges.voter_full_pass.seconds, 5);
    assert!(edges.warnings().is_empty());
    let edges = Cadence::parse(Some("3600"), Some("86400"));
    assert_eq!(edges.snapshot_interval.seconds, 3600);
    assert_eq!(edges.voter_full_pass.seconds, 86_400);
    assert!(edges.warnings().is_empty());
}

#[test]
fn a_snapshot_interval_outside_the_bounds_is_clamped_and_warned_about() {
    for (raw, used, requested) in [
        ("1", 5, 1),
        ("0", 5, 0),
        ("-10", 5, -10),
        ("3601", 3600, 3601),
    ] {
        let setting = parse_snapshot_interval(Some(raw));
        assert_eq!(setting.seconds, used, "{raw}");
        assert_eq!(
            setting.source,
            SettingSource::Clamped { requested },
            "{raw}"
        );
        let warning = setting.warning().expect("a clamped value warns");
        assert!(warning.contains(SNAPSHOT_INTERVAL_ENV), "{warning}");
        assert!(warning.contains(&used.to_string()), "{warning}");
    }
}

#[test]
fn a_value_that_is_not_whole_seconds_falls_back_to_the_default() {
    for raw in ["thirty", "30s", "1.5", "99999999999999999999999"] {
        let setting = parse_snapshot_interval(Some(raw));
        assert_eq!(setting.seconds, DEFAULT_SNAPSHOT_INTERVAL_SECONDS, "{raw}");
        assert_eq!(
            setting.source,
            SettingSource::Unparseable {
                raw: raw.to_string()
            }
        );
        let warning = setting.warning().expect("an unparseable value warns");
        assert!(warning.contains(raw), "{warning}");
    }
    let cadence = Cadence::parse(Some("30"), Some("often"));
    assert_eq!(cadence.voter_full_pass.seconds, 300);
    assert_eq!(cadence.warnings().len(), 1);
}

#[test]
fn the_full_voter_pass_is_never_more_often_than_the_snapshot_interval() {
    let cadence = Cadence::parse(Some("600"), Some("120"));
    assert_eq!(cadence.snapshot_interval.seconds, 600);
    assert_eq!(cadence.voter_full_pass.seconds, 600);
    assert_eq!(
        cadence.voter_full_pass.source,
        SettingSource::Clamped { requested: 120 }
    );
    let warnings = cadence.warnings();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains(VOTER_FULL_PASS_ENV), "{}", warnings[0]);

    let too_rare = parse_voter_full_pass(Some("100000"), 30);
    assert_eq!(too_rare.seconds, MAX_VOTER_FULL_PASS_SECONDS);
}

#[test]
fn an_unset_full_pass_follows_a_snapshot_interval_longer_than_its_default() {
    let cadence = Cadence::parse(Some("900"), None);
    assert_eq!(cadence.voter_full_pass.seconds, 900);
    assert_eq!(cadence.voter_full_pass.source, SettingSource::Default);
    assert!(cadence.warnings().is_empty());

    let unparseable = parse_voter_full_pass(Some("x"), 900);
    assert_eq!(unparseable.seconds, 900);
}

#[test]
fn the_full_pass_bound_follows_the_interval_actually_used() {
    // An interval clamped to 3600 raises the full pass to 3600, not to the
    // 7200 that was asked for.
    let cadence = Cadence::parse(Some("7200"), Some("60"));
    assert_eq!(cadence.snapshot_interval.seconds, 3600);
    assert_eq!(cadence.voter_full_pass.seconds, 3600);
    assert_eq!(cadence.warnings().len(), 2);
}
