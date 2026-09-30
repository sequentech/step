// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What the platform refuses in monitoring configuration.
//!
//! Every number on a dashboard must come from a governed data source, so
//! configuration may choose *which* template of a source to run and with which
//! parameters, and how the result looks — nothing else. That rules out, and
//! this module refuses:
//!
//! - SQL, HTTP and file queries, and data typed in by hand: dbt Charts can do
//!   all of these, and a widget must not;
//! - links, raw HTML, markdown, templates, expressions and URLs, anywhere in
//!   any document;
//! - charts, queries and themes from other files;
//! - anything the data source does not offer: a measure, a group, a template;
//! - selector values outside the options a selector lists.
//!
//! dbt Charts YAML is checked against an allowlist, `dbt_charts_keys.txt`,
//! which classifies every property the dbt Charts schema defines. A key it
//! does not list is refused, so a feature a dbt Charts upgrade adds reaches no
//! widget until someone has decided what it may do.
//!
//! The checks run in two passes. The *content* pass walks the raw YAML, so it
//! can name a forbidden key even in a document that is otherwise malformed.
//! The *semantic* pass runs on the typed document. Both always run, so one
//! save attempt reports every problem at once.
//!
//! dbt Charts' own schema and visual checks are not repeated here: they need
//! the engine, so the renderer runs them and Harvest adds its findings to the
//! same report.

use super::config::{
    ConfigSet, Dashboard, DimensionMapping, Param, Query, Selector,
    SelectorControl, Settings, SortKey, Theme, Widget, DEFAULT_QUERY_NAME,
    DEFAULT_THEME, GRID_COLUMNS,
};
use super::problem::{Code, Problem, Report};
use super::sources::{
    BuiltinDimension, Measure, PostState, QueryTemplate, SourceSpec, TimeGrain,
    VoterDimensions,
};
use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use serde_yaml::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Larger than any real widget, small enough that a pasted data dump is
/// refused before anything parses it.
pub const MAX_DOCUMENT_BYTES: usize = 64 * 1024;

/// Nesting deeper than this is not configuration.
pub const MAX_DEPTH: usize = 32;

pub const MAX_CHARTS_PER_WIDGET: usize = 50;
pub const MAX_ROW_LIMIT: u32 = 1000;
pub const MIN_WIDGET_HEIGHT: u32 = 80;
pub const MAX_WIDGET_HEIGHT: u32 = 2000;

/// The board fields a widget's `chart` may set. Everything else a dbt Charts
/// board can hold — queries, sources, variables, inheritance, text — is either
/// supplied by the platform or not allowed.
const CHART_FIELDS: &[&str] = &["charts", "rows", "cols", "grid", "style"];

/// What a board nested in the layout may set besides placing charts.
const NESTED_BOARD_FIELDS: &[&str] = &[
    "rows", "cols", "grid", "style", "title", "visible", "card_gap", "height",
    "width",
];

/// The fields of a nested board that place charts; it needs one of them.
const LAYOUT_FIELDS: &[&str] = &["rows", "cols", "grid"];

/// Layout that switches what is shown by navigating to another URL. A
/// widget's frame runs no script and navigates nowhere, so these would draw
/// controls that do nothing — and links.
const NAVIGATING_FIELDS: &[&str] = &["tabs", "details"];

const GRID_FIELDS: &[&str] = &["columns", "items"];
const GRID_ITEM_FIELDS: &[&str] = &[
    "item", "col", "col_span", "row", "row_span", "height", "width",
];

/// What a board's `style` sets: the engine's `Style`. Chart styles differ,
/// and a layout key here would reach the board when a theme is merged in.
const BOARD_STYLE_FIELDS: &[&str] = &[
    "accent",
    "background",
    "border",
    "box_shadow",
    "charts",
    "font",
    "footer",
    "formats",
    "frame",
    "gap",
    "layout",
    "margin",
    "muted",
    "opacity",
    "padding",
    "palettes",
    "placeholder",
    "roles",
    "text",
    "timestamp",
    "title",
    "tones",
    "variables",
];

/// Style keys whose text value is a colour. The engine writes some of these
/// straight into an SVG attribute or a CSS rule, so anything else — a quote,
/// a `;` — would break out of it.
const COLOUR_KEYS: &[&str] = &[
    "accent",
    "background",
    "color",
    "color_active",
    "color_disabled",
    "color_inactive",
    "drop_line_color",
    "fill",
    "focus_color",
    "glyph_color",
    "info",
    "muted",
    "negative",
    "null_color",
    "palette",
    "positive",
    "static",
    "stroke",
    "warning",
];

/// Text the engine writes beside a figure. A digit there reads as part of
/// the figure: a prefix of `9` turns 5 votes into 95.
const AFFIX_KEYS: &[&str] = &["glyph", "prefix", "suffix", "value_suffix"];

/// Numbers the engine draws that many of: ticks, bins, bars, legend
/// entries. Each costs the renderer time, and a large one exhausts it.
const COUNT_KEYS: &[&str] = &[
    "bin_maxbins",
    "col",
    "col_span",
    "columns",
    "compact_columns",
    "count",
    "label_max_lines",
    "level",
    "max_bars",
    "max_chars",
    "max_number",
    "page_rows",
    "row",
    "row_span",
    "symbol_limit",
];
pub const MAX_DBT_COUNT: f64 = 100.0;

/// Numbers compared with, or placed at, a figure: a threshold, a domain, a
/// tick value. Figures reach millions, so these are not bounded like sizes.
const DATA_VALUED_KEYS: &[&str] = &[
    "bound",
    "domain",
    "gt",
    "gte",
    "lt",
    "lte",
    "max",
    "min",
    "range",
    "thresholds",
    "values",
    "x",
    "y",
];

/// Every other number in a chart is a size, a ratio or an offset in
/// pixels, which no screen needs above this.
pub const MAX_DBT_NUMBER: f64 = 10_000.0;

/// Chart types whose content is typed in rather than read from a query.
const FORBIDDEN_CHART_TYPES: &[&str] = &["callout", "image"];

/// Map geometry the renderer bundles; any other name dbt Charts would fetch,
/// or read as a URL. dbt Charts' own defaults for these names are URLs, so the
/// renderer points them at the bundled files, and its tests fail if drawing a
/// map opens a connection.
pub const GEO_SOURCES: &[&str] = &["world-countries", "world-50m"];

/// Where a string `text` is a caption rather than markdown content.
const PLAIN_TEXT_PARENTS: &[&str] = &["footer", "overlay"];

/// Where `source` names a column of the widget's query.
const COLUMN_SOURCE_PARENTS: &[&str] = &["support_table", "entries"];

/// Markers of text that a template engine would evaluate.
const TEMPLATE_MARKERS: &[&str] = &["{{", "{%", "{#", "${"];

/// Schemes that run script wherever they appear.
const SCRIPT_SCHEMES: &[&str] = &["javascript:", "vbscript:"];

/// Schemes that point somewhere, refused at the start of a word.
const URL_SCHEMES: &[&str] = &[
    "about", "blob", "data", "file", "ftp", "http", "https", "intent",
    "mailto", "sms", "tel", "ws", "wss",
];

/// What the policy does with a dbt Charts property, as
/// `dbt_charts_keys.txt` classifies it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyRule {
    Allow,
    Forbid,
    ChartType,
    Flag,
    Geo,
    Layout,
    Names,
    Query,
    Source,
    Text,
}

impl KeyRule {
    fn parse(word: &str) -> Option<KeyRule> {
        Some(match word {
            "allow" => KeyRule::Allow,
            "forbid" => KeyRule::Forbid,
            "chart_type" => KeyRule::ChartType,
            "flag" => KeyRule::Flag,
            "geo" => KeyRule::Geo,
            "layout" => KeyRule::Layout,
            "names" => KeyRule::Names,
            "query" => KeyRule::Query,
            "source" => KeyRule::Source,
            "text" => KeyRule::Text,
            _ => return None,
        })
    }
}

const DBT_CHARTS_KEYS: &str = include_str!("dbt_charts_keys.txt");

/// Every dbt Charts property the policy knows, and its rule. A misspelt rule
/// forbids the key; a test keeps the list free of them.
fn dbt_keys() -> &'static HashMap<&'static str, KeyRule> {
    static KEYS: OnceLock<HashMap<&'static str, KeyRule>> = OnceLock::new();
    KEYS.get_or_init(|| {
        key_lines()
            .map(|(name, rule)| {
                (name, KeyRule::parse(rule).unwrap_or(KeyRule::Forbid))
            })
            .collect()
    })
}

fn key_lines() -> impl Iterator<Item = (&'static str, &'static str)> {
    DBT_CHARTS_KEYS
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut words = line.split_whitespace();
            (words.next().unwrap_or(""), words.next().unwrap_or(""))
        })
}

