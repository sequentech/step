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

/// Checks `yaml` as a document of `kind` (`widget`, `dashboard`, `theme` or
/// `settings`). `config_set_json` is the event's [`ConfigSet`] as JSON, or
/// empty to check the document alone.
pub fn check_document(kind: &str, yaml: &str, config_set_json: &str) -> Report {
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
        return check_with(kind, yaml, None);
    }
    match serde_json::from_str::<ConfigSet>(config_set_json) {
        Ok(set) => check_with(kind, yaml, Some(set)),
        Err(why) => {
            let mut report = check_with(kind, yaml, None);
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

/// The document's own checks; with a set, the document replaces its stored
/// copy there and the checks across documents run too.
fn check_with(kind: ConfigKind, yaml: &str, set: Option<ConfigSet>) -> Report {
    let (mut report, placed) = match kind {
        ConfigKind::Widget => {
            let parsed = parse_widget(yaml);
            let placed = set.zip(parsed.value).map(|(mut set, widget)| {
                let own = format!("widgets.{}", widget.id);
                set.widgets.insert(widget.id.clone(), widget);
                (set, own)
            });
            (parsed.report, placed)
        }
        ConfigKind::Dashboard => {
            let parsed = parse_dashboard(yaml);
            let placed = set.zip(parsed.value).map(|(mut set, dashboard)| {
                let own = format!("dashboards.{}", dashboard.id);
                set.dashboards.insert(dashboard.id.clone(), dashboard);
                (set, own)
            });
            (parsed.report, placed)
        }
        ConfigKind::Theme => {
            let parsed = parse_theme(yaml);
            let placed = set.zip(parsed.value).map(|(mut set, theme)| {
                let own = format!("themes.{}", theme.id);
                set.themes.insert(theme.id.clone(), theme);
                (set, own)
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
