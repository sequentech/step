// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use celery::task::TaskResult;
use chrono::Utc;
use tracing::instrument;
use tracing::{event, Level};

#[instrument]
#[celery::task]
pub fn review_boards() -> TaskResult<()> {
    let _current_time = Utc::now();

    event!(Level::INFO, "This is too late");

    Ok(())
}