/// A parsed document and everything said about it. `value` is present exactly
/// when the report has no errors.
#[derive(Debug, Clone)]
pub struct Parsed<T> {
    pub value: Option<T>,
    pub report: Report,
}

pub fn parse_widget(yaml: &str) -> Parsed<Widget> {
    parse(yaml, walk_widget, check_widget)
}

pub fn parse_dashboard(yaml: &str) -> Parsed<Dashboard> {
    parse(yaml, |_, _| {}, check_dashboard)
}

pub fn parse_theme(yaml: &str) -> Parsed<Theme> {
    parse(yaml, walk_theme, check_theme)
}

pub fn parse_settings(yaml: &str) -> Parsed<Settings> {
    parse(yaml, |_, _| {}, check_settings)
}

fn parse<T: DeserializeOwned>(
    yaml: &str,
    walk: impl Fn(&Value, &mut Report),
    check: impl Fn(&T, &mut Report),
) -> Parsed<T> {
    let mut report = Report::default();
    let value = read(yaml, &mut report, walk);
    if let Some(value) = &value {
        check(value, &mut report);
    }
    let value = value.filter(|_| report.is_accepted());
    Parsed { value, report }
}

fn read<T: DeserializeOwned>(
    yaml: &str,
    report: &mut Report,
    walk: impl Fn(&Value, &mut Report),
) -> Option<T> {
    if yaml.len() > MAX_DOCUMENT_BYTES {
        report.push(Problem::error(
            Code::TooLarge,
            "",
            format!(
                "The document is {} bytes; configuration is limited to {MAX_DOCUMENT_BYTES}.",
                yaml.len()
            ),
        ));
        return None;
    }
    let raw: Value = match serde_yaml::from_str(yaml) {
        Ok(raw) => raw,
        Err(why) => {
            report.push(Problem::error(
                Code::Unreadable,
                "",
                format!("This is not valid YAML: {why}"),
            ));
            return None;
        }
    };
    if depth(&raw) > MAX_DEPTH {
        report.push(Problem::error(
            Code::TooLarge,
            "",
            format!("The document nests deeper than {MAX_DEPTH} levels."),
        ));
        return None;
    }
    scan_text(&raw, "", report);
    refuse_colliding_keys(&raw, "", report);
    walk(&raw, report);
    match serde_path_to_error::deserialize::<_, T>(text_keys(raw)) {
        Ok(typed) => Some(typed),
        Err(why) => {
            let message = why.inner().to_string();
            report.push(Problem::error(
                Code::Unreadable,
                error_path(&why.path().to_string()),
                message,
            ));
            None
        }
    }
}

/// serde_path_to_error writes the root as `.`; problems about the whole
/// document have an empty path.
fn error_path(path: &str) -> String {
    if path == "." {
        String::new()
    } else {
        path.to_string()
    }
}

/// Mapping keys as text: YAML reads `10: Top 10` with a number for a key,
/// and every key in configuration is a name.
fn text_keys(value: Value) -> Value {
    match value {
        Value::Mapping(map) => Value::Mapping(
            map.into_iter()
                .map(|(key, value)| {
                    let key = match key {
                        Value::Number(_) | Value::Bool(_) => {
                            Value::String(key_text(&key))
                        }
                        other => other,
                    };
                    (key, text_keys(value))
                })
                .collect(),
        ),
        Value::Sequence(items) => {
            Value::Sequence(items.into_iter().map(text_keys).collect())
        }
        other => other,
    }
}

/// Keys that differ in YAML but read the same as text, `10` and `"10"`:
/// only one would survive [`text_keys`], silently.
fn refuse_colliding_keys(value: &Value, path: &str, report: &mut Report) {
    match value {
        Value::Mapping(map) => {
            let mut seen = std::collections::HashSet::new();
            for (key, value) in map {
                let text = key_text(key);
                let child = join(path, &text);
                if !seen.insert(text.clone()) {
                    report.push(Problem::error(
                        Code::DuplicateId,
                        &child,
                        format!("'{text}' is given twice."),
                    ));
                }
                refuse_colliding_keys(value, &child, report);
            }
        }
        Value::Sequence(items) => {
            for (position, item) in items.iter().enumerate() {
                refuse_colliding_keys(item, &index(path, position), report);
            }
        }
        _ => {}
    }
}

