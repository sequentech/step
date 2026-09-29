// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::monitoring::config::{ConfigKind, ConfigSet, Editability};
use crate::monitoring::problem::{Code, Report};
use serde::Deserialize;

const TURNOUT_BY_GROUP: &str = include_str!("fixtures/turnout_by_group.yaml");
const VOTING_ACTIVITY: &str = include_str!("fixtures/voting_activity.yaml");
const REQ_0260: &str = include_str!("fixtures/req_0260.yaml");
const THEME: &str = include_str!("fixtures/theme_default.yaml");
const SETTINGS: &str = include_str!("fixtures/settings.yaml");
const REFUSED: &str = include_str!("fixtures/refused.yaml");

/// A widget small enough that one changed line is the whole test.
const MINIMAL: &str = "
id: w
title: W
source: voter_turnout
query: {template: summary, measures: [voted]}
chart: {charts: {k: {type: kpi, query: data, value: voted}}, rows: [k]}
";

fn has(report: &Report, code: Code, path: &str) -> bool {
    report
        .problems
        .iter()
        .any(|problem| problem.code == code && problem.path == path)
}

#[track_caller]
fn assert_refused(report: &Report, code: Code, path: &str) {
    assert!(
        has(report, code, path),
        "expected {code:?} at '{path}', got:\n{report}"
    );
    assert!(!report.is_accepted());
}

#[track_caller]
fn assert_accepted(report: &Report) {
    assert!(report.is_accepted(), "expected no errors, got:\n{report}");
}

fn widget_report(yaml: &str) -> Report {
    let parsed = parse_widget(yaml);
    assert_eq!(parsed.value.is_some(), parsed.report.is_accepted());
    parsed.report
}

fn dashboard_report(yaml: &str) -> Report {
    parse_dashboard(yaml).report
}

fn settings_report(yaml: &str) -> Report {
    parse_settings(yaml).report
}

/// The spec's widget with one substitution, which must exist.
fn turnout_with(from: &str, to: &str) -> String {
    assert!(TURNOUT_BY_GROUP.contains(from), "fixture lacks '{from}'");
    TURNOUT_BY_GROUP.replacen(from, to, 1)
}

fn minimal_with(from: &str, to: &str) -> String {
    assert!(MINIMAL.contains(from), "fixture lacks '{from}'");
    MINIMAL.replacen(from, to, 1)
}

fn specification_set() -> ConfigSet {
    let mut set = ConfigSet {
        settings: parse_settings(SETTINGS).value,
        ..ConfigSet::default()
    };
    for widget in [TURNOUT_BY_GROUP, VOTING_ACTIVITY] {
        let widget = parse_widget(widget).value.expect("fixture widget");
        set.widgets.insert(widget.id.clone(), widget);
    }
    let theme = parse_theme(THEME).value.expect("fixture theme");
    set.themes.insert(theme.id.clone(), theme);
    let dashboard = parse_dashboard(REQ_0260).value.expect("fixture dashboard");
    set.dashboards.insert(dashboard.id.clone(), dashboard);
    set
}

// -- what the specification ships must be accepted -------------------------

#[test]
fn the_specification_widget_is_accepted() {
    let parsed = parse_widget(TURNOUT_BY_GROUP);
    assert_accepted(&parsed.report);
    let widget = parsed.value.expect("accepted");
    assert_eq!(widget.id, "turnout-by-group");
    assert_eq!(
        widget.selectors["breakdown"]
            .options
            .keys()
            .collect::<Vec<_>>(),
        ["sex", "age_band", "status"],
        "options keep the order they were written in"
    );
}

#[test]
fn a_widget_with_a_conditional_day_picker_is_accepted() {
    assert_accepted(&widget_report(VOTING_ACTIVITY));
}

#[test]
fn the_specification_dashboard_is_accepted() {
    let parsed = parse_dashboard(REQ_0260);
    assert_accepted(&parsed.report);
    assert_eq!(parsed.value.expect("accepted").layout.len(), 2);
}

#[test]
fn a_theme_with_category_colours_is_accepted() {
    assert_accepted(&parse_theme(THEME).report);
}

#[test]
fn settings_mapping_dimensions_to_attributes_are_accepted() {
    assert_accepted(&settings_report(SETTINGS));
}

