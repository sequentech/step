// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use chrono::{DateTime, Local, Utc, Duration};

pub fn get_current_date() -> String {
    let local: DateTime<Local> = Local::now();
    local.format("%-d/%-m/%Y").to_string()
}

pub fn get_seconds_later(seconds: i64) -> DateTime<Utc> {
    let current_time = Utc::now();
    current_time + Duration::seconds(seconds)
}
