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

/// A snapshot counted before `voted_pre_enrolled` was still reads, and a
/// share of the pre-enrolled is refused as not counted, never shown as
/// zero or as a share of all voters, until the next pass counts it.
#[test]
fn a_snapshot_from_before_a_measure_was_counted_refuses_only_that_measure() {
    let payload: ScopePayload = serde_json::from_value(json!({
        "totals": {"registered": 100, "pre_enrolled": 20, "voted": 30},
    }))
    .expect("an older payload still decodes");
    let mut voted_reg = query(QueryTemplate::Summary);
    voted_reg.ratio = Some(Ratio(Voted, Registered));
    assert_eq!(
        column(&run(VoterTurnout, &voted_reg, &payload), "pct"),
        [json!(0.3)]
    );
    let mut voted_pre = query(QueryTemplate::Summary);
    voted_pre.ratio = Some(Ratio(VotedPreEnrolled, PreEnrolled));
    let refused =
        evaluate(VoterTurnout, &voted_pre, &payload, Some(&settings()))
            .expect_err("not counted yet");
    assert!(!refused.problems.is_empty());
    assert!(refused
        .problems
        .iter()
        .all(|problem| problem.code == Code::NotCounted));
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
            "position",
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
            json!("40-59"),
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
        utc_offset: "+08:00".into(),
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
        [
            "bucket_start",
            "bucket_label",
            "bucket_utc",
            "voted",
            "voted_cumulative"
        ]
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
        utc_offset: "+08:00".into(),
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

/// A chart orders by a column, and a horizontal bar left unsorted orders by
/// value. Each row says where the query put it, so a chart can keep the
/// query's order: bands in order, the sort asked for, Unknown last.
#[test]
fn rows_say_where_the_query_put_them() {
    let mut by_sex = query(QueryTemplate::ByGroup);
    by_sex.group_by = Some("sex".into());
    by_sex.measures = vec![Registered];
    by_sex.sort = Some(Sort {
        by: SortKey::Value,
        order: SortOrder::Asc,
    });
    let result = run(VoterTurnout, &by_sex, &turnout_payload());
    assert_eq!(
        column(&result, "group_key"),
        [json!("M"), json!("F"), json!(UNKNOWN_KEY)]
    );
    assert_eq!(column(&result, "position"), [json!(1), json!(2), json!(3)]);

    let mut by_post = query(QueryTemplate::ByPost);
    by_post.sort = Some(Sort {
        by: SortKey::Label,
        order: SortOrder::Desc,
    });
    let result = run(PollStatus, &by_post, &poll_payload());
    assert_eq!(column(&result, "post"), [json!("Tokyo"), json!("Madrid")]);
    assert_eq!(column(&result, "position"), [json!(1), json!(2)]);

    let mut outcomes = query(QueryTemplate::ByMeasure);
    outcomes.measures = vec![LoginFailures, Logins];
    let payload = ScopePayload {
        totals: counts(&[
            (Logins, 90),
            (LoginFailures, 10),
            (PasswordResets, 1),
        ]),
        ..ScopePayload::default()
    };
    let result = run(AccessSecurity, &outcomes, &payload);
    assert_eq!(column(&result, "position"), [json!(1), json!(2)]);
}

#[test]
fn posts_are_listed_with_their_state() {
    let mut by_post = query(QueryTemplate::ByPost);
    by_post.labels = [("opened".to_string(), "Open".to_string())].into();
    let result = run(PollStatus, &by_post, &poll_payload());
    assert_eq!(
        names(&result),
        [
            "post",
            "post_id",
            "region",
            "state",
            "state_label",
            "position"
        ]
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
    assert_eq!(names(&result), ["measure", "label", "position", "value"]);
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

// -- review fixes -----------------------------------------------------------

#[test]
fn a_filter_that_matches_no_voter_counts_zero_rather_than_nothing() {
    // Nobody in this scope has the status "air".
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Voted];
    summary.ratio = Some(Ratio(Voted, Registered));
    summary.filters = [("status".to_string(), vec!["air".to_string()])].into();
    let result = run(VoterTurnout, &summary, &turnout_payload());
    assert_eq!(column(&result, "voted"), [json!(0)]);
    assert_eq!(column(&result, "pct"), [Value::Null]);
    assert_eq!(column(&result, "pct_label"), [json!("—")]);

    let mut outcomes = query(QueryTemplate::ByMeasure);
    outcomes.measures = vec![Registered, Voted];
    outcomes.filters = summary.filters.clone();
    let result = run(VoterTurnout, &outcomes, &turnout_payload());
    assert_eq!(column(&result, "value"), [json!(0), json!(0)]);
}

#[test]
fn a_filter_still_refuses_a_measure_the_producer_did_not_count() {
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![PreEnrolled];
    summary.filters = [("status".to_string(), vec!["air".to_string()])].into();
    let mut payload = turnout_payload();
    payload.totals.remove(&PreEnrolled);
    let refused = evaluate(VoterTurnout, &summary, &payload, Some(&settings()))
        .expect_err("pre_enrolled was not counted");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::NotCounted));
}

