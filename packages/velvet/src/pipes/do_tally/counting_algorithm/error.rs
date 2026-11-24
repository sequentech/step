// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes::do_tally::CountOverflow;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    EmptyTallyResults,
    InvalidTallyOperation(String),
    CandidateNotFound(String),
    UnexpectedError(String),
    CountOverflow(CountOverflow),
}

impl From<CountOverflow> for Error {
    fn from(error: CountOverflow) -> Self {
        Error::CountOverflow(error)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for Error {}
