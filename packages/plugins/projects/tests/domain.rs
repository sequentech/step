// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_plugin_projects::domain::*;

fn task() -> Task {
    Task {
        title: "Readiness".into(),
        kind: TaskKind::Task,
        start: "2028-03-01T09:00:00+08:00".into(),
        end: Some("2028-03-01T10:00:00+08:00".into()),
        status: TaskStatus::NotStarted,
        notes: String::new(),
        order: 0,
    }
}

#[test]
fn validates_project_name_and_iana_timezone() {
    let mut project = Project {
        name: "Election preparation".into(),
        timezone: "Asia/Manila".into(),
        state: ProjectState::Active,
    };
    assert!(project.validate().is_ok());
    project.name = "  ".into();
    assert!(project.validate().is_err());
    project.name = "Plan".into();
    project.timezone = "+08:00".into();
    assert!(project.validate().is_err());
}

#[test]
fn rejects_missing_offsets_and_nonpositive_durations() {
    assert!(task().validate().is_ok());
    for start in [
        "2028-03-01T09:00:00",
        "invalid",
        "2028-02-30T09:00:00+08:00",
    ] {
        let mut value = task();
        value.start = start.into();
        assert!(value.validate().is_err(), "accepted {start}");
    }
    for end in [
        None,
        Some("2028-03-01T09:00:00+08:00".into()),
        Some("2028-03-01T00:30:00Z".into()),
    ] {
        let mut value = task();
        value.end = end;
        assert!(value.validate().is_err());
    }
}

#[test]
fn milestone_is_an_instant_and_text_is_bounded() {
    let mut value = task();
    value.kind = TaskKind::Milestone;
    value.end = None;
    assert!(value.validate().is_ok());
    value.end = Some("2028-03-01T10:00:00+08:00".into());
    assert!(value.validate().is_err());
    value = task();
    value.title = " ".into();
    assert!(value.validate().is_err());
    value = task();
    value.notes = "a".repeat(16_001);
    assert!(value.validate().is_err());
}

#[test]
fn compares_instants_across_offsets() {
    let mut value = task();
    value.end = Some("2028-03-01T02:00:00Z".into());
    assert!(value.validate().is_ok());
    value.end = Some("2028-03-01T01:00:00Z".into());
    assert!(value.validate().is_err());
}

#[test]
fn span_compares_instants_and_treats_milestones_as_instants() {
    assert_eq!(Task::span(&[]).unwrap(), (None, None));
    let mut early = task();
    early.start = "2028-03-01T01:00:00Z".into();
    early.end = Some("2028-03-01T02:00:00Z".into());
    let mut milestone = task();
    milestone.kind = TaskKind::Milestone;
    milestone.start = "2028-03-05T09:00:00+08:00".into();
    milestone.end = None;
    let (start, end) = Task::span(&[task(), early, milestone]).unwrap();
    assert_eq!(start.as_deref(), Some("2028-03-01T09:00:00+08:00"));
    assert_eq!(end.as_deref(), Some("2028-03-05T09:00:00+08:00"));
    let mut broken = task();
    broken.start = "invalid".into();
    assert!(Task::span(&[broken]).is_err());
}
