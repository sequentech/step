// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Presets: the settings, theme, widgets and dashboards shipped for a kind of
//! deployment, as the YAML an administrator would otherwise write.
//!
//! A preset is data. Resetting an event to one writes each document as a new
//! revision, exactly as saving it in the editor would, and from then on the
//! event's configuration is its own. Nothing in the platform reads a preset's
//! id to decide how to count or what to show, so a second customer's preset
//! is a new directory here and never a code change.
//!
//! Each preset lives in `presets/<id>/`:
//!
//! - `preset.yaml`, the [`PresetManifest`]: what the preset is and which
//!   requirements it leaves to other parts of the platform;
//! - `settings.yaml`, the event settings;
//! - `themes/`, `widgets/` and `dashboards/`, one document per file, named
//!   after the document's id.
//!
//! The build script lists the directories and compiles the files in, so the
//! admin portal, Harvest and the tests all read the same bytes, and a preset
//! that fails validation fails the build's tests rather than an event's
//! reset. A file anywhere else in a preset's directory fails the build.

use super::config::{ConfigKind, ConfigSet, DEFAULT_THEME};
use super::policy::{
    parse_dashboard, parse_settings, parse_theme, parse_widget, validate_set,
};
use super::problem::{Code, Problem, Report};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// One document of a preset, as compiled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetFile {
    pub kind: ConfigKind,
    /// Relative to the preset's directory: `widgets/turnout-summary.yaml`.
    pub path: &'static str,
    pub yaml: &'static str,
}

/// A preset as compiled in, before it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetSource {
    pub id: &'static str,
    pub manifest: &'static str,
    pub files: &'static [PresetFile],
}

/// What a preset is, and what it answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetManifest {
    pub id: String,
    /// Raised whenever a document changes, so an event records which
    /// version it was reset to.
    pub version: u32,
    pub title: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Requirements met by every dashboard's Export action rather than by a
    /// dashboard of their own.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub export_requirements: Vec<String>,

    /// Requirements in the same package that another part of the platform
    /// delivers. Listed so the dashboard switcher can name the owner instead
    /// of leaving the requirement unexplained.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owned_elsewhere: Vec<OwnedElsewhere>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedElsewhere {
    pub requirements: Vec<String>,
    /// What the requirements are about: "Communications logs".
    pub subject: String,
    /// Who delivers them: a team or an epic.
    pub owner: String,
}

/// A document ready to be stored as a revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetDocument {
    pub kind: ConfigKind,
    /// The document's id; [`SETTINGS_KEY`] for the settings.
    pub key: String,
    /// The file's text, comments included, so the editor shows what the
    /// preset's author wrote.
    pub yaml: &'static str,
}

/// The key the event's one settings document is stored under.
pub const SETTINGS_KEY: &str = "settings";

/// A preset, read and checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub manifest: PresetManifest,
    pub set: ConfigSet,
    pub documents: Vec<PresetDocument>,
}

/// Every preset the platform ships: one per directory under `presets/`,
/// listed by the build script, so adding a preset needs no code.
pub const PRESETS: &[PresetSource] =
    include!(concat!(env!("OUT_DIR"), "/monitoring_presets.rs"));

/// The preset with this id, read and checked.
pub fn load(id: &str) -> Option<Result<Preset, Report>> {
    PRESETS
        .iter()
        .find(|source| source.id == id)
        .map(PresetSource::load)
}

/// `path` inside `file`, as a problem's path.
fn within(file: &str, path: &str) -> String {
    if path.is_empty() {
        file.to_string()
    } else {
        format!("{file}:{path}")
    }
}

fn located(file: &str, report: Report) -> impl Iterator<Item = Problem> + '_ {
    report.problems.into_iter().map(move |mut problem| {
        problem.path = within(file, &problem.path);
        problem
    })
}

