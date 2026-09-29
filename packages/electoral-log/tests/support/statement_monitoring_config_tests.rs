// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;

fn revision(
    kind: &str,
    key: &str,
    revision: u32,
    action: MonitoringConfigChangeAction,
) -> MonitoringConfigRevisionRef {
    MonitoringConfigRevisionRef {
        kind: MonitoringConfigKindString(kind.to_string()),
        key: MonitoringConfigKeyString(key.to_string()),
        revision,
        digest: match action {
            MonitoringConfigChangeAction::Upsert => {
                Some(MonitoringConfigDigestString("d1".to_string()))
            }
            MonitoringConfigChangeAction::Delete => None,
        },
        action,
    }
}

fn head(details: MonitoringConfigChangeDetails) -> StatementHead {
    let body =
        StatementBody::MonitoringConfigChanged(EventIdString("event-id".to_string()), details);
    StatementHead::from_body(EventIdString("event-id".to_string()), &body)
}

#[test]
fn an_editor_save_names_the_document_and_its_revision() {
    let head = head(MonitoringConfigChangeDetails {
        origin: MonitoringConfigOrigin::Editor,
        preset: None,
        mode: MonitoringDashboardMode::Configured,
        generation: 12,
        revisions: vec![revision(
            "widget",
            "turnout-summary",
            3,
            MonitoringConfigChangeAction::Upsert,
        )],
    });

    assert!(matches!(head.kind, StatementType::MonitoringConfigChanged));
    assert!(matches!(head.event_type, StatementEventType::USER));
    assert!(matches!(head.log_type, StatementLogType::INFO));
    assert_eq!(
        head.description,
        "Monitoring configuration generation 12 (dashboard Configured): widget turnout-summary revision 3 saved."
    );
}

#[test]
fn an_editor_removal_says_so() {
    let head = head(MonitoringConfigChangeDetails {
        origin: MonitoringConfigOrigin::Editor,
        preset: None,
        mode: MonitoringDashboardMode::Legacy,
        generation: 4,
        revisions: vec![revision(
            "theme",
            "dark",
            2,
            MonitoringConfigChangeAction::Delete,
        )],
    });

    assert_eq!(
        head.description,
        "Monitoring configuration generation 4 (dashboard Legacy): theme dark revision 2 removed."
    );
}

#[test]
fn a_reset_counts_what_it_wrote_and_names_the_preset() {
    let head = head(MonitoringConfigChangeDetails {
        origin: MonitoringConfigOrigin::Preset,
        preset: Some(MonitoringPresetRef {
            id: MonitoringPresetIdString("comelec".to_string()),
            version: 2,
        }),
        mode: MonitoringDashboardMode::Configured,
        generation: 1,
        revisions: vec![
            revision("widget", "a", 1, MonitoringConfigChangeAction::Upsert),
            revision("widget", "b", 4, MonitoringConfigChangeAction::Upsert),
            revision("dashboard", "c", 2, MonitoringConfigChangeAction::Delete),
        ],
    });

    assert_eq!(
        head.description,
        "Monitoring configuration generation 1 (dashboard Configured): reset to preset comelec version 2, 2 documents saved and 1 removed."
    );
}

#[test]
fn a_mode_switch_writes_no_document() {
    let head = head(MonitoringConfigChangeDetails {
        origin: MonitoringConfigOrigin::Editor,
        preset: None,
        mode: MonitoringDashboardMode::Legacy,
        generation: 9,
        revisions: Vec::new(),
    });

    assert_eq!(
        head.description,
        "Monitoring configuration generation 9 (dashboard Legacy): no document changed."
    );
}
