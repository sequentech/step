// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;
use crate::monitoring::compute::{Column, ColumnKind, QueryResult};
use crate::monitoring::policy::{parse_theme, parse_widget};
use serde_json::json;

const TURNOUT_BY_GROUP: &str = include_str!("fixtures/turnout_by_group.yaml");
const THEME: &str = include_str!("fixtures/theme_default.yaml");

fn data() -> IndexMap<String, QueryResult> {
    [(
        "data".to_string(),
        QueryResult {
            columns: vec![
                Column {
                    name: "group".into(),
                    kind: ColumnKind::Text,
                },
                Column {
                    name: "pct".into(),
                    kind: ColumnKind::Number,
                },
            ],
            rows: vec![
                vec![json!("Male"), json!(0.5)],
                vec![json!("Unknown"), serde_json::Value::Null],
            ],
            notices: Vec::new(),
        },
    )]
    .into()
}

fn widget() -> Widget {
    parse_widget(TURNOUT_BY_GROUP).value.expect("fixture")
}

#[test]
fn governed_rows_are_the_only_queries_on_the_board() {
    let board = build_board(&widget(), None, &data());
    assert_eq!(
        board["queries"],
        json!({"data": {"columns": ["group", "pct"], "values": [["Male", 0.5], ["Unknown", null]]}})
    );
    assert_eq!(board["charts"]["bars"]["query"], json!("data"));
    assert_eq!(board["rows"], json!(["bars"]));
}

#[test]
fn the_theme_is_merged_under_the_widget_which_wins() {
    let theme = parse_theme(
        "id: t\nbase: paper\nstyle:\n  background: dbt-grays.canvas\n  font: {color: dbt-grays.ink}\n  charts:\n    category_colors:\n      group:\n        values: {Unknown: dbt-grays.muted}\n",
    )
    .value
    .expect("theme");
    let mut widget = widget();
    widget.chart["style"] = serde_yaml::from_str(
        "font: {color: dbt-grays.muted}\ncharts: {category_colors: {group: {values: {Male: \"category[1]\"}}}}",
    )
    .unwrap();
    let board = build_board(&widget, Some(&theme), &data());
    assert_eq!(board["theme"], json!("paper"));
    assert_eq!(board["style"]["background"], json!("dbt-grays.canvas"));
    assert_eq!(board["style"]["font"]["color"], json!("dbt-grays.muted"));
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"]["values"],
        json!({"Unknown": "dbt-grays.muted", "Male": "category[1]"})
    );
}

#[test]
fn a_theme_without_a_base_adds_only_its_style() {
    let theme = parse_theme(THEME).value.expect("fixture");
    let board = build_board(&widget(), Some(&theme), &data());
    assert!(board.get("theme").is_none());
    assert!(board["style"]["charts"]["category_colors"].is_object());
}

#[test]
fn non_string_keys_survive_as_text() {
    let mut widget = widget();
    widget.chart["style"] =
        serde_yaml::from_str("charts: {category_colors: {group: {values: {2024: \"category[1]\", true: \"category[2]\"}}}}")
            .unwrap();
    let board = build_board(&widget, None, &data());
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"]["values"],
        json!({"2024": "category[1]", "true": "category[2]"})
    );
}

fn by_state() -> IndexMap<String, QueryResult> {
    let rows = |values: &[&str]| QueryResult {
        columns: vec![Column {
            name: "group".into(),
            kind: ColumnKind::Text,
        }],
        rows: values.iter().map(|value| vec![json!(value)]).collect(),
        notices: Vec::new(),
    };
    [
        ("states".to_string(), rows(&["Opened", "Closed"])),
        ("regions".to_string(), rows(&["Europe", "Unknown"])),
    ]
    .into()
}

fn posts_widget(regions_coloured: bool) -> Widget {
    let regions = if regions_coloured { ", color: group" } else { "" };
    parse_widget(&format!(
        "id: w\ntitle: W\nsource: poll_status\nqueries:\n  states: {{template: by_group, group_by: state, measures: [posts]}}\n  regions: {{template: by_group, group_by: region, measures: [posts]}}\nchart:\n  charts:\n    states: {{type: donut, query: states, theta: posts, color: group}}\n    regions: {{type: bar, query: regions, x: group, y: posts{regions}}}\n  rows: [states, regions]\n"
    ))
    .value
    .expect("widget")
}

fn pinning(values: &str) -> Theme {
    parse_theme(&format!(
        "id: t\nstyle:\n  charts:\n    category_colors:\n      group:\n        values: {values}\n"
    ))
    .value
    .expect("theme")
}

/// A theme's pins are event-wide: a widget whose coloured charts never draw
/// a pinned value gets no pin for it, so the engine does not warn that the
/// pin names a value this render never draws.
#[test]
fn a_theme_pin_reaches_a_board_only_when_its_coloured_charts_draw_the_value() {
    let theme = pinning("{Unknown: dbt-grays.muted, Opened: \"category[2]\"}");
    let board = build_board(&posts_widget(false), Some(&theme), &by_state());
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"]["values"],
        json!({"Opened": "category[2]"}),
        "the bar does not colour by group, so its Unknown is not drawn"
    );
    let board = build_board(&posts_widget(true), Some(&theme), &by_state());
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"]["values"],
        json!({"Unknown": "dbt-grays.muted", "Opened": "category[2]"})
    );
    let board = build_board(&posts_widget(false), Some(&pinning("{Unknown: dbt-grays.muted}")), &by_state());
    assert!(
        board["style"]["charts"].get("category_colors").is_none(),
        "{board}"
    );
}