#[test]
fn the_specification_set_is_consistent() {
    assert_accepted(&validate_set(&specification_set()));
}

// -- content that reaches outside the governed data ------------------------

#[derive(Deserialize)]
struct RefusedCase {
    name: String,
    kind: ConfigKind,
    code: Code,
    path: String,
    yaml: String,
}

#[test]
fn every_refused_fixture_is_refused_with_its_code_and_path() {
    let cases: Vec<RefusedCase> =
        serde_yaml::from_str(REFUSED).expect("refused.yaml parses");
    assert!(cases.len() >= 20, "the shared list lost its cases");
    for case in cases {
        let report = match case.kind {
            ConfigKind::Widget => parse_widget(&case.yaml).report,
            ConfigKind::Dashboard => parse_dashboard(&case.yaml).report,
            ConfigKind::Theme => parse_theme(&case.yaml).report,
            ConfigKind::Settings => parse_settings(&case.yaml).report,
        };
        assert!(
            has(&report, case.code, &case.path),
            "{}: expected {:?} at '{}', got:\n{report}",
            case.name,
            case.code,
            case.path
        );
        assert!(!report.is_accepted(), "{} was accepted", case.name);
    }
}

#[test]
fn ordinary_text_that_merely_mentions_data_is_accepted() {
    // The URL and script checks look for schemes, not for words: a title may
    // say "data", "http" or "<" in prose without being refused.
    let yaml = minimal_with(
        "title: W",
        "title: \"Turnout < 50% — data: pending; see the http log\"",
    );
    assert_accepted(&widget_report(&yaml));
}

#[test]
fn text_that_is_not_yaml_is_unreadable() {
    assert_refused(&widget_report("id: [unclosed"), Code::Unreadable, "");
}

#[test]
fn an_oversized_document_is_refused() {
    let padding = format!("# {}\n", "x".repeat(70 * 1024));
    assert_refused(
        &widget_report(&format!("{padding}{MINIMAL}")),
        Code::TooLarge,
        "",
    );
}

#[test]
fn a_deeply_nested_document_is_refused() {
    let deep = format!("{}1{}", "[".repeat(40), "]".repeat(40));
    let yaml = minimal_with("rows: [k]", &format!("rows: [k], style: {deep}"));
    assert_refused(&widget_report(&yaml), Code::TooLarge, "");
}

#[test]
fn chart_keys_outside_the_board_layout_are_refused() {
    let yaml = minimal_with("rows: [k]", "rows: [k], title: Other");
    assert_refused(&widget_report(&yaml), Code::ForbiddenKey, "chart.title");
}

#[test]
fn a_chart_reading_an_undeclared_query_is_refused() {
    let yaml = minimal_with("query: data", "query: other");
    assert_refused(
        &widget_report(&yaml),
        Code::DanglingReference,
        "chart.charts.k.query",
    );
}

#[test]
fn a_chart_may_read_any_of_several_named_queries() {
    let yaml = MINIMAL.replace(
        "query: {template: summary, measures: [voted]}",
        "queries: {totals: {template: summary, measures: [voted]}, \
         groups: {template: by_group, group_by: sex, measures: [voted]}}",
    );
    let yaml = yaml.replace("query: data", "query: totals");
    assert_accepted(&widget_report(&yaml));
}

// -- selectors -------------------------------------------------------------

#[test]
fn a_default_outside_the_options_is_refused() {
    let yaml = turnout_with("default: age_band", "default: region");
    assert_refused(
        &widget_report(&yaml),
        Code::UnknownOption,
        "selectors.breakdown.default",
    );
}

#[test]
fn listed_options_need_a_default() {
    let yaml = turnout_with(", default: age_band", "");
    assert_refused(
        &widget_report(&yaml),
        Code::InvalidValue,
        "selectors.breakdown.default",
    );
}

#[test]
fn maps_must_cover_every_option() {
    let yaml = turnout_with(", pre_reg: [pre_enrolled, registered]}", "}");
    assert_refused(
        &widget_report(&yaml),
        Code::UnknownOption,
        "selectors.measure.maps.pre_reg",
    );
}

#[test]
fn maps_may_not_name_options_that_do_not_exist() {
    let yaml = turnout_with(
        "pre_reg: [pre_enrolled, registered]}",
        "pre_reg: [pre_enrolled, registered], extra: [voted, registered]}",
    );
    assert_refused(
        &widget_report(&yaml),
        Code::UnknownOption,
        "selectors.measure.maps.extra",
    );
}

