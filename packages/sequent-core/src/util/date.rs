// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current system date in the format "day/month/year".
#[must_use]
pub fn get_current_date() -> String {
    let local: DateTime<Local> = Local::now();
    local.format("%-d/%-m/%Y").to_string()
}

/// Get the timestamp for a given number of seconds later than the current time.
/// # Panics
/// If the addition of seconds results in an overflow.
#[must_use]
pub fn get_seconds_later(seconds: i64) -> DateTime<Utc> {
    let current_time = Utc::now();
    current_time
        .checked_add_signed(Duration::seconds(seconds))
        .expect("Overflow when adding seconds to current time")
}

/// Get the current timestamp in seconds since the UNIX epoch.
/// # Errors
/// If the current system time is before the `UNIX_EPOCH`, which is highly unlikely.
pub fn timestamp() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .with_context(|| "Impossible with respect to UNIX_EPOCH")?
        .as_secs())
}
