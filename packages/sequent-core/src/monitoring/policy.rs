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
//! - links, raw HTML, templates and URLs, anywhere in any document;
//! - anything the data source does not offer: a measure, a group, a template;
//! - selector values outside the options a selector lists.
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
    ConfigSet, Dashboard, DimensionMapping, Param, Query, Selector, Settings,
    Theme, Widget, DEFAULT_QUERY_NAME, DEFAULT_THEME, GRID_COLUMNS,
};
use super::problem::{Code, Problem, Report};
use super::sources::{
    BuiltinDimension, Measure, QueryTemplate, SourceSpec, TimeGrain,
    VoterDimensions,
};
use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use serde_yaml::Value;

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
const CHART_FIELDS: &[&str] =
    &["charts", "rows", "cols", "grid", "tabs", "style"];

/// Keys refused anywhere inside dbt Charts YAML, because each would bring in
/// data, markup or a destination from outside the governed sources.
const FORBIDDEN_KEYS: &[&str] = &[
    "auto_link",
    "extends",
    "file",
    "files",
    "geo",
    "href",
    "html",
    "http",
    "image",
    "images",
    "link",
    "links",
    "markdown",
    "message",
    "notes",
    "path",
    "queries",
    "source",
    "sources",
    "sql",
    "template",
    "templates",
    "text",
    "theme",
    "url",
    "urls",
    "variables",
];

/// Chart types whose content is typed in rather than read from a query.
const FORBIDDEN_CHART_TYPES: &[&str] = &["callout", "image"];

