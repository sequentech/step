// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

/// Time zone used when formatting dates in reports and exports.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TimeZone {
    /// Coordinated Universal Time.
    UTC,
    /// Fixed offset from UTC in whole hours (e.g. `+1`, `-4`).
    Offset(i32), // Offset in hours, e.g., +1 or -4
}

/// Date/time display format for PDF reports and exported documents.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DateFormat {
    /// `DD/MM/YY HH:MM`
    DdMmYyHhMm,
    /// `DD/MM/YYYY HH:MM` (default).
    DdMmYyyyHhMm,
    /// `MM/DD/YY HH:MM`
    MmDdYyHhMm,
    /// `MM/DD/YYYY HH:MM`
    MmDdYyyyHhMm,
    /// Custom strftime pattern.
    Custom(String),
    /// Alias for [`DateFormat::DdMmYyyyHhMm`].
    Default,
}

impl Default for TimeZone {
    fn default() -> Self {
        TimeZone::UTC
    }
}

impl Default for DateFormat {
    fn default() -> Self {
        DateFormat::DdMmYyyyHhMm
    }
}

impl DateFormat {
    /// Returns the chrono `strftime` pattern string for this format.
    pub fn to_format_string(&self) -> String {
        match self {
            DateFormat::DdMmYyHhMm => "%d/%m/%y %H:%M".to_string(),
            DateFormat::DdMmYyyyHhMm => "%d/%m/%Y %H:%M".to_string(),
            DateFormat::MmDdYyHhMm => "%m/%d/%y %H:%M".to_string(),
            DateFormat::MmDdYyyyHhMm => "%m/%d/%Y %H:%M".to_string(),
            DateFormat::Custom(fmt) => fmt.clone(),
            DateFormat::Default => "%d/%m/%Y %H:%M".to_string(),
        }
    }
}
