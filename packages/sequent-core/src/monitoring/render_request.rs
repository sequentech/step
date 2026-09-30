// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The dbt Charts board the renderer draws for one widget.
//!
//! Built here, not in the renderer, so the Admin Portal's preview and the
//! server send the same board for the same widget. The board is the widget's
//! chart with the dashboard theme merged underneath it — the widget wins —
//! and the widget's governed rows as its only queries, as inline values.
//! Nothing else reaches the engine: no source, no SQL, no variables.

use super::compute::QueryResult;
use super::config::{Theme, Widget};
use indexmap::IndexMap;
use serde_json::{Map, Value};
use serde_yaml::Value as Yaml;
use std::collections::{HashMap, HashSet};

/// The board for `widget`, with `data` keyed by query name.
pub fn build_board(
    widget: &Widget,
    theme: Option<&Theme>,
    data: &IndexMap<String, QueryResult>,
) -> Value {
    let mut board = match to_json(&widget.chart) {
        Value::Object(board) => board,
        _ => Map::new(),
    };
    if let Some(theme) = theme {
        if let Some(style) = &theme.style {
            let mut theme_style = to_json(style);
            drop_undrawn_pins(&mut theme_style, &board, data);
            let merged = match board.remove("style") {
                Some(own) => merge(theme_style, own),
                None => theme_style,
            };
            board.insert("style".into(), merged);
        }
        if let Some(base) = &theme.base {
            board.insert("theme".into(), Value::String(base.to_string()));
        }
    }
    hide_engine_additions(&mut board);
    let queries: Map<String, Value> = data
        .iter()
        .map(|(name, result)| {
            let columns: Vec<Value> = result
                .columns
                .iter()
                .map(|column| Value::String(column.name.clone()))
                .collect();
            let values: Vec<Value> =
                result.rows.iter().cloned().map(Value::Array).collect();
            let mut query = Map::new();
            query.insert("columns".into(), Value::Array(columns));
            query.insert("values".into(), Value::Array(values));
            (name.clone(), Value::Object(query))
        })
        .collect();
    board.insert("queries".into(), Value::Object(queries));
    Value::Object(board)
}

/// The values each field is coloured with on this board: what the charts
/// with a `color:` channel draw from their query's rows.
fn drawn_categories(
    board: &Map<String, Value>,
    data: &IndexMap<String, QueryResult>,
) -> HashMap<String, HashSet<String>> {
    let mut drawn: HashMap<String, HashSet<String>> = HashMap::new();
    let Some(Value::Object(charts)) = board.get("charts") else {
        return drawn;
    };
    for chart in charts.values() {
        let Some(field) = chart.get("color").and_then(Value::as_str) else {
            continue;
        };
        let values = drawn.entry(field.to_string()).or_default();
        let result = chart
            .get("query")
            .and_then(Value::as_str)
            .and_then(|query| data.get(query));
        let Some(result) = result else {
            continue;
        };
        let Some(at) = result
            .columns
            .iter()
            .position(|column| column.name == field)
        else {
            continue;
        };
        for row in &result.rows {
            match row.get(at) {
                Some(Value::String(text)) => {
                    values.insert(text.clone());
                }
                Some(Value::Null) | None => {}
                Some(other) => {
                    values.insert(other.to_string());
                }
            }
        }
    }
    drawn
}

/// Leaves out the theme's `category_colors` pins for values this board's
/// charts colour by but never draw. A theme pins for every dashboard it
/// styles — Unknown in grey wherever a chart colours by group — and a chart
/// of Post states has no Unknown: the engine would warn, on every render
/// and every save, that the pin was skipped. A field no chart colours by
/// keeps its pins, which the engine ignores; the widget's own pins are
/// merged afterwards and kept, so a misspelt one is still reported.
fn drop_undrawn_pins(
    theme_style: &mut Value,
    board: &Map<String, Value>,
    data: &IndexMap<String, QueryResult>,
) {
    let Some(Value::Object(pins)) = theme_style
        .get_mut("charts")
        .and_then(|charts| charts.get_mut("category_colors"))
    else {
        return;
    };
    let drawn = drawn_categories(board, data);
    pins.retain(|field, binding| {
        let Some(values) = drawn.get(field) else {
            return true;
        };
        if let Some(Value::Object(pinned)) = binding.get_mut("values") {
            pinned.retain(|value, _| values.contains(value));
            if pinned.is_empty() {
                if let Value::Object(binding) = binding {
                    binding.remove("values");
                }
            }
        }
        !matches!(binding, Value::Object(binding) if binding.is_empty())
    });
    if pins.is_empty() {
        if let Some(Value::Object(charts)) = theme_style.get_mut("charts") {
            charts.remove("category_colors");
        }
    }
}