fn depth(value: &Value) -> usize {
    match value {
        Value::Sequence(items) => {
            1 + items.iter().map(depth).max().unwrap_or(0)
        }
        Value::Mapping(map) => {
            1 + map
                .iter()
                .map(|(key, value)| depth(key).max(depth(value)))
                .max()
                .unwrap_or(0)
        }
        Value::Tagged(tagged) => 1 + depth(&tagged.value),
        _ => 0,
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

fn index(path: &str, position: usize) -> String {
    format!("{path}[{position}]")
}

fn key_text(key: &Value) -> String {
    match key {
        Value::String(text) => text.clone(),
        other => serde_yaml::to_string(other)
            .map(|text| text.trim().to_string())
            .unwrap_or_default(),
    }
}

// -- the content pass ------------------------------------------------------

/// Every string, key or value, in every kind of document.
fn scan_text(value: &Value, path: &str, report: &mut Report) {
    match value {
        Value::String(text) => {
            if let Some(what) = interpreted(text) {
                report.push(Problem::error(
                    Code::ForbiddenValue,
                    path,
                    format!("Text may not contain {what}; it would be interpreted rather than shown."),
                ));
            }
        }
        Value::Sequence(items) => {
            for (position, item) in items.iter().enumerate() {
                scan_text(item, &index(path, position), report);
            }
        }
        Value::Mapping(map) => {
            for (key, value) in map {
                let child = join(path, &key_text(key));
                scan_text(key, &child, report);
                scan_text(value, &child, report);
            }
        }
        Value::Tagged(_) => report.push(Problem::error(
            Code::ForbiddenValue,
            path,
            "YAML tags are not accepted in configuration.",
        )),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// What in `text` would be evaluated or followed by something downstream, if
/// anything.
///
/// Looks for syntax rather than words: "data", "http" and "<" all appear in
/// ordinary prose, while `{{`, `<b`, `javascript:` and `scheme://` do not.
/// Browsers ignore tabs, newlines and other control characters inside a URL,
/// so those are dropped before looking for one.
fn interpreted(text: &str) -> Option<&'static str> {
    if TEMPLATE_MARKERS.iter().any(|marker| text.contains(marker)) {
        return Some("a template");
    }
    let bytes = text.as_bytes();
    let markup = bytes.windows(2).any(|pair| {
        pair[0] == b'<'
            && (pair[1].is_ascii_alphabetic()
                || matches!(pair[1], b'/' | b'!' | b'?'))
    });
    if markup {
        return Some("markup");
    }
    if text.contains("](") {
        return Some("a link");
    }
    let lower = text.to_lowercase();
    let without_controls: String =
        lower.chars().filter(|c| !c.is_control()).collect();
    let squeezed: String = without_controls
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if SCRIPT_SCHEMES
        .iter()
        .any(|scheme| squeezed.contains(scheme))
    {
        return Some("a script URL");
    }
    // `url(` as CSS writes it, not the end of a word such as "curl(".
    let css_url = squeezed.match_indices("url(").any(|(at, _)| {
        squeezed[..at]
            .chars()
            .next_back()
            .map_or(true, |before| !before.is_alphanumeric())
    });
    if css_url {
        return Some("a CSS URL");
    }
    let controls_as_spaces: String = lower
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let points_somewhere = [without_controls.as_str(), &controls_as_spaces]
        .into_iter()
        .flat_map(|text| {
            text.split(|c: char| {
                c.is_whitespace()
                    || matches!(
                        c,
                        '"' | '\'' | '(' | ')' | '`' | '[' | ']' | ',' | ';'
                    )
            })
        })
        .any(address);
    // Browsers drop tabs and newlines inside a URL, but not spaces: "day: //"
    // is prose.
    if points_somewhere || has_url(&without_controls) {
        return Some("a URL");
    }
    None
}

/// Whether one word of text is an address: `mailto:…`, `//host`, `www.…`.
fn address(word: &str) -> bool {
    if let Some((scheme, rest)) = word.split_once(':') {
        // A data URL starts with its media type or a comma; "data:5" is a
        // count.
        let addressed = match scheme {
            "data" => {
                rest.starts_with(|c: char| c.is_ascii_alphabetic() || c == ',')
            }
            _ => !rest.is_empty(),
        };
        if URL_SCHEMES.contains(&scheme) && addressed {
            return true;
        }
    }
    let after = |prefix: &str| {
        word.strip_prefix(prefix).is_some_and(|rest| {
            rest.chars().next().is_some_and(char::is_alphanumeric)
        })
    };
    after("//") || after("www.")
}

/// Whether `text` holds `scheme://`, with a scheme as RFC 3986 spells one.
fn has_url(text: &str) -> bool {
    let bytes = text.as_bytes();
    text.match_indices("://").any(|(at, _)| {
        let scheme: Vec<u8> = bytes[..at]
            .iter()
            .rev()
            .take_while(|byte| {
                byte.is_ascii_alphanumeric()
                    || matches!(byte, b'+' | b'.' | b'-')
            })
            .copied()
            .collect();
        scheme.last().is_some_and(u8::is_ascii_alphabetic)
    })
}

/// What a walk through dbt Charts YAML checks references against.
struct Governed<'a> {
    /// The widget's queries; `None` where nothing may query at all.
    queries: Option<&'a [String]>,
    /// The charts the widget defines, which its layout may place.
    charts: &'a [String],
}

/// A widget's `chart`: only board layout and paint, placing only the widget's
/// own charts and reading only its own queries.
fn walk_widget(raw: &Value, report: &mut Report) {
    let Some(Value::Mapping(fields)) = raw.get("chart") else {
        return;
    };
    let queries: Vec<String> = match (raw.get("query"), raw.get("queries")) {
        (Some(_), _) => vec![DEFAULT_QUERY_NAME.to_string()],
        (None, Some(Value::Mapping(queries))) => {
            queries.keys().map(key_text).collect()
        }
        _ => Vec::new(),
    };
    let charts: Vec<String> = match fields.get("charts") {
        Some(Value::Mapping(charts)) => charts.keys().map(key_text).collect(),
        _ => Vec::new(),
    };
    let governed = Governed {
        queries: Some(&queries),
        charts: &charts,
    };
    for (key, value) in fields {
        let key = key_text(key);
        let path = join("chart", &key);
        match key.as_str() {
            "charts" => walk_charts(value, &path, &governed, report),
            _ if CHART_FIELDS.contains(&key.as_str()) => {
                walk_board_field(&key, value, &path, &governed, report)
            }
            _ if NAVIGATING_FIELDS.contains(&key.as_str()) => {
                report.push(navigating(&key, &path))
            }
            _ => report.push(Problem::error(
                Code::ForbiddenKey,
                path,
                format!(
                    "A widget's chart may only set {}; '{key}' is supplied by the platform or not allowed.",
                    CHART_FIELDS.join(", ")
                ),
            )),
        }
    }
}

fn navigating(key: &str, path: &str) -> Problem {
    Problem::error(
        Code::ForbiddenKey,
        path,
        format!("'{key}' switches views by following a link, which a widget's frame cannot do; give the widget a selector instead."),
    )
}

/// A theme's `style`: paint only.
fn walk_theme(raw: &Value, report: &mut Report) {
    if let Some(style) = raw.get("style") {
        let governed = Governed {
            queries: None,
            charts: &[],
        };
        walk_board_style(style, "style", &governed, report);
    }
}

/// A board's `style`, from a widget, a board in its layout or a theme.
fn walk_board_style(
    value: &Value,
    path: &str,
    governed: &Governed,
    report: &mut Report,
) {
    if let Value::Mapping(fields) = value {
        for (key, _) in fields {
            let key = key_text(key);
            if !BOARD_STYLE_FIELDS.contains(&key.as_str()) {
                refuse_other_fields(
                    &key,
                    BOARD_STYLE_FIELDS,
                    "A board's style",
                    &join(path, &key),
                    report,
                );
            }
        }
    }
    walk_dbt(value, path, "style", governed, report);
}

/// `charts`: each one defined here, in full.
fn walk_charts(
    value: &Value,
    path: &str,
    governed: &Governed,
    report: &mut Report,
) {
    let Value::Mapping(charts) = value else {
        report.push(Problem::error(
            Code::InvalidValue,
            path,
            "`charts` maps chart names to chart definitions.",
        ));
        return;
    };
    for (name, chart) in charts {
        let name = key_text(name);
        let child = join(path, &name);
        if chart.is_mapping() {
            walk_dbt(chart, &child, &name, governed, report);
        } else {
            report.push(Problem::error(
                Code::ForbiddenValue,
                child,
                "A chart is defined here in full; it cannot be taken from another file.",
            ));
        }
    }
}

/// One of the fields a board uses to lay out its charts, or its style.
fn walk_board_field(
    key: &str,
    value: &Value,
    path: &str,
    governed: &Governed,
    report: &mut Report,
) {
    match key {
        "rows" | "cols" => walk_layout(value, path, governed, report),
        "grid" => walk_grid(value, path, governed, report),
        "style" => walk_board_style(value, path, governed, report),
        "visible" => check_flag(key, value, "", path, report),
        _ => walk_dbt(value, path, key, governed, report),
    }
}

/// `rows` or `cols`: chart names, or boards nesting further layout.
fn walk_layout(
    value: &Value,
    path: &str,
    governed: &Governed,
    report: &mut Report,
) {
    let Value::Sequence(items) = value else {
        if !value.is_null() {
            report.push(Problem::error(
                Code::InvalidValue,
                path,
                "A layout lists the names of the widget's charts.",
            ));
        }
        return;
    };
    for (position, item) in items.iter().enumerate() {
        let child = index(path, position);
        match item {
            Value::String(name) => place(name, &child, governed, report),
            Value::Mapping(_) => {
                walk_nested_board(item, &child, governed, report)
            }
            _ => report.push(Problem::error(
                Code::InvalidValue,
                child,
                "A layout lists the names of the widget's charts.",
            )),
        }
    }
}

/// A chart placed by name: one of the widget's own.
fn place(name: &str, path: &str, governed: &Governed, report: &mut Report) {
    if !governed.charts.iter().any(|chart| chart == name) {
        report.push(Problem::error(
            Code::DanglingReference,
            path,
            format!(
                "'{name}' is not one of this widget's charts ({}). A layout places the widget's own charts by name.",
                governed.charts.join(", ")
            ),
        ));
    }
}

/// A board inside the layout: more layout, and how it looks.
fn walk_nested_board(
    value: &Value,
    path: &str,
    governed: &Governed,
    report: &mut Report,
) {
    let Value::Mapping(fields) = value else {
        report.push(Problem::error(
            Code::InvalidValue,
            path,
            "A board inside the layout is a mapping of rows, cols or grid.",
        ));
        return;
    };
    // dbt Charts reads a one-key mapping whose value is a chart as that chart,
    // defined inline: `{title: {type: bar, ...}}`. A board that lays out
    // something, with every field of the type it takes, cannot be read so.
    let lays_out = fields
        .keys()
        .any(|key| LAYOUT_FIELDS.contains(&key_text(key).as_str()));
    if !lays_out {
        report.push(Problem::error(
            Code::InvalidValue,
            path,
            "A board inside the layout places the widget's charts with rows, cols or grid.",
        ));
    }
    for (key, value) in fields {
        let key = key_text(key);
        let child = join(path, &key);
        let scalar =
            matches!(value, Value::String(_) | Value::Number(_) | Value::Null);
        match key.as_str() {
            "title" if !matches!(value, Value::String(_) | Value::Null) => {
                report.push(Problem::error(
                    Code::InvalidValue,
                    &child,
                    "A board's title is text.",
                ))
            }
            "card_gap" | "height" | "width" if !scalar => {
                report.push(Problem::error(
                    Code::InvalidValue,
                    &child,
                    format!("'{key}' is a size."),
                ))
            }
            _ if NESTED_BOARD_FIELDS.contains(&key.as_str()) => {
                walk_board_field(&key, value, &child, governed, report)
            }
            _ if NAVIGATING_FIELDS.contains(&key.as_str()) => {
                report.push(navigating(&key, &child))
            }
            "charts" | "type" => report.push(Problem::error(
                Code::ForbiddenKey,
                child,
                "Define charts once, under chart.charts, and place them here by name.",
            )),
            _ => refuse_other_fields(
                &key,
                NESTED_BOARD_FIELDS,
                "A board inside the layout",
                &child,
                report,
            ),
        }
    }
}

fn refuse_other_fields(
    key: &str,
    allowed: &[&str],
    what: &str,
    path: &str,
    report: &mut Report,
) {
    report.push(Problem::error(
        Code::ForbiddenKey,
        path,
        format!(
            "{what} may set {}; '{key}' is not allowed.",
            allowed.join(", ")
        ),
    ));
}

fn walk_grid(
    value: &Value,
    path: &str,
    governed: &Governed,
    report: &mut Report,
) {
    let Value::Mapping(fields) = value else {
        report.push(Problem::error(
            Code::InvalidValue,
            path,
            "A grid is a mapping of `columns` and `items`.",
        ));
        return;
    };
    for (key, value) in fields {
        let key = key_text(key);
        let child = join(path, &key);
        match key.as_str() {
            "items" => {
                let Value::Sequence(cells) = value else {
                    report.push(Problem::error(
                        Code::InvalidValue,
                        &child,
                        "A grid's items are a list of cells.",
                    ));
                    continue;
                };
                for (position, cell) in cells.iter().enumerate() {
                    walk_grid_item(
                        cell,
                        &index(&child, position),
                        governed,
                        report,
                    );
                }
            }
            _ if GRID_FIELDS.contains(&key.as_str()) => {
                walk_dbt(value, &child, &key, governed, report)
            }
            _ => {
                refuse_other_fields(&key, GRID_FIELDS, "A grid", &child, report)
            }
        }
    }
}

fn walk_grid_item(
    value: &Value,
    path: &str,
    governed: &Governed,
    report: &mut Report,
) {
    let Value::Mapping(fields) = value else {
        report.push(Problem::error(
            Code::InvalidValue,
            path,
            "A grid cell is a mapping with the `item` it holds and where.",
        ));
        return;
    };
    if !fields.contains_key("item") {
        report.push(Problem::error(
            Code::InvalidValue,
            path,
            "A grid cell names the `item` it holds.",
        ));
    }
    for (key, value) in fields {
        let key = key_text(key);
        let child = join(path, &key);
        match (key.as_str(), value) {
            ("item", Value::String(name)) => {
                place(name, &child, governed, report)
            }
            ("item", Value::Mapping(_)) => {
                walk_nested_board(value, &child, governed, report)
            }
            ("item", _) => report.push(Problem::error(
                Code::InvalidValue,
                child,
                "A grid cell holds the name of one of the widget's charts.",
            )),
            _ if GRID_ITEM_FIELDS.contains(&key.as_str()) => {
                walk_dbt(value, &child, &key, governed, report)
            }
            _ => refuse_other_fields(
                &key,
                GRID_ITEM_FIELDS,
                "A grid cell",
                &child,
                report,
            ),
        }
    }
}

/// Chart definitions and styles. Only keys dbt Charts defines and the
/// policy accepts get through; `parent` is the key this value sits under,
/// which decides what `source` and `text` mean.
fn walk_dbt(
    value: &Value,
    path: &str,
    parent: &str,
    governed: &Governed,
    report: &mut Report,
) {
    match value {
        Value::Mapping(map) => {
            for (key, value) in map {
                let key = key_text(key);
                let child = join(path, &key);
                walk_dbt_key(&key, value, &child, parent, governed, report);
            }
        }
        Value::Sequence(items) => {
            for (position, item) in items.iter().enumerate() {
                walk_dbt(
                    item,
                    &index(path, position),
                    parent,
                    governed,
                    report,
                );
            }
        }
        Value::String(text) => check_dbt_text(text, path, parent, report),
        Value::Number(number) => check_dbt_number(number, path, parent, report),
        Value::Bool(_) | Value::Null | Value::Tagged(_) => {}
    }
}

/// Whether `path` is inside a `style`, where text is paint.
fn in_style(path: &str) -> bool {
    path.split('.')
        .any(|segment| segment == "style" || segment.starts_with("style["))
}

/// Text in dbt Charts YAML, where some of it ends up inside an SVG attribute
/// or a CSS rule that the engine does not escape.
fn check_dbt_text(text: &str, path: &str, parent: &str, report: &mut Report) {
    let refuse = |message: String, report: &mut Report| {
        report.push(Problem::error(Code::ForbiddenValue, path, message))
    };
    if text.contains(['"', '\\']) {
        refuse(
            "Chart text may not contain a double quote or a backslash; use typographic quotes (“ ”).".to_string(),
            report,
        );
        return;
    }
    let style = in_style(path);
    if style && text.contains([';', '{', '}']) {
        refuse(
            "A style value is one value; ';', '{' and '}' would start another."
                .to_string(),
            report,
        );
        return;
    }
    let colour = COLOUR_KEYS.contains(&parent)
        || parent == "thresholds"
        || (parent == "values" && path.contains(".category_colors."));
    if style && colour && !is_colour(text) {
        refuse(
            format!("'{text}' is not a colour: use #rrggbb, rgb(), hsl(), a colour name or a theme token such as dbt-grays.muted."),
            report,
        );
    }
    if AFFIX_KEYS.contains(&parent) && text.chars().any(|c| c.is_ascii_digit())
    {
        refuse(
            format!("'{parent}' is written beside a figure, so it may not contain digits: they would read as part of the figure."),
            report,
        );
    }
}

/// A CSS colour, or a name the engine resolves to one: `#1f77b4`,
/// `rgb(10, 20, 30)`, `transparent`, `dbt-grays.muted`, `category[2]`.
fn is_colour(text: &str) -> bool {
    if let Some(hex) = text.strip_prefix('#') {
        return matches!(hex.len(), 3 | 4 | 6 | 8)
            && hex.chars().all(|c| c.is_ascii_hexdigit());
    }
    for function in ["rgb(", "rgba(", "hsl(", "hsla("] {
        if let Some(inner) = text
            .strip_prefix(function)
            .and_then(|rest| rest.strip_suffix(')'))
        {
            return inner.chars().all(|c| {
                c.is_ascii_digit()
                    || matches!(c, '.' | ',' | '%' | ' ' | '/' | '-')
            });
        }
    }
    let (name, position) = match text.strip_suffix(']') {
        Some(rest) => match rest.split_once('[') {
            Some((name, position)) => (name, Some(position)),
            None => return false,
        },
        None => (text, None),
    };
    position.map_or(true, |position| {
        !position.is_empty() && position.chars().all(|c| c.is_ascii_digit())
    }) && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// A number in dbt Charts YAML. Sizes and counts are bounded: the engine
/// allocates or loops by them, and one large enough stops the renderer.
fn check_dbt_number(
    number: &serde_yaml::Number,
    path: &str,
    parent: &str,
    report: &mut Report,
) {
    let value = number.as_f64().unwrap_or(f64::INFINITY);
    if COUNT_KEYS.contains(&parent) && value.abs() > MAX_DBT_COUNT {
        report.push(Problem::error(
            Code::TooLarge,
            path,
            format!("'{parent}' is at most {MAX_DBT_COUNT}: the renderer draws that many of something."),
        ));
    } else if !DATA_VALUED_KEYS.contains(&parent)
        && !(value.abs() <= MAX_DBT_NUMBER)
    {
        report.push(Problem::error(
            Code::TooLarge,
            path,
            format!("Sizes in a chart are at most {MAX_DBT_NUMBER}."),
        ));
    }
}

fn walk_dbt_key(
    key: &str,
    value: &Value,
    path: &str,
    parent: &str,
    governed: &Governed,
    report: &mut Report,
) {
    let forbidden =
        |message: String| Problem::error(Code::ForbiddenKey, path, message);
    let Some(rule) = dbt_keys().get(key) else {
        report.push(forbidden(format!(
            "'{key}' is not a dbt Charts field the platform accepts."
        )));
        return;
    };
    match rule {
        KeyRule::Allow => walk_dbt(value, path, key, governed, report),
        KeyRule::Forbid => report.push(forbidden(forbidden_because(key))),
        KeyRule::Layout => report.push(forbidden(format!(
            "'{key}' belongs in the widget's layout."
        ))),
        KeyRule::Names => match value {
            Value::Mapping(map) => {
                for (name, value) in map {
                    let name = key_text(name);
                    let child = join(path, &name);
                    if name.contains(['"', '\\']) {
                        report.push(Problem::error(
                            Code::ForbiddenValue,
                            &child,
                            "A name may not contain a double quote or a backslash.",
                        ));
                    }
                    walk_dbt(value, &child, key, governed, report);
                }
            }
            _ => walk_dbt(value, path, key, governed, report),
        },
        KeyRule::Query => walk_query(value, path, governed, report),
        KeyRule::ChartType => refuse_typed_in_chart(value, path, report),
        KeyRule::Flag => check_flag(key, value, parent, path, report),
        KeyRule::Geo => check_geo(value, path, report),
        KeyRule::Source if parent == "basemap" => check_geo(value, path, report),
        KeyRule::Source if COLUMN_SOURCE_PARENTS.contains(&parent) => {
            check_column(value, path, report)
        }
        KeyRule::Source => report.push(forbidden(
            "A chart reads its widget's governed queries; it cannot name a source of its own.".to_string(),
        )),
        KeyRule::Text => match value {
            Value::String(text) if PLAIN_TEXT_PARENTS.contains(&parent) => {
                check_dbt_text(text, path, parent, report)
            }
            Value::String(_) => report.push(forbidden(
                "Markdown text is not accepted; titles and labels say what a chart shows.".to_string(),
            )),
            _ => walk_dbt(value, path, key, governed, report),
        },
    }
}

/// Why a key the list forbids is refused.
fn forbidden_because(key: &str) -> String {
    match key {
        "step" => "A tick step turns the data's range into any number of ticks; set `ticks.count` instead.".to_string(),
        "aggregate" => "The renderer does not aggregate: every figure comes from the data source, counted once.".to_string(),
        "footer" | "timestamp" => "The dashboard says when its figures are from; the engine's footer and freshness line stay off.".to_string(),
        _ => format!("'{key}' would bring data, a destination, markup or an expression from outside the governed sources."),
    }
}

/// `visible` and `enabled`: decided by the author, never by an expression
/// or a query probe the renderer would evaluate. `auto` is one: the engine
/// reads it as a template variable.
fn check_flag(
    key: &str,
    value: &Value,
    parent: &str,
    path: &str,
    report: &mut Report,
) {
    if !matches!(value, Value::Bool(_) | Value::Null) {
        report.push(Problem::error(
            Code::ForbiddenValue,
            path,
            format!("'{key}' is true or false; an expression or a query probe would be evaluated."),
        ));
    } else if parent == "total" && *value == Value::Bool(true) {
        report.push(Problem::error(
            Code::ForbiddenValue,
            path,
            "The renderer does not add slices up: groups need not sum to the scope, so a total comes from a summary query.",
        ));
    }
}

fn check_geo(value: &Value, path: &str, report: &mut Report) {
    let bundled = match value {
        Value::Null => true,
        Value::String(name) => GEO_SOURCES.contains(&name.as_str()),
        _ => false,
    };
    if !bundled {
        report.push(Problem::error(
            Code::ForbiddenValue,
            path,
            format!(
                "Maps draw the geometry bundled with the platform: {}.",
                GEO_SOURCES.join(", ")
            ),
        ));
    }
}

fn check_column(value: &Value, path: &str, report: &mut Report) {
    let column = match value {
        Value::String(name) => {
            !name.is_empty()
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    };
    if !column {
        report.push(Problem::error(
            Code::ForbiddenValue,
            path,
            "A support table reads a column of the widget's query, by name.",
        ));
    }
}

fn walk_query(
    value: &Value,
    path: &str,
    governed: &Governed,
    report: &mut Report,
) {
    let Some(queries) = governed.queries else {
        report.push(Problem::error(
            Code::ForbiddenKey,
            path,
            "A theme styles charts; it does not query anything.",
        ));
        return;
    };
    match value {
        Value::String(name) if queries.contains(name) => {}
        Value::String(name) => report.push(Problem::error(
            Code::DanglingReference,
            path,
            format!(
                "'{name}' is not one of this widget's queries ({}). A chart reads its widget's governed queries by name.",
                queries.join(", ")
            ),
        )),
        Value::Mapping(_) => {
            report.push(Problem::error(
                Code::ForbiddenKey,
                path,
                "A chart reads one of its widget's governed queries by name; it cannot define its own.",
            ));
            walk_dbt(value, path, "query", governed, report);
        }
        _ => report.push(Problem::error(
            Code::InvalidValue,
            path,
            "A chart's query is the name of one of its widget's queries.",
        )),
    }
}

fn refuse_typed_in_chart(value: &Value, path: &str, report: &mut Report) {
    if let Value::String(chart_type) = value {
        if FORBIDDEN_CHART_TYPES.contains(&chart_type.as_str()) {
            report.push(Problem::error(
                Code::ForbiddenValue,
                path,
                format!("A '{chart_type}' chart shows typed-in content, not governed data."),
            ));
        }
    }
}

// -- the semantic pass -----------------------------------------------------

/// Widget, dashboard and theme ids: `turnout-by-group`, `req-0260`.
pub(crate) fn is_id(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|first| {
        first.is_ascii_lowercase() || first.is_ascii_digit()
    }) && text.len() <= 64
        && chars.all(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || matches!(c, '-' | '_')
        })
}

/// Selector, query and dimension names: `breakdown`, `age_band`.
fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && text.len() <= 40
        && chars
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Option values: whatever a query parameter takes, which is a word.
fn is_option_value(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// `YYYY-MM-DD`, and a day the calendar has.
fn is_calendar_date(text: &str) -> bool {
    let shaped = text.len() == 10
        && text.bytes().enumerate().all(|(at, byte)| match at {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        });
    shaped && chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").is_ok()
}

/// `UTC`, or an IANA `Area/Location` name. Whether the zone exists is checked
/// where a zone database is at hand; the browser build carries none.
fn is_time_zone_name(text: &str) -> bool {
    if text == "UTC" {
        return true;
    }
    let parts: Vec<&str> = text.split('/').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.chars().all(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '+')
                })
        })
}

