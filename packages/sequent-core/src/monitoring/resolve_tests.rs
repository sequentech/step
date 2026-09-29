// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::monitoring::config::{Ratio, Widget};
use crate::monitoring::policy::parse_widget;
use crate::monitoring::problem::Code;
use crate::monitoring::sources::{Measure, TimeGrain};
use indexmap::IndexMap;

const TURNOUT_BY_GROUP: &str = include_str!("fixtures/turnout_by_group.yaml");
const VOTING_ACTIVITY: &str = include_str!("fixtures/voting_activity.yaml");

fn widget(yaml: &str) -> Widget {
    let parsed = parse_widget(yaml);
    parsed
        .value
        .unwrap_or_else(|| panic!("fixture refused:\n{}", parsed.report))
}

fn values(pairs: &[(&str, &str)]) -> IndexMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn days(list: &[&str]) -> DynamicOptionValues {
    DynamicOptionValues {
        event_days: list.iter().map(|day| day.to_string()).collect(),
    }
}

fn resolved(
    widget: &Widget,
    dashboard: &[(&str, &str)],
    requested: &[(&str, &str)],
    dynamic: &DynamicOptionValues,
) -> ResolvedWidget {
    resolve_widget(widget, &values(dashboard), &values(requested), dynamic)
        .unwrap_or_else(|report| panic!("refused:\n{report}"))
}

#[test]
fn selector_defaults_feed_the_query() {
    let turnout = widget(TURNOUT_BY_GROUP);
    let result = resolved(&turnout, &[], &[], &days(&[]));
    let query = &result.queries["data"];
    assert_eq!(query.group_by.as_deref(), Some("age_band"));
    assert_eq!(
        query.ratio,
        Some(Ratio(Measure::Voted, Measure::Registered))
    );
    assert_eq!(
        result.selectors["measure"],
        SelectorState::Value("voted_reg".into())
    );
}

#[test]
fn a_dashboard_default_wins_over_the_widget_default() {
    let turnout = widget(TURNOUT_BY_GROUP);
    let result =
        resolved(&turnout, &[("measure", "voted_pre")], &[], &days(&[]));
    assert_eq!(
        result.queries["data"].ratio,
        Some(Ratio(Measure::Voted, Measure::PreEnrolled))
    );
}

#[test]
fn what_the_viewer_picked_wins_over_both() {
    let turnout = widget(TURNOUT_BY_GROUP);
    let result = resolved(
        &turnout,
        &[("measure", "voted_pre")],
        &[("measure", "pre_reg"), ("breakdown", "sex")],
        &days(&[]),
    );
    let query = &result.queries["data"];
    assert_eq!(
        query.ratio,
        Some(Ratio(Measure::PreEnrolled, Measure::Registered))
    );
    assert_eq!(query.group_by.as_deref(), Some("sex"));
}

#[test]
fn a_value_that_is_not_an_option_is_refused() {
    let turnout = widget(TURNOUT_BY_GROUP);
    let refused = resolve_widget(
        &turnout,
        &values(&[]),
        &values(&[("breakdown", "income")]),
        &days(&[]),
    )
    .expect_err("not an option");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::UnknownOption
            && problem.path == "breakdown"));
}

#[test]
fn a_selector_the_widget_does_not_have_is_refused() {
    let turnout = widget(TURNOUT_BY_GROUP);
    let refused = resolve_widget(
        &turnout,
        &values(&[]),
        &values(&[("colour", "red")]),
        &days(&[]),
    )
    .expect_err("no such selector");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::UnknownSelector
            && problem.path == "colour"));
}

#[test]
fn a_hidden_selector_takes_no_value_and_feeds_nothing() {
    let activity = widget(VOTING_ACTIVITY);
    let result = resolved(
        &activity,
        &[],
        &[("grain", "day"), ("day", "2026-05-04")],
        &days(&["2026-05-04"]),
    );
    assert_eq!(result.selectors["day"], SelectorState::Hidden);
    let query = &result.queries["data"];
    assert_eq!(query.grain, Some(TimeGrain::Day));
    assert_eq!(query.day, None);
}

#[test]
fn an_hourly_series_defaults_to_the_latest_day() {
    let activity = widget(VOTING_ACTIVITY);
    let result = resolved(
        &activity,
        &[],
        &[("grain", "hour")],
        &days(&["2026-05-03", "2026-05-04"]),
    );
    assert_eq!(
        result.selectors["day"],
        SelectorState::Value("2026-05-04".into())
    );
    let query = &result.queries["data"];
    assert_eq!(query.grain, Some(TimeGrain::Hour));
    assert_eq!(query.day.as_deref(), Some("2026-05-04"));
}

#[test]
fn a_day_can_be_picked_but_only_one_with_data() {
    let activity = widget(VOTING_ACTIVITY);
    let dynamic = days(&["2026-05-03", "2026-05-04"]);
    let result = resolved(
        &activity,
        &[],
        &[("grain", "hour"), ("day", "2026-05-03")],
        &dynamic,
    );
    assert_eq!(result.queries["data"].day.as_deref(), Some("2026-05-03"));

    let refused = resolve_widget(
        &activity,
        &values(&[]),
        &values(&[("grain", "hour"), ("day", "2026-05-09")]),
        &dynamic,
    )
    .expect_err("no data that day");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::UnknownOption));
}

#[test]
fn a_day_picker_with_no_days_yet_is_empty() {
    let activity = widget(VOTING_ACTIVITY);
    let result = resolved(&activity, &[], &[("grain", "hour")], &days(&[]));
    assert_eq!(result.selectors["day"], SelectorState::NoOptions);
    assert_eq!(result.queries["data"].day, None);
}
