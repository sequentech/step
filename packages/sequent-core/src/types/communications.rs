// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum AudienceSelection {
    #[strum(serialize = "ALL_USERS")]
    ALL_USERS,
    #[strum(serialize = "NOT_VOTED")]
    NOT_VOTED,
    #[strum(serialize = "VOTED")]
    VOTED,
    #[strum(serialize = "SELECTED")]
    SELECTED,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum CommunicationType {
    #[strum(serialize = "CREDENTIALS")]
    CREDENTIALS,
    #[strum(serialize = "BALLOT_RECEIPT")]
    BALLOT_RECEIPT,
    #[strum(serialize = "PARTICIPATION_REPORT")]
    PARTICIPATION_REPORT,
    #[strum(serialize = "ELECTORAL_RESULTS")]
    ELECTORAL_RESULTS,
    #[strum(serialize = "OTP")]
    OTP,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum CommunicationMethod {
    #[strum(serialize = "EMAIL")]
    EMAIL,
    #[strum(serialize = "SMS")]
    SMS,
}
