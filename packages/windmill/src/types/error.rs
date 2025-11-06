// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery;
use celery::prelude::TaskError;
use handlebars;
use keycloak;
use sequent_core::util::integrity_check::HashFileVerifyError;
use serde_json;
use strand::util::StrandError;
quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Anyhow(err: anyhow::Error) {
            from()
        }
        Csv(err: csv::Error) {
            from()
        }
        String(err: String) {
            from()
            from(err: &str) -> (err.into())
        }
        Postgres(err: tokio_postgres::Error) {
            from()
        }
        FileAccess(path: std::path::PathBuf, err: std::io::Error) {
            display("An error occurred while accessing the file at '{}': {}", path.display(), err)
        }
        TryFromIntError(err: std::num::TryFromIntError) {
            from()
        }
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

pub type Result<T, E = Error> = std::result::Result<T, E>;
