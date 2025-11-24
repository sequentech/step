// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TimeZone {
    UTC,
    Offset(i32), // Offset in hours, e.g., +1 or -4
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DateFormat {
    DdMmYyHhMm,
    DdMmYyyyHhMm,
    MmDdYyHhMm,
    MmDdYyyyHhMm,
    Custom(String),
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
