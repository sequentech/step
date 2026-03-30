// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use uuid::Uuid;

/// Result type alias for pipe operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors that can occur during pipe operations.
#[derive(Debug)]
pub enum Error {
    /// An element ID was not found.
    IDNotFound,
    /// Election configuration could not be found for the given election ID.
    ElectionConfigNotFound(Uuid),
    /// Contest configuration could not be found for the given contest ID.
    ContestConfigNotFound(Uuid),
    /// Area configuration could not be found for the given area ID.
    AreaConfigNotFound(Uuid),
    /// File system access error with path and underlying I/O error.
    FileAccess(std::path::PathBuf, std::io::Error),
    /// Generic I/O error.
    IO(std::io::Error),
    /// JSON deserialization or parsing error.
    JsonParse(serde_json::Error),
    /// Unexpected error with a custom message.
    UnexpectedError(String),
    /// Wrapper for anyhow errors.
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
