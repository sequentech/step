// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::date_time::{DateFormat, TimeZone};
use chrono::{
    DateTime, Duration, FixedOffset, Local, TimeZone as ChronoTimeZone, Utc,
};

pub const PHILIPPINO_TIMEZONE: TimeZone = TimeZone::Offset(8);

pub fn get_system_timezone() -> TimeZone {
    let now = Local::now();
    let offset = now.offset();
    let duration = Duration::seconds(offset.local_minus_utc() as i64);
    let hours = duration.num_hours() as i32;
    if hours == 0 {
        TimeZone::UTC
    } else {
        TimeZone::Offset(hours)
    }
}

pub fn get_date_and_time() -> String {
    let current_date_time = Local::now();
    let printed_datetime = current_date_time.to_rfc3339();
    printed_datetime
}

pub fn generate_timestamp(
    time_zone: Option<TimeZone>,
    date_format: Option<DateFormat>,
    date_time: Option<DateTime<Utc>>,
) -> String {
    let time_zone = time_zone.unwrap_or_default();
    let date_format = date_format.unwrap_or_default().to_format_string();

    let now = date_time.unwrap_or(Utc::now());

    match time_zone {
        TimeZone::UTC => now.format(&date_format).to_string(),
        TimeZone::Offset(offset) => {
            let duration = Duration::hours(offset as i64);
            let fixed_offset =
                FixedOffset::east_opt(duration.num_seconds() as i32);
            match fixed_offset {
                Some(fixed) => fixed
                    .from_utc_datetime(&now.naive_utc())
                    .format(&date_format)
                    .to_string(),
                None => now.format(&date_format).to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_timestamp_default() {
        let timestamp = generate_timestamp(None, None, None);
        println!("Default timestamp: {}", timestamp);
    }

    #[test]
    fn test_generate_timestamp_custom_offset() {
        let timestamp = generate_timestamp(
            Some(TimeZone::Offset(2)),
            Some(DateFormat::MmDdYyyyHhMm),
            None,
        );
        println!("Custom timestamp: {}", timestamp);
    }
}
