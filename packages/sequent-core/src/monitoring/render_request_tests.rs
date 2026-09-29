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
