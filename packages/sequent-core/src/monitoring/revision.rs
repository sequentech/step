// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What an event's stored documents amount to, and what a save or a reset to
//! a preset may write. Storage is the caller's; this decides.

use super::config::{ConfigKind, ConfigSet, Editability, DEFAULT_THEME};
use super::policy::{
    is_id, parse_dashboard, parse_settings, parse_theme, parse_widget,
    validate_set, Parsed,
};
use super::presets::Preset;
use super::problem::{Code, Problem, Report, Severity};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use strum_macros::{Display, EnumIter, EnumString};

/// Which Dashboard tab an election event shows.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DashboardMode {
    /// The standard dashboard, as every event without monitoring
    /// configuration has.
    Legacy,
    /// The configured monitoring dashboards.
    Configured,
}

/// Who wrote a revision.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum RevisionOrigin {
    /// A reset to a preset.
    Preset,
    /// An administrator, in the Admin Portal.
    Editor,
}

/// What a revision does to its document.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    Display,
    EnumString,
    EnumIter,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DocumentChange {
    /// The document is this text from now on.
    Upsert,
    /// The document is gone; its key may be used again.
    Delete,
}

/// Lowercase hex SHA-256 of a document's UTF-8 text: what the revision
/// tables and the electoral log bind a revision to.
pub fn document_digest(yaml: &str) -> String {
    hex::encode(Sha256::digest(yaml.as_bytes()))
}

/// A document an event has now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveDocument<'a> {
    pub kind: ConfigKind,
    pub key: &'a str,
    pub yaml: &'a str,
}

/// The newest revision of a document an event has had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveHead {
    pub kind: ConfigKind,
    pub key: String,
    /// The digest of the document; absent when the revision removed it.
    pub digest: Option<String>,
}

/// An event's documents as one set.
#[derive(Debug, Clone, PartialEq)]
pub struct Assembled {
    pub set: ConfigSet,
    /// Why a document was left out: it was saved before a check that it no
    /// longer passes. Paths are prefixed with the document's place in the
    /// set, `widgets.turnout`.
    pub report: Report,
}

/// Reads the documents into one set. One that no longer passes is left out
/// rather than failing the rest; the set-wide checks are the caller's.
pub fn assemble<'a>(
    documents: impl IntoIterator<Item = LiveDocument<'a>>,
) -> Assembled {
    let mut set = ConfigSet::default();
    let mut report = Report::default();
    for document in documents {
        let problems =
            insert(&mut set, document.kind, document.key, document.yaml);
        let place = place(document.kind, document.key);
        for mut problem in problems.problems {
            problem.path = if problem.path.is_empty() {
                place.clone()
            } else {
                format!("{place}.{}", problem.path)
            };
            report.push(problem);
        }
    }
    Assembled { set, report }
}

/// Where a document sits in a [`ConfigSet`], as the set-wide checks name it.
fn place(kind: ConfigKind, key: &str) -> String {
    match kind {
        ConfigKind::Settings => "settings".to_string(),
        ConfigKind::Theme => format!("themes.{key}"),
        ConfigKind::Widget => format!("widgets.{key}"),
        ConfigKind::Dashboard => format!("dashboards.{key}"),
    }
}

/// Parses `yaml` and puts it in the set under `key`, replacing the document
/// there and keeping its place. What the document's own checks say comes
/// back; with an error, the set is left as it was.
fn insert(
    set: &mut ConfigSet,
    kind: ConfigKind,
    key: &str,
    yaml: &str,
) -> Report {
    fn put<T>(
        documents: &mut IndexMap<String, T>,
        key: &str,
        parsed: Parsed<T>,
    ) -> Report {
        if let Some(value) = parsed.value {
            documents.insert(key.to_string(), value);
        }
        parsed.report
    }
    match kind {
        ConfigKind::Settings => {
            let parsed = parse_settings(yaml);
            if let Some(settings) = parsed.value {
                set.settings = Some(settings);
            }
            parsed.report
        }
        ConfigKind::Theme => put(&mut set.themes, key, parse_theme(yaml)),
        ConfigKind::Widget => put(&mut set.widgets, key, parse_widget(yaml)),
        ConfigKind::Dashboard => {
            put(&mut set.dashboards, key, parse_dashboard(yaml))
        }
    }
}

fn remove(set: &mut ConfigSet, kind: ConfigKind, key: &str) {
    match kind {
        ConfigKind::Settings => set.settings = None,
        ConfigKind::Theme => {
            set.themes.shift_remove(key);
        }
        ConfigKind::Widget => {
            set.widgets.shift_remove(key);
        }
        ConfigKind::Dashboard => {
            set.dashboards.shift_remove(key);
        }
    }
}

