// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What validation reports.
//!
//! Structured rather than a formatted string, because the same problem has to be
//! rendered three ways: a line in `step-cli`'s output, a row in a browser's
//! problem list, and an error from a server-side import. Each wants the pieces
//! arranged differently, and only the caller knows which.
//!
//! Every problem carries a machine-readable [`Code`] so a front end can group,
//! translate or link them without matching on English text.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Whether a problem stops an import or merely deserves saying out loud.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The bundle will not import, or will import into something broken.
    Error,
    /// The bundle imports, but something about it is very likely a mistake.
    ///
    /// A warning is not a lesser error: it is a statement that this file is
    /// self-consistent but probably not what its author meant. An election with
    /// no voting window imports perfectly and then never opens.
    Warning,
}

/// What kind of problem this is.
///
/// Stable identifiers: a front end may match on these, so renaming one is a
/// breaking change in a way that rewording a message is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    /// A required field is absent or empty.
    MissingField,
    /// A value is not one the platform accepts.
    InvalidValue,
    /// A reference points at something not in the bundle.
    DanglingReference,
    /// Two entities share an identifier.
    DuplicateId,
    /// The area parent chain loops.
    AreaCycle,
    /// A contest's vote counts contradict each other or its candidate list.
    ContestArithmetic,
    /// `voting_type` and `counting_algorithm` disagree.
    TallyMismatch,
    /// Something that would be on a ballot is not, or vice versa.
    BallotCoverage,
    /// An entity is scoped to a permission label, which hides it.
    PermissionLabel,
    /// An election has no scheduled voting window.
    MissingSchedule,
}

/// One thing wrong with a bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub severity: Severity,
    pub code: Code,

    /// Where in the bundle, as a dotted path — `contests[2].max_votes`.
    ///
    /// Indexed rather than named because a bundle is what is being validated;
    /// the tool that produced it maps this back to a wizard step or a
    /// spreadsheet cell, which only it can do.
    pub path: String,

    /// What is wrong, in one sentence, in English.
    pub message: String,

    /// The entity's `external_id` where it has one.
    ///
    /// The bundle's UUIDs are regenerated on import and mean nothing to whoever
    /// has to fix the source, whereas an `external_id` is what they typed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
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
            external_id: None,
        }
    }

    pub fn warning(
        code: Code,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Problem {
            severity: Severity::Warning,
            code,
            path: path.into(),
            message: message.into(),
            external_id: None,
        }
    }

    pub fn about(mut self, external_id: Option<&str>) -> Self {
        self.external_id = external_id.map(str::to_string);
        self
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(formatter, "{label}: {}: {}", self.path, self.message)
    }
}

/// Everything validation found, in the order it found it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub problems: Vec<Problem>,
}

impl Report {
    pub fn push(&mut self, problem: Problem) {
        self.problems.push(problem);
    }

    /// Whether the bundle should be refused.
    pub fn has_errors(&self) -> bool {
        self.problems
            .iter()
            .any(|problem| problem.severity == Severity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &Problem> {
        self.iter_severity(Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Problem> {
        self.iter_severity(Severity::Warning)
    }

    fn iter_severity(
        &self,
        severity: Severity,
    ) -> impl Iterator<Item = &Problem> {
        self.problems
            .iter()
            .filter(move |problem| problem.severity == severity)
    }

    pub fn is_empty(&self) -> bool {
        self.problems.is_empty()
    }
}

impl fmt::Display for Report {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for problem in &self.problems {
            writeln!(formatter, "  {problem}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_with_only_warnings_is_importable() {
        // The distinction is the point: a warning must not block a build.
        let mut report = Report::default();
        report.push(Problem::warning(
            Code::MissingSchedule,
            "elections[0]",
            "no window",
        ));
        assert!(!report.has_errors());
        assert_eq!(report.warnings().count(), 1);
        assert_eq!(report.errors().count(), 0);
    }

    #[test]
    fn one_error_condemns_the_report() {
        let mut report = Report::default();
        report.push(Problem::warning(Code::MissingSchedule, "a", "b"));
        report.push(Problem::error(Code::MissingField, "c", "d"));
        assert!(report.has_errors());
    }

    #[test]
    fn a_problem_reads_as_a_line() {
        let problem = Problem::error(
            Code::DanglingReference,
            "candidates[3].contest_id",
            "no contest with that id is in the bundle",
        );
        assert_eq!(
            problem.to_string(),
            "error: candidates[3].contest_id: no contest with that id is in the bundle"
        );
    }

    #[test]
    fn the_code_survives_serialization() {
        // A front end matches on this rather than on the English.
        let problem = Problem::error(Code::TallyMismatch, "p", "m")
            .about(Some("president"));
        let json = serde_json::to_value(&problem).unwrap();
        assert_eq!(json["code"], "tally_mismatch");
        assert_eq!(json["severity"], "error");
        assert_eq!(json["external_id"], "president");
    }

    #[test]
    fn an_absent_external_id_is_omitted_rather_than_null() {
        let problem = Problem::error(Code::MissingField, "p", "m");
        let json = serde_json::to_value(&problem).unwrap();
        assert!(json.get("external_id").is_none());
    }
}