fn settings_with(extra: &str) -> Settings {
    let parsed = parse_settings(&format!("{SETTINGS}\n{extra}"));
    parsed
        .value
        .unwrap_or_else(|| panic!("refused:\n{}", parsed.report))
}

#[test]
fn the_unknown_group_is_labelled_by_the_settings() {
    let settings = settings_with("unknown_label: Desconocido");
    let mut by_sex = query(QueryTemplate::ByGroup);
    by_sex.group_by = Some("sex".into());
    by_sex.measures = vec![Registered];
    let result =
        evaluate(VoterTurnout, &by_sex, &turnout_payload(), Some(&settings))
            .expect("evaluates");
    assert_eq!(column(&result, "group").last(), Some(&json!("Desconocido")));
    assert_eq!(
        column(&result, "group_key").last(),
        Some(&json!(UNKNOWN_KEY)),
        "the key stays, so colour pins and exports do not depend on wording"
    );

    let mut by_post = query(QueryTemplate::ByGroup);
    by_post.group_by = Some("post".into());
    by_post.measures = vec![Registered];
    let result =
        evaluate(VoterTurnout, &by_post, &turnout_payload(), Some(&settings))
            .expect("evaluates");
    assert_eq!(column(&result, "group").last(), Some(&json!("Desconocido")));
}

#[test]
fn configured_values_without_voters_are_shown_as_zero() {
    // Nobody here is 40-59, but the band is still a bar: bars do not shift
    // between scopes and nobody wonders where the band went.
    let mut by_age = query(QueryTemplate::ByGroup);
    by_age.group_by = Some("age_band".into());
    by_age.measures = vec![Voted];
    by_age.ratio = Some(Ratio(Voted, Registered));
    let result = run(VoterTurnout, &by_age, &turnout_payload());
    assert_eq!(
        column(&result, "group"),
        [
            json!("18-24"),
            json!("25-39"),
            json!("40-59"),
            json!("60+"),
            json!("Unknown")
        ]
    );
    assert_eq!(column(&result, "voted")[2], json!(0));
    assert_eq!(column(&result, "pct_label")[2], json!("—"));
}

#[test]
fn a_cube_cell_of_the_wrong_shape_is_refused_not_a_crash() {
    let mut payload = turnout_payload();
    payload.cube.as_mut().unwrap().cells.push(CubeCell {
        values: vec!["F".into()],
        counts: counts(&[(Registered, 1), (PreEnrolled, 0), (Voted, 0)]),
    });
    let mut by_age = query(QueryTemplate::ByGroup);
    by_age.group_by = Some("age_band".into());
    by_age.measures = vec![Voted];
    let refused = evaluate(VoterTurnout, &by_age, &payload, Some(&settings()))
        .expect_err("malformed");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::MalformedSnapshot));

    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Voted];
    summary.filters = [("sex".to_string(), vec!["F".to_string()])].into();
    assert!(
        evaluate(VoterTurnout, &summary, &payload, Some(&settings())).is_err()
    );
}

#[test]
fn regions_and_countries_take_their_labels_from_the_settings() {
    let mut settings = settings();
    settings.scope.region.labels =
        [("NCR".to_string(), "National Capital Region".to_string())].into();
    let payload = ScopePayload {
        totals: counts(&[(Registered, 3)]),
        groups: [(
            "region".to_string(),
            vec![
                GroupRow {
                    key: "NCR".into(),
                    label: None,
                    counts: counts(&[(Registered, 2)]),
                },
                GroupRow {
                    key: "R3".into(),
                    label: None,
                    counts: counts(&[(Registered, 1)]),
                },
            ],
        )]
        .into(),
        ..ScopePayload::default()
    };
    let mut by_region = query(QueryTemplate::ByGroup);
    by_region.group_by = Some("region".into());
    by_region.measures = vec![Registered];
    let result = evaluate(VoterTurnout, &by_region, &payload, Some(&settings))
        .expect("evaluates");
    assert_eq!(
        column(&result, "group"),
        [json!("National Capital Region"), json!("R3")]
    );
}

