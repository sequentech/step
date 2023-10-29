// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::prelude::{TaskError, TaskResult};

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
    }
}

impl From<Error> for TaskError {
    fn from(err: Error) -> Self {
        match err {
            Error::Anyhow(err) =>
                TaskError::UnexpectedError(format!("{:?}", err)),
            Error::String(err) =>
                TaskError::UnexpectedError(err),
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
