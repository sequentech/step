// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::error::Error as StdError;

pub type Result<T, E = Box<dyn StdError>> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    TallyTypeNotFound,
    TallyTypeNotImplemented(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        match self {
            Self::TallyTypeNotFound => write!(fmt, "TallyTypeNotFound"),
            Self::TallyTypeNotImplemented(kind) => write!(fmt, "TallyTypeNotImplemented({kind:?})"),
        }
    }
}

impl std::error::Error for Error {}