#[test]
fn a_post_without_a_region_is_in_the_unknown_region() {
    let mut payload = poll_payload();
    payload.posts[1].region = None;
    let result = run(PollStatus, &query(QueryTemplate::ByPost), &payload);
    assert_eq!(
        column(&result, "region"),
        [json!("Europe"), json!("Unknown")]
    );
}

#[test]
fn post_lists_have_the_same_columns_whatever_the_data() {
    let empty = ScopePayload {
        totals: counts(&[(Posts, 0)]),
        ..ScopePayload::default()
    };
    let result = run(PollStatus, &query(QueryTemplate::ByPost), &empty);
    assert_eq!(
        names(&result),
        [
            "post",
            "post_id",
            "region",
            "state",
            "state_label",
            "position"
        ]
    );
    assert!(result.rows.is_empty());
}

#[test]
fn labels_sort_as_people_read_them() {
    let group = |key: &str| GroupRow {
        key: key.into(),
        label: None,
        counts: counts(&[(Registered, 1)]),
    };
    let payload = ScopePayload {
        totals: counts(&[(Registered, 3)]),
        groups: [(
            "region".to_string(),
            vec![group("bicol"), group("Zamboanga"), group("Ilocos")],
        )]
        .into(),
        ..ScopePayload::default()
    };
    let mut by_region = query(QueryTemplate::ByGroup);
    by_region.group_by = Some("region".into());
    by_region.measures = vec![Registered];
    by_region.sort = Some(Sort {
        by: SortKey::Label,
        order: SortOrder::Asc,
    });
    let result = run(VoterTurnout, &by_region, &payload);
    assert_eq!(
        column(&result, "group"),
        [json!("bicol"), json!("Ilocos"), json!("Zamboanga")]
    );
}

#[test]
fn a_template_the_source_lacks_is_refused() {
    let refused = evaluate(
        VotingEnrollmentActivity,
        &query(QueryTemplate::ByPost),
        &ScopePayload::default(),
        Some(&settings()),
    )
    .expect_err("activity has no Post list");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::UnsupportedBySource));
}

#[test]
fn a_parameter_the_template_does_not_read_is_refused_not_ignored() {
    // A filter a template ignored would show the unfiltered figure as if it
    // were filtered.
    let mut series = query(QueryTemplate::Timeseries);
    series.measures = vec![Voted];
    series.grain = Some(TimeGrain::Day);
    series.filters = [("sex".to_string(), vec!["F".to_string()])].into();
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Voted];
    summary.group_by = Some("sex".into());
    let mut limited = query(QueryTemplate::Summary);
    limited.measures = vec![Voted];
    limited.limit = Some(3);
    let mut grained = query(QueryTemplate::ByMeasure);
    grained.measures = vec![Voted];
    grained.grain = Some(TimeGrain::Hour);
    for (template_query, parameter) in [
        (series, "filters"),
        (summary, "group_by"),
        (limited, "limit"),
        (grained, "grain"),
    ] {
        let refused = evaluate(
            VoterTurnout,
            &template_query,
            &turnout_payload(),
            Some(&settings()),
        )
        .expect_err(parameter);
        assert!(
            refused.problems.iter().any(|problem| {
                problem.code == Code::TemplateParameter
                    && problem.path == parameter
            }),
            "{parameter}: {refused}"
        );
    }
}

#[test]
fn posts_can_be_counted_by_where_they_stand() {
    let mut by_state = query(QueryTemplate::ByGroup);
    by_state.group_by = Some("state".into());
    by_state.measures = vec![Posts];
    by_state.labels = [("opened".to_string(), "Open".to_string())].into();
    let result = run(PollStatus, &by_state, &poll_payload());
    // Every state, in the order Posts move through them; each Post once.
    assert_eq!(
        column(&result, "group_key"),
        [
            json!("not_initialized"),
            json!("initialized"),
            json!("opened"),
            json!("paused"),
            json!("closed")
        ]
    );
    assert_eq!(column(&result, "group")[2], json!("Open"));
    assert_eq!(
        column(&result, "posts"),
        [json!(0), json!(0), json!(1), json!(0), json!(1)]
    );
}