fn require_id(id: &str, path: &str, report: &mut Report) {
    if !is_id(id) {
        report.push(Problem::error(
            Code::InvalidId,
            path,
            format!("'{id}' is not an id: use lowercase letters, digits, '-' and '_'."),
        ));
    }
}

fn require_name(name: &str, path: &str, report: &mut Report) {
    if !is_name(name) {
        report.push(Problem::error(
            Code::InvalidId,
            path,
            format!("'{name}' is not a name: start with a lowercase letter; use lowercase letters, digits and '_'."),
        ));
    }
}

fn require_text(text: &str, path: &str, report: &mut Report) {
    if text.trim().is_empty() {
        report.push(Problem::error(
            Code::InvalidValue,
            path,
            "This may not be empty.",
        ));
    }
}

fn check_widget(widget: &Widget, report: &mut Report) {
    require_id(&widget.id, "id", report);
    require_text(&widget.title, "title", report);
    if let Some(description) = &widget.description {
        require_text(description, "description", report);
    }
    for (position, requirement) in widget.requirements.iter().enumerate() {
        require_text(requirement, &index("requirements", position), report);
    }
    let spec = widget.source.spec();
    if let Some(follows) = &widget.follows {
        refuse_duplicates(follows, "follows", report);
        for (position, selector) in follows.iter().enumerate() {
            if !spec.builtin_dimensions.contains(&selector.dimension()) {
                report.push(Problem::error(
                    Code::UnsupportedBySource,
                    index("follows", position),
                    format!("{} cannot be narrowed by {selector}.", spec.id),
                ));
            }
        }
    }
    if let Some(height) = widget.height {
        if !(MIN_WIDGET_HEIGHT..=MAX_WIDGET_HEIGHT).contains(&height) {
            report.push(Problem::error(
                Code::InvalidValue,
                "height",
                format!("A widget is between {MIN_WIDGET_HEIGHT} and {MAX_WIDGET_HEIGHT} pixels high."),
            ));
        }
    }

    let declared: Vec<&String> = widget.selectors.keys().collect();
    for (position, (name, selector)) in widget.selectors.iter().enumerate() {
        check_selector(
            widget,
            name,
            selector,
            &declared[..position],
            &join("selectors", name),
            report,
        );
    }

    match (&widget.query, widget.queries.is_empty()) {
        (Some(_), false) => report.push(Problem::error(
            Code::TemplateParameter,
            "queries",
            "Give either one `query` or named `queries`, not both.",
        )),
        (None, true) => report.push(Problem::error(
            Code::TemplateParameter,
            "query",
            "A widget needs a query.",
        )),
        _ => {}
    }
    if let Some(query) = &widget.query {
        check_query(widget, &spec, query, "query", report);
    }
    for (name, query) in &widget.queries {
        let path = join("queries", name);
        require_name(name, &path, report);
        check_query(widget, &spec, query, &path, report);
    }

    match &widget.chart {
        Value::Mapping(chart) => {
            let charts = chart
                .get("charts")
                .and_then(Value::as_mapping)
                .map_or(0, |charts| charts.len());
            if charts > MAX_CHARTS_PER_WIDGET {
                report.push(Problem::error(
                    Code::TooLarge,
                    "chart.charts",
                    format!("A widget draws at most {MAX_CHARTS_PER_WIDGET} charts."),
                ));
            }
        }
        _ => report.push(Problem::error(
            Code::Unreadable,
            "chart",
            "A widget's chart is dbt Charts YAML: a mapping with `charts` and a layout.",
        )),
    }
}

