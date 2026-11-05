// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::monitoring::config::{
    ConfigKind, ConfigSet, Editability, ScopeSelector,
};
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

// -- sections: where a dashboard sits in the switcher -----------------------

#[test]
fn a_dashboard_may_name_the_section_it_is_listed_under() {
    let yaml =
        REQ_0260.replacen("\ntitle:", "\nsection: Voter turnout\ntitle:", 1);
    let parsed = parse_dashboard(&yaml);
    assert_accepted(&parsed.report);
    assert_eq!(
        parsed.value.expect("accepted").section.as_deref(),
        Some("Voter turnout")
    );
    let blank = REQ_0260.replacen("\ntitle:", "\nsection: \" \"\ntitle:", 1);
    assert_refused(&dashboard_report(&blank), Code::InvalidValue, "section");
}

// -- descriptions: a line under a title, for the reader --------------------

#[test]
fn a_widget_may_say_what_its_figures_are_taken_of() {
    let yaml =
        minimal_with("title: W\n", "title: W\ndescription: Of voters.\n");
    let parsed = parse_widget(&yaml);
    assert_accepted(&parsed.report);
    assert_eq!(
        parsed.value.expect("accepted").description.as_deref(),
        Some("Of voters.")
    );
}

#[test]
fn a_blank_widget_description_is_refused() {
    let yaml = minimal_with("title: W\n", "title: W\ndescription: \" \"\n");
    assert_refused(&widget_report(&yaml), Code::InvalidValue, "description");
}