#[test]
fn a_mapped_value_must_fit_the_parameter_it_feeds() {
    let yaml =
        turnout_with("voted_reg: [voted, registered]", "voted_reg: [voted]");
    assert_refused(
        &widget_report(&yaml),
        Code::TemplateParameter,
        "query.ratio",
    );
}

#[test]
fn a_parameter_naming_an_undeclared_selector_is_refused() {
    let yaml = turnout_with("{selector: breakdown}", "{selector: missing}");
    assert_refused(
        &widget_report(&yaml),
        Code::UnknownSelector,
        "query.group_by.selector",
    );
}

#[test]
fn a_condition_may_only_name_an_earlier_selector() {
    let yaml = VOTING_ACTIVITY.replace(
        "    default: day\n",
        "    default: day\n    when: {selector: day, in: [x]}\n",
    );
    assert_refused(
        &widget_report(&yaml),
        Code::UnknownSelector,
        "selectors.grain.when.selector",
    );
}

#[test]
fn a_condition_may_only_name_listed_values() {
    let yaml = VOTING_ACTIVITY.replace("in: [hour]", "in: [minute]");
    assert_refused(
        &widget_report(&yaml),
        Code::UnknownOption,
        "selectors.day.when.in[0]",
    );
}

#[test]
fn a_selector_lists_options_or_takes_them_from_the_data_not_both() {
    let yaml = VOTING_ACTIVITY.replace(
        "options_from: event_days",
        "options_from: event_days\n    options: {x: X}",
    );
    assert_refused(
        &widget_report(&yaml),
        Code::InvalidValue,
        "selectors.day.options",
    );
}

#[test]
fn dynamic_options_only_feed_the_day() {
    let yaml = VOTING_ACTIVITY
        .replace("grain: {selector: grain}", "grain: {selector: day}");
    assert_refused(
        &widget_report(&yaml),
        Code::TemplateParameter,
        "query.grain",
    );
}

#[test]
fn selector_names_are_slugs() {
    let yaml = turnout_with("  breakdown: {", "  Break-Down: {")
        .replace("{selector: breakdown}", "{selector: Break-Down}");
    assert_refused(
        &widget_report(&yaml),
        Code::InvalidId,
        "selectors.Break-Down",
    );
}

// -- the data source decides what can be asked -----------------------------

#[test]
fn a_measure_the_source_lacks_is_refused() {
    let yaml = minimal_with("measures: [voted]", "measures: [logins]");
    assert_refused(
        &widget_report(&yaml),
        Code::UnsupportedBySource,
        "query.measures[0]",
    );
}

#[test]
fn a_template_the_source_lacks_is_refused() {
    let yaml =
        VOTING_ACTIVITY.replace("template: timeseries", "template: by_post");
    assert_refused(
        &widget_report(&yaml),
        Code::UnsupportedBySource,
        "query.template",
    );
}

#[test]
fn a_group_the_source_lacks_is_refused() {
    let yaml = minimal_with("source: voter_turnout", "source: poll_status")
        .replace(
            "{template: summary, measures: [voted]}",
            "{template: by_group, group_by: country, measures: [posts]}",
        )
        .replace("value: voted", "value: posts");
    assert_refused(
        &widget_report(&yaml),
        Code::UnsupportedBySource,
        "query.group_by",
    );
}

#[test]
fn every_option_of_a_group_by_selector_must_be_a_group_of_the_source() {
    let yaml = turnout_with(
        "status: Status abroad}",
        "status: Status abroad, reason: Reason}",
    );
    assert_refused(
        &widget_report(&yaml),
        Code::UnsupportedBySource,
        "query.group_by",
    );
}

#[test]
fn by_group_needs_a_group() {
    let yaml = turnout_with("  group_by: {selector: breakdown}\n", "");
    assert_refused(
        &widget_report(&yaml),
        Code::TemplateParameter,
        "query.group_by",
    );
}

#[test]
fn a_summary_takes_no_group() {
    let yaml =
        minimal_with("measures: [voted]", "measures: [voted], group_by: sex");
    assert_refused(
        &widget_report(&yaml),
        Code::TemplateParameter,
        "query.group_by",
    );
}

