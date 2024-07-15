// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use chrono::{Utc, FixedOffset, TimeZone as ChronoTimeZone};

use crate::types::date_time::{DateFormat, TimeZone};

pub fn generate_timestamp(
    time_zone: Option<TimeZone>,
    date_format: Option<DateFormat>,
) -> String {
    let time_zone = time_zone.unwrap_or_default();
    let date_format = date_format.unwrap_or_default().to_format_string();

    let now = Utc::now();
    
    match time_zone {
        TimeZone::UTC => now.format(&date_format).to_string(),
        TimeZone::Offset(offset) => {
            let fixed_offset = FixedOffset::east_opt(offset * 3600);
            match fixed_offset{
                Some(fixed) => fixed.from_utc_datetime(&now.naive_utc()).format(&date_format).to_string(),
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
        let timestamp = generate_timestamp(None, None);
        println!("Default timestamp: {}", timestamp);
    }

    #[test]
    fn test_generate_timestamp_custom_offset() {
        let timestamp = generate_timestamp(Some(TimeZone::Offset(2)), Some(DateFormat::MmDdYyyyHhMm));
        println!("Custom timestamp: {}", timestamp);
    }
}