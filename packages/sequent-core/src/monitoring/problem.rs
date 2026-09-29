// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What validation reports.
//!
//! Structured for the same reason as
//! [`crate::election_config::problem`]: the Admin Portal lists problems beside
//! the line they are about, Harvest returns them from a refused save, and the
//! preset tests print them. Each needs the pieces, not a sentence.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Whether a problem refuses the configuration or only deserves saying.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The configuration is refused; the previous revision stays live.
    Error,
    /// The configuration is accepted, but probably not what was meant.
    Warning,
}

/// What kind of problem this is.
///
/// Stable identifiers: the Admin Portal matches on these, so renaming one is a
/// breaking change in a way that rewording a message is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    /// The text is not YAML, or not the shape this kind of configuration has.
    Unreadable,
    /// The document is larger or deeper than configuration is allowed to be.
    TooLarge,
    /// A key that would reach outside the governed data: SQL, a URL, a file, a
    /// link, markup, or data typed in by hand.
    ForbiddenKey,
    /// A string that would be interpreted rather than shown: a template, markup
    /// or a URL.
    ForbiddenValue,
    /// An identifier is not a lowercase slug.
    InvalidId,
    /// A value is not one this field accepts: a malformed date or time zone,
    /// a missing default, two origins where one is allowed.
    InvalidValue,
    /// Two entities share an identifier.
    DuplicateId,
    /// A reference points at something that does not exist.
    DanglingReference,
    /// A selector value is not one of the options it lists.
    UnknownOption,
    /// A reference to a selector the widget does not declare.
    UnknownSelector,
    /// The data source has no such template, measure, dimension or grain.
    UnsupportedBySource,
    /// A query parameter is missing, or given where its template takes none.
    TemplateParameter,
    /// A layout width outside the 12-column grid.
    LayoutWidth,
    /// A dashboard selector that narrows none of the dashboard's widgets.
    UnusedSelector,
    /// The snapshot has no count for what a query asks: the producer did not
    /// count that measure or dimension. Refused rather than shown as zero.
    NotCounted,
    /// The snapshot is not in the shape its producer promises: a cube cell
    /// with the wrong number of values, an unreadable offset. Refused rather
    /// than guessed at.
    MalformedSnapshot,
    /// dbt Charts refused the chart, or warned about it. The engine's own code
    /// travels in [`Problem::engine_code`].
    ChartSchema,
    /// A kind of document only a reset to a preset writes.
    PresetOnly,
    /// The Dashboard tab would show the configured dashboards, and the event
    /// has none.
    NoDashboard,
}

/// One thing wrong with a configuration document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub severity: Severity,
    pub code: Code,

    /// Where in the document, as a dotted path — `chart.charts.bars.query`.
    /// Empty when the complaint is about the whole document.
    pub path: String,

    /// What is wrong, in one sentence, in English.
    pub message: String,

    /// The dbt Charts diagnostic code (`ERR-…`, `WARN-…`) behind a
    /// [`Code::ChartSchema`] problem, so the editor can link its documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_code: Option<String>,
}

impl Problem {
    pub fn error(
        code: Code,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Problem {
            severity: Severity::Error,
            code,
            path: path.into(),
            message: message.into(),
            engine_code: None,
        }
    }

    pub fn warning(
        code: Code,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Problem {
            severity: Severity::Warning,
            ..Problem::error(code, path, message)
        }
    }

    pub fn with_engine_code(mut self, engine_code: impl Into<String>) -> Self {
        self.engine_code = Some(engine_code.into());
        self
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        if self.path.is_empty() {
            write!(formatter, "{label}: {}", self.message)
        } else {
            write!(formatter, "{label}: {}: {}", self.path, self.message)
        }
    }
}

/// Everything validation found, in the order it found it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub problems: Vec<Problem>,
}

impl Report {
    /// Adds a problem, unless the report already says it: a check that runs
    /// once per row finds the same problem in every row.
    pub fn push(&mut self, problem: Problem) {
        if !self.problems.contains(&problem) {
            self.problems.push(problem);
        }
    }

    pub fn extend(&mut self, other: Report) {
        for problem in other.problems {
            self.push(problem);
        }
    }

    /// Whether the configuration may be saved: warnings do not stop it.
    pub fn is_accepted(&self) -> bool {
        self.problems
            .iter()
            .all(|problem| problem.severity != Severity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &Problem> {
        self.problems
            .iter()
            .filter(|problem| problem.severity == Severity::Error)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for problem in &self.problems {
            writeln!(formatter, "{problem}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "problem_tests.rs"]
mod problem_tests;