#[test]
fn percent_labels_never_claim_all_or_none_when_it_is_not() {
    assert_eq!(percent_label(9995, 10000), "99.9%");
    assert_eq!(percent_label(10000, 10000), "100.0%");
    assert_eq!(percent_label(1, 100000), "0.1%");
    assert_eq!(percent_label(0, 100000), "0.0%");
    assert_eq!(percent_label(1, 16), "6.3%", "half way rounds up");
}

fn madrid_autumn() -> ScopePayload {
    // 2026-10-25: clocks go back at 03:00 CEST to 02:00 CET, so 02:00 is
    // lived twice.
    let hour = |start: &str, offset: &str, voted| Bucket {
        start: start.into(),
        day: start[..10].into(),
        utc_offset: offset.into(),
        counts: counts(&[(Voted, voted), (Approved, 0)]),
    };
    ScopePayload {
        totals: counts(&[(Voted, 6), (Approved, 0)]),
        series: vec![
            hour("2026-10-25T01:00:00", "+02:00", 1),
            hour("2026-10-25T02:00:00", "+02:00", 2),
            hour("2026-10-25T02:00:00", "+01:00", 3),
            hour("2026-10-25T03:00:00", "+01:00", 0),
        ],
        ..ScopePayload::default()
    }
}

#[test]
fn the_hour_lived_twice_when_clocks_go_back_is_two_distinct_buckets() {
    let mut hourly = query(QueryTemplate::Timeseries);
    hourly.measures = vec![Voted];
    hourly.grain = Some(TimeGrain::Hour);
    hourly.day = Some("2026-10-25".into());
    let result = run(VotingEnrollmentActivity, &hourly, &madrid_autumn());
    assert_eq!(
        column(&result, "bucket_label"),
        [
            json!("01:00"),
            json!("02:00 (UTC+02:00)"),
            json!("02:00 (UTC+01:00)"),
            json!("03:00")
        ]
    );
    assert_eq!(
        column(&result, "bucket_utc"),
        [
            json!("2026-10-24T23:00:00Z"),
            json!("2026-10-25T00:00:00Z"),
            json!("2026-10-25T01:00:00Z"),
            json!("2026-10-25T02:00:00Z")
        ]
    );

    let mut daily = hourly.clone();
    daily.grain = Some(TimeGrain::Day);
    let result = run(VotingEnrollmentActivity, &daily, &madrid_autumn());
    assert_eq!(column(&result, "voted"), [json!(6)]);
    assert_eq!(
        column(&result, "bucket_utc"),
        [json!("2026-10-24T22:00:00Z")]
    );
}

#[test]
fn a_day_starts_at_its_local_midnight_even_when_its_first_hour_is_later() {
    // Votes began at 04:00 in Manila: the day still starts at 00:00+08:00.
    let hour = |start: &str, voted| Bucket {
        start: start.into(),
        day: start[..10].into(),
        utc_offset: "+08:00".into(),
        counts: counts(&[(Voted, voted), (Approved, 0)]),
    };
    let payload = ScopePayload {
        totals: counts(&[(Voted, 3), (Approved, 0)]),
        series: vec![
            hour("2026-09-30T04:00:00", 2),
            hour("2026-09-30T05:00:00", 1),
        ],
        ..ScopePayload::default()
    };
    let mut daily = query(QueryTemplate::Timeseries);
    daily.measures = vec![Voted];
    daily.grain = Some(TimeGrain::Day);
    let result = run(VotingEnrollmentActivity, &daily, &payload);
    assert_eq!(
        column(&result, "bucket_start"),
        [json!("2026-09-30T00:00:00")]
    );
    assert_eq!(
        column(&result, "bucket_utc"),
        [json!("2026-09-29T16:00:00Z")]
    );
    assert_eq!(column(&result, "voted"), [json!(3)]);
}

#[test]
fn hours_across_days_are_labelled_with_their_day() {
    let mut hourly = query(QueryTemplate::Timeseries);
    hourly.measures = vec![Voted];
    hourly.grain = Some(TimeGrain::Hour);
    let result = run(VotingEnrollmentActivity, &hourly, &activity_payload());
    assert_eq!(
        column(&result, "bucket_label"),
        [
            json!("2026-05-03 22:00"),
            json!("2026-05-03 23:00"),
            json!("2026-05-04 00:00"),
            json!("2026-05-04 01:00")
        ]
    );
}