fn refuse_duplicates<T: PartialEq + std::fmt::Display>(
    items: &[T],
    path: &str,
    report: &mut Report,
) {
    for (position, item) in items.iter().enumerate() {
        if items[..position].contains(item) {
            report.push(Problem::error(
                Code::DuplicateId,
                index(path, position),
                format!("'{item}' is listed twice."),
            ));
        }
    }
}

fn check_selector(
    widget: &Widget,
    name: &str,
    selector: &Selector,
    earlier: &[&String],
    path: &str,
    report: &mut Report,
) {
    require_name(name, path, report);
    require_text(&selector.label, &join(path, "label"), report);
    let dynamic = selector.options_from.is_some();

    match (dynamic, selector.options.is_empty()) {
        (true, false) => report.push(Problem::error(
            Code::InvalidValue,
            join(path, "options"),
            "A selector lists its options or takes them from the data, not both.",
        )),
        (false, true) => report.push(Problem::error(
            Code::InvalidValue,
            join(path, "options"),
            "A selector needs options.",
        )),
        _ => {}
    }
    if selector.control == SelectorControl::Toggle
        && (dynamic || selector.options.len() != 2)
    {
        report.push(Problem::error(
            Code::InvalidValue,
            join(path, "options"),
            "A toggle switches between exactly two listed options.",
        ));
    }
    for (value, label) in &selector.options {
        let option_path = join(&join(path, "options"), value);
        if !is_option_value(value) {
            report.push(Problem::error(
                Code::InvalidId,
                &option_path,
                format!("'{value}' is not an option value: use letters, digits, '_', '-' and '.'."),
            ));
        }
        require_text(label, &option_path, report);
    }

    let default_path = join(path, "default");
    match (&selector.default, dynamic) {
        (None, false) => report.push(Problem::error(
            Code::InvalidValue,
            default_path,
            "A selector with listed options needs a default.",
        )),
        (Some(default), false) if !selector.options.contains_key(default) => {
            report.push(Problem::error(
                Code::UnknownOption,
                default_path,
                format!("'{default}' is not one of this selector's options."),
            ))
        }
        (Some(_), true) => report.push(Problem::error(
            Code::InvalidValue,
            default_path,
            "Options taken from the data default to the latest one.",
        )),
        _ => {}
    }

    if let Some(maps) = &selector.maps {
        let maps_path = join(path, "maps");
        if dynamic {
            report.push(Problem::error(
                Code::InvalidValue,
                &maps_path,
                "Options taken from the data cannot be mapped.",
            ));
        }
        for value in selector.options.keys() {
            if !maps.contains_key(value) {
                report.push(Problem::error(
                    Code::UnknownOption,
                    join(&maps_path, value),
                    format!("Option '{value}' has no mapping."),
                ));
            }
        }
        for value in maps.keys() {
            if !selector.options.contains_key(value) {
                report.push(Problem::error(
                    Code::UnknownOption,
                    join(&maps_path, value),
                    format!("'{value}' is not one of this selector's options."),
                ));
            }
        }
    }

    if let Some(condition) = &selector.when {
        let when_path = join(path, "when");
        let target = earlier
            .iter()
            .find(|earlier| earlier.as_str() == condition.selector)
            .and_then(|name| widget.selectors.get(name.as_str()));
        match target {
            None => report.push(Problem::error(
                Code::UnknownSelector,
                join(&when_path, "selector"),
                format!(
                    "'{}' is not a selector declared before this one.",
                    condition.selector
                ),
            )),
            Some(target) => {
                if condition.one_of.is_empty() {
                    report.push(Problem::error(
                        Code::InvalidValue,
                        join(&when_path, "in"),
                        "A condition lists at least one value.",
                    ));
                }
                if target.options_from.is_none() {
                    for (position, value) in condition.one_of.iter().enumerate()
                    {
                        if !target.options.contains_key(value) {
                            report.push(Problem::error(
                                Code::UnknownOption,
                                index(&join(&when_path, "in"), position),
                                format!(
                                    "'{value}' is not an option of '{}'.",
                                    condition.selector
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Whether a parameter may take its value from a selector whose options come
/// from the data.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DynamicSource {
    Accepted,
    Refused,
}

/// Every value `param` can take: itself if literal, else each of the options
/// of the selector it names, mapped. Values that do not fit `T` are reported.
fn possible_values<T: DeserializeOwned + Clone>(
    widget: &Widget,
    param: &Param<T>,
    path: &str,
    dynamic: DynamicSource,
    report: &mut Report,
) -> Vec<T> {
    let reference = match param {
        Param::Literal(value) => return vec![value.clone()],
        Param::Selector(reference) => reference,
    };
    let Some(selector) = widget.selectors.get(&reference.selector) else {
        report.push(Problem::error(
            Code::UnknownSelector,
            join(path, "selector"),
            format!(
                "'{}' is not a selector of this widget.",
                reference.selector
            ),
        ));
        return Vec::new();
    };
    if selector.options_from.is_some() {
        if dynamic == DynamicSource::Refused {
            report.push(Problem::error(
                Code::TemplateParameter,
                path,
                format!(
                    "'{}' takes its options from the data, which only a day can.",
                    reference.selector
                ),
            ));
        }
        return Vec::new();
    }
    let mut values = Vec::new();
    for option in selector.options.keys() {
        match option_value::<T>(selector, option) {
            Ok(value) => values.push(value),
            Err(why) => report.push(Problem::error(
                Code::TemplateParameter,
                path,
                format!(
                    "Option '{option}' of '{}' does not fit this parameter: {why}",
                    reference.selector
                ),
            )),
        }
    }
    values
}

/// What choosing `option` passes to the parameter the selector feeds.
fn option_value<T: DeserializeOwned>(
    selector: &Selector,
    option: &str,
) -> Result<T, serde_yaml::Error> {
    selector.option_value(option)
}

/// A parameter a query cannot run without must not come from a selector
/// that can be hidden: hidden, it feeds nothing.
fn refuse_conditional<T>(
    widget: &Widget,
    param: Option<&Param<T>>,
    path: &str,
    report: &mut Report,
) {
    let Some(Param::Selector(reference)) = param else {
        return;
    };
    let conditional = widget
        .selectors
        .get(&reference.selector)
        .is_some_and(|selector| selector.when.is_some());
    if conditional {
        report.push(Problem::error(
            Code::TemplateParameter,
            path,
            format!(
                "'{}' is hidden while its condition does not hold, and the query cannot run without this parameter.",
                reference.selector
            ),
        ));
    }
}

fn check_measure(
    spec: &SourceSpec,
    measure: Measure,
    path: &str,
    report: &mut Report,
) {
    if !spec.has_measure(measure) {
        report.push(Problem::error(
            Code::UnsupportedBySource,
            path,
            format!("{} has no measure '{measure}'.", spec.id),
        ));
    }
}

fn check_group(spec: &SourceSpec, name: &str, path: &str, report: &mut Report) {
    if !spec.may_group_by(name) {
        report.push(Problem::error(
            Code::UnsupportedBySource,
            path,
            format!("{} cannot be grouped by '{name}'.", spec.id),
        ));
    }
}

fn misplaced(path: &str, what: &str, template: QueryTemplate) -> Problem {
    Problem::error(
        Code::TemplateParameter,
        path,
        format!("The {template} template takes no {what}."),
    )
}

fn check_query(
    widget: &Widget,
    spec: &SourceSpec,
    query: &Query,
    path: &str,
    report: &mut Report,
) {
    let template = query.template;
    if !spec.has_template(template) {
        report.push(Problem::error(
            Code::UnsupportedBySource,
            join(path, "template"),
            format!("{} has no {template} template.", spec.id),
        ));
    }

    if let Some(measures) = &query.measures {
        let measures_path = join(path, "measures");
        let sets = possible_values(
            widget,
            measures,
            &measures_path,
            DynamicSource::Refused,
            report,
        );
        for set in sets {
            if set.is_empty() {
                report.push(Problem::error(
                    Code::TemplateParameter,
                    &measures_path,
                    "List at least one measure.",
                ));
            }
            for (position, measure) in set.iter().enumerate() {
                let measure_path = index(&measures_path, position);
                if set[..position].contains(measure) {
                    report.push(Problem::error(
                        Code::DuplicateId,
                        &measure_path,
                        format!("'{measure}' is listed twice; each measure is one column."),
                    ));
                }
                check_measure(spec, *measure, &measure_path, report);
                if template == QueryTemplate::Timeseries
                    && spec.has_measure(*measure)
                    && !spec.counts_per_hour(*measure)
                {
                    report.push(Problem::error(
                        Code::UnsupportedBySource,
                        &measure_path,
                        format!("{} counts '{measure}' as a total, not per hour, so a timeseries cannot show it.", spec.id),
                    ));
                }
            }
        }
    }
    if let Some(ratio) = &query.ratio {
        let ratio_path = join(path, "ratio");
        for ratio in possible_values(
            widget,
            ratio,
            &ratio_path,
            DynamicSource::Refused,
            report,
        ) {
            check_measure(spec, ratio.numerator(), &ratio_path, report);
            check_measure(spec, ratio.denominator(), &ratio_path, report);
        }
    }
    let series_like = matches!(
        template,
        QueryTemplate::Timeseries | QueryTemplate::ByMeasure
    );
    if series_like {
        if query.ratio.is_some() {
            report.push(misplaced(&join(path, "ratio"), "ratio", template));
        }
        if query.measures.is_none() {
            report.push(Problem::error(
                Code::TemplateParameter,
                join(path, "measures"),
                format!("The {template} template needs `measures`."),
            ));
        }
    } else if query.measures.is_none()
        && query.ratio.is_none()
        && template != QueryTemplate::ByPost
    {
        report.push(Problem::error(
            Code::TemplateParameter,
            path,
            "Say what to count: `measures`, a `ratio`, or both.",
        ));
    }

    let group_path = join(path, "group_by");
    match (&query.group_by, template) {
        (None, QueryTemplate::ByGroup) => report.push(Problem::error(
            Code::TemplateParameter,
            &group_path,
            "The by_group template needs `group_by`.",
        )),
        (Some(_), QueryTemplate::ByGroup) => {}
        (Some(_), other) => {
            report.push(misplaced(&group_path, "group_by", other))
        }
        (None, _) => {}
    }
    let mut builtin_group = false;
    if let Some(group_by) = &query.group_by {
        for name in possible_values(
            widget,
            group_by,
            &group_path,
            DynamicSource::Refused,
            report,
        ) {
            builtin_group |= name.parse::<BuiltinDimension>().is_ok();
            check_group(spec, &name, &group_path, report);
        }
    }

    // Filters narrow a partition of the scope's voters; Posts, series and
    // built-in groups are counted on their own and have none to narrow.
    if !query.filters.is_empty() {
        let unfilterable = match template {
            QueryTemplate::ByPost | QueryTemplate::Timeseries => {
                Some(format!("The {template} template takes no filters."))
            }
            _ if builtin_group => Some(
                "Groups by region, Post, country, reason or state are counted on their own; they cannot be filtered by voter dimensions.".to_string(),
            ),
            _ => None,
        };
        if let Some(message) = unfilterable {
            report.push(Problem::error(
                Code::TemplateParameter,
                join(path, "filters"),
                message,
            ));
        }
    }
    refuse_conditional(
        widget,
        query.measures.as_ref(),
        &join(path, "measures"),
        report,
    );
    refuse_conditional(
        widget,
        query.ratio.as_ref(),
        &join(path, "ratio"),
        report,
    );
    refuse_conditional(widget, query.group_by.as_ref(), &group_path, report);
    refuse_conditional(
        widget,
        query.grain.as_ref(),
        &join(path, "grain"),
        report,
    );

    for (dimension, values) in &query.filters {
        let filter_path = join(&join(path, "filters"), dimension);
        if dimension.parse::<BuiltinDimension>().is_ok() {
            report.push(Problem::error(
                Code::UnsupportedBySource,
                &filter_path,
                format!("Narrow by '{dimension}' with the dashboard selectors; filters apply to voter dimensions."),
            ));
        } else {
            check_group(spec, dimension, &filter_path, report);
        }
        for set in possible_values(
            widget,
            values,
            &filter_path,
            DynamicSource::Refused,
            report,
        )
        .into_iter()
        .flatten()
        {
            if set.is_empty() {
                report.push(Problem::error(
                    Code::TemplateParameter,
                    &filter_path,
                    "A filter keeps at least one value.",
                ));
            }
        }
    }

    let grain_path = join(path, "grain");
    if let Some(grain) = &query.grain {
        possible_values(
            widget,
            grain,
            &grain_path,
            DynamicSource::Refused,
            report,
        );
    }
    match (&query.grain, template) {
        (None, QueryTemplate::Timeseries) => report.push(Problem::error(
            Code::TemplateParameter,
            &grain_path,
            "The timeseries template needs a `grain`.",
        )),
        (Some(_), QueryTemplate::Timeseries) | (None, _) => {}
        (Some(_), other) => report.push(misplaced(&grain_path, "grain", other)),
    }

    if let Some(day) = &query.day {
        let day_path = join(path, "day");
        if template != QueryTemplate::Timeseries {
            report.push(misplaced(&day_path, "day", template));
        } else if matches!(query.grain, Some(Param::Literal(TimeGrain::Day))) {
            report.push(Problem::error(
                Code::TemplateParameter,
                &day_path,
                "A day narrows an hourly series; this one is daily.",
            ));
        }
        if let (Param::Selector(picker), Some(Param::Selector(grain))) =
            (day, &query.grain)
        {
            check_day_picker(
                widget,
                &picker.selector,
                &grain.selector,
                &day_path,
                report,
            );
        }
        for value in possible_values(
            widget,
            day,
            &day_path,
            DynamicSource::Accepted,
            report,
        ) {
            if !is_calendar_date(&value) {
                report.push(Problem::error(
                    Code::InvalidValue,
                    &day_path,
                    format!("'{value}' is not a date written as YYYY-MM-DD."),
                ));
            }
        }
    }

    for (key, label) in &query.labels {
        let label_path = join(&join(path, "labels"), key);
        let known = match (key.parse::<Measure>(), key.parse::<PostState>()) {
            (Ok(measure), _) => spec.has_measure(measure),
            (_, Ok(state)) => spec.states.contains(&state),
            _ => false,
        };
        if !known {
            report.push(Problem::error(
                Code::UnsupportedBySource,
                &label_path,
                format!(
                    "{} has no measure or Post state '{key}' to label.",
                    spec.id
                ),
            ));
        }
        require_text(label, &label_path, report);
    }

    let lists =
        matches!(template, QueryTemplate::ByGroup | QueryTemplate::ByPost);
    if let Some(sort) = &query.sort {
        let sort_path = join(path, "sort");
        if !lists {
            report.push(misplaced(&sort_path, "sort", template));
        }
        for sort in possible_values(
            widget,
            sort,
            &sort_path,
            DynamicSource::Refused,
            report,
        )
        .into_iter()
        .flatten()
        {
            let missing = match sort.by {
                SortKey::Ratio if query.ratio.is_none() => Some("a `ratio`"),
                SortKey::Value
                    if query.measures.is_none() && query.ratio.is_none() =>
                {
                    Some("`measures` or a `ratio`")
                }
                _ => None,
            };
            if let Some(missing) = missing {
                report.push(Problem::error(
                    Code::TemplateParameter,
                    &sort_path,
                    format!("Sorting by {} needs {missing}.", sort.by),
                ));
            }
        }
    }
    if let Some(limit) = &query.limit {
        let limit_path = join(path, "limit");
        if !lists {
            report.push(misplaced(&limit_path, "limit", template));
        }
        for limit in possible_values(
            widget,
            limit,
            &limit_path,
            DynamicSource::Refused,
            report,
        )
        .into_iter()
        .flatten()
        {
            if !(1..=MAX_ROW_LIMIT).contains(&limit) {
                report.push(Problem::error(
                    Code::InvalidValue,
                    &limit_path,
                    format!("A limit is between 1 and {MAX_ROW_LIMIT}."),
                ));
            }
        }
    }
}

/// A day picker makes sense only while the series is hourly, so one fed
/// alongside a grain selector must be shown only for hourly options.
fn check_day_picker(
    widget: &Widget,
    picker: &str,
    grain: &str,
    path: &str,
    report: &mut Report,
) {
    let Some(grain_selector) = widget.selectors.get(grain) else {
        return;
    };
    let condition = widget
        .selectors
        .get(picker)
        .and_then(|selector| selector.when.as_ref());
    // Shown for exactly the hourly options: for a daily one the day narrows
    // nothing, and an hourly one without it runs over every day at once.
    let hourly: Vec<&String> = grain_selector
        .options
        .keys()
        .filter(|option| {
            matches!(
                option_value::<TimeGrain>(grain_selector, option),
                Ok(TimeGrain::Hour)
            )
        })
        .collect();
    let hourly_only = condition.is_some_and(|condition| {
        condition.selector == grain
            && !condition.one_of.is_empty()
            && condition
                .one_of
                .iter()
                .all(|option| hourly.contains(&option))
            && hourly
                .iter()
                .all(|option| condition.one_of.contains(option))
    });
    if !hourly_only {
        report.push(Problem::error(
            Code::TemplateParameter,
            path,
            format!(
                "A day narrows an hourly series: show '{picker}' for every hourly option of '{grain}' and no other, with `when: {{selector: {grain}, in: [...]}}`."
            ),
        ));
    }
}

fn check_dashboard(dashboard: &Dashboard, report: &mut Report) {
    require_id(&dashboard.id, "id", report);
    require_text(&dashboard.title, "title", report);
    if let Some(description) = &dashboard.description {
        require_text(description, "description", report);
    }
    if let Some(section) = &dashboard.section {
        require_text(section, "section", report);
    }
    for (position, requirement) in dashboard.requirements.iter().enumerate() {
        require_text(requirement, &index("requirements", position), report);
    }
    refuse_duplicates(&dashboard.selectors, "selectors", report);
    if let Some(theme) = &dashboard.theme {
        require_id(theme, "theme", report);
    }
    if dashboard.layout.is_empty() {
        report.push(Problem::error(
            Code::InvalidValue,
            "layout",
            "A dashboard shows at least one widget.",
        ));
    }
    for (position, item) in dashboard.layout.iter().enumerate() {
        let path = index("layout", position);
        require_id(&item.widget, &join(&path, "widget"), report);
        if !(1..=GRID_COLUMNS).contains(&item.width) {
            report.push(Problem::error(
                Code::LayoutWidth,
                join(&path, "width"),
                format!("A widget is 1 to {GRID_COLUMNS} columns wide."),
            ));
        }
        for name in item.values.keys() {
            require_name(name, &join(&join(&path, "values"), name), report);
        }
    }
}

fn check_theme(theme: &Theme, report: &mut Report) {
    require_id(&theme.id, "id", report);
    if let Some(style) = &theme.style {
        if !style.is_mapping() {
            report.push(Problem::error(
                Code::InvalidValue,
                "style",
                "A theme's style is a mapping of dbt Charts style fields.",
            ));
        }
    }
}

fn check_settings(settings: &Settings, report: &mut Report) {
    if !is_time_zone_name(&settings.time_zone) {
        report.push(Problem::error(
            Code::InvalidValue,
            "time_zone",
            format!(
                "'{}' is not a time zone name such as UTC or Asia/Manila.",
                settings.time_zone
            ),
        ));
    }
    check_mapping(&settings.scope.region, "scope.region", report);
    check_mapping(&settings.scope.country, "scope.country", report);
    require_text(
        &settings.pre_enrolled.voter_attribute,
        "pre_enrolled.voter_attribute",
        report,
    );
    require_text(&settings.pre_enrolled.equals, "pre_enrolled.equals", report);
    if let Some(label) = &settings.unknown_label {
        require_text(label, "unknown_label", report);
    }
    for (name, mapping) in &settings.dimensions {
        let path = join("dimensions", name);
        require_name(name, &path, report);
        if name.parse::<BuiltinDimension>().is_ok() {
            report.push(Problem::error(
                Code::DuplicateId,
                &path,
                format!("'{name}' is a built-in dimension; give this one another name."),
            ));
        }
        check_mapping(mapping, &path, report);
    }
    for (selector, words) in &settings.selectors {
        let path = join("selectors", &selector.to_string());
        require_text(&words.label, &join(&path, "label"), report);
        require_text(&words.all, &join(&path, "all"), report);
    }
}

fn check_mapping(mapping: &DimensionMapping, path: &str, report: &mut Report) {
    if mapping.origin().is_none() {
        report.push(Problem::error(
            Code::InvalidValue,
            path,
            "Read a dimension from exactly one of voter_attribute, election_annotation and area_annotation.",
        ));
    }
    for (field, name) in [
        ("voter_attribute", &mapping.voter_attribute),
        ("election_annotation", &mapping.election_annotation),
        ("area_annotation", &mapping.area_annotation),
    ] {
        if let Some(name) = name {
            require_text(name, &join(path, field), report);
        }
    }
    if let Some(split) = &mapping.split {
        if split.separator.is_empty() {
            report.push(Problem::error(
                Code::InvalidValue,
                join(path, "split.separator"),
                "A split needs a separator.",
            ));
        }
    }
    let last = mapping.age_bands.len().saturating_sub(1);
    let mut previous: Option<u32> = None;
    for (position, band) in mapping.age_bands.iter().enumerate() {
        let band_path = index(&join(path, "age_bands"), position);
        require_text(&band.label, &join(&band_path, "label"), report);
        match band.to {
            None if position != last => report.push(Problem::error(
                Code::InvalidValue,
                join(&band_path, "to"),
                "Only the last age band may be open-ended.",
            )),
            Some(to) if previous.is_some_and(|previous| to <= previous) => {
                report.push(Problem::error(
                    Code::InvalidValue,
                    join(&band_path, "to"),
                    "Age bands rise: each ends above the one before it.",
                ))
            }
            _ => {}
        }
        previous = band.to.or(previous);
    }
}

// -- across documents ------------------------------------------------------

/// The checks that need more than one document: references between them, and
/// voter dimensions against the settings that define them.
pub fn validate_set(set: &ConfigSet) -> Report {
    let mut report = Report::default();
    for (key, widget) in &set.widgets {
        stored_under_id(key, &widget.id, "widgets", &mut report);
    }
    for (key, theme) in &set.themes {
        stored_under_id(key, &theme.id, "themes", &mut report);
    }
    for (key, dashboard) in &set.dashboards {
        stored_under_id(key, &dashboard.id, "dashboards", &mut report);
    }

    for (key, widget) in &set.widgets {
        let spec = widget.source.spec();
        if spec.voter_dimensions != VoterDimensions::Configured {
            continue;
        }
        for (name, query) in widget.named_queries() {
            let query_path = match &widget.query {
                Some(_) => join(&join("widgets", key), "query"),
                None => join(&join(&join("widgets", key), "queries"), &name),
            };
            let mut groups: Vec<(String, String)> = Vec::new();
            if let Some(group_by) = &query.group_by {
                let path = join(&query_path, "group_by");
                for value in possible_values(
                    widget,
                    group_by,
                    &path,
                    DynamicSource::Refused,
                    &mut Report::default(),
                ) {
                    groups.push((value, path.clone()));
                }
            }
            for dimension in query.filters.keys() {
                groups.push((
                    dimension.clone(),
                    join(&join(&query_path, "filters"), dimension),
                ));
            }
            for (group, path) in groups {
                if group.parse::<BuiltinDimension>().is_ok() {
                    continue;
                }
                match &set.settings {
                    None => {
                        report.push(Problem::error(
                            Code::DanglingReference,
                            path,
                            format!("'{group}' is a dimension settings configure, and this event has no settings."),
                        ));
                    }
                    Some(settings)
                        if !settings.dimensions.contains_key(&group) =>
                    {
                        report.push(Problem::error(
                            Code::UnsupportedBySource,
                            path,
                            format!("'{group}' is not a dimension these settings configure."),
                        ));
                    }
                    Some(_) => {}
                }
            }
        }
    }

    for (key, dashboard) in &set.dashboards {
        let dashboard_path = join("dashboards", key);
        let theme = dashboard.theme.as_deref().unwrap_or(DEFAULT_THEME);
        if !set.themes.contains_key(theme) {
            report.push(Problem::error(
                Code::DanglingReference,
                join(&dashboard_path, "theme"),
                format!("There is no theme '{theme}'."),
            ));
        }
        for (position, selector) in dashboard.selectors.iter().enumerate() {
            let applies = dashboard.layout.iter().any(|item| {
                set.widgets
                    .get(&item.widget)
                    .is_some_and(|widget| widget.narrowed_by(*selector))
            });
            if !applies {
                report.push(Problem::warning(
                    Code::UnusedSelector,
                    index(&join(&dashboard_path, "selectors"), position),
                    format!("No widget on this dashboard can be narrowed by the {selector} selector; a viewer would change it to no effect."),
                ));
            }
        }
        for (position, item) in dashboard.layout.iter().enumerate() {
            let item_path = index(&join(&dashboard_path, "layout"), position);
            let Some(widget) = set.widgets.get(&item.widget) else {
                report.push(Problem::error(
                    Code::DanglingReference,
                    join(&item_path, "widget"),
                    format!("There is no widget '{}'.", item.widget),
                ));
                continue;
            };
            check_layout_values(
                widget,
                &item.values,
                &join(&item_path, "values"),
                &mut report,
            );
        }
    }
    report
}

fn stored_under_id(key: &str, id: &str, collection: &str, report: &mut Report) {
    if key != id {
        report.push(Problem::error(
            Code::InvalidId,
            join(&join(collection, key), "id"),
            format!("Stored as '{key}' but its id is '{id}'."),
        ));
    }
}

fn check_layout_values(
    widget: &Widget,
    values: &IndexMap<String, String>,
    path: &str,
    report: &mut Report,
) {
    for (name, value) in values {
        let value_path = join(path, name);
        let Some(selector) = widget.selectors.get(name) else {
            report.push(Problem::error(
                Code::UnknownSelector,
                value_path,
                format!("'{}' has no selector '{name}'.", widget.id),
            ));
            continue;
        };
        if selector.options_from.is_some() {
            report.push(Problem::error(
                Code::InvalidValue,
                value_path,
                format!("'{name}' takes its options from the data; a dashboard cannot fix one in advance."),
            ));
            continue;
        }
        if !selector.options.contains_key(value) {
            report.push(Problem::error(
                Code::UnknownOption,
                value_path,
                format!("'{value}' is not an option of '{name}'."),
            ));
        }
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