#[test]
fn a_query_needs_something_to_count() {
    let yaml = minimal_with(
        "{template: summary, measures: [voted]}",
        "{template: summary}",
    );
    assert_refused(&widget_report(&yaml), Code::TemplateParameter, "query");
}

#[test]
fn a_timeseries_needs_a_grain_and_only_a_timeseries_takes_one() {
    let yaml = VOTING_ACTIVITY.replace("  grain: {selector: grain}\n", "");
    assert_refused(
        &widget_report(&yaml),
        Code::TemplateParameter,
        "query.grain",
    );
    let yaml =
        minimal_with("measures: [voted]", "measures: [voted], grain: day");
    assert_refused(
        &widget_report(&yaml),
        Code::TemplateParameter,
        "query.grain",
    );
}

#[test]
fn a_day_only_narrows_an_hourly_series() {
    let yaml = VOTING_ACTIVITY
        .replace("grain: {selector: grain}", "grain: day")
        .replace("day: {selector: day}", "day: \"2026-05-04\"");
    assert_refused(&widget_report(&yaml), Code::TemplateParameter, "query.day");
}

#[test]
fn a_literal_day_is_a_calendar_date() {
    let yaml = VOTING_ACTIVITY
        .replace("grain: {selector: grain}", "grain: hour")
        .replace("day: {selector: day}", "day: yesterday");
    assert_refused(&widget_report(&yaml), Code::InvalidValue, "query.day");
}

#[test]
fn a_widget_has_exactly_one_way_of_naming_its_queries() {
    let both = minimal_with(
        "chart:",
        "queries: {x: {template: summary, measures: [voted]}}\nchart:",
    );
    assert_refused(&widget_report(&both), Code::TemplateParameter, "queries");
    let neither =
        minimal_with("query: {template: summary, measures: [voted]}\n", "");
    assert_refused(&widget_report(&neither), Code::TemplateParameter, "query");
}

#[test]
fn widget_ids_are_slugs() {
    let yaml = minimal_with("id: w", "id: \"W 1\"");
    assert_refused(&widget_report(&yaml), Code::InvalidId, "id");
}

// -- dashboards ------------------------------------------------------------

#[test]
fn widths_stay_on_the_twelve_column_grid() {
    for width in ["0", "13"] {
        let yaml = REQ_0260.replacen("width: 6", &format!("width: {width}"), 1);
        assert_refused(
            &dashboard_report(&yaml),
            Code::LayoutWidth,
            "layout[0].width",
        );
    }
}

#[test]
fn a_dashboard_lists_each_selector_once() {
    let yaml =
        REQ_0260.replace("[region, post, country]", "[region, post, region]");
    assert_refused(&dashboard_report(&yaml), Code::DuplicateId, "selectors[2]");
}

// -- settings --------------------------------------------------------------

#[test]
fn a_dimension_reads_from_exactly_one_place() {
    let yaml = SETTINGS.replace(
        "{voter_attribute: sex, labels",
        "{voter_attribute: sex, area_annotation: sex, labels",
    );
    assert_refused(
        &settings_report(&yaml),
        Code::InvalidValue,
        "dimensions.sex",
    );
}

#[test]
fn a_dimension_may_not_shadow_a_built_in_one() {
    let yaml = SETTINGS.replace("  status: {", "  post: {");
    assert_refused(
        &settings_report(&yaml),
        Code::DuplicateId,
        "dimensions.post",
    );
}

#[test]
fn age_bands_rise_and_only_the_last_is_open() {
    let yaml = SETTINGS
        .replace("{label: \"25-39\", to: 39}", "{label: \"25-39\", to: 20}");
    assert_refused(
        &settings_report(&yaml),
        Code::InvalidValue,
        "dimensions.age_band.age_bands[1].to",
    );
    let yaml =
        SETTINGS.replace("{label: \"25-39\", to: 39}", "{label: \"25-39\"}");
    assert_refused(
        &settings_report(&yaml),
        Code::InvalidValue,
        "dimensions.age_band.age_bands[1].to",
    );
}

#[test]
fn the_time_zone_is_an_iana_name() {
    let yaml = SETTINGS.replace("Asia/Manila", "Manila time");
    assert_refused(&settings_report(&yaml), Code::InvalidValue, "time_zone");
    for accepted in ["UTC", "Etc/GMT+8", "America/Argentina/Buenos_Aires"] {
        assert_accepted(&settings_report(
            &SETTINGS.replace("Asia/Manila", accepted),
        ));
    }
}