#[test]
fn a_bucket_with_an_unreadable_offset_is_refused() {
    let mut payload = madrid_autumn();
    payload.series[0].utc_offset = "CEST".into();
    let mut hourly = query(QueryTemplate::Timeseries);
    hourly.measures = vec![Voted];
    hourly.grain = Some(TimeGrain::Hour);
    let refused = evaluate(
        VotingEnrollmentActivity,
        &hourly,
        &payload,
        Some(&settings()),
    )
    .expect_err("offset");
    assert!(refused
        .problems
        .iter()
        .any(|problem| problem.code == Code::MalformedSnapshot));
}

/// The codes and paths `query` is refused with.
fn refusal(
    source: DataSourceId,
    query: &ResolvedQuery,
    payload: &ScopePayload,
) -> Vec<(Code, String)> {
    evaluate(source, query, payload, Some(&settings()))
        .expect_err("refused")
        .problems
        .into_iter()
        .map(|problem| (problem.code, problem.path))
        .collect()
}

fn refused_as(code: Code, path: &str) -> Vec<(Code, String)> {
    vec![(code, path.to_string())]
}

#[test]
fn a_measure_the_source_lacks_is_refused() {
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Registered];
    assert_eq!(
        refusal(VotingEnrollmentActivity, &summary, &activity_payload()),
        refused_as(Code::UnsupportedBySource, "measures")
    );
    let mut share = query(QueryTemplate::Summary);
    share.ratio = Some(Ratio(Voted, Registered));
    assert_eq!(
        refusal(VotingEnrollmentActivity, &share, &activity_payload()),
        refused_as(Code::UnsupportedBySource, "measures")
    );
}

#[test]
fn an_offset_out_of_shape_or_range_is_refused() {
    let mut hourly = query(QueryTemplate::Timeseries);
    hourly.measures = vec![Voted];
    hourly.grain = Some(TimeGrain::Hour);
    let offsets = [
        "", "08:00", "+8:00", "+08:0", "+ab:00", "+08:xx", "+15:00", "+08:60",
        "+08",
    ];
    for offset in offsets {
        let mut payload = madrid_autumn();
        payload.series[1].utc_offset = offset.into();
        assert_eq!(
            refusal(VotingEnrollmentActivity, &hourly, &payload),
            refused_as(Code::MalformedSnapshot, ""),
            "{offset:?}"
        );
    }
    let mut payload = madrid_autumn();
    payload.series[1].start = "2026-10-25 02:00".into();
    assert_eq!(
        refusal(VotingEnrollmentActivity, &hourly, &payload),
        refused_as(Code::MalformedSnapshot, ""),
        "a start that is not a wall-clock time"
    );
    let mut payload = madrid_autumn();
    payload.series[0].utc_offset = "-14:59".into();
    let result = run(VotingEnrollmentActivity, &hourly, &payload);
    assert_eq!(
        column(&result, "bucket_utc")[0],
        json!("2026-10-25T15:59:00Z")
    );
}

#[test]
fn a_filter_without_voter_counts_is_refused_not_ignored() {
    let mut summary = query(QueryTemplate::Summary);
    summary.measures = vec![Voted];
    summary.filters = [("sex".to_string(), vec!["F".to_string()])].into();
    let payload = ScopePayload {
        totals: counts(&[(Voted, 30)]),
        ..ScopePayload::default()
    };
    assert_eq!(
        refusal(VoterTurnout, &summary, &payload),
        refused_as(Code::NotCounted, "")
    );
    summary.filters = [("religion".to_string(), vec!["x".to_string()])].into();
    assert_eq!(
        refusal(VoterTurnout, &summary, &turnout_payload()),
        refused_as(Code::NotCounted, "")
    );
}

#[test]
fn groups_need_a_dimension_and_a_series_needs_a_grain() {
    let mut by_group = query(QueryTemplate::ByGroup);
    by_group.measures = vec![Voted];
    assert_eq!(
        refusal(VoterTurnout, &by_group, &turnout_payload()),
        refused_as(Code::TemplateParameter, "group_by")
    );
    let mut series = query(QueryTemplate::Timeseries);
    series.measures = vec![Voted];
    assert_eq!(
        refusal(VotingEnrollmentActivity, &series, &activity_payload()),
        refused_as(Code::TemplateParameter, "grain")
    );
}

