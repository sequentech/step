// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::monitoring::presets::{self, Preset, SETTINGS_KEY};
use crate::monitoring::problem::Severity;
use strum::IntoEnumIterator;

fn campus() -> Preset {
    presets::load("campus").unwrap().unwrap()
}

fn yaml_of(preset: &Preset, kind: ConfigKind, key: &str) -> &'static str {
    preset
        .documents
        .iter()
        .find(|document| document.kind == kind && document.key == key)
        .unwrap()
        .yaml
}

fn live(preset: &Preset) -> Vec<LiveDocument<'_>> {
    preset
        .documents
        .iter()
        .map(|document| LiveDocument {
            kind: document.kind,
            key: &document.key,
            yaml: document.yaml,
        })
        .collect()
}

fn codes(report: &Report) -> Vec<Code> {
    report.errors().map(|problem| problem.code).collect()
}

#[test]
fn a_document_is_digested_as_its_utf8_text() {
    assert_eq!(
        document_digest("id: w\n"),
        "9d603766b896b909b9ed80083670b44be1229dfea2689dff116703b585c102a2"
    );
}

#[test]
fn modes_origins_and_changes_are_named_as_the_tables_store_them() {
    let names = |values: Vec<String>| values;
    assert_eq!(
        names(DashboardMode::iter().map(|mode| mode.to_string()).collect()),
        ["LEGACY", "CONFIGURED"]
    );
    assert_eq!(
        names(
            RevisionOrigin::iter()
                .map(|origin| origin.to_string())
                .collect()
        ),
        ["PRESET", "EDITOR"]
    );
    assert_eq!(
        names(
            DocumentChange::iter()
                .map(|change| change.to_string())
                .collect()
        ),
        ["UPSERT", "DELETE"]
    );
    assert_eq!(
        serde_json::to_value(DashboardMode::Configured).unwrap(),
        serde_json::json!("CONFIGURED")
    );
    assert_eq!("LEGACY".parse::<DashboardMode>(), Ok(DashboardMode::Legacy));
}

#[test]
fn live_documents_assemble_into_the_set_they_were_saved_as() {
    let preset = campus();
    let assembled = assemble(live(&preset));
    assert!(assembled.report.is_accepted(), "{}", assembled.report);
    assert_eq!(assembled.set, preset.set);
}

#[test]
fn a_stored_document_that_no_longer_passes_is_left_out_and_said_why() {
    let preset = campus();
    let mut documents = live(&preset);
    documents.push(LiveDocument {
        kind: ConfigKind::Widget,
        key: "broken",
        yaml: "id: [",
    });
    let assembled = assemble(documents);
    assert!(!assembled.set.widgets.contains_key("broken"));
    assert_eq!(assembled.set.widgets.len(), preset.set.widgets.len());
    let problems: Vec<(&str, Code)> = assembled
        .report
        .errors()
        .map(|problem| (problem.path.as_str(), problem.code))
        .collect();
    assert_eq!(problems, [("widgets.broken", Code::Unreadable)]);
}

#[test]
fn an_edit_is_checked_and_returns_the_set_it_leaves() {
    let preset = campus();
    let yaml = yaml_of(&preset, ConfigKind::Dashboard, "operations")
        .replace("title: Operations", "title: Operations room");
    let checked = check_edit(
        &preset.set,
        ConfigKind::Dashboard,
        "operations",
        Edit::Upsert(&yaml),
    )
    .unwrap();
    assert_eq!(
        checked.set.dashboards["operations"].title,
        "Operations room"
    );
    assert_eq!(checked.set.widgets, preset.set.widgets);

    let removed = check_edit(
        &preset.set,
        ConfigKind::Dashboard,
        "operations",
        Edit::Delete,
    )
    .unwrap();
    assert!(!removed.set.dashboards.contains_key("operations"));
}