// -- across documents ------------------------------------------------------

#[test]
fn a_layout_must_name_a_widget_that_exists() {
    let mut set = specification_set();
    set.widgets.shift_remove("voting-activity");
    assert_refused(
        &validate_set(&set),
        Code::DanglingReference,
        "dashboards.req-0260.layout[1].widget",
    );
}

#[test]
fn per_dashboard_defaults_must_be_options_of_the_widget() {
    let mut set = specification_set();
    let values = &mut set.dashboards["req-0260"].layout[0].values;
    values.insert("measure".into(), "everyone".into());
    values.insert("colour".into(), "red".into());
    let report = validate_set(&set);
    assert_refused(
        &report,
        Code::UnknownOption,
        "dashboards.req-0260.layout[0].values.measure",
    );
    assert_refused(
        &report,
        Code::UnknownSelector,
        "dashboards.req-0260.layout[0].values.colour",
    );
}

#[test]
fn a_dashboard_theme_must_exist() {
    let mut set = specification_set();
    set.dashboards["req-0260"].theme = Some("neon".into());
    assert_refused(
        &validate_set(&set),
        Code::DanglingReference,
        "dashboards.req-0260.theme",
    );
}

#[test]
fn a_dashboard_without_a_theme_needs_the_default_one() {
    let mut set = specification_set();
    set.themes.clear();
    assert_refused(
        &validate_set(&set),
        Code::DanglingReference,
        "dashboards.req-0260.theme",
    );
}

#[test]
fn a_voter_group_must_be_a_configured_dimension() {
    let mut set = specification_set();
    if let Some(settings) = set.settings.as_mut() {
        settings.dimensions.shift_remove("status");
    }
    assert_refused(
        &validate_set(&set),
        Code::UnsupportedBySource,
        "widgets.turnout-by-group.query.group_by",
    );
}

#[test]
fn voter_groups_need_settings() {
    let mut set = specification_set();
    set.settings = None;
    assert_refused(&validate_set(&set), Code::DanglingReference, "settings");
}

#[test]
fn a_document_is_stored_under_its_own_id() {
    let mut set = specification_set();
    let widget = set
        .widgets
        .shift_remove("voting-activity")
        .expect("fixture");
    set.widgets.insert("activity".into(), widget);
    assert_refused(&validate_set(&set), Code::InvalidId, "widgets.activity.id");
}

// -- templates and parameters that compute relies on -----------------------

#[test]
fn by_measure_lists_measures_as_rows() {
    assert_accepted(&widget_report(&minimal_with(
        "{template: summary, measures: [voted]}",
        "{template: by_measure, measures: [pre_enrolled, voted], labels: {voted: Voted so far}}",
    )));
    assert_refused(
        &widget_report(&minimal_with(
            "{template: summary, measures: [voted]}",
            "{template: by_measure, ratio: [voted, registered]}",
        )),
        Code::TemplateParameter,
        "query.ratio",
    );
    assert_refused(
        &widget_report(&minimal_with(
            "{template: summary, measures: [voted]}",
            "{template: by_measure}",
        )),
        Code::TemplateParameter,
        "query.measures",
    );
}

#[test]
fn a_series_counts_first_events_and_takes_no_ratio() {
    assert_refused(
        &widget_report(&VOTING_ACTIVITY.replacen(
            "measures: [voted]",
            "measures: [voted]\n  ratio: [voted, pre_enrolled]",
            1,
        )),
        Code::TemplateParameter,
        "query.ratio",
    );
}

#[test]
fn filters_narrow_by_voter_dimensions_only() {
    let summary = "{template: summary, measures: [voted]}";
    assert_accepted(&widget_report(&minimal_with(
        summary,
        "{template: summary, measures: [voted], filters: {status: [sea]}}",
    )));
    assert_refused(
        &widget_report(&minimal_with(
            summary,
            "{template: summary, measures: [voted], filters: {region: [NCR]}}",
        )),
        Code::UnsupportedBySource,
        "query.filters.region",
    );
    assert_refused(
        &widget_report(&minimal_with(
            summary,
            "{template: by_group, group_by: post, measures: [voted], filters: {sex: [F]}}",
        )),
        Code::TemplateParameter,
        "query.filters",
    );
}