/// Groups by Post are counted Post by Post, not voter by voter: no voter
/// dimension can narrow them.
#[test]
fn builtin_groups_cannot_be_filtered_by_voter_dimensions() {
    let mut by_post = query(QueryTemplate::ByGroup);
    by_post.group_by = Some("post".into());
    by_post.measures = vec![Voted];
    by_post.filters = [("sex".to_string(), vec!["F".to_string()])].into();
    assert_eq!(
        refusal(VoterTurnout, &by_post, &turnout_payload()),
        refused_as(Code::TemplateParameter, "filters")
    );
}

#[test]
fn a_voter_dimension_the_cube_lacks_is_refused() {
    let mut by_religion = query(QueryTemplate::ByGroup);
    by_religion.group_by = Some("religion".into());
    by_religion.measures = vec![Voted];
    assert_eq!(
        refusal(VoterTurnout, &by_religion, &turnout_payload()),
        refused_as(Code::NotCounted, "")
    );
}

#[test]
fn a_source_that_does_not_count_posts_has_no_states() {
    let mut by_state = query(QueryTemplate::ByGroup);
    by_state.group_by = Some("state".into());
    by_state.measures = vec![Voted];
    assert_eq!(
        refusal(VoterTurnout, &by_state, &turnout_payload()),
        refused_as(Code::NotCounted, "")
    );
}

/// A Post without a state, or in a state this source does not have, is in
/// the Unknown group, last, and never dropped.
#[test]
fn a_post_in_no_known_state_is_in_the_unknown_state() {
    let mut payload = poll_payload();
    payload.posts[0].state = None;
    payload.posts[1].state = Some(PostState::Tested);
    let mut by_state = query(QueryTemplate::ByGroup);
    by_state.group_by = Some("state".into());
    by_state.measures = vec![Posts];
    let result = run(PollStatus, &by_state, &payload);
    assert_eq!(column(&result, "group_key")[5], json!(UNKNOWN_KEY));
    assert_eq!(column(&result, "group")[5], json!("Unknown"));
    assert_eq!(
        column(&result, "posts"),
        [json!(0), json!(0), json!(0), json!(0), json!(0), json!(2)]
    );
}

#[test]
fn a_listed_post_without_a_state_has_no_state_cells() {
    let mut payload = poll_payload();
    payload.posts[1].state = None;
    let result = run(PollStatus, &query(QueryTemplate::ByPost), &payload);
    assert_eq!(column(&result, "state"), [json!("closed"), Value::Null]);
    assert_eq!(
        column(&result, "state_label"),
        [json!("Closed"), Value::Null]
    );
}

#[test]
fn undefined_ratios_sort_last_in_either_order() {
    let group = |key: &str, registered, voted| GroupRow {
        key: key.into(),
        label: None,
        counts: counts(&[(Registered, registered), (Voted, voted)]),
    };
    let payload = ScopePayload {
        totals: counts(&[(Registered, 5), (Voted, 2)]),
        groups: [(
            "region".to_string(),
            vec![
                group("B", 4, 1),
                group("A", 0, 0),
                group("D", 1, 1),
                group("C", 0, 0),
            ],
        )]
        .into(),
        ..ScopePayload::default()
    };
    let mut by_region = query(QueryTemplate::ByGroup);
    by_region.group_by = Some("region".into());
    by_region.ratio = Some(Ratio(Voted, Registered));
    for (order, expected) in [
        (SortOrder::Asc, ["B", "D", "A", "C"]),
        (SortOrder::Desc, ["D", "B", "A", "C"]),
    ] {
        by_region.sort = Some(Sort {
            by: SortKey::Ratio,
            order,
        });
        let result = run(VoterTurnout, &by_region, &payload);
        assert_eq!(column(&result, "group"), expected.map(|key| json!(key)));
    }
}

#[test]
fn a_filter_on_a_dimension_the_cube_lacks_refuses_its_groups_and_measures() {
    let mut by_sex = query(QueryTemplate::ByGroup);
    by_sex.group_by = Some("sex".into());
    by_sex.measures = vec![Voted];
    by_sex.filters = [("religion".to_string(), vec!["x".to_string()])].into();
    assert_eq!(
        refusal(VoterTurnout, &by_sex, &turnout_payload()),
        refused_as(Code::NotCounted, "")
    );
    let mut outcomes = query(QueryTemplate::ByMeasure);
    outcomes.measures = vec![Voted];
    outcomes.filters = by_sex.filters.clone();
    assert_eq!(
        refusal(VoterTurnout, &outcomes, &turnout_payload()),
        refused_as(Code::NotCounted, "")
    );
}

