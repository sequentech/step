// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`]: every shipped preset is valid, every widget in it
//! draws for every choice a viewer can make, and the COMELEC preset answers
//! every monitoring record.

use super::*;
use crate::monitoring::compute::{evaluate, QueryResult};
use crate::monitoring::config::Ratio;
use crate::monitoring::config::{Dashboard, DynamicOptions, Theme, Widget};
use crate::monitoring::render_request::build_board;
use crate::monitoring::resolve::{
    resolve_widget, DynamicOptionValues, ResolvedWidget, SelectorState,
};
use crate::monitoring::sample::{sample_payload, SAMPLE_DAYS};
use crate::monitoring::sources::{DataSourceId, Measure};
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;

fn monitoring_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/monitoring")
}

fn loaded(id: &str) -> Preset {
    match load(id).unwrap_or_else(|| panic!("no preset {id}")) {
        Ok(preset) => preset,
        Err(report) => panic!("preset {id} refused:\n{report}"),
    }
}

fn all_presets() -> Vec<Preset> {
    PRESETS.iter().map(|source| loaded(source.id)).collect()
}

// -- every preset ----------------------------------------------------------

#[test]
fn every_preset_loads_without_a_single_problem() {
    for source in PRESETS {
        match source.load() {
            Ok(_) => {}
            Err(report) => panic!("{}:\n{report}", source.id),
        }
    }
    // Warnings too: a preset is the example administrators copy.
    for source in PRESETS {
        let mut report = Report::default();
        for file in source.files {
            let parsed = match file.kind {
                ConfigKind::Widget => parse_widget(file.yaml).report,
                ConfigKind::Dashboard => parse_dashboard(file.yaml).report,
                ConfigKind::Theme => parse_theme(file.yaml).report,
                ConfigKind::Settings => parse_settings(file.yaml).report,
            };
            report.problems.extend(located(file.path, parsed));
        }
        assert!(report.problems.is_empty(), "{}:\n{report}", source.id);
    }
}

#[test]
fn the_build_lists_every_file_of_every_preset_directory() {
    let mut ids = BTreeSet::new();
    for source in PRESETS {
        assert!(ids.insert(source.id), "{} is listed twice", source.id);
        let root = monitoring_dir().join("presets").join(source.id);
        let mut on_disk = BTreeSet::new();
        let mut pending = vec![root.clone()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).expect("preset directory") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let relative = path.strip_prefix(&root).unwrap();
                    on_disk.insert(relative.to_string_lossy().to_string());
                }
            }
        }
        let mut listed: BTreeSet<String> = source
            .files
            .iter()
            .map(|file| file.path.to_string())
            .collect();
        listed.insert("preset.yaml".to_string());
        assert_eq!(on_disk, listed, "presets/{}", source.id);
    }
    let directories: BTreeSet<String> =
        std::fs::read_dir(monitoring_dir().join("presets"))
            .unwrap()
            .map(|entry| {
                entry.unwrap().file_name().to_string_lossy().to_string()
            })
            .collect();
    let registered: BTreeSet<String> =
        PRESETS.iter().map(|source| source.id.to_string()).collect();
    assert_eq!(
        directories, registered,
        "every preset directory is registered"
    );
}

#[test]
fn documents_are_stored_under_their_ids_with_their_comments() {
    let preset = loaded("comelec");
    let settings: Vec<&PresetDocument> = preset
        .documents
        .iter()
        .filter(|document| document.kind == ConfigKind::Settings)
        .collect();
    assert_eq!(settings.len(), 1);
    assert_eq!(settings[0].key, SETTINGS_KEY);
    for document in &preset.documents {
        assert!(
            document.yaml.starts_with("# SPDX-FileCopyrightText"),
            "{} {} keeps its header",
            document.kind,
            document.key
        );
        match document.kind {
            ConfigKind::Widget => {
                assert!(preset.set.widgets.contains_key(&document.key))
            }
            ConfigKind::Dashboard => {
                assert!(preset.set.dashboards.contains_key(&document.key))
            }
            ConfigKind::Theme => {
                assert!(preset.set.themes.contains_key(&document.key))
            }
            ConfigKind::Settings => {}
        }
    }
    assert_eq!(
        preset.documents.len(),
        1 + preset.set.themes.len()
            + preset.set.widgets.len()
            + preset.set.dashboards.len()
    );
}

