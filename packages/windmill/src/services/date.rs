// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use std::str::FromStr;
use time::OffsetDateTime;

// format: 2023-08-10T22:05:22.214163+00:00
pub struct ISO8601;

impl ISO8601 {
    // parse something like 2023-08-10T22:05:22.214163+00:00
    pub fn to_date(input: &str) -> Result<NaiveDateTime> {
        let date_str = input
            .split("+")
            .next()
            .ok_or(anyhow!("invalid date format {}", input))?;
        Ok(NaiveDateTime::from_str(date_str)?)
    }

    pub fn from_date(date: &NaiveDateTime) -> String {
        date.format("%Y-%m-%dT%H:%M:%S%.f+00:00").to_string()
    }
}

pub fn get_now_utc_unix() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
