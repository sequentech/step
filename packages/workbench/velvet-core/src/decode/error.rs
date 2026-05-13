// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Errors produced by the ballot decoder.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// Ballot line could not be parsed as a `BigUint`, or the resulting
    /// plaintext could not be decoded against the contest definition.
    InvalidBallot(String),
    /// Underlying reader I/O error (only produced by the reader-based
    /// helpers, not by `decode_ballot_line`).
    Io(std::io::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidBallot(s) => write!(f, "invalid ballot: {s}"),
            Error::Io(e) => write!(f, "decode I/O error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidBallot(_) => None,
            Error::Io(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