#[test]
fn a_broken_preset_says_which_file_is_wrong() {
    const BROKEN: PresetSource = PresetSource {
        id: "broken",
        manifest: "id: broken\nversion: 1\ntitle: Broken\n",
        files: &[
            PresetFile {
                kind: ConfigKind::Settings,
                path: "settings.yaml",
                yaml: include_str!("fixtures/settings.yaml"),
            },
            PresetFile {
                kind: ConfigKind::Widget,
                path: "widgets/misnamed.yaml",
                yaml: include_str!("fixtures/turnout_by_group.yaml"),
            },
            PresetFile {
                kind: ConfigKind::Widget,
                path: "widgets/bad.yaml",
                yaml: "id: bad\ntitle: Bad\nsource: voter_turnout\nquery: {template: summary, measures: [voted]}\nchart: {charts: {k: {type: kpi, query: data, value: voted, sql: SELECT 1}}, rows: [k]}\n",
            },
        ],
    };
    let report = BROKEN.load().expect_err("broken");
    let paths: Vec<&str> = report
        .problems
        .iter()
        .map(|problem| problem.path.as_str())
        .collect();
    assert!(paths.contains(&"widgets/misnamed.yaml:id"), "{report}");
    assert!(
        paths.contains(&"widgets/bad.yaml:chart.charts.k.sql"),
        "{report}"
    );
}

/// A widget shown outside a dashboard, in the catalog or its editor, takes
/// the default theme; without one it would look unlike the preset.
#[test]
fn a_preset_has_a_default_theme() {
    const THEMELESS: PresetSource = PresetSource {
        id: "themeless",
        manifest: "id: themeless\nversion: 1\ntitle: Themeless\n",
        files: &[
            PresetFile {
                kind: ConfigKind::Settings,
                path: "settings.yaml",
                yaml: include_str!("fixtures/settings.yaml"),
            },
            PresetFile {
                kind: ConfigKind::Theme,
                path: "themes/other.yaml",
                yaml: "id: other\nbase: paper\n",
            },
        ],
    };
    let report = THEMELESS.load().expect_err("no default theme");
    let paths: Vec<&str> = report
        .problems
        .iter()
        .map(|problem| problem.path.as_str())
        .collect();
    assert_eq!(paths, ["themes/default.yaml"], "{report}");
}

/// The directory says what a document is; a widget in `dashboards/` is a
/// mistake, not a dashboard.
#[test]
fn a_document_is_where_its_kind_belongs() {
    const MISPLACED: PresetSource = PresetSource {
        id: "misplaced",
        manifest: "id: misplaced\nversion: 1\ntitle: Misplaced\nexport_requirements: [R-1, R-1]\n",
        files: &[
            PresetFile {
                kind: ConfigKind::Settings,
                path: "settings.yaml",
                yaml: include_str!("fixtures/settings.yaml"),
            },
            PresetFile {
                kind: ConfigKind::Theme,
                path: "themes/default.yaml",
                yaml: include_str!("fixtures/theme_default.yaml"),
            },
            PresetFile {
                kind: ConfigKind::Widget,
                path: "dashboards/turnout-by-group.yaml",
                yaml: include_str!("fixtures/turnout_by_group.yaml"),
            },
        ],
    };
    let report = MISPLACED.load().expect_err("misplaced");
    let paths: Vec<&str> = report
        .problems
        .iter()
        .map(|problem| problem.path.as_str())
        .collect();
    assert!(
        paths.contains(&"dashboards/turnout-by-group.yaml"),
        "{report}"
    );
    assert!(
        paths.contains(&"preset.yaml:export_requirements[1]"),
        "{report}"
    );
}

#[test]
fn a_requirement_is_answered_in_one_place() {
    let manifest = "id: twice\nversion: 1\ntitle: Twice\nexport_requirements: [R-1]\nowned_elsewhere:\n  - {requirements: [R-2], subject: Logs, owner: Team}\n";
    const DASHBOARD: &str = "id: d\ntitle: D\nrequirements: [R-1, R-2, SW-F-0260, SW-F-0372]\nlayout: [{widget: turnout-by-group, width: 12}]\n";
    let source = PresetSource {
        id: "twice",
        manifest,
        files: &[
            PresetFile {
                kind: ConfigKind::Settings,
                path: "settings.yaml",
                yaml: include_str!("fixtures/settings.yaml"),
            },
            PresetFile {
                kind: ConfigKind::Theme,
                path: "themes/default.yaml",
                yaml: include_str!("fixtures/theme_default.yaml"),
            },
            PresetFile {
                kind: ConfigKind::Widget,
                path: "widgets/turnout-by-group.yaml",
                yaml: include_str!("fixtures/turnout_by_group.yaml"),
            },
            PresetFile {
                kind: ConfigKind::Dashboard,
                path: "dashboards/d.yaml",
                yaml: DASHBOARD,
            },
        ],
    };
    let report = source.load().expect_err("claimed twice");
    let text = report.to_string();
    assert!(text.contains("R-1 is answered by Export"), "{text}");
    assert!(text.contains("R-2 is owned elsewhere"), "{text}");
}