#[test]
fn series_and_post_lists_take_no_filters() {
    assert_refused(
        &widget_report(&VOTING_ACTIVITY.replacen(
            "measures: [voted]",
            "measures: [voted]\n  filters: {sex: [F]}",
            1,
        )),
        Code::TemplateParameter,
        "query.filters",
    );
    let poll = MINIMAL
        .replace("voter_turnout", "poll_status")
        .replace(
            "{template: summary, measures: [voted]}",
            "{template: by_post, filters: {sex: [F]}}",
        )
        .replace("value: voted", "value: posts");
    assert_refused(
        &widget_report(&poll),
        Code::TemplateParameter,
        "query.filters",
    );
}

#[test]
fn labels_name_measures_or_post_states_of_the_source() {
    let poll = MINIMAL
        .replace("voter_turnout", "poll_status")
        .replace(
            "{template: summary, measures: [voted]}",
            "{template: by_post, labels: {opened: Open, closed: Closed}}",
        )
        .replace("value: voted", "value: posts");
    assert_accepted(&widget_report(&poll));
    assert_refused(
        &widget_report(&minimal_with(
            "{template: summary, measures: [voted]}",
            "{template: summary, measures: [voted], labels: {logins: Logins}}",
        )),
        Code::UnsupportedBySource,
        "query.labels.logins",
    );
    assert_refused(
        &widget_report(&minimal_with(
            "{template: summary, measures: [voted]}",
            "{template: summary, measures: [voted], labels: {opened: Open}}",
        )),
        Code::UnsupportedBySource,
        "query.labels.opened",
    );
    assert_refused(
        &widget_report(&minimal_with(
            "{template: summary, measures: [voted]}",
            "{template: summary, measures: [voted], labels: {voted: ' '}}",
        )),
        Code::InvalidValue,
        "query.labels.voted",
    );
}

#[test]
fn chart_features_that_read_only_governed_data_are_accepted() {
    let widget = "
id: w
title: W
source: voter_turnout
query: {template: by_group, group_by: country, ratio: [voted, registered]}
chart:
  style:
    text: {align: left}
    padding: 8
    placeholder: {overlay: {text: No votes yet}}
    charts: {axis_x: {ticks: {visible: false, count: 6}}}
  charts:
    world:
      type: geoshape
      query: data
      geo_source: world-countries
      lookup: group_key
      value: pct
      style: {basemap: {source: world-50m}}
    trend:
      type: bar
      query: data
      x: group
      y: numerator
      support_table: [{source: denominator, label: Registered, format: integer}]
      style: {marks: {text: {font: {size: 11}}}}
  rows: [{visible: false, rows: [world]}, trend]
  grid: {columns: 2, items: [{item: world, col: 0}, {item: trend, col: 1}]}
";
    assert_accepted(&widget_report(widget));
    assert_accepted(&parse_theme("id: t\nbase: paper\n").report);
}

#[test]
fn every_dbt_charts_property_is_classified() {
    let schema: std::collections::BTreeSet<&str> =
        include_str!("fixtures/dbt_charts_properties.txt")
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
    let classified: std::collections::BTreeSet<&str> =
        dbt_keys().keys().copied().collect();
    let unclassified: Vec<_> = schema.difference(&classified).collect();
    let stale: Vec<_> = classified.difference(&schema).collect();
    assert!(
        unclassified.is_empty() && stale.is_empty(),
        "unclassified: {unclassified:?}; not in the schema: {stale:?}"
    );
}

#[test]
fn a_widget_follows_only_selectors_its_source_can_be_narrowed_by() {
    let poll = MINIMAL
        .replace("voter_turnout", "poll_status")
        .replace("[voted]", "[posts]")
        .replace("value: voted", "value: posts");
    assert_accepted(&widget_report(&poll.replace(
        "source: poll_status",
        "source: poll_status\nfollows: [region, post]",
    )));
    assert_refused(
        &widget_report(&poll.replace(
            "source: poll_status",
            "source: poll_status\nfollows: [post, country]",
        )),
        Code::UnsupportedBySource,
        "follows[1]",
    );
}

