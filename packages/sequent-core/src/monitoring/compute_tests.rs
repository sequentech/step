// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::monitoring::config::{Ratio, Sort, SortKey, SortOrder};
use crate::monitoring::payload::{
    Bucket, Counts, Cube, CubeCell, GroupRow, Notice, PostRow, ScopePayload,
    UNKNOWN_KEY,
};
use crate::monitoring::policy::parse_settings;
use crate::monitoring::problem::Code;
use crate::monitoring::resolve::ResolvedQuery;
use crate::monitoring::sources::{
    DataSourceId, Measure, PostState, QueryTemplate, TimeGrain,
};
use indexmap::IndexMap;
use serde_json::{json, Value};

const SETTINGS: &str = include_str!("fixtures/settings.yaml");

fn settings() -> Settings {
    let parsed = parse_settings(SETTINGS);
    parsed
        .value
        .unwrap_or_else(|| panic!("fixture refused:\n{}", parsed.report))
}

fn counts(pairs: &[(Measure, u64)]) -> Counts {
    pairs.iter().copied().collect()
}

fn query(template: QueryTemplate) -> ResolvedQuery {
    ResolvedQuery {
        template,
        measures: Vec::new(),
        ratio: None,
        group_by: None,
        filters: IndexMap::new(),
        grain: None,
        day: None,
        sort: None,
        limit: None,
        labels: IndexMap::new(),
    }
}

fn column(result: &QueryResult, name: &str) -> Vec<Value> {
    let at = result
        .columns
        .iter()
        .position(|column| column.name == name)
        .unwrap_or_else(|| panic!("no column {name}: {:?}", result.columns));
    result.rows.iter().map(|row| row[at].clone()).collect()
}

fn names(result: &QueryResult) -> Vec<&str> {
    result
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .collect()
}

fn run(
    source: DataSourceId,
    query: &ResolvedQuery,
    payload: &ScopePayload,
) -> QueryResult {
    evaluate(source, query, payload, Some(&settings()))
        .unwrap_or_else(|report| panic!("refused:\n{report}"))
}

use DataSourceId::*;
use Measure::*;

fn turnout_payload() -> ScopePayload {
    let cell = |sex: &str, age: &str, status: &str, reg, pre, voted| CubeCell {
        values: vec![sex.into(), age.into(), status.into()],
        counts: counts(&[
            (Registered, reg),
            (PreEnrolled, pre),
            (Voted, voted),
        ]),
    };
    ScopePayload {
        totals: counts(&[(Registered, 100), (PreEnrolled, 60), (Voted, 30)]),
        cube: Some(Cube {
            dimensions: vec!["sex".into(), "age_band".into(), "status".into()],
            cells: vec![
                cell("F", "25-39", "land", 30, 20, 10),
                cell("M", "60+", "sea", 25, 15, 5),
                cell("F", "18-24", "sea", 20, 10, 10),
                cell(UNKNOWN_KEY, UNKNOWN_KEY, "land", 25, 15, 5),
            ],
        }),
        groups: [(
            "post".to_string(),
            vec![
                GroupRow {
                    key: "e2".into(),
                    label: Some("Madrid".into()),
                    counts: counts(&[(Registered, 70), (Voted, 20)]),
                },
                GroupRow {
                    key: "e1".into(),
                    label: Some("Tokyo".into()),
                    counts: counts(&[(Registered, 50), (Voted, 0)]),
                },
                GroupRow {
                    key: UNKNOWN_KEY.into(),
                    label: None,
                    counts: counts(&[(Registered, 5), (Voted, 5)]),
                },
            ],
        )]
        .into(),
        ..ScopePayload::default()
    }
}

#[test]
fn a_ratio_is_computed_from_its_numerator_and_denominator() {
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Voted];
    summary.ratio = Some(Ratio(Voted, Registered));
    let result = run(VoterTurnout, &summary, &turnout_payload());
    assert_eq!(
        names(&result),
        ["voted", "numerator", "denominator", "pct", "pct_label"]
    );
    assert_eq!(
        result.rows,
        vec![vec![
            json!(30),
            json!(30),
            json!(100),
            json!(0.3),
            json!("30.0%")
        ]]
    );
}

#[test]
fn a_zero_denominator_shows_a_dash_not_zero() {
    let mut summary = query(QueryTemplate::Summary);
    summary.ratio = Some(Ratio(Voted, Registered));
    let payload = ScopePayload {
        totals: counts(&[(Registered, 0), (Voted, 0)]),
        ..ScopePayload::default()
    };
    let result = run(VoterTurnout, &summary, &payload);
    assert_eq!(column(&result, "pct"), [Value::Null]);
    assert_eq!(column(&result, "pct_label"), [json!("—")]);
}

