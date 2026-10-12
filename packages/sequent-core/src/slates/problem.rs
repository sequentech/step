// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What checking a slate configuration reports.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Whether a problem stops the configuration from being used.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The configuration is refused.
    Error,
    /// The configuration is accepted, but probably not what was meant.
    Warning,
}

/// What kind of problem this is. Stable identifiers a front end may match on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    /// A required field is absent or empty.
    MissingField,
    /// A value is not one the platform accepts.
    InvalidValue,
    /// A reference points at a contest or candidate that is not there.
    DanglingReference,
    /// Two entries share an identifier.
    DuplicateId,
    /// A slate lists more candidates than a contest allows.
    ContestArithmetic,
    /// The configuration is not JSON.
    Unreadable,
    /// The configuration has a version this platform cannot read.
    IncompatibleVersion,
}

/// One thing wrong with a slate configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    pub severity: Severity,
    pub code: Code,
    /// Where in the configuration, as a dotted path: `slates[2].name.en`.
    pub path: String,
    /// What is wrong, in one sentence, in English.
    pub message: String,
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
        }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
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
