// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::task::TaskResult;
use tracing::instrument;

#[instrument]
#[celery::task]
pub fn add(x: i32, y: i32) -> TaskResult<i32> {
    Ok(x + y)
}
