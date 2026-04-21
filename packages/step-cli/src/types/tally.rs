// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString)]
/// Tally execution status
pub enum TallyExecutionStatus {
    /// Not started
    NOT_STARTED,
    /// Started
    STARTED,
    /// Connected
    CONNECTED,
    /// In progress
    IN_PROGRESS,
    /// Success
    SUCCESS,
    /// Cancelled
    CANCELLED,
}
