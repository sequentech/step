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
    #[strum(serialize = "TALLY_REPORT")]
    TALLY_REPORT,
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
    #[strum(serialize = "DOCUMENT")]
    DOCUMENT,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct EmailConfig {
    pub subject: String,
    pub plaintext_body: String,
    pub html_body: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SmsConfig {
    pub message: String,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SendCommunicationBody {
    pub audience_selection: Option<AudienceSelection>,
    pub audience_voter_ids: Option<Vec<String>>,
    pub r#type: Option<CommunicationType>,
    pub communication_method: Option<CommunicationMethod>,
    pub schedule_now: Option<bool>,
    pub schedule_date: Option<String>,
    pub email: Option<EmailConfig>,
    pub sms: Option<SmsConfig>,
    pub document: Option<String>,
    pub name: Option<String>,
    pub alias: Option<String>,
}