// -- every widget, every choice --------------------------------------------

fn dynamic() -> DynamicOptionValues {
    DynamicOptionValues {
        event_days: SAMPLE_DAYS.iter().map(|day| day.to_string()).collect(),
    }
}

/// Every set of selector values a viewer can pick, hidden selectors left out.
fn combinations(widget: &Widget) -> Vec<IndexMap<String, String>> {
    let mut combinations = vec![IndexMap::new()];
    for (name, selector) in &widget.selectors {
        let options: Vec<String> = match selector.options_from {
            Some(DynamicOptions::EventDays) => dynamic().event_days,
            None => selector.options.keys().cloned().collect(),
        };
        combinations = combinations
            .into_iter()
            .flat_map(|chosen: IndexMap<String, String>| {
                let shown = selector.when.as_ref().map_or(true, |when| {
                    chosen
                        .get(&when.selector)
                        .is_some_and(|value| when.one_of.contains(value))
                });
                if !shown {
                    return vec![chosen];
                }
                options
                    .iter()
                    .map(|option| {
                        let mut chosen = chosen.clone();
                        chosen.insert(name.clone(), option.clone());
                        chosen
                    })
                    .collect()
            })
            .collect();
    }
    combinations
}

/// A dashboard's theme, which the policy has checked exists; a widget drawn
/// on its own takes the default theme when the preset has one.
fn theme_for<'p>(
    preset: &'p Preset,
    dashboard: Option<&Dashboard>,
) -> Option<&'p Theme> {
    let id = dashboard
        .and_then(|dashboard| dashboard.theme.as_deref())
        .unwrap_or(crate::monitoring::config::DEFAULT_THEME);
    preset.set.themes.get(id)
}

/// Resolves, evaluates against the sample and builds the board, or says
/// why not.
fn draw(
    preset: &Preset,
    widget: &Widget,
    dashboard: Option<&Dashboard>,
    values: &IndexMap<String, String>,
    requested: &IndexMap<String, String>,
) -> Result<(ResolvedWidget, Value), String> {
    let resolved = resolve_widget(widget, values, requested, &dynamic())
        .map_err(|report| format!("resolve: {report}"))?;
    let settings = preset.set.settings.as_ref();
    let payload = sample_payload(widget.source, settings);
    let mut data: IndexMap<String, QueryResult> = IndexMap::new();
    for (name, query) in &resolved.queries {
        let result = evaluate(widget.source, query, &payload, settings)
            .map_err(|report| format!("query {name}: {report}"))?;
        data.insert(name.clone(), result);
    }
    let board = build_board(widget, theme_for(preset, dashboard), &data);
    let unread = unknown_columns(&board);
    if !unread.is_empty() {
        return Err(format!("chart reads columns its query lacks: {unread:?}"));
    }
    let unoriented = unoriented_bars(&board);
    if !unoriented.is_empty() {
        return Err(format!(
            "bars that leave their orientation to the engine: {unoriented:?}"
        ));
    }
    let unlisted = unlisted_table_columns(&board);
    if !unlisted.is_empty() {
        return Err(format!(
            "a table shows columns it does not list: {unlisted:?}"
        ));
    }
    Ok((resolved, board))
}

/// Chart fields that name a column of the chart's query.
const COLUMN_FIELDS: &[&str] = &[
    "x",
    "y",
    "color",
    "theta",
    "value",
    "size",
    "shape",
    "opacity",
    "lookup",
    "latitude",
    "longitude",
];