#[test]
fn an_hour_missing_a_measure_is_refused_rather_than_shown_as_zero() {
    let mut payload = activity_payload();
    payload.series[2].counts.remove(&Voted);
    let mut hourly = query(QueryTemplate::Timeseries);
    hourly.measures = vec![Voted];
    hourly.grain = Some(TimeGrain::Hour);
    assert_eq!(
        refusal(VotingEnrollmentActivity, &hourly, &payload),
        refused_as(Code::NotCounted, "")
    );
}

/// A value sort reads the first measure, or else the ratio's numerator; a
/// sort with nothing to read keeps the rows in their order.
#[test]
fn a_sort_with_nothing_to_read_keeps_the_rows_in_order() {
    let payload = ScopePayload {
        totals: counts(&[(Registered, 3), (Voted, 3)]),
        groups: [(
            "region".to_string(),
            vec![
                GroupRow {
                    key: "A".into(),
                    label: None,
                    counts: counts(&[(Registered, 1), (Voted, 1)]),
                },
                GroupRow {
                    key: "B".into(),
                    label: None,
                    counts: counts(&[(Registered, 2), (Voted, 2)]),
                },
            ],
        )]
        .into(),
        ..ScopePayload::default()
    };
    let mut by_region = query(QueryTemplate::ByGroup);
    by_region.group_by = Some("region".into());
    by_region.sort = Some(Sort {
        by: SortKey::Value,
        order: SortOrder::Desc,
    });
    let groups = |query: &ResolvedQuery| {
        column(&run(VoterTurnout, query, &payload), "group")
    };
    assert_eq!(groups(&by_region), [json!("A"), json!("B")]);
    by_region.sort = Some(Sort {
        by: SortKey::Ratio,
        order: SortOrder::Desc,
    });
    assert_eq!(groups(&by_region), [json!("A"), json!("B")]);
    by_region.ratio = Some(Ratio(Voted, Registered));
    by_region.sort = Some(Sort {
        by: SortKey::Value,
        order: SortOrder::Desc,
    });
    assert_eq!(groups(&by_region), [json!("B"), json!("A")]);
}

/// Labels equal but for case keep one order: capitals first.
#[test]
fn labels_equal_but_for_case_sort_capitals_first() {
    let group = |key: &str| GroupRow {
        key: key.into(),
        label: None,
        counts: counts(&[(Registered, 1)]),
    };
    let payload = ScopePayload {
        totals: counts(&[(Registered, 3)]),
        groups: [(
            "region".to_string(),
            vec![group("bicol"), group("Bicol"), group("abra")],
        )]
        .into(),
        ..ScopePayload::default()
    };
    let mut by_region = query(QueryTemplate::ByGroup);
    by_region.group_by = Some("region".into());
    by_region.measures = vec![Registered];
    by_region.sort = Some(Sort {
        by: SortKey::Label,
        order: SortOrder::Asc,
    });
    let result = run(VoterTurnout, &by_region, &payload);
    assert_eq!(
        column(&result, "group"),
        [json!("abra"), json!("Bicol"), json!("bicol")]
    );
}

#[test]
fn a_ratio_sort_over_a_group_missing_a_measure_is_refused() {
    let mut payload = turnout_payload();
    let posts = payload.groups.get_mut("post").unwrap();
    posts[0].counts.remove(&Registered);
    let mut by_post = query(QueryTemplate::ByGroup);
    by_post.group_by = Some("post".into());
    by_post.ratio = Some(Ratio(Voted, Registered));
    by_post.sort = Some(Sort {
        by: SortKey::Ratio,
        order: SortOrder::Desc,
    });
    let refused = refusal(VoterTurnout, &by_post, &payload);
    assert!(!refused.is_empty());
    assert!(refused.iter().all(|(code, _)| *code == Code::NotCounted));
    payload.groups.get_mut("post").unwrap()[1]
        .counts
        .remove(&Voted);
    assert!(refusal(VoterTurnout, &by_post, &payload)
        .iter()
        .all(|(code, _)| *code == Code::NotCounted));
}