#[test]
fn a_new_document_joins_the_set_under_its_id() {
    let preset = campus();
    let yaml = yaml_of(&preset, ConfigKind::Widget, "polls")
        .replace("id: polls", "id: polls-again");
    let checked = check_edit(
        &preset.set,
        ConfigKind::Widget,
        "polls-again",
        Edit::Upsert(&yaml),
    )
    .unwrap();
    assert_eq!(checked.set.widgets.len(), preset.set.widgets.len() + 1);
    assert_eq!(
        checked.set.widgets.get_index_of("polls-again"),
        Some(preset.set.widgets.len()),
        "a new document goes last, an edited one keeps its place"
    );
}

#[test]
fn settings_are_written_only_by_a_reset() {
    let preset = campus();
    let yaml = yaml_of(&preset, ConfigKind::Settings, SETTINGS_KEY);
    for edit in [Edit::Upsert(yaml), Edit::Delete] {
        let refused =
            check_edit(&preset.set, ConfigKind::Settings, SETTINGS_KEY, edit)
                .unwrap_err();
        assert_eq!(codes(&refused), [Code::PresetOnly]);
    }
}

#[test]
fn an_edit_that_breaks_the_document_or_the_set_is_refused() {
    let preset = campus();
    let polls = yaml_of(&preset, ConfigKind::Widget, "polls");
    let cases: [(ConfigKind, &str, Edit, Code); 6] = [
        (
            ConfigKind::Widget,
            "Polls",
            Edit::Upsert(polls),
            Code::InvalidId,
        ),
        (
            ConfigKind::Widget,
            "polls",
            Edit::Delete,
            Code::DanglingReference,
        ),
        (
            ConfigKind::Widget,
            "other",
            Edit::Upsert(polls),
            Code::InvalidId,
        ),
        (
            ConfigKind::Widget,
            "polls",
            Edit::Upsert("id: ["),
            Code::Unreadable,
        ),
        (
            ConfigKind::Theme,
            DEFAULT_THEME,
            Edit::Delete,
            Code::DanglingReference,
        ),
        (ConfigKind::Widget, "Polls", Edit::Delete, Code::InvalidId),
    ];
    for (kind, key, edit, code) in cases {
        let refused = check_edit(&preset.set, kind, key, edit).unwrap_err();
        assert!(
            codes(&refused).contains(&code),
            "{kind} {key}: expected {code:?} in {refused}"
        );
    }
}

#[test]
fn problems_the_set_already_had_do_not_block_an_unrelated_edit() {
    let preset = campus();
    let mut broken = preset.set.clone();
    broken.widgets.shift_remove("polls");
    assert!(!validate_set(&broken).is_accepted());

    let theme = yaml_of(&preset, ConfigKind::Theme, DEFAULT_THEME)
        .replace("title: Calm", "title: Quiet");
    let checked = check_edit(
        &broken,
        ConfigKind::Theme,
        DEFAULT_THEME,
        Edit::Upsert(&theme),
    )
    .unwrap();
    assert_eq!(
        checked.set.themes[DEFAULT_THEME].title.as_deref(),
        Some("Quiet")
    );
    assert!(checked.report.is_accepted());
    assert!(
        checked
            .report
            .problems
            .iter()
            .all(|problem| problem.severity == Severity::Warning),
        "only what is still worth saying comes back: {}",
        checked.report
    );
}

#[test]
fn a_saved_document_passes_the_set_checks_whatever_the_set_already_lacked() {
    let preset = campus();
    let mut unset = preset.set.clone();
    unset.settings = None;
    assert!(!validate_set(&unset).is_accepted());
    let by_role =
        yaml_of(&preset, ConfigKind::Widget, "participation-by-faculty")
            .replace(
                "id: participation-by-faculty",
                "id: participation-by-role",
            );
    let refused = check_edit(
        &unset,
        ConfigKind::Widget,
        "participation-by-role",
        Edit::Upsert(&by_role),
    )
    .unwrap_err();
    assert!(
        refused
            .errors()
            .any(|problem| problem.code == Code::DanglingReference
                && problem.path
                    == "widgets.participation-by-role.query.group_by"),
        "a new widget grouping voters needs settings: {refused}"
    );
}