#[test]
fn a_dashboard_may_say_what_it_shows() {
    let yaml =
        REQ_0260.replacen("\ntitle:", "\ndescription: Turnout.\ntitle:", 1);
    let parsed = parse_dashboard(&yaml);
    assert_accepted(&parsed.report);
    assert_eq!(
        parsed.value.expect("accepted").description.as_deref(),
        Some("Turnout.")
    );
    let blank = REQ_0260.replacen("\ntitle:", "\ndescription: \"\"\ntitle:", 1);
    assert_refused(
        &dashboard_report(&blank),
        Code::InvalidValue,
        "description",
    );
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

/// A timeseries draws what its source counts per hour. Registered voters
/// are a total the producer never splits into hours: a widget showing them
/// hourly would only ever say "not counted".
#[test]
fn a_timeseries_shows_only_measures_its_source_counts_per_hour() {
    let turnout = VOTING_ACTIVITY.replace(
        "source: voting_enrollment_activity",
        "source: voter_turnout",
    );
    assert_accepted(&widget_report(&turnout));
    let stock =
        turnout.replace("measures: [voted]", "measures: [voted, registered]");
    assert_refused(
        &widget_report(&stock),
        Code::UnsupportedBySource,
        "query.measures[1]",
    );
    // A summary of the same source counts it.
    assert_accepted(&widget_report(&minimal_with(
        "measures: [voted]",
        "measures: [voted, registered]",
    )));
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

/// The event's zone is its presentation's (`presentation.timezones`): the
/// settings name none, and one that still does is refused.
#[test]
fn the_settings_name_no_time_zone() {
    assert!(!SETTINGS.contains("time_zone"));
    let report =
        settings_report(&format!("time_zone: Asia/Manila\n{SETTINGS}"));
    assert!(!report.is_accepted());
    assert!(report.to_string().contains("time_zone"), "{report}");
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
    let report = validate_set(&set);
    assert_refused(
        &report,
        Code::DanglingReference,
        "widgets.turnout-by-group.query.group_by",
    );
    assert!(
        report
            .problems
            .iter()
            .all(|problem| problem.path != "settings"),
        "each widget grouping voters is named: {report}"
    );
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
fn posts_can_be_grouped_by_state_only_where_posts_are_counted() {
    let poll = "
id: w
title: W
source: poll_status
query: {template: by_group, group_by: state, measures: [posts]}
chart: {charts: {k: {type: donut, query: data, theta: posts, color: group}}, rows: [k]}
";
    assert_accepted(&widget_report(poll));
    let turnout = minimal_with(
        "{template: summary, measures: [voted]}",
        "{template: by_group, group_by: state, measures: [voted]}",
    );
    assert_refused(
        &widget_report(&turnout),
        Code::UnsupportedBySource,
        "query.group_by",
    );
}

#[test]
fn settings_may_name_the_unknown_group_but_not_leave_it_blank() {
    assert_accepted(&settings_report(&format!(
        "{SETTINGS}\nunknown_label: Hindi alam\n"
    )));
    assert_refused(
        &settings_report(&format!("{SETTINGS}\nunknown_label: \" \"\n")),
        Code::InvalidValue,
        "unknown_label",
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

#[test]
fn settings_may_word_the_dashboard_selectors_but_not_leave_them_blank() {
    let worded = format!(
        "{SETTINGS}\nselectors:\n  region: {{label: Campus, all: All campuses}}\n  post: {{label: Polling station, all: All stations}}\n"
    );
    let parsed = parse_settings(&worded);
    assert_accepted(&parsed.report);
    let settings = parsed.value.expect("worded settings");
    let region = settings
        .selector_words(ScopeSelector::Region)
        .expect("region words");
    assert_eq!(
        (region.label.as_str(), region.all.as_str()),
        ("Campus", "All campuses")
    );
    assert!(
        settings.selector_words(ScopeSelector::Country).is_none(),
        "unworded selectors fall back to the portal's words"
    );
    assert!(parse_settings(SETTINGS).value.unwrap().selectors.is_empty());

    for (field, path) in [
        ("{label: \" \", all: All}", "selectors.region.label"),
        ("{label: Region, all: \"\"}", "selectors.region.all"),
    ] {
        assert_refused(
            &settings_report(&format!(
                "{SETTINGS}\nselectors:\n  region: {field}\n"
            )),
            Code::InvalidValue,
            path,
        );
    }
    assert!(!settings_report(&format!(
        "{SETTINGS}\nselectors:\n  district: {{label: District, all: All}}\n"
    ))
    .is_accepted());
}

#[test]
fn a_dashboard_selector_no_widget_on_it_can_apply_is_warned_of() {
    let mut set = specification_set();
    let poll = parse_widget(
        "
id: poll
title: Poll
source: poll_status
query: {template: summary, ratio: [opened, posts]}
chart: {charts: {k: {type: kpi, query: data, value: pct_label}}, rows: [k]}
",
    )
    .value
    .expect("poll widget");
    set.widgets.insert(poll.id.clone(), poll);
    let dashboard = parse_dashboard(
        "
id: polls
title: Polls
selectors: [region, post, country]
layout: [{widget: poll, width: 12}]
",
    )
    .value
    .expect("polls dashboard");
    set.dashboards.insert(dashboard.id.clone(), dashboard);
    let report = validate_set(&set);
    assert!(report.is_accepted(), "a warning, not an error:\n{report}");
    let unused: Vec<&str> = report
        .problems
        .iter()
        .filter(|problem| problem.code == Code::UnusedSelector)
        .map(|problem| problem.path.as_str())
        .collect();
    // Posts have no country; the specification's dashboard applies all three.
    assert_eq!(unused, ["dashboards.polls.selectors[2]"]);
}

// -- the content pass: whatever the document is ----------------------------

#[test]
fn a_misspelt_key_rule_is_no_rule() {
    assert_eq!(KeyRule::parse("allow"), Some(KeyRule::Allow));
    assert_eq!(KeyRule::parse("alow"), None);
}

#[test]
fn a_document_missing_a_required_field_is_unreadable_as_a_whole() {
    assert_refused(
        &widget_report(&minimal_with("id: w\n", "")),
        Code::Unreadable,
        "",
    );
}

#[test]
fn a_yaml_tag_is_refused_wherever_it_appears() {
    let yaml = minimal_with("title: W", "title: !shout W");
    assert_refused(&widget_report(&yaml), Code::ForbiddenValue, "title");
}

#[test]
fn an_address_in_a_scheme_of_its_own_is_refused_but_a_scheme_starts_with_a_letter(
) {
    let yaml = minimal_with("title: W", "title: \"Open x-app://ballots\"");
    assert_refused(&widget_report(&yaml), Code::ForbiddenValue, "title");
    let yaml = minimal_with("title: W", "title: \"Ratio 1://2\"");
    assert_accepted(&widget_report(&yaml));
}

// -- the content pass: a widget's chart ------------------------------------

const KPI: &str = "k: {type: kpi, query: data, value: voted}";

/// [`MINIMAL`] with `chart` for its chart.
fn minimal_charting(chart: &str) -> String {
    minimal_with(
        "{charts: {k: {type: kpi, query: data, value: voted}}, rows: [k]}",
        chart,
    )
}

/// [`MINIMAL`] with more fields on its one chart.
fn minimal_chart_with(fields: &str) -> String {
    minimal_with("value: voted}", &format!("value: voted, {fields}}}"))
}

#[test]
fn a_chart_that_is_not_a_board_is_unreadable() {
    assert_refused(
        &widget_report(&minimal_charting("[k]")),
        Code::Unreadable,
        "chart",
    );
}

#[test]
fn layout_that_is_not_the_shape_dbt_charts_reads_is_refused() {
    for (chart, code, path) in [
        ("{charts: [k], rows: [k]}".to_string(), Code::InvalidValue, "chart.charts"),
        (format!("{{charts: {{{KPI}}}, rows: k}}"), Code::InvalidValue, "chart.rows"),
        (format!("{{charts: {{{KPI}}}, rows: [5]}}"), Code::InvalidValue, "chart.rows[0]"),
        (
            format!("{{charts: {{{KPI}}}, rows: [{{rows: [k], card_gap: [8]}}]}}"),
            Code::InvalidValue,
            "chart.rows[0].card_gap",
        ),
        (
            format!("{{charts: {{{KPI}}}, grid: {{items: [{{item: k}}], gap: 8}}}}"),
            Code::ForbiddenKey,
            "chart.grid.gap",
        ),
        (
            format!("{{charts: {{{KPI}}}, grid: {{items: [{{col: 0}}]}}}}"),
            Code::InvalidValue,
            "chart.grid.items[0]",
        ),
        (
            format!("{{charts: {{{KPI}}}, grid: {{items: [{{item: 5}}]}}}}"),
            Code::InvalidValue,
            "chart.grid.items[0].item",
        ),
        (
            format!("{{charts: {{{KPI}}}, grid: {{items: [{{item: k, colour: red}}]}}}}"),
            Code::ForbiddenKey,
            "chart.grid.items[0].colour",
        ),
    ] {
        assert_refused(&widget_report(&minimal_charting(&chart)), code, path);
    }
}

#[test]
fn a_chart_definition_holds_no_layout_markdown_or_unbundled_source() {
    for (fields, code, path) in [
        ("items: [k]", Code::ForbiddenKey, "chart.charts.k.items"),
        ("text: Hello", Code::ForbiddenKey, "chart.charts.k.text"),
        (
            "geo_source: 5",
            Code::ForbiddenValue,
            "chart.charts.k.geo_source",
        ),
        (
            "support_table: [{source: 5}]",
            Code::ForbiddenValue,
            "chart.charts.k.support_table[0].source",
        ),
        (
            "style: {category_colors: {'gr\"oup': {values: {M: red}}}}",
            Code::ForbiddenValue,
            "chart.charts.k.style.category_colors.gr\"oup",
        ),
    ] {
        assert_refused(&widget_report(&minimal_chart_with(fields)), code, path);
    }
    assert_refused(
        &widget_report(&minimal_with("query: data", "query: 5")),
        Code::InvalidValue,
        "chart.charts.k.query",
    );
}

#[test]
fn values_the_engine_reads_as_absent_and_boards_in_grid_cells_are_accepted() {
    assert_accepted(&widget_report(&minimal_charting(&format!(
        "{{charts: {{{KPI}}}, grid: {{items: [{{item: {{rows: [k]}}}}]}}, style: {{padding: null}}}}"
    ))));
    assert_accepted(&widget_report(&minimal_chart_with(
        "geo_source: null, style: {category_colors: null}",
    )));
}

#[test]
fn only_typed_in_chart_types_are_refused_here_the_renderer_reads_the_rest() {
    let report = widget_report(&minimal_with("type: kpi", "type: [kpi]"));
    assert!(
        !has(&report, Code::ForbiddenValue, "chart.charts.k.type"),
        "{report}"
    );
}

// -- the content pass: a theme ---------------------------------------------

#[test]
fn a_theme_s_style_is_a_mapping_of_paint_that_queries_nothing() {
    assert_refused(
        &parse_theme("id: t\nstyle: 5\n").report,
        Code::InvalidValue,
        "style",
    );
    assert_refused(
        &parse_theme("id: t\nstyle: {charts: {query: data}}\n").report,
        Code::ForbiddenKey,
        "style.charts.query",
    );
    assert_refused(
        &parse_theme("id: t\nstyle: {accent: \"red]\"}\n").report,
        Code::ForbiddenValue,
        "style.accent",
    );
}

// -- the semantic pass: widgets --------------------------------------------

#[test]
fn a_widget_s_height_stays_within_what_a_screen_shows() {
    let tall = |height: u32| {
        minimal_with(
            "source: voter_turnout",
            &format!("source: voter_turnout\nheight: {height}"),
        )
    };
    assert_accepted(&widget_report(&tall(400)));
    for height in [MIN_WIDGET_HEIGHT - 1, MAX_WIDGET_HEIGHT + 1] {
        assert_refused(
            &widget_report(&tall(height)),
            Code::InvalidValue,
            "height",
        );
    }
}

#[test]
fn a_widget_draws_a_bounded_number_of_charts() {
    let charts: Vec<String> = (0..=MAX_CHARTS_PER_WIDGET)
        .map(|n| format!("c{n}: {{type: kpi, query: data, value: voted}}"))
        .collect();
    let yaml = minimal_charting(&format!(
        "{{charts: {{{}}}, rows: [c0]}}",
        charts.join(", ")
    ));
    assert_refused(&widget_report(&yaml), Code::TooLarge, "chart.charts");
}

#[test]
fn a_selector_lists_options_whose_values_are_words() {
    let with_selector = |selector: &str| {
        minimal_with("query:", &format!("selectors:\n  s: {selector}\nquery:"))
    };
    assert_refused(
        &widget_report(&with_selector("{label: S, default: x}")),
        Code::InvalidValue,
        "selectors.s.options",
    );
    assert_refused(
        &widget_report(&with_selector(
            "{label: S, options: {\"a b\": A}, default: \"a b\"}",
        )),
        Code::InvalidId,
        "selectors.s.options.a b",
    );
}

#[test]
fn options_taken_from_the_data_are_neither_defaulted_nor_mapped() {
    let yaml = VOTING_ACTIVITY.replacen(
        "    options_from: event_days\n",
        "    options_from: event_days\n    default: \"2026-05-04\"\n    maps: {}\n",
        1,
    );
    let report = widget_report(&yaml);
    assert_refused(&report, Code::InvalidValue, "selectors.day.default");
    assert_refused(&report, Code::InvalidValue, "selectors.day.maps");
}

#[test]
fn a_condition_lists_at_least_one_value() {
    let yaml = VOTING_ACTIVITY.replacen("in: [hour]", "in: []", 1);
    assert_refused(
        &widget_report(&yaml),
        Code::InvalidValue,
        "selectors.day.when.in",
    );
}

#[test]
fn a_condition_on_options_from_the_data_is_not_held_to_a_list() {
    let yaml = VOTING_ACTIVITY.replacen(
        "\nquery:\n",
        "\n  note:\n    label: Note\n    options: {a: A}\n    default: a\n    when: {selector: day, in: [\"2026-05-04\"]}\nquery:\n",
        1,
    );
    assert_accepted(&widget_report(&yaml));
}

#[test]
fn a_query_counts_at_least_one_measure_and_a_filter_keeps_at_least_one_value() {
    assert_refused(
        &widget_report(&minimal_with("measures: [voted]", "measures: []")),
        Code::TemplateParameter,
        "query.measures",
    );
    assert_refused(
        &widget_report(&minimal_with(
            "measures: [voted]",
            "measures: [voted], filters: {status: []}",
        )),
        Code::TemplateParameter,
        "query.filters.status",
    );
}

#[test]
fn a_post_state_is_labelled_only_where_posts_are_counted() {
    let poll = MINIMAL
        .replace("voter_turnout", "poll_status")
        .replace(
            "{template: summary, measures: [voted]}",
            "{template: by_post, labels: {not_initialized: Not yet}}",
        )
        .replace("value: voted", "value: posts");
    assert_accepted(&widget_report(&poll));
    for key in ["not_initialized", "nonsense"] {
        assert_refused(
            &widget_report(&minimal_with(
                "measures: [voted]",
                &format!("measures: [voted], labels: {{{key}: Label}}"),
            )),
            Code::UnsupportedBySource,
            &format!("query.labels.{key}"),
        );
    }
}

#[test]
fn sorting_by_value_needs_something_counted() {
    let poll = MINIMAL
        .replace("voter_turnout", "poll_status")
        .replace(
            "{template: summary, measures: [voted]}",
            "{template: by_post, sort: {by: value}}",
        )
        .replace("value: voted", "value: posts");
    assert_refused(
        &widget_report(&poll),
        Code::TemplateParameter,
        "query.sort",
    );
}

#[test]
fn a_day_picker_beside_an_undeclared_grain_selector_is_refused_once() {
    let yaml = VOTING_ACTIVITY.replacen(
        "grain: {selector: grain}",
        "grain: {selector: missing}",
        1,
    );
    let report = widget_report(&yaml);
    assert_refused(&report, Code::UnknownSelector, "query.grain.selector");
    assert!(
        !has(&report, Code::TemplateParameter, "query.day"),
        "{report}"
    );
}

// -- the semantic pass: dashboards, settings and sets ----------------------

#[test]
fn a_dashboard_shows_a_widget_and_names_its_theme_by_id() {
    assert_refused(
        &dashboard_report(&REQ_0260.replacen(
            "\ntitle:",
            "\ntheme: Neon\ntitle:",
            1,
        )),
        Code::InvalidId,
        "theme",
    );
    assert_accepted(&dashboard_report(&REQ_0260.replacen(
        "\ntitle:",
        "\ntheme: neon\ntitle:",
        1,
    )));
    assert_refused(
        &dashboard_report("id: d\ntitle: D\nlayout: []\n"),
        Code::InvalidValue,
        "layout",
    );
}

#[test]
fn a_split_needs_a_separator() {
    let yaml = SETTINGS.replace("separator: \"/\"", "separator: \"\"");
    assert_refused(
        &settings_report(&yaml),
        Code::InvalidValue,
        "scope.country.split.separator",
    );
}

#[test]
fn a_filter_must_be_a_configured_dimension() {
    let mut set = specification_set();
    let widget = parse_widget(&minimal_with(
        "measures: [voted]",
        "measures: [voted], filters: {status: [sea]}",
    ))
    .value
    .expect("filtered widget");
    set.widgets.insert(widget.id.clone(), widget);
    assert_accepted(&validate_set(&set));
    if let Some(settings) = set.settings.as_mut() {
        settings.dimensions.shift_remove("status");
    }
    assert_refused(
        &validate_set(&set),
        Code::UnsupportedBySource,
        "widgets.w.query.filters.status",
    );
}