/// A change an administrator asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit<'a> {
    Upsert(&'a str),
    Delete,
}

/// An edit that may be saved.
#[derive(Debug, Clone, PartialEq)]
pub struct Checked {
    /// The event's documents once it is saved.
    pub set: ConfigSet,
    /// Warnings about the document, and about the set that it introduces.
    pub report: Report,
}

/// Whether `kind`/`key` may be edited at all: the kind is not one only a
/// reset writes, and the key is an id. The first of [`check_edit`]'s checks,
/// for a store to make before it looks at the document's revision.
pub fn check_target(kind: ConfigKind, key: &str) -> Result<(), Report> {
    let mut report = Report::default();
    if kind.editability() == Editability::PresetOnly {
        report.push(Problem::error(
            Code::PresetOnly,
            "",
            format!("The {kind} are written by resetting the event to a preset, not in the editor."),
        ));
        return Err(report);
    }
    if !is_id(key) {
        report.push(Problem::error(
            Code::InvalidId,
            "id",
            format!("'{key}' is not an id: use lowercase letters, digits, '-' and '_'."),
        ));
        return Err(report);
    }
    Ok(())
}

/// Whether `path`, as the set-wide checks name it, lies in the document at
/// `place`.
fn lies_in(path: &str, place: &str) -> bool {
    path.strip_prefix(place)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

/// What saving `edit` to the document `kind`/`key` would leave the event
/// with, or why it may not be saved: [`check_target`] refuses it; the
/// document does not pass its own checks; the edit removes the default
/// theme; the document saved has a set-wide error of its own; or the set
/// would have an error elsewhere it did not already have. Errors elsewhere
/// that the set already had do not block the edit, so one stale document
/// cannot freeze the rest.
///
/// Whether the document exists, and at which revision, is the store's to
/// check.
pub fn check_edit(
    live: &ConfigSet,
    kind: ConfigKind,
    key: &str,
    edit: Edit<'_>,
) -> Result<Checked, Report> {
    check_target(kind, key)?;
    let mut report = Report::default();

    let mut set = live.clone();
    match edit {
        Edit::Upsert(yaml) => {
            report.extend(insert(&mut set, kind, key, yaml));
            if !report.is_accepted() {
                return Err(report);
            }
        }
        Edit::Delete => {
            if kind == ConfigKind::Theme && key == DEFAULT_THEME {
                report.push(Problem::error(
                    Code::DanglingReference,
                    "",
                    format!("The '{DEFAULT_THEME}' theme is kept: widgets shown outside a dashboard take it."),
                ));
                return Err(report);
            }
            remove(&mut set, kind, key);
        }
    }

    let before = validate_set(live);
    let edited = place(kind, key);
    for problem in validate_set(&set).problems {
        if lies_in(&problem.path, &edited)
            || !before.problems.contains(&problem)
        {
            report.push(problem);
        }
    }
    if report.is_accepted() {
        Ok(Checked { set, report })
    } else {
        report
            .problems
            .retain(|problem| problem.severity == Severity::Error);
        Err(report)
    }
}

/// A revision a reset writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned<'a> {
    pub kind: ConfigKind,
    pub key: String,
    pub change: DocumentChange,
    /// The document's text; absent for a removal.
    pub yaml: Option<&'a str>,
}

/// What resetting an event to `preset` writes, given the heads it has: each
/// preset document whose text differs from the event's, in the preset's
/// order, then a removal of each document the preset does not have. The
/// event is left with exactly the preset's documents.
pub fn plan_reset<'a>(
    preset: &'a Preset,
    heads: &[LiveHead],
) -> Vec<Planned<'a>> {
    let head = |kind: ConfigKind, key: &str| {
        heads
            .iter()
            .find(|head| head.kind == kind && head.key == key)
    };
    let mut planned: Vec<Planned<'a>> = preset
        .documents
        .iter()
        .filter(|document| {
            head(document.kind, &document.key)
                .and_then(|head| head.digest.as_deref())
                != Some(document_digest(document.yaml).as_str())
        })
        .map(|document| Planned {
            kind: document.kind,
            key: document.key.clone(),
            change: DocumentChange::Upsert,
            yaml: Some(document.yaml),
        })
        .collect();
    planned.extend(
        heads
            .iter()
            .filter(|head| head.digest.is_some())
            .filter(|head| {
                !preset.documents.iter().any(|document| {
                    document.kind == head.kind && document.key == head.key
                })
            })
            .map(|head| Planned {
                kind: head.kind,
                key: head.key.clone(),
                change: DocumentChange::Delete,
                yaml: None,
            }),
    );
    planned
}

#[cfg(test)]
#[path = "revision_tests.rs"]
mod revision_tests;
