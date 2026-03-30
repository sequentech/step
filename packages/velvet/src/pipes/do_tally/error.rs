// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::error::Error as StdError;

/// Result type for tally operations.
pub type Result<T, E = Box<dyn StdError>> = std::result::Result<T, E>;

/// Errors that can occur during tally operations.
#[derive(Debug)]
pub enum Error {
    /// The tally type was not found for the contest.
    TallyTypeNotFound,
    /// The tally type is not implemented.
    TallyTypeNotImplemented(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
