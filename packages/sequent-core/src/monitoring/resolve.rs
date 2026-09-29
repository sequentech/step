// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Turning selector values into the concrete queries a widget runs.
//!
//! A selector's value comes from, in order: what the viewer picked, the
//! dashboard's `values` for the widget, and the selector's own default. Only
//! the options a selector lists (or, for dynamic options, the days the data
//! has) are accepted, so a request can never smuggle in a parameter the
//! configuration did not offer. A request for anything else is refused; a
//! dashboard value that no longer fits — a renamed option — falls back to the
//! default, since the viewer did not ask for it.
//!
//! Expects a widget the policy has accepted; anything it cannot resolve is
//! reported, never guessed.

use super::config::{
    DynamicOptions, Param, Query, Ratio, Selector, Sort, Widget,
};
use super::problem::{Code, Problem, Report};
use super::sources::{Measure, QueryTemplate, TimeGrain};
use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

/// The option lists that come from the data rather than the configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicOptionValues {
    /// Days with activity, `YYYY-MM-DD`, oldest first.
    pub event_days: Vec<String>,
}

impl DynamicOptionValues {
    fn get(&self, options: DynamicOptions) -> &[String] {
        match options {
            DynamicOptions::EventDays => &self.event_days,
        }
    }
}

/// Where a selector stands once resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectorState {
    Value(String),
    /// Its `when` condition does not hold: not shown, feeds nothing.
    Hidden,
    /// Its options come from the data, and there are none yet.
    NoOptions,
}

/// A query with every parameter decided.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedQuery {
    pub template: QueryTemplate,
    pub measures: Vec<Measure>,
    pub ratio: Option<Ratio>,
    pub group_by: Option<String>,
    pub filters: IndexMap<String, Vec<String>>,
    pub grain: Option<TimeGrain>,
    pub day: Option<String>,
    pub sort: Option<Sort>,
    pub limit: Option<u32>,
    pub labels: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedWidget {
    pub selectors: IndexMap<String, SelectorState>,
    pub queries: IndexMap<String, ResolvedQuery>,
}

pub fn resolve_widget(
    widget: &Widget,
    dashboard_values: &IndexMap<String, String>,
    requested: &IndexMap<String, String>,
    dynamic: &DynamicOptionValues,
) -> Result<ResolvedWidget, Report> {
    let mut report = Report::default();
    for name in requested.keys() {
        if !widget.selectors.contains_key(name) {
            report.push(Problem::error(
                Code::UnknownSelector,
                name,
                format!("'{}' has no selector '{name}'.", widget.id),
            ));
        }
    }

    let mut selectors: IndexMap<String, SelectorState> = IndexMap::new();
    for (name, selector) in &widget.selectors {
        let state = selector_state(
            name,
            selector,
            &selectors,
            requested.get(name),
            dashboard_values.get(name),
            dynamic,
            &mut report,
        );
        selectors.insert(name.clone(), state);
    }

    let mut queries = IndexMap::new();
    for (name, query) in widget.named_queries() {
        let resolved = resolve_query(widget, query, &selectors, &mut report);
        queries.insert(name, resolved);
    }

    if report.is_accepted() {
        Ok(ResolvedWidget { selectors, queries })
    } else {
        Err(report)
    }
}

fn selector_state(
    name: &str,
    selector: &Selector,
    earlier: &IndexMap<String, SelectorState>,
    requested: Option<&String>,
    dashboard: Option<&String>,
    dynamic: &DynamicOptionValues,
    report: &mut Report,
) -> SelectorState {
    if let Some(condition) = &selector.when {
        let holds = matches!(
            earlier.get(&condition.selector),
            Some(SelectorState::Value(value)) if condition.one_of.contains(value)
        );
        if !holds {
            return SelectorState::Hidden;
        }
    }
    let options: Vec<&String> = match selector.options_from {
        Some(from) => dynamic.get(from).iter().collect(),
        None => selector.options.keys().collect(),
    };
    if let Some(value) = requested {
        if options.contains(&value) {
            return SelectorState::Value(value.clone());
        }
        report.push(Problem::error(
            Code::UnknownOption,
            name,
            format!("'{value}' is not an option of '{name}'."),
        ));
        return SelectorState::NoOptions;
    }
    if let Some(value) = dashboard.filter(|value| options.contains(value)) {
        return SelectorState::Value(value.clone());
    }
    let fallback = match selector.options_from {
        Some(_) => options.last().copied(),
        None => selector.default.as_ref().or(options.first().copied()),
    };
    match fallback {
        Some(value) => SelectorState::Value(value.clone()),
        None => SelectorState::NoOptions,
    }
}

fn resolve_query(
    widget: &Widget,
    query: &Query,
    selectors: &IndexMap<String, SelectorState>,
    report: &mut Report,
) -> ResolvedQuery {
    let grain = resolve(widget, query.grain.as_ref(), selectors, report);
    // A day narrows an hourly series; a daily one covers every day.
    let day = match grain {
        Some(TimeGrain::Day) => None,
        _ => resolve(widget, query.day.as_ref(), selectors, report),
    };
    ResolvedQuery {
        template: query.template,
        measures: resolve(widget, query.measures.as_ref(), selectors, report)
            .unwrap_or_default(),
        ratio: resolve(widget, query.ratio.as_ref(), selectors, report),
        group_by: resolve(widget, query.group_by.as_ref(), selectors, report),
        filters: query
            .filters
            .iter()
            .filter_map(|(dimension, values)| {
                resolve(widget, Some(values), selectors, report)
                    .flatten()
                    .map(|values| (dimension.clone(), values))
            })
            .collect(),
        grain,
        day,
        sort: resolve(widget, query.sort.as_ref(), selectors, report).flatten(),
        limit: resolve(widget, query.limit.as_ref(), selectors, report)
            .flatten(),
        labels: query.labels.clone(),
    }
}

/// A parameter's value: its literal, or what the selector it names stands
/// for. `None` when that selector is hidden or has no options yet.
fn resolve<T: DeserializeOwned + Clone>(
    widget: &Widget,
    param: Option<&Param<T>>,
    selectors: &IndexMap<String, SelectorState>,
    report: &mut Report,
) -> Option<T> {
    let reference = match param? {
        Param::Literal(value) => return Some(value.clone()),
        Param::Selector(reference) => reference,
    };
    let Some(SelectorState::Value(value)) = selectors.get(&reference.selector)
    else {
        return None;
    };
    let raw = widget
        .selectors
        .get(&reference.selector)
        .and_then(|selector| selector.maps.as_ref())
        .and_then(|maps| maps.get(value))
        .cloned()
        .unwrap_or_else(|| Value::String(value.clone()));
    match serde_yaml::from_value(raw) {
        Ok(value) => Some(value),
        Err(why) => {
            report.push(Problem::error(
                Code::TemplateParameter,
                &reference.selector,
                format!("Option '{value}' does not fit the parameter it feeds: {why}"),
            ));
            None
        }
    }
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod resolve_tests;
