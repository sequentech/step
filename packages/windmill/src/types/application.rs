// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString, EnumVariantNames};

#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum ApplicationStatus {
    PENDING,
    ACCEPTED,
    REJECTED,
}
impl ApplicationStatus {
    pub fn to_string(&self) -> String {
        match self {
            ApplicationStatus::PENDING => "PENDING".to_string(),
            ApplicationStatus::ACCEPTED => "ACCEPTED".to_string(),
            ApplicationStatus::REJECTED => "REJECTED".to_string(),
        }
    }
}

#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum ApplicationType {
    AUTOMATIC,
    MANUAL,
}
