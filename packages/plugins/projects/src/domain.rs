// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Project timezone and task invariants.
use chrono::{DateTime, Datelike, FixedOffset};
use sequent_plugin_api::PluginError;
use serde::{Deserialize, Serialize};

pub const MAX_TASKS: usize = 1000;

fn invalid(message: &str) -> PluginError {
    PluginError::InvalidInput(message.into())
}

/// RFC 3339 always includes an offset, so a viewer cannot reinterpret a date.
pub fn instant(value: &str) -> Result<DateTime<FixedOffset>, PluginError> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .map_err(|_| invalid("Use an ISO 8601 date and time with a UTC offset"))?;
    if !(1900..=2200).contains(&parsed.year()) {
        return Err(invalid("Dates must be between 1900 and 2200"));
    }
    Ok(parsed)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Task,
    Milestone,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Done,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectState {
    Active,
    Archived,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub timezone: String,
    pub state: ProjectState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub title: String,
    #[serde(rename = "type")]
    pub kind: TaskKind,
    pub start: String,
    pub end: Option<String>,
    pub status: TaskStatus,
    pub notes: String,
    pub order: i32,
}

impl Project {
    pub fn validate(&self) -> Result<(), PluginError> {
        if self.name.trim().is_empty() || self.name.chars().count() > 200 {
            return Err(invalid(
                "Project name is required and must be at most 200 characters",
            ));
        }
        self.timezone
            .parse::<chrono_tz::Tz>()
            .map_err(|_| invalid("Choose an IANA project timezone"))?;
        Ok(())
    }
}
impl Task {
    /// The earliest start and latest end (a milestone's end is its start),
    /// as stored, or `None` for a project without tasks.
    pub fn span(tasks: &[Task]) -> Result<(Option<String>, Option<String>), PluginError> {
        let mut first: Option<(DateTime<FixedOffset>, &str)> = None;
        let mut last: Option<(DateTime<FixedOffset>, &str)> = None;
        for task in tasks {
            let start = instant(&task.start)?;
            let end_text = task.end.as_deref().unwrap_or(&task.start);
            let end = instant(end_text)?;
            if first.is_none_or(|(at, _)| start < at) {
                first = Some((start, &task.start));
            }
            if last.is_none_or(|(at, _)| end > at) {
                last = Some((end, end_text));
            }
        }
        Ok((
            first.map(|(_, text)| text.to_owned()),
            last.map(|(_, text)| text.to_owned()),
        ))
    }

    pub fn validate(&self) -> Result<(), PluginError> {
        if self.title.trim().is_empty() || self.title.chars().count() > 200 {
            return Err(invalid(
                "Task title is required and must be at most 200 characters",
            ));
        }
        if self.notes.chars().count() > 16_000 {
            return Err(invalid("Notes must be at most 16000 characters"));
        }
        if !(0..=1_000_000).contains(&self.order) {
            return Err(invalid("Task order must be between 0 and 1000000"));
        }
        let start = instant(&self.start)?;
        match (&self.kind, &self.end) {
            (TaskKind::Milestone, None) => Ok(()),
            (TaskKind::Task, Some(end)) if instant(end)? > start => Ok(()),
            (TaskKind::Milestone, _) => {
                Err(invalid("A milestone has a start time and no end time"))
            }
            _ => Err(invalid("Task end must be after its start")),
        }
    }
}
