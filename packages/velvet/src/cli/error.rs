// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::pipes;
use pipes::error::Error as PipesError;

/// Result type alias for CLI operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error types for CLI operations.
#[derive(Debug)]
pub enum Error {
    /// Configuration file not found.
    ConfigNotFound,
    /// Cannot open configuration file.
    CannotOpenConfig,
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),
    /// Invalid stage definition.
    StageDefinition(String),
    /// Pipeline not found.
    PipeNotFound,
    /// Error from pipeline operation.
    FromPipe(PipesError),
}

impl core::fmt::Display for Error {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl From<PipesError> for Error {
    fn from(val: PipesError) -> Self {
        Self::FromPipe(val)
    }
}

impl From<serde_json::Error> for Error {
    fn from(val: serde_json::Error) -> Self {
        Self::Json(val)
    }
}

impl std::error::Error for Error {}