#[test]
fn a_measure_that_was_not_counted_is_refused_rather_than_shown_as_zero() {
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![PreEnrolled];
    let payload = ScopePayload {
        totals: counts(&[(Registered, 5)]),
        ..ScopePayload::default()
    };
    let refused = evaluate(VoterTurnout, &summary, &payload, Some(&settings()))
        .expect_err("not counted");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::NotCounted));
}

#[test]
fn a_summary_reads_the_scope_totals_and_never_adds_up_groups() {
    // The Post rows add up to 125 registered voters because some voters are
    // in more than one Post; the scope's own count is 100.
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Registered];
    let result = run(VoterTurnout, &summary, &turnout_payload());
    assert_eq!(column(&result, "registered"), [json!(100)]);
}

#[test]
fn voter_groups_are_labelled_ordered_and_add_up_to_the_totals() {
    let mut by_sex = query(QueryTemplate::ByGroup);
    by_sex.group_by = Some("sex".into());
    by_sex.measures = vec![Registered];
    by_sex.ratio = Some(Ratio(Voted, Registered));
    let result = run(VoterTurnout, &by_sex, &turnout_payload());
    assert_eq!(
        names(&result),
        [
            "group",
            "group_key",
            "registered",
            "numerator",
            "denominator",
            "pct",
            "pct_label"
        ]
    );
    // Settings list M before F; Unknown is always last.
    assert_eq!(
        column(&result, "group"),
        [json!("Male"), json!("Female"), json!("Unknown")]
    );
    assert_eq!(
        column(&result, "group_key"),
        [json!("M"), json!("F"), json!(UNKNOWN_KEY)]
    );
    assert_eq!(
        column(&result, "registered"),
        [json!(25), json!(50), json!(25)]
    );
    let total: u64 = column(&result, "registered")
        .iter()
        .map(|value| value.as_u64().unwrap())
        .sum();
    assert_eq!(total, 100, "a partition adds up to the totals");
    assert_eq!(column(&result, "pct_label")[1], json!("40.0%"));
}

#[test]
fn age_bands_keep_their_configured_order() {
    let mut by_age = query(QueryTemplate::ByGroup);
    by_age.group_by = Some("age_band".into());
    by_age.measures = vec![Voted];
    let result = run(VoterTurnout, &by_age, &turnout_payload());
    assert_eq!(
        column(&result, "group"),
        [
            json!("18-24"),
            json!("25-39"),
            json!("60+"),
            json!("Unknown")
        ]
    );
}

#[test]
fn a_value_without_a_label_is_shown_as_it_is_after_the_labelled_ones() {
    let mut payload = turnout_payload();
    payload.cube.as_mut().unwrap().cells.push(CubeCell {
        values: vec!["X".into(), "18-24".into(), "land".into()],
        counts: counts(&[(Registered, 1), (PreEnrolled, 0), (Voted, 0)]),
    });
    let mut by_sex = query(QueryTemplate::ByGroup);
    by_sex.group_by = Some("sex".into());
    by_sex.measures = vec![Registered];
    let result = run(VoterTurnout, &by_sex, &payload);
    assert_eq!(
        column(&result, "group"),
        [json!("Male"), json!("Female"), json!("X"), json!("Unknown")]
    );
}

#[test]
fn filters_keep_only_the_listed_values_of_a_voter_dimension() {
    let mut by_sex = query(QueryTemplate::ByGroup);
    by_sex.group_by = Some("sex".into());
    by_sex.measures = vec![Registered];
    by_sex.filters = [("status".to_string(), vec!["sea".to_string()])].into();
    let result = run(VoterTurnout, &by_sex, &turnout_payload());
    assert_eq!(column(&result, "group"), [json!("Male"), json!("Female")]);
    assert_eq!(column(&result, "registered"), [json!(25), json!(20)]);

    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Voted];
    summary.filters = [("status".to_string(), vec!["land".to_string()])].into();
    let result = run(VoterTurnout, &summary, &turnout_payload());
    assert_eq!(column(&result, "voted"), [json!(15)]);
}

#[test]
fn a_filter_on_unknown_keeps_the_voters_missing_the_value() {
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Registered];
    summary.filters =
        [("sex".to_string(), vec![UNKNOWN_KEY.to_string()])].into();
    let result = run(VoterTurnout, &summary, &turnout_payload());
    assert_eq!(column(&result, "registered"), [json!(25)]);
}

#[test]
fn builtin_groups_come_from_their_own_counts() {
    let mut by_post = query(QueryTemplate::ByGroup);
    by_post.group_by = Some("post".into());
    by_post.ratio = Some(Ratio(Voted, Registered));
    let result = run(VoterTurnout, &by_post, &turnout_payload());
    assert_eq!(
        column(&result, "group"),
        [json!("Madrid"), json!("Tokyo"), json!("Unknown")]
    );
    assert_eq!(column(&result, "pct_label")[1], json!("0.0%"));
}

