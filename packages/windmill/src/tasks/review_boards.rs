// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::task::TaskResult;
use tracing::instrument;
use tracing::{event, Level};
use chrono::{Utc, DateTime};

#[instrument]
#[celery::task]
pub fn review_boards() -> TaskResult<()> {
    let current_time = Utc::now();

    event!(Level::INFO, "This is too late");

    Ok(())
}