impl PresetSource {
    /// Reads every document, then checks them as one set: the same checks
    /// saving each of them in the editor would make, so a preset can never
    /// write what an administrator could not.
    pub fn load(&self) -> Result<Preset, Report> {
        let mut report = Report::default();
        let manifest =
            match serde_yaml::from_str::<PresetManifest>(self.manifest) {
                Ok(manifest) => Some(manifest),
                Err(why) => {
                    report.push(Problem::error(
                        Code::Unreadable,
                        "preset.yaml",
                        why.to_string(),
                    ));
                    None
                }
            };
        if let Some(manifest) = &manifest {
            check_manifest(self.id, manifest, &mut report);
        }

        let mut set = ConfigSet::default();
        let mut documents = Vec::new();
        for file in self.files {
            let directory = match file.kind {
                ConfigKind::Settings => "",
                ConfigKind::Theme => "themes/",
                ConfigKind::Widget => "widgets/",
                ConfigKind::Dashboard => "dashboards/",
            };
            let placed = match file.kind {
                ConfigKind::Settings => file.path == "settings.yaml",
                _ => file.path.strip_prefix(directory).is_some_and(|name| {
                    !name.contains('/') && name.ends_with(".yaml")
                }),
            };
            if !placed {
                report.push(Problem::error(
                    Code::InvalidValue,
                    file.path,
                    format!(
                        "A {} is kept in {}.",
                        file.kind,
                        if directory.is_empty() {
                            "settings.yaml"
                        } else {
                            directory
                        }
                    ),
                ));
            }
            let key = match file.kind {
                ConfigKind::Settings => {
                    let parsed = parse_settings(file.yaml);
                    located(file.path, parsed.report)
                        .for_each(|problem| report.push(problem));
                    if set.settings.is_some() {
                        report.push(Problem::error(
                            Code::DuplicateId,
                            file.path,
                            "A preset has one settings document.",
                        ));
                    }
                    set.settings = parsed.value.or(set.settings.take());
                    Some(SETTINGS_KEY.to_string())
                }
                ConfigKind::Theme => {
                    let parsed = parse_theme(file.yaml);
                    located(file.path, parsed.report)
                        .for_each(|problem| report.push(problem));
                    parsed.value.map(|theme| {
                        let id = theme.id.clone();
                        insert(
                            &mut set.themes,
                            id.clone(),
                            theme,
                            file,
                            &mut report,
                        );
                        id
                    })
                }
                ConfigKind::Widget => {
                    let parsed = parse_widget(file.yaml);
                    located(file.path, parsed.report)
                        .for_each(|problem| report.push(problem));
                    parsed.value.map(|widget| {
                        let id = widget.id.clone();
                        insert(
                            &mut set.widgets,
                            id.clone(),
                            widget,
                            file,
                            &mut report,
                        );
                        id
                    })
                }
                ConfigKind::Dashboard => {
                    let parsed = parse_dashboard(file.yaml);
                    located(file.path, parsed.report)
                        .for_each(|problem| report.push(problem));
                    parsed.value.map(|dashboard| {
                        let id = dashboard.id.clone();
                        insert(
                            &mut set.dashboards,
                            id.clone(),
                            dashboard,
                            file,
                            &mut report,
                        );
                        id
                    })
                }
            };
            if let Some(key) = key {
                documents.push(PresetDocument {
                    kind: file.kind,
                    key,
                    yaml: file.yaml,
                });
            }
        }
        if set.settings.is_none()
            && !self
                .files
                .iter()
                .any(|file| file.kind == ConfigKind::Settings)
        {
            report.push(Problem::error(
                Code::DanglingReference,
                "settings.yaml",
                "A preset sets the event's settings.",
            ));
        }
        // A widget shown outside a dashboard, in the catalog or its editor,
        // takes the default theme.
        let default_theme = format!("themes/{DEFAULT_THEME}.yaml");
        if !self.files.iter().any(|file| {
            file.kind == ConfigKind::Theme && file.path == default_theme
        }) {
            report.push(Problem::error(
                Code::DanglingReference,
                default_theme,
                format!("A preset has a '{DEFAULT_THEME}' theme, which widgets shown outside a dashboard take."),
            ));
        }
        report.extend(validate_set(&set));
        if let Some(manifest) = &manifest {
            check_requirements(manifest, &set, &mut report);
        }

        match manifest {
            Some(manifest) if report.is_accepted() => Ok(Preset {
                manifest,
                set,
                documents,
            }),
            _ => Err(report),
        }
    }
}

