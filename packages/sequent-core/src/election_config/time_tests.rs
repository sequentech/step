// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a plan's times have to survive to reach the scheduler.

use super::*;
use crate::election_config::problem::Severity;

/// Los Angeles in March, after the clocks move: UTC-7.
const LA_SUMMER: i32 = -7 * 60;

fn problems(stamp: &Timestamp) -> Vec<Problem> {
    let mut found = Vec::new();
    check(stamp, "schedule.voting_opens", &mut found);
    found
}

fn errors(stamp: &Timestamp) -> Vec<Problem> {
    problems(stamp)
        .into_iter()
        .filter(|problem| problem.severity == Severity::Error)
        .collect()
}

fn says(problems: &[Problem], fragment: &str) -> bool {
    problems
        .iter()
        .any(|problem| problem.message.contains(fragment))
}

// -- the bug this module exists for ---------------------------------------

/// The whole point. `ISO8601::to_date` is `DateTime::parse_from_rfc3339`, so a
/// value it rejects is a scheduled event that never fires — with no error
/// anywhere on the path from the plan to the day nothing happens.
#[test]
fn the_scheduled_date_parses_the_way_the_platform_parses_it() {
    let stamp =
        Timestamp::new("2027-03-01T09:00", "America/Los_Angeles", LA_SUMMER);
    let written = stamp.to_rfc3339().expect("a sound time should render");

    let parsed = chrono::DateTime::parse_from_rfc3339(&written);

    assert!(
        parsed.is_ok(),
        "the platform's own parser rejected {written:?}: {:?}",
        parsed.unwrap_err()
    );
    assert_eq!(written, "2027-03-01T09:00:00-07:00");
}

/// What the wizard used to write. Kept as a test so the regression is named.
#[test]
fn a_bare_wall_clock_is_what_the_scheduler_cannot_read() {
    assert!(
        chrono::DateTime::parse_from_rfc3339("2027-03-01T09:00").is_err(),
        "if this ever parses, the reason for this module has gone away"
    );
}

#[test]
fn utc_renders_with_an_offset_too() {
    let stamp = Timestamp::utc("2027-03-01T09:00");
    assert_eq!(stamp.to_rfc3339().unwrap(), "2027-03-01T09:00:00+00:00");
}

#[test]
fn seconds_are_accepted_as_well_as_omitted() {
    let with = Timestamp::utc("2027-03-01T09:00:30");
    assert_eq!(with.to_rfc3339().unwrap(), "2027-03-01T09:00:30+00:00");
}

// -- reading a plan --------------------------------------------------------

