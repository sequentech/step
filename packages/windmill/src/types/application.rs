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
impl ApplicationType {
    pub fn to_string(&self) -> String {
        match self {
            ApplicationType::AUTOMATIC => "AUTOMATIC".to_string(),
            ApplicationType::MANUAL => "MANUAL".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct ApplicationStatusUpdateEvent {
    pub application_type: ApplicationType,
    pub application_status: ApplicationStatus,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Debug, PartialEq, Eq, Clone, EnumString, EnumVariantNames, Serialize, Deserialize,
)]
pub enum ApplicationRejectReason {
    INSUFFICIENT_INFORMATION,
    NO_VOTER,
    ALREADY_APPROVED,
    OTHER, //mandatory comment
}
impl ApplicationRejectReason {
    pub fn to_string(&self) -> String {
        match self {
            ApplicationRejectReason::INSUFFICIENT_INFORMATION => {
                "insufficient-information".to_string()
            }
            ApplicationRejectReason::NO_VOTER => "no-matching-voter".to_string(),
            ApplicationRejectReason::ALREADY_APPROVED => "voter-already-approved".to_string(),
            ApplicationRejectReason::OTHER => "other".to_string(),
        }
    }
}