/// Every column a chart names that its query does not return, as
/// `chart.field: column`.
fn unknown_columns(board: &Value) -> Vec<String> {
    let mut unknown = Vec::new();
    let Some(charts) = board["charts"].as_object() else {
        return vec!["no charts".to_string()];
    };
    let columns_of = |query: &Value| -> Option<BTreeSet<String>> {
        let name = query.as_str()?;
        Some(
            board["queries"][name]["columns"]
                .as_array()?
                .iter()
                .filter_map(|column| column.as_str().map(str::to_string))
                .collect(),
        )
    };
    for (name, chart) in charts {
        let Some(columns) = columns_of(&chart["query"]) else {
            unknown.push(format!("{name}: no governed query"));
            continue;
        };
        let mut check =
            |field: String, value: &Value, columns: &BTreeSet<String>| {
                let named: Vec<&str> = match value {
                    Value::String(column) => vec![column.as_str()],
                    Value::Array(items) => {
                        items.iter().filter_map(Value::as_str).collect()
                    }
                    Value::Object(object) => object
                        .get("field")
                        .and_then(Value::as_str)
                        .into_iter()
                        .collect(),
                    _ => Vec::new(),
                };
                for column in named {
                    if !columns.contains(column) {
                        unknown.push(format!("{name}.{field}: {column}"));
                    }
                }
            };
        for field in COLUMN_FIELDS {
            if let Some(value) = chart.get(*field) {
                check(field.to_string(), value, &columns);
            }
        }
        for (pointer, field) in [
            ("/multiples/rows", "multiples.rows"),
            ("/multiples/columns", "multiples.columns"),
            ("/stroke/color", "stroke.color"),
            ("/stroke/width", "stroke.width"),
        ] {
            if let Some(value) = chart.pointer(pointer) {
                check(field.into(), value, &columns);
            }
        }
        // As a string, a background is a colour.
        if let Some(value) = chart.get("background").filter(|v| v.is_object()) {
            check("background".into(), value, &columns);
        }
        if let Some(value) = chart.pointer("/support/value") {
            check("support.value".into(), value, &columns);
        }
        if let Some(by) = chart.pointer("/sort/by") {
            check("sort.by".into(), by, &columns);
        }
        for keyed in ["/style/columns", "/conditional_formatting"] {
            if let Some(object) =
                chart.pointer(keyed).and_then(Value::as_object)
            {
                for column in object.keys() {
                    check(
                        keyed.into(),
                        &Value::String(column.clone()),
                        &columns,
                    );
                }
            }
        }
        if let Some(entries) = chart["support_table"].as_array() {
            for entry in entries {
                for field in ["source", "per_series"] {
                    if let Some(value) = entry.get(field) {
                        check(
                            format!("support_table.{field}"),
                            value,
                            &columns,
                        );
                    }
                }
            }
        }
        if let Some(layers) = chart["layers"].as_array() {
            for (position, layer) in layers.iter().enumerate() {
                let layer_columns = match layer.get("query") {
                    Some(query) => columns_of(query).unwrap_or_default(),
                    None => columns.clone(),
                };
                for field in ["x", "y", "color"] {
                    if let Some(value) = layer.get(field) {
                        check(
                            format!("layers[{position}].{field}"),
                            value,
                            &layer_columns,
                        );
                    }
                }
            }
        }
    }
    unknown
}

/// The engine turns a bar over a text column on its side, and a bar on its
/// side ranks by value, so an hourly series left to it reads 09:00, 10:00,
/// …, 00:00. A preset's bars say which way they stand.
fn unoriented_bars(board: &Value) -> Vec<String> {
    board["charts"]
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, chart)| chart["type"] == "bar")
        .filter(|(_, chart)| chart.pointer("/style/orientation").is_none())
        .map(|(name, _)| name.clone())
        .collect()
}

/// A table shows every column it does not hide, so a preset's tables list
/// each column of their query, shown with its words or hidden: a column the
/// platform adds later never appears unannounced.
fn unlisted_table_columns(board: &Value) -> Vec<String> {
    let mut unlisted = Vec::new();
    let Some(charts) = board["charts"].as_object() else {
        return unlisted;
    };
    for (name, chart) in charts {
        if chart["type"] != "table" {
            continue;
        }
        let listed = chart.pointer("/style/columns").and_then(Value::as_object);
        let Some(query) = chart["query"].as_str() else {
            continue;
        };
        for column in board["queries"][query]["columns"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !listed.is_some_and(|listed| listed.contains_key(column)) {
                unlisted.push(format!("{name}: {column}"));
            }
        }
    }
    unlisted
}