/// Markers of text that a template engine would evaluate.
const TEMPLATE_MARKERS: &[&str] = &["{{", "{%", "{#", "${"];

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
    walk(&raw, report);
    match serde_path_to_error::deserialize::<_, T>(raw) {
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

/// What in `text` would be evaluated by something downstream, if anything.
///
/// Looks for syntax rather than words: "data", "http" and "<" all appear in
/// ordinary prose, while `{{`, `<b`, `javascript:` and `scheme://` do not.
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
    let lower = text.to_ascii_lowercase();
    for token in lower.split(|c: char| {
        c.is_whitespace() || matches!(c, '"' | '\'' | '(' | ')' | '`')
    }) {
        if token.starts_with("javascript:") || token.starts_with("vbscript:") {
            return Some("a script URL");
        }
        if token.starts_with("data:") && token.contains(',') {
            return Some("a data URI");
        }
    }
    if has_url(&lower) {
        return Some("a URL");
    }
    None
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

/// A widget's `chart`: only board layout and paint, reading only the widget's
/// own queries.
fn walk_widget(raw: &Value, report: &mut Report) {
    let Some(chart) = raw.get("chart") else {
        return;
    };
    let governed: Vec<String> = match (raw.get("query"), raw.get("queries")) {
        (Some(_), _) => vec![DEFAULT_QUERY_NAME.to_string()],
        (None, Some(Value::Mapping(queries))) => {
            queries.keys().map(key_text).collect()
        }
        _ => Vec::new(),
    };
    let Value::Mapping(fields) = chart else {
        return;
    };
    for (key, value) in fields {
        let key = key_text(key);
        let path = join("chart", &key);
        if CHART_FIELDS.contains(&key.as_str()) {
            walk_dbt(value, &path, Some(&governed), report);
        } else {
            report.push(Problem::error(
                Code::ForbiddenKey,
                path,
                format!(
                    "A widget's chart may only set {}; '{key}' is supplied by the platform or not allowed.",
                    CHART_FIELDS.join(", ")
                ),
            ));
        }
    }
}

/// A theme's `style`: paint only.
fn walk_theme(raw: &Value, report: &mut Report) {
    if let Some(style) = raw.get("style") {
        walk_dbt(style, "style", None, report);
    }
}

/// dbt Charts YAML, for keys that bring in outside data or markup and for
/// queries other than the widget's own. `governed` is `None` where no query
/// may appear at all.
fn walk_dbt(
    value: &Value,
    path: &str,
    governed: Option<&[String]>,
    report: &mut Report,
) {
    match value {
        Value::Mapping(map) => {
            for (key, value) in map {
                let key = key_text(key);
                let child = join(path, &key);
                if FORBIDDEN_KEYS.contains(&key.as_str()) {
                    report.push(Problem::error(
                        Code::ForbiddenKey,
                        child,
                        format!("'{key}' would bring data, markup or a destination from outside the governed sources."),
                    ));
                } else if key == "query" {
                    walk_query(value, &child, governed, report);
                } else {
                    if key == "type" {
                        refuse_typed_in_chart(value, &child, report);
                    }
                    walk_dbt(value, &child, governed, report);
                }
            }
        }
        Value::Sequence(items) => {
            for (position, item) in items.iter().enumerate() {
                walk_dbt(item, &index(path, position), governed, report);
            }
        }
        _ => {}
    }
}

fn walk_query(
    value: &Value,
    path: &str,
    governed: Option<&[String]>,
    report: &mut Report,
) {
    let Some(governed) = governed else {
        report.push(Problem::error(
            Code::ForbiddenKey,
            path,
            "A theme styles charts; it does not query anything.",
        ));
        return;
    };
    match value {
        Value::String(name) if governed.contains(name) => {}
        Value::String(name) => report.push(Problem::error(
            Code::DanglingReference,
            path,
            format!(
                "'{name}' is not one of this widget's queries ({}). A chart reads its widget's governed queries by name.",
                governed.join(", ")
            ),
        )),
        Value::Mapping(_) => {
            report.push(Problem::error(
                Code::ForbiddenKey,
                path,
                "A chart reads one of its widget's governed queries by name; it cannot define its own.",
            ));
            walk_dbt(value, path, Some(governed), report);
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
fn is_id(text: &str) -> bool {
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

fn is_calendar_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let digits = |part: &str, len: usize| {
        part.len() == len && part.chars().all(|c| c.is_ascii_digit())
    };
    let [year, month, day] = parts.as_slice() else {
        return false;
    };
    if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return false;
    }
    let (month, day): (u32, u32) =
        (month.parse().unwrap_or(0), day.parse().unwrap_or(0));
    (1..=12).contains(&month) && (1..=31).contains(&day)
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
    for (position, requirement) in widget.requirements.iter().enumerate() {
        require_text(requirement, &index("requirements", position), report);
    }
    if let Some(follows) = &widget.follows {
        refuse_duplicates(follows, "follows", report);
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
    let spec = widget.source.spec();
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
        let raw = selector
            .maps
            .as_ref()
            .and_then(|maps| maps.get(option))
            .cloned()
            .unwrap_or_else(|| Value::String(option.clone()));
        match serde_yaml::from_value::<T>(raw) {
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
            for (position, measure) in set.into_iter().enumerate() {
                check_measure(
                    spec,
                    measure,
                    &index(&measures_path, position),
                    report,
                );
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
    if query.measures.is_none()
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
    if let Some(group_by) = &query.group_by {
        for name in possible_values(
            widget,
            group_by,
            &group_path,
            DynamicSource::Refused,
            report,
        ) {
            check_group(spec, &name, &group_path, report);
        }
    }

    for (dimension, values) in &query.filters {
        let filter_path = join(&join(path, "filters"), dimension);
        check_group(spec, dimension, &filter_path, report);
        for set in possible_values(
            widget,
            values,
            &filter_path,
            DynamicSource::Refused,
            report,
        ) {
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

    if let Some(limit) = query.limit {
        if !(1..=MAX_ROW_LIMIT).contains(&limit) {
            report.push(Problem::error(
                Code::InvalidValue,
                join(path, "limit"),
                format!("A limit is between 1 and {MAX_ROW_LIMIT}."),
            ));
        }
    }
}

fn check_dashboard(dashboard: &Dashboard, report: &mut Report) {
    require_id(&dashboard.id, "id", report);
    require_text(&dashboard.title, "title", report);
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
    if let Some(base) = &theme.base {
        require_id(base, "base", report);
    }
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

    let mut settings_missing_reported = false;
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
                    None if !settings_missing_reported => {
                        settings_missing_reported = true;
                        report.push(Problem::error(
                            Code::DanglingReference,
                            "settings",
                            "Widgets group voters by configured dimensions, and this event has no settings defining them.",
                        ));
                    }
                    None => {}
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
        let accepted = match selector.options_from {
            Some(_) => is_calendar_date(value),
            None => selector.options.contains_key(value),
        };
        if !accepted {
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
