// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_plugin_projects::csv_io::{export, preview};
use sequent_plugin_projects::domain::*;
use uuid::Uuid;

const HEADER: &str = "id,title,type,start,end,status,notes\r\n";

#[test]
fn previews_every_row_and_reports_line_errors() {
    let csv = format!("{HEADER},Readiness,task,2028-03-01T09:00:00+08:00,2028-03-01T10:00:00+08:00,not_started,ok\r\n,Bad,task,2028-03-01T09:00:00,,done,\r\n,Milestone,milestone,2028-03-02T12:00:00+08:00,,done,\r\n");
    let result = preview(&csv).unwrap();
    assert_eq!(result.rows.len(), 3);
    assert!(result.rows[0].errors.is_empty());
    assert_eq!(result.rows[1].line, 3);
    assert!(!result.rows[1].errors.is_empty());
    assert!(result.rows[2].errors.is_empty());
    assert!(!result.valid);
}

#[test]
fn rejects_duplicate_ids_unknown_columns_and_oversized_imports() {
    let id = "00000000-0000-4000-8000-000000000001";
    let line = format!("{id},Phase,milestone,2028-03-01T09:00:00+08:00,,done,\n");
    assert!(!preview(&format!("{HEADER}{line}{line}")).unwrap().valid);
    assert!(preview("title,tenant_id\nPlan,other\n").is_err());
    assert!(preview(&"x".repeat(1_048_577)).is_err());
    assert!(!preview(HEADER).unwrap().valid);
}

#[test]
fn reads_rfc4180_quotes_bom_and_multiline_notes() {
    let csv = format!("\u{feff}{HEADER},\"A, B\",milestone,2028-03-01T09:00:00+08:00,,done,\"first\r\nsecond \"\"quoted\"\"\"\r\n");
    let result = preview(&csv).unwrap();
    assert!(result.valid);
    let task = result.rows[0].task.as_ref().unwrap();
    assert_eq!(task.title, "A, B");
    assert_eq!(task.notes, "first\r\nsecond \"quoted\"");
}

#[test]
fn export_has_known_columns_and_safe_spreadsheet_cells() {
    let task = Task {
        title: "=1+1".into(),
        kind: TaskKind::Milestone,
        start: "2028-03-01T01:00:00Z".into(),
        end: None,
        status: TaskStatus::Done,
        notes: "'literal".into(),
        order: 0,
    };
    let text = export(&[(Uuid::nil(), task.clone())], "Asia/Manila").unwrap();
    assert!(text.starts_with(HEADER));
    assert!(text.contains(",'=1+1,milestone,2028-03-01T09:00:00+08:00,,done,''literal\r\n"));
    let imported = preview(&text).unwrap();
    assert!(imported.valid);
    assert_eq!(imported.rows[0].task.as_ref().unwrap().title, task.title);
    assert_eq!(imported.rows[0].task.as_ref().unwrap().notes, task.notes);
}

#[test]
fn export_file_names_are_portable() {
    use sequent_plugin_projects::csv_io::filename;
    assert_eq!(
        filename("2028 Elections: readiness plan"),
        "2028-elections-readiness-plan.csv"
    );
    assert_eq!(filename("../../etc/passwd"), "etc-passwd.csv");
    assert_eq!(filename("Planificación"), "planificaci-n.csv");
    assert_eq!(filename("***"), "project.csv");
}

#[test]
fn rejects_unknown_types_statuses_and_short_rows() {
    let csv = format!("{HEADER},A,phase,2028-03-01T09:00:00+08:00,,done,\r\n,B,milestone,2028-03-01T09:00:00+08:00,,finished,\r\n,C,milestone\r\n");
    let result = preview(&csv).unwrap();
    assert!(!result.valid);
    assert!(result.rows.iter().all(|row| !row.errors.is_empty()));
    assert_eq!(result.rows[2].line, 4);
}