#[test]
fn sorting_and_limits_keep_unknown_last_and_shown() {
    let mut by_post = query(QueryTemplate::ByGroup);
    by_post.group_by = Some("post".into());
    by_post.measures = vec![Registered];
    by_post.sort = Some(Sort {
        by: SortKey::Label,
        order: SortOrder::Asc,
    });
    let result = run(VoterTurnout, &by_post, &turnout_payload());
    assert_eq!(
        column(&result, "group"),
        [json!("Madrid"), json!("Tokyo"), json!("Unknown")]
    );

    by_post.sort = Some(Sort {
        by: SortKey::Value,
        order: SortOrder::Asc,
    });
    by_post.limit = Some(1);
    let result = run(VoterTurnout, &by_post, &turnout_payload());
    assert_eq!(column(&result, "group"), [json!("Tokyo"), json!("Unknown")]);

    by_post.sort = Some(Sort {
        by: SortKey::Ratio,
        order: SortOrder::Desc,
    });
    by_post.ratio = Some(Ratio(Voted, Registered));
    by_post.limit = None;
    let result = run(VoterTurnout, &by_post, &turnout_payload());
    assert_eq!(
        column(&result, "group"),
        [json!("Madrid"), json!("Tokyo"), json!("Unknown")]
    );
}

#[test]
fn a_ratio_sort_puts_undefined_ratios_last() {
    let payload = ScopePayload {
        totals: counts(&[(Registered, 3), (Voted, 1)]),
        groups: [(
            "region".to_string(),
            vec![
                GroupRow {
                    key: "A".into(),
                    label: None,
                    counts: counts(&[(Registered, 0), (Voted, 0)]),
                },
                GroupRow {
                    key: "B".into(),
                    label: None,
                    counts: counts(&[(Registered, 3), (Voted, 1)]),
                },
            ],
        )]
        .into(),
        ..ScopePayload::default()
    };
    let mut by_region = query(QueryTemplate::ByGroup);
    by_region.group_by = Some("region".into());
    by_region.ratio = Some(Ratio(Voted, Registered));
    by_region.sort = Some(Sort {
        by: SortKey::Ratio,
        order: SortOrder::Asc,
    });
    let result = run(VoterTurnout, &by_region, &payload);
    assert_eq!(column(&result, "group"), [json!("B"), json!("A")]);
    assert_eq!(column(&result, "pct_label"), [json!("33.3%"), json!("—")]);
}

#[test]
fn a_dimension_the_payload_lacks_is_refused() {
    let mut by_group = query(QueryTemplate::ByGroup);
    by_group.group_by = Some("region".into());
    by_group.measures = vec![Voted];
    let refused = evaluate(
        VoterTurnout,
        &by_group,
        &turnout_payload(),
        Some(&settings()),
    )
    .expect_err("no region rows");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::NotCounted));
}

fn activity_payload() -> ScopePayload {
    let hour = |start: &str, voted| Bucket {
        start: start.into(),
        day: start[..10].into(),
        counts: counts(&[(Voted, voted), (Approved, 0)]),
    };
    ScopePayload {
        totals: counts(&[(Voted, 9), (Approved, 0)]),
        series: vec![
            hour("2026-05-03T22:00:00", 2),
            hour("2026-05-03T23:00:00", 3),
            hour("2026-05-04T00:00:00", 0),
            hour("2026-05-04T01:00:00", 4),
        ],
        ..ScopePayload::default()
    }
}

#[test]
fn daily_buckets_are_the_sum_of_their_hours_and_add_up_to_the_totals() {
    let mut daily = query(QueryTemplate::Timeseries);
    daily.measures = vec![Voted];
    daily.grain = Some(TimeGrain::Day);
    let result = run(VotingEnrollmentActivity, &daily, &activity_payload());
    assert_eq!(
        names(&result),
        ["bucket_start", "bucket_label", "voted", "voted_cumulative"]
    );
    assert_eq!(
        column(&result, "bucket_start"),
        [json!("2026-05-03T00:00:00"), json!("2026-05-04T00:00:00")]
    );
    assert_eq!(column(&result, "voted"), [json!(5), json!(4)]);
    assert_eq!(column(&result, "voted_cumulative"), [json!(5), json!(9)]);
}

#[test]
fn an_hourly_series_shows_one_day_and_carries_the_running_total_in() {
    let mut hourly = query(QueryTemplate::Timeseries);
    hourly.measures = vec![Voted];
    hourly.grain = Some(TimeGrain::Hour);
    hourly.day = Some("2026-05-04".into());
    let result = run(VotingEnrollmentActivity, &hourly, &activity_payload());
    assert_eq!(
        column(&result, "bucket_label"),
        [json!("00:00"), json!("01:00")]
    );
    assert_eq!(column(&result, "voted"), [json!(0), json!(4)]);
    assert_eq!(column(&result, "voted_cumulative"), [json!(5), json!(9)]);
}

