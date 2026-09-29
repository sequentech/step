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
            let theme_style = to_json(style);
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

/// What dbt Charts draws unless told not to, and a dashboard must not show:
/// a footer linking to dbt Charts, a "Data as of" line in UTC (the dashboard
/// shows its snapshot time in the event's zone), and a donut's total, which
/// adds up slices that need not add up to the scope. Set after the theme and
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
            let pie = matches!(
                chart.get("type").and_then(Value::as_str),
                Some("pie" | "donut")
            );
            if !pie {
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
