// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use headless_chrome::types::PrintToPdfOptions;
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
pub enum TemplateType {
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
    #[strum(serialize = "MANUALLY_VERIFY_VOTER")]
    MANUALLY_VERIFY_VOTER,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum TemplateMethod {
    #[strum(serialize = "EMAIL")]
    EMAIL,
    #[strum(serialize = "SMS")]
    SMS,
    #[strum(serialize = "DOCUMENT")]
    DOCUMENT,
}

#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub struct EmailConfig {
    pub subject: String,
    pub plaintext_body: String,
    pub html_body: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub struct SmsConfig {
    pub message: String,
}

#[derive(Deserialize, Debug, Serialize /* , Clone */)]
pub struct SendTemplateBody {
    // TODO: Rename this struct
    pub audience_selection: Option<AudienceSelection>,
    pub audience_voter_ids: Option<Vec<String>>,
    pub r#type: Option<TemplateType>,
    pub communication_method: Option<TemplateMethod>,
    pub schedule_now: Option<bool>,
    pub schedule_date: Option<String>,
    pub email: Option<EmailConfig>,
    pub sms: Option<SmsConfig>,
    pub document: Option<String>,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub pdf_options: Option<PrintToPdfOptions>, /* TODO: Fix Clone issue if
                                                 * it's
                                                 * really needed */
}

/// Struct for the DEFAULT extra_config JSON file.
#[derive(Serialize, Deserialize, Debug)]
pub struct ReportExtraConfig {
    pub pdf_options: PrintToPdfOptions,
    pub communication_templates: CommunicationTemplatesExtraConfig,
}

/// Struct for DEFAULT Communication Templates in extra_config JSON file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommunicationTemplatesExtraConfig {
    pub email_config: EmailConfig,
    pub sms_config: SmsConfig,
}
