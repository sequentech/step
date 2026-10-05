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
    let regions = if regions_coloured {
        ", color: group"
    } else {
        ""
    };
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
    let board = build_board(
        &posts_widget(false),
        Some(&pinning("{Unknown: dbt-grays.muted}")),
        &by_state(),
    );
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

fn styled(style: &str) -> Theme {
    Theme {
        id: "t".into(),
        title: None,
        base: None,
        style: Some(serde_yaml::from_str(style).unwrap()),
    }
}

fn group_rows(rows: Vec<Vec<Value>>) -> IndexMap<String, QueryResult> {
    [(
        "data".to_string(),
        QueryResult {
            columns: vec![Column {
                name: "group".into(),
                kind: ColumnKind::Text,
            }],
            rows,
            notices: Vec::new(),
        },
    )]
    .into()
}

#[test]
fn a_chart_that_is_not_a_mapping_is_still_a_board_of_governed_rows() {
    let mut widget = widget();
    widget.chart = serde_yaml::Value::Null;
    let board = build_board(&widget, None, &data());
    assert_eq!(board["queries"]["data"]["columns"], json!(["group", "pct"]));
    assert_eq!(board["style"]["footer"], json!({"visible": false}));
    assert!(board.get("charts").is_none());
}

#[test]
fn a_theme_with_only_a_base_sets_the_base_and_no_style() {
    let theme = parse_theme("id: t\nbase: paper\n").value.expect("theme");
    let board = build_board(&widget(), Some(&theme), &data());
    assert_eq!(board["theme"], json!("paper"));
    assert_eq!(
        board["style"],
        json!({"footer": {"visible": false}, "timestamp": {"visible": false}})
    );
}

#[test]
fn a_board_without_charts_keeps_every_theme_pin() {
    let mut widget = widget();
    widget.chart = serde_yaml::from_str("rows: []").unwrap();
    let board = build_board(
        &widget,
        Some(&pinning("{Unknown: dbt-grays.muted}")),
        &data(),
    );
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"]["values"],
        json!({"Unknown": "dbt-grays.muted"})
    );
    assert!(board.get("charts").is_none());
}

/// A chart colouring by a field it has no rows for draws none of its
/// values, so the theme pins none of them.
#[test]
fn a_coloured_chart_without_its_rows_or_column_draws_no_pinned_value() {
    let theme = pinning("{Unknown: dbt-grays.muted}");
    let mut widget = widget();
    widget.chart = serde_yaml::from_str(
        "charts:\n  lost: {type: donut, query: missing, theta: pct, color: group}\nrows: [lost]\n",
    )
    .unwrap();
    let board = build_board(&widget, Some(&theme), &data());
    assert!(
        board["style"]["charts"].get("category_colors").is_none(),
        "{board}"
    );

    widget.chart = serde_yaml::from_str(
        "charts:\n  pie: {type: donut, query: data, theta: pct, color: sex}\nrows: [pie]\n",
    )
    .unwrap();
    let theme = styled(
        "charts: {category_colors: {sex: {values: {Unknown: dbt-grays.muted}}}}",
    );
    let board = build_board(&widget, Some(&theme), &data());
    assert!(
        board["style"]["charts"].get("category_colors").is_none(),
        "{board}"
    );
}

/// A number is drawn as its text; a null or a missing cell draws nothing.
#[test]
fn a_pin_matches_a_drawn_number_by_its_text_and_no_null() {
    let theme = pinning(
        "{\"2024\": \"category[1]\", \"null\": \"category[2]\", Male: \"category[3]\"}",
    );
    let mut widget = widget();
    widget.chart = serde_yaml::from_str(
        "charts:\n  pie: {type: donut, query: data, theta: group, color: group}\nrows: [pie]\n",
    )
    .unwrap();
    let rows = vec![vec![json!(2024)], vec![Value::Null], vec![]];
    let board = build_board(&widget, Some(&theme), &group_rows(rows));
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"]["values"],
        json!({"2024": "category[1]"})
    );
}

#[test]
fn a_theme_pin_without_values_is_kept_whole() {
    let theme = styled(
        "charts: {category_colors: {group: {fallback: dbt-grays.muted}}}",
    );
    let mut widget = widget();
    widget.chart = serde_yaml::from_str(
        "charts:\n  pie: {type: donut, query: data, theta: pct, color: group}\nrows: [pie]\n",
    )
    .unwrap();
    let board = build_board(&widget, Some(&theme), &data());
    assert_eq!(
        board["style"]["charts"]["category_colors"]["group"],
        json!({"fallback": "dbt-grays.muted"})
    );
}

/// The hidden additions are set whatever the chart put in their place.
#[test]
fn a_style_that_is_not_a_mapping_still_hides_the_engine_additions() {
    let mut widget = widget();
    widget.chart = serde_yaml::from_str(
        "style: plain\ncharts:\n  note: plain\n  posts: {type: table, query: data, style: plain}\nrows: [posts]\n",
    )
    .unwrap();
    let board = build_board(&widget, None, &data());
    assert_eq!(
        board["style"],
        json!({"footer": {"visible": false}, "timestamp": {"visible": false}})
    );
    assert_eq!(board["charts"]["note"], json!("plain"));
    assert_eq!(
        board["charts"]["posts"]["style"],
        json!({"pagination": {"enabled": false}})
    );
}

#[test]
fn tags_are_read_through_and_every_kind_of_key_survives_as_text() {
    let mut widget = widget();
    widget.chart = serde_yaml::from_str(
        "title: !note Turnout\nlabels:\n  ~: none\n  1.5: half\n  ? [a, b]\n  : pair\n",
    )
    .unwrap();
    let board = build_board(&widget, None, &data());
    assert_eq!(board["title"], json!("Turnout"));
    assert_eq!(
        board["labels"],
        json!({"null": "none", "1.5": "half", "- a\n- b": "pair"})
    );
}

#[test]
fn a_boards_figure_affixes_are_its_formats_prefixes_suffixes_and_glyphs() {
    let board = json!({
        "charts": {
            "kpi": {
                "type": "kpi",
                "format": {"prefix": "≈", "suffix": " votes"},
                "support": [{"value_suffix": " pts", "label": "turnout"}],
            },
            "bars": {"type": "bar", "labels": {"glyph": "▲", "prefix": ""}},
        },
        "style": {"formats": {"money": {"suffix": " votes"}}},
        "queries": {"data": {"columns": ["prefix"], "values": [["prefix"]]}},
    });

    assert_eq!(
        figure_affixes(&board),
        [" pts", " votes", "≈", "▲"].map(String::from)
    );
}

#[test]
fn a_board_without_formats_has_no_figure_affixes() {
    assert!(figure_affixes(&json!({"charts": {}, "queries": {}})).is_empty());
}
