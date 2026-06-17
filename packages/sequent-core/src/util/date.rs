// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

/// Returns today's date formatted as `d/m/YYYY` in the local timezone.
pub fn get_current_date() -> String {
    let local: DateTime<Local> = Local::now();
    local.format("%-d/%-m/%Y").to_string()
}

/// Returns a UTC timestamp `seconds` in the future from now.
pub fn get_seconds_later(seconds: i64) -> DateTime<Utc> {
    let current_time = Utc::now();
    current_time + Duration::seconds(seconds)
}

/// Returns the current Unix timestamp in seconds.
pub fn timestamp() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .with_context(|| "Impossible with respect to UNIX_EPOCH")?
        .as_secs())
}