/// What dbt Charts draws unless told not to, and a dashboard must not show:
/// a footer linking to dbt Charts, a "Data as of" line in UTC (the dashboard
/// shows its snapshot time in the event's zone), and a donut's total, which
/// adds up slices that need not add up to the scope; and a table's pager,
/// which runs on a script the dashboard never runs, so a table shows every
/// row and its frame scrolls. Set after the theme and
/// the widget, so neither can bring them back.
fn hide_engine_additions(board: &mut Map<String, Value>) {
    let hidden = || {
        Value::Object(Map::from_iter([("visible".into(), Value::Bool(false))]))
    };
    let style = board
        .entry("style")
        .or_insert_with(|| Value::Object(Map::new()));
    if !style.is_object() {
        *style = Value::Object(Map::new());
    }
    if let Value::Object(style) = style {
        style.insert("footer".into(), hidden());
        style.insert("timestamp".into(), hidden());
    }
    if let Some(Value::Object(charts)) = board.get_mut("charts") {
        for chart in charts.values_mut() {
            let Value::Object(chart) = chart else {
                continue;
            };
            let kind = chart.get("type").and_then(Value::as_str);
            if kind == Some("table") {
                let style = chart
                    .entry("style")
                    .or_insert_with(|| Value::Object(Map::new()));
                if !style.is_object() {
                    *style = Value::Object(Map::new());
                }
                if let Value::Object(style) = style {
                    style.insert(
                        "pagination".into(),
                        Value::Object(Map::from_iter([(
                            "enabled".into(),
                            Value::Bool(false),
                        )])),
                    );
                }
                continue;
            }
            if !matches!(kind, Some("pie" | "donut")) {
                continue;
            }
            match chart.get_mut("total") {
                Some(Value::Object(total)) => {
                    total.insert("visible".into(), Value::Bool(false));
                }
                _ => {
                    chart.insert("total".into(), hidden());
                }
            }
        }
    }
}

/// `over` merged onto `base`: mappings key by key, lists and values
/// replaced whole. A null in `over` says nothing, so the theme's value stays.
fn merge(base: Value, over: Value) -> Value {
    match (base, over) {
        (base, Value::Null) => base,
        (Value::Object(mut base), Value::Object(over)) => {
            for (key, value) in over {
                let merged = match base.remove(&key) {
                    Some(existing) => merge(existing, value),
                    None => value,
                };
                base.insert(key, merged);
            }
            Value::Object(base)
        }
        (_, over) => over,
    }
}

/// YAML to JSON, keeping keys YAML allows and JSON does not as their text.
fn to_json(value: &Yaml) -> Value {
    match value {
        Yaml::Null => Value::Null,
        Yaml::Bool(flag) => Value::Bool(*flag),
        Yaml::Number(number) => {
            serde_json::to_value(number).unwrap_or(Value::Null)
        }
        Yaml::String(text) => Value::String(text.clone()),
        Yaml::Sequence(items) => {
            Value::Array(items.iter().map(to_json).collect())
        }
        Yaml::Mapping(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key_text(key), to_json(value)))
                .collect(),
        ),
        Yaml::Tagged(tagged) => to_json(&tagged.value),
    }
}

fn key_text(key: &Yaml) -> String {
    match key {
        Yaml::String(text) => text.clone(),
        Yaml::Bool(flag) => flag.to_string(),
        Yaml::Number(number) => number.to_string(),
        Yaml::Null => "null".to_string(),
        other => serde_yaml::to_string(other)
            .map(|text| text.trim().to_string())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
#[path = "render_request_tests.rs"]
mod render_request_tests;
