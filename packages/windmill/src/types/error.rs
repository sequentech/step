// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery;
use celery::prelude::TaskError;
use handlebars;
use serde_json;
use strand::util::StrandError;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Anyhow(err: anyhow::Error) {
            from()
        }
        String(err: String) {
            from()
            from(err: &str) -> (err.into())
        }
        FileAccess(path: std::path::PathBuf, err: std::io::Error) {
            display("An error occurred while accessing the file at '{}': {}", path.display(), err)
        }
    }
}

impl From<Error> for TaskError {
    fn from(err: Error) -> Self {
        match err {
            Error::Anyhow(err) => TaskError::UnexpectedError(format!("{:?}", err)),
            Error::String(err) => TaskError::UnexpectedError(err),
            Error::FileAccess(path, err) => TaskError::UnexpectedError(format!(
                "An error occurred while accessing the file at '{}': {}",
                path.display(),
                err
            )),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
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

pub type Result<T, E = Error> = std::result::Result<T, E>;