const TOP_GROUPS: &str = "
id: w
title: W
source: voter_turnout
selectors:
  top: {label: Show, options: {ten: Top 10, all: All}, default: ten, maps: {ten: 10, all: null}}
  order: {label: Order, options: {most: Most first, name: By name}, default: most, maps: {most: {by: value}, name: {by: label, order: asc}}}
  status: {label: Status, options: {all: All, sea: Sea-based}, default: all, maps: {all: null, sea: [sea]}}
query:
  template: by_group
  group_by: sex
  measures: [voted]
  sort: {selector: order}
  limit: {selector: top}
  filters: {status: {selector: status}}
chart: {charts: {k: {type: bar, query: data, x: group, y: voted}}, rows: [k]}
";

#[test]
fn sort_limit_and_filters_may_come_from_selectors_and_all_means_none() {
    assert_accepted(&widget_report(TOP_GROUPS));
    assert_refused(
        &widget_report(&TOP_GROUPS.replace(
            "maps: {ten: 10, all: null}",
            "maps: {ten: 10, all: 5000}",
        )),
        Code::InvalidValue,
        "query.limit",
    );
}

#[test]
fn sort_and_limit_only_order_lists() {
    let summary = "{template: summary, measures: [voted]}";
    assert_refused(
        &widget_report(&minimal_with(
            summary,
            "{template: summary, measures: [voted], limit: 5}",
        )),
        Code::TemplateParameter,
        "query.limit",
    );
    assert_refused(
        &widget_report(&minimal_with(
            summary,
            "{template: summary, measures: [voted], sort: {by: label}}",
        )),
        Code::TemplateParameter,
        "query.sort",
    );
    assert_refused(
        &widget_report(&minimal_with(
            summary,
            "{template: by_group, group_by: sex, measures: [voted], sort: {by: ratio}}",
        )),
        Code::TemplateParameter,
        "query.sort",
    );
}

#[test]
fn a_toggle_switches_between_two_options() {
    assert_refused(
        &widget_report(&VOTING_ACTIVITY.replacen(
            "options: {hour: Hourly, day: Daily}",
            "options: {hour: Hourly, day: Daily, week: Weekly}",
            1,
        )),
        Code::InvalidValue,
        "selectors.grain.options",
    );
}

#[test]
fn a_day_exists_on_the_calendar() {
    let report = widget_report(&VOTING_ACTIVITY.replacen(
        "day: {selector: day}",
        "day: \"2026-02-31\"",
        1,
    ));
    assert_refused(&report, Code::InvalidValue, "query.day");
    assert!(is_calendar_date("2028-02-29"));
    assert!(!is_calendar_date("2026-02-29"));
    assert!(!is_calendar_date("2026-2-01"));
}

#[test]
fn a_dashboard_cannot_pin_a_day_the_data_has_not_got() {
    let set = specification_set();
    let widget = &set.widgets["voting-activity"];
    let mut values = IndexMap::new();
    values.insert("day".to_string(), "2026-05-12".to_string());
    let mut report = Report::default();
    check_layout_values(widget, &values, "values", &mut report);
    assert_refused(&report, Code::InvalidValue, "values.day");
}

#[test]
fn a_conditional_selector_feeds_only_parameters_a_query_can_do_without() {
    let widget = "
id: w
title: W
source: voter_turnout
selectors:
  view: {label: View, options: {groups: Groups, total: Total}, default: groups}
  breakdown: {label: Breakdown, options: {sex: Sex}, default: sex, when: {selector: view, in: [groups]}}
query: {template: by_group, group_by: {selector: breakdown}, measures: [voted]}
chart: {charts: {k: {type: bar, query: data, x: group, y: voted}}, rows: [k]}
";
    assert_refused(
        &widget_report(widget),
        Code::TemplateParameter,
        "query.group_by",
    );
}

#[test]
fn a_day_picker_shows_only_while_the_series_is_hourly() {
    assert_refused(
        &widget_report(&VOTING_ACTIVITY.replacen(
            "    when: {selector: grain, in: [hour]}\n",
            "",
            1,
        )),
        Code::TemplateParameter,
        "query.day",
    );
    assert_refused(
        &widget_report(&VOTING_ACTIVITY.replacen(
            "when: {selector: grain, in: [hour]}",
            "when: {selector: grain, in: [day]}",
            1,
        )),
        Code::TemplateParameter,
        "query.day",
    );
}

