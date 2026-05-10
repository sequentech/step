// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Top-level error enum and conversions from external failures.

use celery;
use celery::prelude::TaskError;
use handlebars;
use keycloak;
use sequent_core::util::integrity_check::HashFileVerifyError;
use serde_json;
use strand::util::StrandError;

quick_error! {
    /// Error surface for tasks and helpers, convertible to Celery [`TaskError`].
    #[derive(Debug)]
    pub enum Error {
        /// Failure propagated through an [`anyhow::Error`] context chain.
        Anyhow(err: anyhow::Error) {
            from()
        }
        /// CSV export/import error.
        Csv(err: csv::Error) {
            from()
        }
        /// Free-form message, including stringified errors from external crates.
        String(err: String) {
            from()
            from(err: &str) -> (err.into())
        }
        /// PostgreSQL driver or query execution failure.
        Postgres(err: tokio_postgres::Error) {
            from()
        }
        /// I/O error on a specific filesystem path.
        FileAccess(path: std::path::PathBuf, err: std::io::Error) {
            display("An error occurred while accessing the file at '{}': {}", path.display(), err)
        }
        /// Integer conversion exceeded the target type range.
        TryFromIntError(err: std::num::TryFromIntError) {
            from()
        }
        /// Ballot or artifact hash did not match the expected digest.
        HashFileVerifyError(err: HashFileVerifyError) {
            from()
            display("{}", err.to_string())
        }
    }
}

impl From<Error> for TaskError {
    fn from(err: Error) -> Self {
        match err {
            Error::Anyhow(err) => TaskError::UnexpectedError(format!("{:?}", err)),
            Error::String(err) => TaskError::UnexpectedError(err),
            Error::Csv(err) => TaskError::UnexpectedError(format!("{:?}", err)),
            Error::Postgres(err) => TaskError::UnexpectedError(format!("{:?}", err)),
            Error::FileAccess(path, err) => TaskError::UnexpectedError(format!(
                "An error occurred while accessing the file at '{}': {}",
                path.display(),
                err
            )),
            Error::TryFromIntError(err) => TaskError::UnexpectedError(format!("{err:?}")),
            Error::HashFileVerifyError(err) => TaskError::UnexpectedError(format!("{err:?}")),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::String(format!("{:?}", err))
    }
}

impl From<serde_path_to_error::Error<serde_json::Error>> for Error {
    fn from(err: serde_path_to_error::Error<serde_json::Error>) -> Self {
        Error::String(format!("{:?}", err))
    }
}

impl From<celery::error::CeleryError> for Error {
    fn from(err: celery::error::CeleryError) -> Self {
        Error::String(format!("{:?}", err))
    }
}

impl From<handlebars::RenderError> for Error {
    fn from(err: handlebars::RenderError) -> Self {
        Error::String(format!("{:?}", err))
    }
}

impl From<StrandError> for Error {
    fn from(err: StrandError) -> Self {
        Error::String(format!("{:?}", err))
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::String(format!("{:?}", err))
    }
}

impl From<keycloak::KeycloakError> for Error {
    fn from(err: keycloak::KeycloakError) -> Self {
        Error::String(format!("{:?}", err))
    }
}

impl From<lapin::Error> for Error {
    fn from(err: lapin::Error) -> Self {
        Error::String(format!("{:?}", err))
    }
}

/// Result type specialized with [`Error`] as the default `E`.
pub type Result<T, E = Error> = std::result::Result<T, E>;
