// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One document checked as a save would check it, for an editor that asks
//! on every keystroke.
//!
//! The document is checked alone, and — when the event's other documents
//! are given — against them, reporting only what concerns this document.
//! The event's documents must be the whole set: a partial one makes every
//! reference to a missing document look dangling.

use crate::monitoring::config::{ConfigKind, ConfigSet};
use crate::monitoring::policy::{
    parse_dashboard, parse_settings, parse_theme, parse_widget, validate_set,
};
use crate::monitoring::problem::{Code, Problem, Report};
use std::str::FromStr;

/// Checks `yaml` as the document of `kind` (`widget`, `dashboard`, `theme`
/// or `settings`) stored, or to be stored, under `key`. `config_set_json`
/// is the event's [`ConfigSet`] as JSON, or empty to check the document
/// alone.
pub fn check_document(
    kind: &str,
    key: &str,
    yaml: &str,
    config_set_json: &str,
) -> Report {
    let Ok(kind) = ConfigKind::from_str(kind) else {
        let mut report = Report::default();
        report.push(Problem::error(
            Code::InvalidValue,
            "",
            format!("'{kind}' is not a kind of monitoring document."),
        ));
        return report;
    };
    if config_set_json.trim().is_empty() {
        return check_with(kind, key, yaml, None);
    }
    match serde_json::from_str::<ConfigSet>(config_set_json) {
        Ok(set) => check_with(kind, key, yaml, Some(set)),
        Err(why) => {
            let mut report = check_with(kind, key, yaml, None);
            report.push(Problem::warning(
                Code::Unreadable,
                "",
                format!(
                    "The event's other documents could not be read, so only this one was checked: {why}"
                ),
            ));
            report
        }
    }
}

/// The document's own checks; with a set, the draft takes the place of the
/// document stored under `key` (even when the draft changed its id, which
/// the checks then report) and the checks across documents run too.
fn check_with(
    kind: ConfigKind,
    key: &str,
    yaml: &str,
    set: Option<ConfigSet>,
) -> Report {
    let (mut report, placed) = match kind {
        ConfigKind::Widget => {
            let parsed = parse_widget(yaml);
            let placed = set.zip(parsed.value).map(|(mut set, widget)| {
                let key = key_or(key, &widget.id);
                set.widgets.shift_remove(&key);
                set.widgets.insert(key.clone(), widget);
                (set, format!("widgets.{key}"))
            });
            (parsed.report, placed)
        }
        ConfigKind::Dashboard => {
            let parsed = parse_dashboard(yaml);
            let placed = set.zip(parsed.value).map(|(mut set, dashboard)| {
                let key = key_or(key, &dashboard.id);
                set.dashboards.shift_remove(&key);
                set.dashboards.insert(key.clone(), dashboard);
                (set, format!("dashboards.{key}"))
            });
            (parsed.report, placed)
        }
        ConfigKind::Theme => {
            let parsed = parse_theme(yaml);
            let placed = set.zip(parsed.value).map(|(mut set, theme)| {
                let key = key_or(key, &theme.id);
                set.themes.shift_remove(&key);
                set.themes.insert(key.clone(), theme);
                (set, format!("themes.{key}"))
            });
            (parsed.report, placed)
        }
        // Settings have no path of their own in the set's problems: what
        // they break is reported on the widgets that group by a dimension.
        ConfigKind::Settings => (parse_settings(yaml).report, None),
    };
    if let Some((set, own)) = placed {
        report.extend(own_problems(validate_set(&set), &own));
    }
    report
}

/// The key the draft is checked under: the one it was opened as, or its own
/// id for a document not stored yet.
fn key_or(key: &str, id: &str) -> String {
    if key.is_empty() { id } else { key }.to_string()
}

/// The problems of the document at `own` (`widgets.<id>`), with paths made
/// relative to it.
fn own_problems(report: Report, own: &str) -> Report {
    let prefix = format!("{own}.");
    let mut mine = Report::default();
    for mut problem in report.problems {
        if problem.path == own {
            problem.path = String::new();
        } else if let Some(rest) = problem.path.strip_prefix(&prefix) {
            problem.path = rest.to_string();
        } else {
            continue;
        }
        mine.push(problem);
    }
    mine
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod check_tests;
