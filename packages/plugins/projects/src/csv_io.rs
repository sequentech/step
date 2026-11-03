// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded RFC 4180 import preview and spreadsheet-safe export.
use crate::domain::{instant, Task, TaskKind, TaskStatus, MAX_TASKS};
use sequent_plugin_api::{PluginError, MAX_REQUEST_BYTES};
use serde::Serialize;
use std::collections::HashSet;
use uuid::Uuid;

const HEADERS: [&str; 7] = ["id", "title", "type", "start", "end", "status", "notes"];

#[derive(Debug, Serialize)]
pub struct ImportRow {
    pub line: u64,
    pub task: Option<Task>,
    pub errors: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct ImportPreview {
    pub rows: Vec<ImportRow>,
    pub valid: bool,
}
fn invalid(message: impl ToString) -> PluginError {
    PluginError::InvalidInput(message.to_string())
}

fn formula_prefix(value: &str) -> bool {
    value.starts_with(['\'', '=', '+', '-', '@', '\t', '\r', '\n'])
}

fn spreadsheet_text(value: &str) -> String {
    if formula_prefix(value) {
        format!("'{value}")
    } else {
        value.to_owned()
    }
}

fn restore_text(value: &str) -> String {
    match value.strip_prefix('\'') {
        Some(rest) if formula_prefix(rest) => rest.to_owned(),
        _ => value.to_owned(),
    }
}

/// Preview only: the application must run this again on commit and must reject
/// the complete import if even one row fails. IDs are source identifiers, never
/// a request to overwrite an existing record.
pub fn preview(text: &str) -> Result<ImportPreview, PluginError> {
    if text.len() > MAX_REQUEST_BYTES {
        return Err(invalid("CSV must be at most 1 MiB"));
    }
    let text = text.trim_start_matches('\u{feff}');
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    if reader.headers().map_err(invalid)?.iter().ne(HEADERS) {
        return Err(invalid(
            "Expected columns: id,title,type,start,end,status,notes",
        ));
    }
    let mut rows = Vec::new();
    let mut ids = HashSet::new();
    for row in reader.records() {
        if rows.len() >= MAX_TASKS {
            return Err(invalid("A project can contain at most 1000 tasks"));
        }
        let record = row.map_err(invalid)?;
        // The csv parser can leave a CRLF record's start position at the LF
        // following the previous CR. Count the skipped terminator too.
        let line = record
            .position()
            .map(|position| {
                let skipped_lines = text
                    .as_bytes()
                    .iter()
                    .skip(usize::try_from(position.byte()).unwrap_or(text.len()))
                    .take_while(|byte| matches!(byte, b'\r' | b'\n'))
                    .filter(|byte| **byte == b'\n')
                    .count();
                position.line() + u64::try_from(skipped_lines).unwrap_or(0)
            })
            .unwrap_or(0);
        let mut errors = Vec::new();
        if record.len() != HEADERS.len() {
            rows.push(ImportRow {
                line,
                task: None,
                errors: vec!["Expected seven columns".into()],
            });
            continue;
        }
        let cell = |index| record.get(index).unwrap_or_default();
        if !cell(0).is_empty() {
            match Uuid::parse_str(cell(0)) {
                Ok(id) if ids.insert(id) => {}
                Ok(_) => errors.push("Duplicate source task id".into()),
                Err(_) => errors.push("id must be blank or a UUID".into()),
            }
        }
        let kind = match cell(2) {
            "task" => Some(TaskKind::Task),
            "milestone" => Some(TaskKind::Milestone),
            _ => {
                errors.push("type must be task or milestone".into());
                None
            }
        };
        let status = match cell(5) {
            "not_started" => Some(TaskStatus::NotStarted),
            "in_progress" => Some(TaskStatus::InProgress),
            "done" => Some(TaskStatus::Done),
            _ => {
                errors.push("status must be not_started, in_progress or done".into());
                None
            }
        };
        let task = if let (Some(kind), Some(status)) = (kind, status) {
            let task = Task {
                title: restore_text(cell(1)),
                kind,
                start: cell(3).into(),
                end: (!cell(4).is_empty()).then(|| cell(4).to_owned()),
                status,
                notes: restore_text(cell(6)),
                order: i32::try_from(rows.len()).map_err(invalid)?,
            };
            if let Err(error) = task.validate() {
                errors.push(error.to_string());
            }
            Some(task)
        } else {
            None
        };
        rows.push(ImportRow { line, task, errors });
    }
    let valid = !rows.is_empty() && rows.iter().all(|row| row.errors.is_empty());
    Ok(ImportPreview { rows, valid })
}

/// A portable file name: lowercase ASCII letters and digits joined by dashes.
pub fn filename(project_name: &str) -> String {
    let words: Vec<String> = project_name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if words.is_empty() {
        "project.csv".into()
    } else {
        format!("{}.csv", words.join("-"))
    }
}

/// Dates are exported with the project's offset at each instant (including
/// daylight-saving changes). Text cells use a reversible apostrophe escape to
/// avoid evaluating spreadsheet formulas when opened outside the application.
pub fn export(tasks: &[(Uuid, Task)], timezone: &str) -> Result<String, PluginError> {
    let zone: chrono_tz::Tz = timezone.parse().map_err(invalid)?;
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    writer.write_record(HEADERS).map_err(invalid)?;
    for (id, task) in tasks {
        task.validate()?;
        let start = instant(&task.start)?.with_timezone(&zone).to_rfc3339();
        let end = task
            .end
            .as_deref()
            .map(instant)
            .transpose()?
            .map(|date| date.with_timezone(&zone).to_rfc3339())
            .unwrap_or_default();
        let kind = match task.kind {
            TaskKind::Task => "task",
            TaskKind::Milestone => "milestone",
        };
        let status = match task.status {
            TaskStatus::NotStarted => "not_started",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
        };
        writer
            .write_record([
                id.to_string(),
                spreadsheet_text(&task.title),
                kind.into(),
                start,
                end,
                status.into(),
                spreadsheet_text(&task.notes),
            ])
            .map_err(invalid)?;
    }
    String::from_utf8(writer.into_inner().map_err(invalid)?).map_err(invalid)
}
