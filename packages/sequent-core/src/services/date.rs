// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, LocalResult, TimeZone, Utc};
use time::OffsetDateTime;

/// RFC 3339 date-time parsing and conversion helpers. format: "2023-08-10T22:05:22.214163+00:00"
pub struct ISO8601;

impl ISO8601 {
    /// Parses an RFC 3339 string into a UTC [`DateTime`].
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not valid RFC 3339.
    pub fn to_date_utc(date_string: &str) -> Result<DateTime<Utc>> {
        let date_time_utc = DateTime::parse_from_rfc3339(date_string)
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(date_time_utc.with_timezone(&Utc))
    }

    /// Parses an RFC 3339 string into the local timezone.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not valid RFC 3339.
    pub fn to_date(date_string: &str) -> Result<DateTime<Local>> {
        let date_time_utc = DateTime::parse_from_rfc3339(date_string)
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(date_time_utc.with_timezone(&Local))
    }

    /// Formats a local datetime as an RFC 3339 string.
    pub fn to_string(date: &DateTime<Local>) -> String {
        date.to_rfc3339()
    }

    /// Returns the current local datetime.
    pub fn now() -> DateTime<Local> {
        Local::now()
    }

    /// Converts a UTC Unix timestamp in milliseconds to local time.
    pub fn timestamp_ms_utc_to_date(millis: i64) -> DateTime<Local> {
        // Convert Unix timestamp in milliseconds to DateTime<Utc>
        let date_time_utc = Utc.timestamp_millis_opt(millis).unwrap();

        // Convert Utc DateTime to Local DateTime
        date_time_utc.with_timezone(&Local)
    }

    /// Converts a UTC Unix timestamp in milliseconds to local time, or returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error when the timestamp is ambiguous or out of range.
    pub fn timestamp_ms_utc_to_date_opt(
        millis: i64,
    ) -> Result<DateTime<Local>> {
        // Convert Unix timestamp in milliseconds to DateTime<Utc>
        let date_time_utc = match Utc.timestamp_millis_opt(millis) {
            LocalResult::Single(data) => data,
            _ => {
                return Err(anyhow!("error parsing timestamp"));
            }
        };

        // Convert Utc DateTime to Local DateTime
        Ok(date_time_utc.with_timezone(&Local))
    }

    /// Converts a UTC Unix timestamp in seconds to local time, or returns an error.
    ///
    /// # Errors
    ///
    /// Returns an error when the timestamp is ambiguous or out of range.
    pub fn timestamp_secs_utc_to_date_opt(
        secs: i64,
    ) -> Result<DateTime<Local>> {
        Self::timestamp_ms_utc_to_date_opt(secs * 1000)
    }
}

/// Returns the current UTC time as a Unix timestamp in milliseconds.
pub fn get_now_utc_unix_ms() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp() * 1000
}
