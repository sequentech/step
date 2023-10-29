// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use celery::error::TaskError;

pub fn into_task_error<T: std::fmt::Debug>(err: T) -> TaskError {
    TaskError::UnexpectedError(format!("{:?}", err))
}
