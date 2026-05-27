// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Result type alias for counting algorithm operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that can occur during counting algorithm operations.
#[derive(Debug)]
pub enum Error {
    /// Tally results are empty, cannot count votes.
    EmptyTallyResults,
    /// Invalid tally operation with error message.
    InvalidTallyOperation(String),
    /// Candidate was not found in the results.
    CandidateNotFound(String),
    /// Unexpected error during counting with error message.
    Unexpected(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
