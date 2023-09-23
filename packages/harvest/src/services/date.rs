// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use chrono::NaiveDateTime;
use std::str::FromStr;

// parse something like 2023-08-10T22:05:22.214163+00:00
pub fn parse_iso_8601_timezone(input: &str) -> Result<NaiveDateTime> {
    let date_str = input
        .split("+")
        .next()
        .ok_or(anyhow!("invalid date format {}", input))?;
    Ok(NaiveDateTime::from_str(date_str)?)
}