#[test]
fn settings_change_counting_so_only_a_preset_sets_them() {
    assert_eq!(ConfigKind::Settings.editability(), Editability::PresetOnly);
    for kind in [ConfigKind::Widget, ConfigKind::Dashboard, ConfigKind::Theme] {
        assert_eq!(kind.editability(), Editability::Editor);
    }
}

#[test]
fn numeric_option_values_are_read_as_text() {
    let widget = "
id: w
title: W
source: voter_turnout
selectors:
  top: {label: Show, options: {5: Top 5, 10: Top 10}, default: \"10\"}
query: {template: by_group, group_by: sex, measures: [voted], limit: {selector: top}}
chart: {charts: {k: {type: bar, query: data, x: group, y: voted}}, rows: [k]}
";
    assert_accepted(&widget_report(widget));
}

#[test]
fn a_misread_parameter_says_where_inside_it() {
    let report = widget_report(&minimal_with("[voted]", "[voted, nonsense]"));
    let problem = report
        .problems
        .iter()
        .find(|problem| problem.path == "query.measures")
        .unwrap_or_else(|| panic!("no problem at query.measures:\n{report}"));
    assert!(problem.message.contains("[1]"), "{}", problem.message);
}

#[test]
fn the_key_list_is_sorted_unique_and_every_rule_is_known() {
    let lines: Vec<(&str, &str)> = key_lines().collect();
    for (name, rule) in &lines {
        assert!(
            KeyRule::parse(rule).is_some(),
            "{name}: unknown rule '{rule}'"
        );
    }
    let names: Vec<&str> = lines.iter().map(|(name, _)| *name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        names, sorted,
        "keep dbt_charts_keys.txt sorted, one line per key"
    );
}

#[test]
fn colours_in_every_notation_the_engine_reads_are_accepted() {
    let theme = "
id: t
style:
  accent: \"#1f77b4\"
  background: \"#fff\"
  muted: dbt-grays.muted
  tones: {positive: \"rgb(10, 120, 40)\", negative: \"hsl(0, 70%, 45%)\", warning: \"#f5a623cc\"}
  charts:
    color: {categorical: {palette: [\"category[1]\", \"#333333\", transparent]}}
    category_colors:
      group: {values: {Unknown: dbt-grays.muted, \"Asia Pacific\": \"category[2]\"}}
";
    assert_accepted(&parse_theme(theme).report);
}

#[test]
fn prose_with_apostrophes_digits_and_colons_is_accepted() {
    for title in [
        "Voters' turnout",
        "Top 10 Posts, 2026",
        "Data:5 votes",
        "Votes (data:2024)",
        "Curl (x) rate",
        "Turnout by day: // hourly",
    ] {
        let yaml = minimal_with("title: W", &format!("title: {title:?}"));
        assert_accepted(&widget_report(&yaml));
    }
    let bar = "
id: w
title: W
source: voter_turnout
query: {template: by_group, group_by: sex, measures: [voted]}
chart: {charts: {k: {type: bar, query: data, x: group, y: voted, subtitle: \"Each voter's first vote\", style: {marks: {bar: {labels: {format: \".1%\"}}}}}}, rows: [k]}
";
    assert_accepted(&widget_report(bar));
}

#[test]
fn a_nested_board_lays_out_the_widget_s_own_charts() {
    let widget = "
id: w
title: W
source: voter_turnout
query: {template: by_group, group_by: sex, measures: [voted]}
chart:
  charts:
    a: {type: bar, query: data, x: group, y: voted}
    b: {type: table, query: data}
  rows: [{title: Turnout, cols: [a, b], height: 300}]
";
    assert_accepted(&widget_report(widget));
}

#[test]
fn a_day_picker_shows_for_every_hourly_option_and_only_those() {
    let hours = VOTING_ACTIVITY
        .replacen(
            "options: {hour: Hourly, day: Daily}",
            "options: {hour: Hourly, day: Daily, hours: Every hour}\n    maps: {hour: hour, day: day, hours: hour}",
            1,
        )
        .replacen("    control: toggle\n", "", 1);
    assert_refused(
        &widget_report(&hours),
        Code::TemplateParameter,
        "query.day",
    );
    assert_accepted(&widget_report(&hours.replacen(
        "in: [hour]",
        "in: [hour, hours]",
        1,
    )));
}