#[test]
fn every_widget_draws_for_every_choice_a_viewer_can_make() {
    let mut drawn = 0;
    let mut failures = Vec::new();
    for preset in all_presets() {
        for (key, widget) in &preset.set.widgets {
            for requested in combinations(widget) {
                match draw(&preset, widget, None, &IndexMap::new(), &requested)
                {
                    Ok(_) => drawn += 1,
                    Err(why) => failures.push(format!(
                        "{}/{key} {requested:?}: {why}",
                        preset.manifest.id
                    )),
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(drawn > 40, "only {drawn} boards drawn");
}

#[test]
fn every_dashboard_default_is_the_value_the_widget_opens_with() {
    for preset in all_presets() {
        for (id, dashboard) in &preset.set.dashboards {
            for item in &dashboard.layout {
                let widget = &preset.set.widgets[&item.widget];
                let (resolved, _) = draw(
                    &preset,
                    widget,
                    Some(dashboard),
                    &item.values,
                    &IndexMap::new(),
                )
                .unwrap_or_else(|why| panic!("{id}/{}: {why}", item.widget));
                for (selector, value) in &item.values {
                    assert_eq!(
                        resolved.selectors[selector],
                        SelectorState::Value(value.clone()),
                        "{id}/{}: {selector}",
                        item.widget
                    );
                }
            }
        }
    }
}

/// The boards the renderer is sent for each widget as it opens, and as each
/// dashboard that sets its own defaults opens it. The renderer's tests draw
/// these with the pinned dbt Charts, so a chart the engine refuses fails CI
/// even though this crate cannot run the engine.
///
/// `MONITORING_UPDATE_BOARDS=1` rewrites them.
#[test]
fn the_boards_match_the_files_the_renderer_is_tested_with() {
    let root = monitoring_dir().join("fixtures/boards");
    let update = std::env::var_os("MONITORING_UPDATE_BOARDS").is_some();
    let mut expected: BTreeMap<PathBuf, String> = BTreeMap::new();
    for preset in all_presets() {
        let dir = root.join(&preset.manifest.id);
        for (key, widget) in &preset.set.widgets {
            let shown_on = preset.set.dashboards.values().find(|dashboard| {
                dashboard.layout.iter().any(|item| item.widget == *key)
            });
            let (_, board) = draw(
                &preset,
                widget,
                shown_on,
                &IndexMap::new(),
                &IndexMap::new(),
            )
            .unwrap_or_else(|why| panic!("{key}: {why}"));
            expected.insert(dir.join(format!("{key}.json")), pretty(&board));
        }
        for (id, dashboard) in &preset.set.dashboards {
            for item in dashboard
                .layout
                .iter()
                .filter(|item| !item.values.is_empty())
            {
                let widget = &preset.set.widgets[&item.widget];
                let (_, board) = draw(
                    &preset,
                    widget,
                    Some(dashboard),
                    &item.values,
                    &IndexMap::new(),
                )
                .unwrap_or_else(|why| panic!("{id}/{}: {why}", item.widget));
                expected.insert(
                    dir.join(format!("{}.{id}.json", item.widget)),
                    pretty(&board),
                );
            }
        }
    }
    if let Some(dir) = std::env::var_os("MONITORING_ALL_BOARDS_DIR") {
        write_every_combination(Path::new(&dir));
    }
    if update {
        let _ = std::fs::remove_dir_all(&root);
        for (path, text) in &expected {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
    }
    let mut on_disk = BTreeSet::new();
    if root.exists() {
        for preset in std::fs::read_dir(&root).unwrap() {
            for file in std::fs::read_dir(preset.unwrap().path()).unwrap() {
                on_disk.insert(file.unwrap().path());
            }
        }
    }
    let wanted: BTreeSet<PathBuf> = expected.keys().cloned().collect();
    assert_eq!(
        on_disk, wanted,
        "board files differ; run with MONITORING_UPDATE_BOARDS=1"
    );
    for (path, text) in &expected {
        let found = std::fs::read_to_string(path).unwrap();
        assert!(
            found == *text,
            "{} is stale; run with MONITORING_UPDATE_BOARDS=1",
            path.display()
        );
    }
}

fn pretty(board: &Value) -> String {
    let mut text = serde_json::to_string_pretty(board).unwrap();
    text.push('\n');
    text
}

/// Every combination's board, for drawing them all with the engine by hand.
fn write_every_combination(dir: &Path) {
    for preset in all_presets() {
        for (key, widget) in &preset.set.widgets {
            for (position, requested) in
                combinations(widget).into_iter().enumerate()
            {
                let (_, board) =
                    draw(&preset, widget, None, &IndexMap::new(), &requested)
                        .unwrap();
                let path = dir
                    .join(&preset.manifest.id)
                    .join(format!("{key}.{position}.json"));
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, pretty(&board)).unwrap();
            }
        }
    }
}

// -- COMELEC ---------------------------------------------------------------

/// The monitoring records, by the data source whose figures answer them.
const RECORDS_BY_SOURCE: &[(DataSourceId, &[&str])] = &[
    (
        DataSourceId::VoterTurnout,
        &[
            "SW-F-0259",
            "SW-F-0260",
            "SW-F-0261",
            "SW-F-0371",
            "SW-F-0372",
            "SW-F-0373",
        ],
    ),
    (DataSourceId::TestVoting, &["SW-F-0253", "SW-F-0367"]),
    (
        DataSourceId::EnrollmentDecisions,
        &["SW-F-0249", "SW-F-0250", "SW-F-0366"],
    ),
    (DataSourceId::VotingCredentials, &["SW-F-0251"]),
    (
        DataSourceId::PollStatus,
        &[
            "SW-F-0256",
            "SW-F-0257",
            "SW-F-0258",
            "SW-F-0262",
            "SW-F-0368",
            "SW-F-0369",
            "SW-F-0370",
        ],
    ),
    (
        DataSourceId::FinalTestingLockdown,
        &["SW-F-0254", "SW-F-0255"],
    ),
    (
        DataSourceId::CountingTransmission,
        &["SW-F-0263", "SW-F-0264", "SW-F-0265"],
    ),
    (
        DataSourceId::VotingEnrollmentActivity,
        &["SW-F-0267", "SW-F-0282"],
    ),
    (
        DataSourceId::AccessSecurity,
        &["SW-F-0252", "SW-F-0268", "SW-F-0280", "SW-F-0281"],
    ),
    (DataSourceId::AttackDetections, &["SW-F-0283"]),
    (DataSourceId::Helpdesk, &["SW-F-0269"]),
];

const OVERVIEW_RECORDS: &[&str] = &["SW-F-0247", "SW-F-0279", "SW-F-0365"];
const EXPORT_RECORDS: &[&str] = &["SW-F-0270"];

#[test]
fn comelec_answers_all_36_monitoring_records() {
    let preset = loaded("comelec");
    let mut records: BTreeSet<&str> = RECORDS_BY_SOURCE
        .iter()
        .flat_map(|(_, records)| records.iter().copied())
        .collect();
    records.extend(OVERVIEW_RECORDS);
    records.extend(EXPORT_RECORDS);
    assert_eq!(records.len(), 36);

    let mut answered: BTreeSet<&str> = preset
        .set
        .dashboards
        .values()
        .flat_map(|dashboard| dashboard.requirements.iter().map(String::as_str))
        .collect();
    let exported: BTreeSet<&str> = preset
        .manifest
        .export_requirements
        .iter()
        .map(String::as_str)
        .collect();
    assert_eq!(exported, EXPORT_RECORDS.iter().copied().collect());
    answered.extend(&exported);
    assert_eq!(answered, records);
}

#[test]
fn each_record_is_shown_with_the_figures_of_its_data_source() {
    let preset = loaded("comelec");
    for (source, records) in RECORDS_BY_SOURCE {
        for record in *records {
            let dashboards: Vec<&Dashboard> = preset
                .set
                .dashboards
                .values()
                .filter(|dashboard| {
                    dashboard.requirements.iter().any(|r| r == record)
                })
                .collect();
            assert!(!dashboards.is_empty(), "{record} has no dashboard");
            for dashboard in dashboards {
                let shows_source = dashboard.layout.iter().any(|item| {
                    let widget = &preset.set.widgets[&item.widget];
                    widget.source == *source
                        && widget.requirements.iter().any(|r| r == record)
                });
                assert!(
                    shows_source,
                    "{} answers {record} without a {source} widget for it",
                    dashboard.id
                );
            }
        }
    }
    let overview = &preset.set.dashboards["overview"];
    for record in OVERVIEW_RECORDS {
        assert!(overview.requirements.iter().any(|r| r == record));
    }
}

#[test]
fn comelec_has_a_widget_for_every_data_source() {
    let preset = loaded("comelec");
    let sources: BTreeSet<DataSourceId> = preset
        .set
        .widgets
        .values()
        .map(|widget| widget.source)
        .collect();
    let every: BTreeSet<DataSourceId> = DataSourceId::iter().collect();
    assert_eq!(sources, every);
}

#[test]
fn every_comelec_widget_is_on_a_dashboard() {
    let preset = loaded("comelec");
    for key in preset.set.widgets.keys() {
        assert!(
            preset.set.dashboards.values().any(|dashboard| dashboard
                .layout
                .iter()
                .any(|item| item.widget == *key)),
            "{key} is on no dashboard"
        );
    }
}

#[test]
fn comelec_names_the_owners_of_the_six_records_it_leaves() {
    let preset = loaded("comelec");
    let owners: BTreeMap<&str, &str> = preset
        .manifest
        .owned_elsewhere
        .iter()
        .flat_map(|owned| {
            owned.requirements.iter().map(move |requirement| {
                (requirement.as_str(), owned.owner.as_str())
            })
        })
        .collect();
    let expected: BTreeMap<&str, &str> = [
        ("SW-F-0248", "DEV-COMMUNICATIONS"),
        ("SW-F-0272", "DEV-COMMUNICATIONS"),
        ("SW-F-0273", "DEV-COMMUNICATIONS"),
        ("SW-F-0266", "DEV-REPORTS"),
        ("SW-F-0278", "DEV-PLATFORM-SECURITY"),
        ("SW-F-0374", "DEV-AUDIT-LOGS"),
    ]
    .into();
    assert_eq!(owners, expected);
}

/// The three turnout ratios each have a dashboard that opens on them, and
/// the poll milestones each on theirs.
#[test]
fn dashboards_open_on_the_figure_their_record_asks_for() {
    let preset = loaded("comelec");
    let opened_on = |dashboard: &str, widget: &str, query: &str| {
        let dashboard = &preset.set.dashboards[dashboard];
        let item = dashboard
            .layout
            .iter()
            .find(|item| item.widget == widget)
            .unwrap_or_else(|| panic!("{} lacks {widget}", dashboard.id));
        let (resolved, _) = draw(
            &preset,
            &preset.set.widgets[widget],
            Some(dashboard),
            &item.values,
            &IndexMap::new(),
        )
        .unwrap();
        resolved.queries[query].ratio
    };
    use Measure::*;
    let ratio = |numerator, denominator| Some(Ratio(numerator, denominator));
    assert_eq!(
        opened_on("req-0259", "turnout-by-group", "data"),
        ratio(Voted, Registered)
    );
    assert_eq!(
        opened_on("req-0260", "turnout-by-group", "data"),
        ratio(Voted, PreEnrolled)
    );
    assert_eq!(
        opened_on("req-0261", "turnout-by-group", "data"),
        ratio(PreEnrolled, Registered)
    );
    assert_eq!(
        opened_on("req-0256", "poll-status", "milestone"),
        ratio(Initialized, Posts)
    );
    assert_eq!(
        opened_on("req-0257", "poll-status", "milestone"),
        ratio(Opened, Posts)
    );
    assert_eq!(
        opened_on("req-0258", "poll-status", "milestone"),
        ratio(Closed, Posts)
    );
    for (dashboard, figure) in [
        ("req-0259", ratio(Voted, Registered)),
        ("req-0260", ratio(Voted, PreEnrolled)),
        ("req-0261", ratio(PreEnrolled, Registered)),
    ] {
        assert_eq!(opened_on(dashboard, "turnout-by-post", "data"), figure);
        // The summary shows all three figures on every turnout dashboard.
        assert_eq!(
            opened_on(dashboard, "turnout-summary", "voted_reg"),
            ratio(Voted, Registered)
        );
        assert_eq!(
            opened_on(dashboard, "turnout-summary", "voted_pre"),
            ratio(Voted, PreEnrolled)
        );
        assert_eq!(
            opened_on(dashboard, "turnout-summary", "pre_reg"),
            ratio(PreEnrolled, Registered)
        );
    }
    assert_eq!(
        opened_on("req-0254", "final-testing-lockdown", "milestone"),
        ratio(Tested, Posts)
    );
    assert_eq!(
        opened_on("req-0263", "counting-transmission", "milestone"),
        ratio(Tallied, Posts)
    );
    assert_eq!(
        opened_on("req-0264", "counting-transmission", "milestone"),
        ratio(Transmitted, Posts)
    );
}

/// Each choice a viewer can make shows other figures: an option that draws
/// the same figures as another is one the viewer cannot tell apart.
#[test]
fn every_choice_a_viewer_can_make_shows_other_figures() {
    let mut same = Vec::new();
    for preset in all_presets() {
        for (key, widget) in &preset.set.widgets {
            let mut seen: BTreeMap<String, IndexMap<String, String>> =
                BTreeMap::new();
            for requested in combinations(widget) {
                let (_, board) =
                    draw(&preset, widget, None, &IndexMap::new(), &requested)
                        .unwrap_or_else(|why| panic!("{key}: {why}"));
                let figures = board["queries"].to_string();
                if let Some(earlier) = seen.get(&figures) {
                    same.push(format!(
                        "{}/{key}: {earlier:?} and {requested:?}",
                        preset.manifest.id
                    ));
                } else {
                    seen.insert(figures, requested);
                }
            }
        }
    }
    assert!(same.is_empty(), "same figures:\n{}", same.join("\n"));
}

// -- a second customer -----------------------------------------------------

#[test]
fn a_second_customer_s_preset_has_its_own_words_and_widgets_on_the_same_code() {
    let comelec = loaded("comelec");
    let campus = loaded("campus");
    let ids = |preset: &Preset| -> BTreeSet<String> {
        preset.set.widgets.keys().cloned().collect()
    };
    assert!(ids(&comelec).is_disjoint(&ids(&campus)));
    let dimensions = |preset: &Preset| -> BTreeSet<String> {
        preset
            .set
            .settings
            .as_ref()
            .unwrap()
            .dimensions
            .keys()
            .cloned()
            .collect()
    };
    assert!(dimensions(&comelec).is_disjoint(&dimensions(&campus)));
    assert_ne!(
        comelec.set.settings.as_ref().unwrap().time_zone,
        campus.set.settings.as_ref().unwrap().time_zone
    );
    assert!(campus.manifest.owned_elsewhere.is_empty());
}

/// Nothing outside the preset registry names a preset: no code path is
/// chosen by customer.
#[test]
fn no_code_is_chosen_by_preset() {
    let mut offenders = Vec::new();
    for entry in std::fs::read_dir(monitoring_dir()).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if !name.ends_with(".rs")
            || name.ends_with("_tests.rs")
            || name == "presets.rs"
        {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
        for preset in PRESETS {
            if text.contains(&format!("\"{}\"", preset.id))
                || text.contains(&preset.id.to_lowercase())
            {
                offenders.push(format!("{name} names {}", preset.id));
            }
        }
    }
    assert!(offenders.is_empty(), "{offenders:?}");
}

/// A dashboard selector is worded in the preset's language, as its widgets'
/// titles are; the portal's own words are only for configurations that
/// predate this.
#[test]
fn every_dashboard_selector_is_worded_by_its_preset() {
    for preset in all_presets() {
        let settings = preset.set.settings.as_ref().expect("settings");
        for dashboard in preset.set.dashboards.values() {
            for selector in &dashboard.selectors {
                assert!(
                    settings.selector_words(*selector).is_some(),
                    "{}: dashboard '{}' shows the {selector} selector, which settings.yaml does not word",
                    preset.manifest.id,
                    dashboard.id
                );
            }
        }
    }
}

/// FNV-1a: stable across platforms and releases, enough to notice a change.
fn digest(preset: &PresetSource) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes.iter().chain([0_u8].iter()) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(preset.manifest.as_bytes());
    for file in preset.files {
        feed(file.path.as_bytes());
        feed(file.yaml.as_bytes());
    }
    format!("{hash:016x}")
}

/// An event records the preset version it was reset to, so a preset whose
/// documents change is given a new version. `preset_versions.txt` holds each
/// preset's version and digest; `MONITORING_UPDATE_PRESET_VERSIONS=1`
/// rewrites it once the version is raised.
#[test]
fn a_preset_that_changes_is_given_a_new_version() {
    let path = monitoring_dir().join("fixtures/preset_versions.txt");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let recorded: BTreeMap<String, (u32, String)> = text
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(fields.len(), 3, "id version digest: '{line}'");
            (
                fields[0].to_string(),
                (fields[1].parse().unwrap(), fields[2].to_string()),
            )
        })
        .collect();
    let mut current = BTreeMap::new();
    let mut unbumped = Vec::new();
    for source in PRESETS {
        let version = loaded(source.id).manifest.version;
        let digest = digest(source);
        if let Some((was, digested)) = recorded.get(source.id) {
            assert!(version >= *was, "{}: version went back", source.id);
            if version == *was && *digested != digest {
                unbumped.push(source.id);
            }
        }
        current.insert(source.id.to_string(), (version, digest));
    }
    assert!(
        unbumped.is_empty(),
        "{unbumped:?} changed; raise `version` in preset.yaml"
    );
    if std::env::var_os("MONITORING_UPDATE_PRESET_VERSIONS").is_some() {
        let mut text = String::from(
            "# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>\n#\n# SPDX-License-Identifier: AGPL-3.0-only\n\n# preset version digest\n",
        );
        for (id, (version, digest)) in &current {
            text.push_str(&format!("{id} {version} {digest}\n"));
        }
        std::fs::write(&path, text).unwrap();
        return;
    }
    assert_eq!(
        recorded, current,
        "run with MONITORING_UPDATE_PRESET_VERSIONS=1 to record new versions"
    );
}

#[test]
fn the_column_check_reads_every_field_that_names_a_column() {
    let board = serde_json::json!({
        "queries": {"data": {"columns": ["group", "voted"], "values": []}},
        "charts": {"k": {
            "type": "bar", "query": "data", "x": "group", "y": "voted",
            "multiples": {"rows": "region", "columns": "country"},
            "stroke": {"color": {"field": "state"}, "width": {"field": "weight"}},
            "background": {"field": "shade"},
        }},
    });
    assert_eq!(
        unknown_columns(&board),
        [
            "k.multiples.rows: region",
            "k.multiples.columns: country",
            "k.stroke.color: state",
            "k.stroke.width: weight",
            "k.background: shade",
        ]
    );
    let painted = serde_json::json!({
        "queries": {"data": {"columns": ["group"], "values": []}},
        "charts": {"k": {"type": "bar", "query": "data", "x": "group",
            "background": "dbt-grays.canvas"}},
    });
    assert!(
        unknown_columns(&painted).is_empty(),
        "a colour is not a column"
    );
}