/// A widget's own pin is the author's, for this widget: it is kept, and the
/// engine says so if the value is never drawn.
#[test]
fn a_widget_pin_reaches_the_board_whatever_it_draws() {
    let mut widget = posts_widget(false);
    widget.chart["style"] = serde_yaml::from_str(
        "charts: {category_colors: {group: {values: {Paused: \"category[3]\"}}}}",
    )
    .unwrap();
    let theme = pinning("{Unknown: dbt-grays.muted}");
    let board = build_board(&widget, Some(&theme), &by_state());
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"]["values"],
        json!({"Paused": "category[3]"})
    );
}

/// A pin for a field no chart colours by is left alone: the engine binds
/// nothing to it, so it cannot warn.
#[test]
fn a_theme_pin_on_a_field_no_chart_colours_by_is_left_alone() {
    let theme = pinning("{Unknown: dbt-grays.muted}");
    let board = build_board(&widget(), Some(&theme), &data());
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"]["values"],
        json!({"Unknown": "dbt-grays.muted"})
    );
}

fn paper() -> Theme {
    parse_theme(
        "id: t\nstyle:\n  background: dbt-grays.canvas\n  font: {color: dbt-grays.ink, size: 13}\n",
    )
    .value
    .expect("theme")
}

#[test]
fn a_null_in_the_widget_style_leaves_the_theme_value() {
    let mut widget = widget();
    widget.chart["style"] = serde_yaml::Value::Null;
    let board = build_board(&widget, Some(&paper()), &data());
    assert_eq!(board["style"]["background"], json!("dbt-grays.canvas"));

    widget.chart["style"] =
        serde_yaml::from_str("font: {color: null, size: 15}").unwrap();
    let board = build_board(&widget, Some(&paper()), &data());
    assert_eq!(board["style"]["font"]["color"], json!("dbt-grays.ink"));
    assert_eq!(board["style"]["font"]["size"], json!(15));
}

#[test]
fn a_list_in_the_widget_style_replaces_the_theme_list_whole() {
    let theme = parse_theme(
        "id: t\nstyle:\n  charts: {color: {categorical: {palette: [\"#111111\", \"#222222\"]}}}\n",
    )
    .value
    .expect("theme");
    let mut widget = widget();
    widget.chart["style"] = serde_yaml::from_str(
        "charts: {color: {categorical: {palette: [\"#333333\"]}}}",
    )
    .unwrap();
    let board = build_board(&widget, Some(&theme), &data());
    assert_eq!(
        board["style"]["charts"]["color"]["categorical"]["palette"],
        json!(["#333333"])
    );
}

#[test]
fn governed_rows_replace_any_queries_the_chart_carries() {
    // The policy refuses `queries` in a chart; the builder does not rely on it.
    let mut widget = widget();
    widget.chart["queries"] =
        serde_yaml::from_str("data: {sql: SELECT 1}").unwrap();
    let board = build_board(&widget, None, &data());
    assert_eq!(board["queries"]["data"]["columns"], json!(["group", "pct"]));
    assert!(board["queries"]["data"].get("sql").is_none());
}

#[test]
fn the_engine_adds_no_footer_freshness_line_or_total_of_its_own() {
    // The dashboard says when its figures are from, and a donut's slices
    // need not add up to the scope: the engine's defaults would say both.
    let mut widget = widget();
    widget.chart = serde_yaml::from_str(
        "charts:\n  pie: {type: donut, query: data, theta: pct, color: group, total: {label: Voters}}\n  bars: {type: bar, query: data, x: group, y: pct}\nrows: [pie, bars]\n",
    )
    .unwrap();
    let board = build_board(&widget, Some(&paper()), &data());
    assert_eq!(board["style"]["footer"], json!({"visible": false}));
    assert_eq!(board["style"]["timestamp"], json!({"visible": false}));
    assert_eq!(
        board["charts"]["pie"]["total"],
        json!({"label": "Voters", "visible": false})
    );
    assert!(board["charts"]["bars"].get("total").is_none());
    assert_eq!(board["style"]["background"], json!("dbt-grays.canvas"));

    let board = build_board(&widget, None, &data());
    assert_eq!(board["style"]["footer"], json!({"visible": false}));
}

#[test]
fn a_table_shows_every_row_without_a_pager() {
    // The engine pages a long table with a script, which the dashboard's
    // sanitiser and sandboxed frame never run: a pager there would be dead.
    let mut widget = widget();
    widget.chart = serde_yaml::from_str(
        "charts:\n  posts: {type: table, query: data, style: {pagination: {enabled: true, page_rows: 5}}}\n  plain: {type: table, query: data}\n  bars: {type: bar, query: data, x: group, y: pct}\nrows: [posts, plain, bars]\n",
    )
    .unwrap();
    let board = build_board(&widget, Some(&paper()), &data());
    for table in ["posts", "plain"] {
        assert_eq!(
            board["charts"][table]["style"]["pagination"]["enabled"],
            json!(false),
            "{table}"
        );
    }
    assert!(board["charts"]["bars"]
        .pointer("/style/pagination")
        .is_none());
}