/// Stores a document under its id, which must also name its file.
fn insert<T>(
    documents: &mut indexmap::IndexMap<String, T>,
    id: String,
    document: T,
    file: &PresetFile,
    report: &mut Report,
) {
    let stem = file
        .path
        .rsplit('/')
        .next()
        .and_then(|name| name.strip_suffix(".yaml"));
    if stem != Some(id.as_str()) {
        report.push(Problem::error(
            Code::InvalidValue,
            within(file.path, "id"),
            format!("The file is named after its document; '{id}' belongs in {id}.yaml."),
        ));
    }
    if documents.insert(id.clone(), document).is_some() {
        report.push(Problem::error(
            Code::DuplicateId,
            within(file.path, "id"),
            format!("Another document of this preset is also '{id}'."),
        ));
    }
}

fn check_manifest(id: &str, manifest: &PresetManifest, report: &mut Report) {
    if manifest.id != id {
        report.push(Problem::error(
            Code::InvalidValue,
            "preset.yaml:id",
            format!(
                "The preset in presets/{id}/ is '{id}', not '{}'.",
                manifest.id
            ),
        ));
    }
    if manifest.version == 0 {
        report.push(Problem::error(
            Code::InvalidValue,
            "preset.yaml:version",
            "Versions start at 1.",
        ));
    }
    if manifest.title.trim().is_empty() {
        report.push(Problem::error(
            Code::InvalidValue,
            "preset.yaml:title",
            "This may not be empty.",
        ));
    }
    let mut exported = BTreeSet::new();
    for (position, requirement) in
        manifest.export_requirements.iter().enumerate()
    {
        if !exported.insert(requirement) {
            report.push(Problem::error(
                Code::DuplicateId,
                format!("preset.yaml:export_requirements[{position}]"),
                format!("{requirement} is listed twice."),
            ));
        }
    }
    for (position, owned) in manifest.owned_elsewhere.iter().enumerate() {
        let path = format!("preset.yaml:owned_elsewhere[{position}]");
        if owned.requirements.is_empty()
            || owned.subject.trim().is_empty()
            || owned.owner.trim().is_empty()
        {
            report.push(Problem::error(
                Code::InvalidValue,
                path,
                "Name the requirements, what they are about and who owns them.",
            ));
        }
    }
}

/// A requirement is answered in one way: by a dashboard, by the Export
/// action, or by another part of the platform.
fn check_requirements(
    manifest: &PresetManifest,
    set: &ConfigSet,
    report: &mut Report,
) {
    let on_dashboards: BTreeSet<&str> = set
        .dashboards
        .values()
        .flat_map(|dashboard| dashboard.requirements.iter().map(String::as_str))
        .collect();
    let exported: BTreeSet<&str> = manifest
        .export_requirements
        .iter()
        .map(String::as_str)
        .collect();
    let mut elsewhere: BTreeSet<&str> = BTreeSet::new();
    for owned in &manifest.owned_elsewhere {
        for requirement in &owned.requirements {
            if !elsewhere.insert(requirement) {
                report.push(Problem::error(
                    Code::DuplicateId,
                    "preset.yaml:owned_elsewhere",
                    format!("{requirement} is listed twice."),
                ));
            }
        }
    }
    for requirement in &elsewhere {
        if on_dashboards.contains(requirement) || exported.contains(requirement)
        {
            report.push(Problem::error(
                Code::InvalidValue,
                "preset.yaml:owned_elsewhere",
                format!("{requirement} is owned elsewhere, yet this preset answers it too."),
            ));
        }
    }
    for requirement in &exported {
        if on_dashboards.contains(requirement) {
            report.push(Problem::error(
                Code::InvalidValue,
                "preset.yaml:export_requirements",
                format!("{requirement} is answered by Export; no dashboard should claim it."),
            ));
        }
    }
    // A widget's requirement is shown on a dashboard that answers it.
    for (key, widget) in &set.widgets {
        for requirement in &widget.requirements {
            let shown = set.dashboards.values().any(|dashboard| {
                dashboard.requirements.contains(requirement)
                    && dashboard.layout.iter().any(|item| item.widget == *key)
            });
            if !shown {
                report.push(Problem::error(
                    Code::DanglingReference,
                    format!("widgets.{key}.requirements"),
                    format!(
                        "No dashboard answering {requirement} shows '{key}'."
                    ),
                ));
            }
        }
    }
}

#[cfg(test)]
#[path = "presets_tests.rs"]
mod presets_tests;
