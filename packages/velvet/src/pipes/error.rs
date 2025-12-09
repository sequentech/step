// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use uuid::Uuid;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    IDNotFound,
    ElectionConfigNotFound(Uuid),
    ContestConfigNotFound(Uuid),
    AreaConfigNotFound(Uuid),
    FileAccess(std::path::PathBuf, std::io::Error),
    IO(std::io::Error),
    JsonParse(serde_json::Error),
    UnexpectedError(String),
    Anyhow(anyhow::Error),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl From<std::io::Error> for Error {
    fn from(val: std::io::Error) -> Self {
        Self::IO(val)
    }
}

impl From<serde_json::Error> for Error {
    fn from(val: serde_json::Error) -> Self {
        Self::JsonParse(val)
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Anyhow(err)
    }
}

impl std::error::Error for Error {}