#[test]
fn a_document_saved_with_an_error_of_its_own_is_refused_even_if_it_had_it() {
    let preset = campus();
    let mut broken = preset.set.clone();
    broken.widgets.shift_remove("polls");
    let operations = yaml_of(&preset, ConfigKind::Dashboard, "operations");

    let retitled = operations.replacen("title:", "title: Live", 1);
    let refused = check_edit(
        &broken,
        ConfigKind::Dashboard,
        "operations",
        Edit::Upsert(&retitled),
    )
    .unwrap_err();
    assert_eq!(codes(&refused), [Code::DanglingReference], "{refused}");

    let without_polls =
        operations.replace("  - {widget: polls, width: 6}\n", "");
    assert_ne!(without_polls, operations);
    let checked = check_edit(
        &broken,
        ConfigKind::Dashboard,
        "operations",
        Edit::Upsert(&without_polls),
    )
    .unwrap();
    assert!(validate_set(&checked.set).is_accepted());
}

#[test]
fn only_the_target_of_an_edit_is_checked_before_its_revision() {
    assert_eq!(check_target(ConfigKind::Widget, "polls"), Ok(()));
    let cases = [
        (ConfigKind::Settings, SETTINGS_KEY, Code::PresetOnly),
        (ConfigKind::Widget, "Polls", Code::InvalidId),
        (ConfigKind::Dashboard, &"d".repeat(65), Code::InvalidId),
    ];
    for (kind, key, code) in cases {
        let refused = check_target(kind, key).unwrap_err();
        assert_eq!(codes(&refused), [code], "{kind} {key}");
    }
}

#[test]
fn a_reset_writes_what_differs_and_removes_what_the_preset_lacks() {
    let preset = campus();
    let from_nothing = plan_reset(&preset, &[]);
    let everything: Vec<(ConfigKind, &str, DocumentChange)> = from_nothing
        .iter()
        .map(|planned| (planned.kind, planned.key.as_str(), planned.change))
        .collect();
    let expected: Vec<(ConfigKind, &str, DocumentChange)> = preset
        .documents
        .iter()
        .map(|document| {
            (document.kind, document.key.as_str(), DocumentChange::Upsert)
        })
        .collect();
    assert_eq!(
        everything, expected,
        "an event without documents takes them all"
    );
    assert!(plan_reset(&preset, &[])
        .iter()
        .all(|planned| planned.yaml.is_some()));

    let heads: Vec<LiveHead> = preset
        .documents
        .iter()
        .map(|document| LiveHead {
            kind: document.kind,
            key: document.key.clone(),
            digest: Some(document_digest(document.yaml)),
        })
        .collect();
    assert!(
        plan_reset(&preset, &heads).is_empty(),
        "an event already on the preset gets nothing new"
    );

    let mut drifted = heads.clone();
    for head in &mut drifted {
        if head.key == "polls" {
            head.digest = Some(document_digest("edited"));
        }
    }
    drifted.push(LiveHead {
        kind: ConfigKind::Widget,
        key: "extra".to_string(),
        digest: Some(document_digest("id: extra\n")),
    });
    drifted.push(LiveHead {
        kind: ConfigKind::Theme,
        key: "gone".to_string(),
        digest: None,
    });
    let restored = plan_reset(&preset, &drifted);
    let planned: Vec<(ConfigKind, &str, DocumentChange, bool)> = restored
        .iter()
        .map(|planned| {
            (
                planned.kind,
                planned.key.as_str(),
                planned.change,
                planned.yaml.is_some(),
            )
        })
        .collect();
    assert_eq!(
        planned,
        [
            (ConfigKind::Widget, "polls", DocumentChange::Upsert, true),
            (ConfigKind::Widget, "extra", DocumentChange::Delete, false),
        ],
        "an edited document is restored, an added one removed, a removed one left removed"
    );
}

#[test]
fn the_default_theme_is_kept_even_when_no_dashboard_uses_it() {
    let preset = campus();
    let mut bare = preset.set.clone();
    bare.dashboards.clear();
    let refused =
        check_edit(&bare, ConfigKind::Theme, DEFAULT_THEME, Edit::Delete)
            .unwrap_err();
    assert_eq!(codes(&refused), [Code::DanglingReference]);
}
