// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// Represents a timezone for date/time formatting and conversion.
/// Used to specify UTC or a fixed offset in hours for formatting timestamps and reports.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum TimeZone {
    /// UTC timezone.
    #[default]
    UTC,
    /// Fixed offset in hours from UTC (e.g., +1 or -4).
    Offset(i32),
}

/// Represents a date/time format for displaying or parsing timestamps.
/// Used to control the string format for dates in reports, logs, and receipts.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub enum DateFormat {
    /// Day/Month/Year (2-digit) Hour:Minute.
    DdMmYyHhMm,
    /// Day/Month/Year (4-digit) Hour:Minute.
    #[default]
    DdMmYyyyHhMm,
    /// Month/Day/Year (2-digit) Hour:Minute.
    MmDdYyHhMm,
    /// Month/Day/Year (4-digit) Hour:Minute.
    MmDdYyyyHhMm,
    /// Custom format string.
    Custom(String),
}

impl DateFormat {
    #[must_use]
    /// Converts the `DateFormat` enum variant to a corresponding format string.
    pub fn to_format_string(&self) -> String {
        match self {
            DateFormat::DdMmYyHhMm => "%d/%m/%y %H:%M".to_string(),
            DateFormat::DdMmYyyyHhMm => "%d/%m/%Y %H:%M".to_string(),
            DateFormat::MmDdYyHhMm => "%m/%d/%y %H:%M".to_string(),
            DateFormat::MmDdYyyyHhMm => "%m/%d/%Y %H:%M".to_string(),
            DateFormat::Custom(fmt) => fmt.clone(),
        }
    }
}