#[test]
fn an_hourly_series_without_a_day_shows_every_hour() {
    let mut hourly = query(QueryTemplate::Timeseries);
    hourly.measures = vec![Voted];
    hourly.grain = Some(TimeGrain::Hour);
    let result = run(VotingEnrollmentActivity, &hourly, &activity_payload());
    assert_eq!(result.rows.len(), 4);
}

#[test]
fn the_days_on_offer_are_the_days_with_activity() {
    let mut payload = activity_payload();
    payload.series.push(Bucket {
        start: "2026-05-05T00:00:00".into(),
        day: "2026-05-05".into(),
        counts: counts(&[(Voted, 0), (Approved, 0)]),
    });
    assert_eq!(event_days(&payload), ["2026-05-03", "2026-05-04"]);
}

fn poll_payload() -> ScopePayload {
    let post =
        |id: &str, name: &str, state: PostState, opened, closed| PostRow {
            post_id: id.into(),
            post: name.into(),
            region: Some("Europe".into()),
            state: Some(state),
            counts: counts(&[(Posts, 1), (Opened, opened), (Closed, closed)]),
        };
    ScopePayload {
        totals: counts(&[(Posts, 2), (Opened, 2), (Closed, 1)]),
        posts: vec![
            post("e1", "Madrid", PostState::Closed, 1, 1),
            post("e2", "Tokyo", PostState::Opened, 1, 0),
        ],
        ..ScopePayload::default()
    }
}

#[test]
fn posts_are_listed_with_their_state() {
    let mut by_post = query(QueryTemplate::ByPost);
    by_post.labels = [("opened".to_string(), "Open".to_string())].into();
    let result = run(PollStatus, &by_post, &poll_payload());
    assert_eq!(
        names(&result),
        ["post", "post_id", "region", "state", "state_label"]
    );
    assert_eq!(column(&result, "post"), [json!("Madrid"), json!("Tokyo")]);
    assert_eq!(column(&result, "state"), [json!("closed"), json!("opened")]);
    assert_eq!(
        column(&result, "state_label"),
        [json!("Closed"), json!("Open")]
    );
}

#[test]
fn milestones_are_ratios_over_the_posts_in_scope() {
    let mut summary = query(QueryTemplate::Summary);
    summary.ratio = Some(Ratio(Closed, Posts));
    let result = run(PollStatus, &summary, &poll_payload());
    assert_eq!(column(&result, "pct_label"), [json!("50.0%")]);
    assert_eq!(column(&result, "denominator"), [json!(2)]);
}

#[test]
fn by_measure_turns_measures_into_rows_for_a_pie() {
    let mut outcomes = query(QueryTemplate::ByMeasure);
    outcomes.measures = vec![Logins, LoginFailures];
    outcomes.labels =
        [("login_failures".to_string(), "Failed".to_string())].into();
    let payload = ScopePayload {
        totals: counts(&[
            (Logins, 90),
            (LoginFailures, 10),
            (PasswordResets, 1),
        ]),
        notices: vec![Notice::UnregisteredAttemptsAtEventScopeOnly],
        ..ScopePayload::default()
    };
    let result = run(AccessSecurity, &outcomes, &payload);
    assert_eq!(names(&result), ["measure", "label", "value"]);
    assert_eq!(column(&result, "label"), [json!("Logins"), json!("Failed")]);
    assert_eq!(column(&result, "value"), [json!(90), json!(10)]);
    assert_eq!(
        result.notices,
        [Notice::UnregisteredAttemptsAtEventScopeOnly]
    );
}

#[test]
fn columns_say_what_they_hold() {
    let mut by_sex = query(QueryTemplate::ByGroup);
    by_sex.group_by = Some("sex".into());
    by_sex.ratio = Some(Ratio(Voted, Registered));
    let result = run(VoterTurnout, &by_sex, &turnout_payload());
    let kinds: Vec<ColumnKind> =
        result.columns.iter().map(|column| column.kind).collect();
    assert_eq!(
        kinds,
        [
            ColumnKind::Text,
            ColumnKind::Text,
            ColumnKind::Integer,
            ColumnKind::Integer,
            ColumnKind::Number,
            ColumnKind::Text
        ]
    );
}

#[test]
fn percent_labels_round_half_away_from_zero_to_one_decimal() {
    assert_eq!(percent_label(2, 3), "66.7%");
    assert_eq!(percent_label(1, 8), "12.5%");
    assert_eq!(percent_label(532, 1000), "53.2%");
    assert_eq!(percent_label(1, 0), "—");
    assert_eq!(percent_label(5, 4), "125.0%");
}