#[test]
fn a_plan_saved_before_timezones_existed_still_opens() {
    let stamp: Timestamp =
        serde_json::from_str(r#""2027-03-01T09:00""#).unwrap();

    assert_eq!(stamp.local, "2027-03-01T09:00");
    assert_eq!(stamp.offset_minutes, 0, "a bare time meant UTC, as before");
    assert!(stamp.zone.is_empty());
}

#[test]
fn a_plan_that_names_its_zone_round_trips() {
    let stamp =
        Timestamp::new("2027-03-01T09:00", "America/Los_Angeles", LA_SUMMER);
    let text = serde_json::to_string(&stamp).unwrap();
    let read: Timestamp = serde_json::from_str(&text).unwrap();

    assert_eq!(read, stamp);
}

#[test]
fn an_object_missing_its_optional_parts_reads_as_utc() {
    let stamp: Timestamp =
        serde_json::from_str(r#"{"local": "2027-03-01T09:00"}"#).unwrap();

    assert_eq!(stamp.offset_minutes, 0);
    assert!(stamp.zone.is_empty());
}

/// A newer build's extra key is not this type's business to refuse — the plan's
/// own version check is what refuses a plan from the future.
#[test]
fn an_unknown_key_does_not_stop_a_plan_opening() {
    let stamp: Timestamp =
        serde_json::from_str(r#"{"local": "2027-03-01T09:00", "era": "ce"}"#)
            .unwrap();

    assert_eq!(stamp.local, "2027-03-01T09:00");
}

// -- ordering --------------------------------------------------------------

/// The reason comparing text is not good enough. As strings, "09:00" sorts
/// after "08:00"; as instants, Tokyo's 09:00 is seventeen hours earlier.
#[test]
fn two_times_in_different_zones_are_ordered_by_the_instant() {
    let tokyo = Timestamp::new("2027-03-01T09:00", "Asia/Tokyo", 9 * 60);
    let los_angeles =
        Timestamp::new("2027-03-01T08:00", "America/Los_Angeles", LA_SUMMER);

    assert_eq!(compare(&tokyo, &los_angeles), std::cmp::Ordering::Less);
    assert!(
        tokyo.local > los_angeles.local,
        "and the text comparison this replaces would have said the opposite"
    );
}

#[test]
fn an_unparseable_time_still_orders_rather_than_panicking() {
    let sound = Timestamp::utc("2027-03-01T09:00");
    let broken = Timestamp::utc("next Tuesday");

    let _ = compare(&sound, &broken);
}

// -- what validation says --------------------------------------------------

#[test]
fn a_sound_time_in_a_named_zone_has_nothing_to_report() {
    let stamp =
        Timestamp::new("2027-03-01T09:00", "America/Los_Angeles", LA_SUMMER);
    assert!(problems(&stamp).is_empty());
}

#[test]
fn a_blank_time_is_not_a_problem_here() {
    assert!(problems(&Timestamp::utc("")).is_empty());
}

#[test]
fn something_that_is_not_a_date_is_refused() {
    let found = errors(&Timestamp::utc("1st March"));
    assert_eq!(found.len(), 1);
    assert!(says(&found, "is not a date and time"));
    assert_eq!(found[0].path, "schedule.voting_opens");
}

#[test]
fn an_offset_no_zone_uses_is_refused() {
    let found = errors(&Timestamp::new("2027-03-01T09:00", "Nowhere", 20 * 60));
    assert!(says(&found, "not a real UTC offset"));
}

/// In range, so the bounds check passes, but no zone on earth is offset by it.
/// A plausible-looking number is exactly the kind that survives review.
#[test]
fn an_offset_that_is_not_a_multiple_of_fifteen_minutes_is_refused() {
    let found = errors(&Timestamp::new("2027-03-01T09:00", "Nowhere", -421));
    assert!(says(&found, "not a multiple of 15"));
}

/// Seconds in a minutes field is the mistake that produces these, and it lands
/// far outside the range rather than looking almost right.
#[test]
fn an_offset_given_in_seconds_is_refused_as_out_of_range() {
    let found = errors(&Timestamp::new("2027-03-01T09:00", "Nowhere", -25200));
    assert!(says(&found, "not a real UTC offset"));
}

#[test]
fn a_time_with_no_zone_named_is_a_warning_not_an_error() {
    let found = problems(&Timestamp::utc("2027-03-01T09:00"));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].severity, Severity::Warning);
    assert!(says(&found, "read as UTC"));
}

/// India is +05:30 and Nepal +05:45. Neither is an hour, and both are real.
#[test]
fn a_zone_that_is_not_a_whole_hour_is_fine() {
    let kolkata = Timestamp::new("2027-03-01T09:00", "Asia/Kolkata", 330);
    let kathmandu = Timestamp::new("2027-03-01T09:00", "Asia/Kathmandu", 345);

    assert!(problems(&kolkata).is_empty());
    assert!(problems(&kathmandu).is_empty());
    assert_eq!(kolkata.to_rfc3339().unwrap(), "2027-03-01T09:00:00+05:30");
}

#[test]
fn an_offset_too_large_to_multiply_is_reported_not_panicked() {
    // `offset_minutes * 60` is i32 arithmetic and `instant` runs before `check` does,
    // so a value above `i32::MAX / 60` used to panic in debug and wrap in release.
    let stamp = Timestamp {
        local: "2027-03-01T16:00:00".to_string(),
        zone: String::new(),
        offset_minutes: i32::MAX,
    };

    let problem = stamp
        .instant()
        .expect_err("an unusable offset is a problem");
    assert!(
        problem.message.contains("not a usable offset"),
        "unexpected message: {}",
        problem.message
    );
}
