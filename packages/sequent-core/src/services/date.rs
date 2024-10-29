// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, LocalResult, TimeZone, Utc};
use time::OffsetDateTime;

// format: 2023-08-10T22:05:22.214163+00:00
pub struct ISO8601;

impl ISO8601 {
    pub fn to_date_utc(date_string: &str) -> Result<DateTime<Utc>> {
        let date_time_utc = DateTime::parse_from_rfc3339(date_string)
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(date_time_utc.with_timezone(&Utc))
    }

    // parse something like 2023-08-10T22:05:22.214163+00:00
    pub fn to_date(date_string: &str) -> Result<DateTime<Local>> {
        let date_time_utc = DateTime::parse_from_rfc3339(date_string)
            .map_err(|err| anyhow!("{:?}", err))?;
        Ok(date_time_utc.with_timezone(&Local))
    }

    pub fn to_string(date: &DateTime<Local>) -> String {
        date.to_rfc3339()
    }

    pub fn now() -> DateTime<Local> {
        Local::now()
    }

    pub fn timestamp_ms_utc_to_date(millis: i64) -> DateTime<Local> {
        // Convert Unix timestamp in milliseconds to DateTime<Utc>
        let date_time_utc = Utc.timestamp_millis_opt(millis).unwrap();

        // Convert Utc DateTime to Local DateTime
        date_time_utc.with_timezone(&Local)
    }

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

    pub fn timestamp_secs_utc_to_date_opt(
        secs: i64,
    ) -> Result<DateTime<Local>> {
        Self::timestamp_ms_utc_to_date_opt(secs * 1000)
    }
}

// get the unix timestamp in milliseconds
pub fn get_now_utc_unix_ms() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp() * 1000
}
