// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    #[strum(serialize = "MANUALLY_VERIFY_APPROVAL")]
    MANUALLY_VERIFY_APPROVAL,
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

/// A replica of headless_chrome::types::PrintToPdfOptions version = "1.0.12"
/// that implements Clone
#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub struct PrintToPdfOptionsLocal {
    pub landscape: Option<bool>,
    pub display_header_footer: Option<bool>,
    pub print_background: Option<bool>,
    pub scale: Option<f64>,
    pub paper_width: Option<f64>,
    pub paper_height: Option<f64>,
    pub margin_top: Option<f64>,
    pub margin_bottom: Option<f64>,
    pub margin_left: Option<f64>,
    pub margin_right: Option<f64>,
    pub page_ranges: Option<String>,
    pub ignore_invalid_page_ranges: Option<bool>,
    pub header_template: Option<String>,
    pub footer_template: Option<String>,
    pub prefer_css_page_size: Option<bool>,
    pub transfer_mode: Option<String>,
}

impl PrintToPdfOptionsLocal {
    pub fn from_pdf_options(
        pdf_options: PrintToPdfOptions,
    ) -> PrintToPdfOptionsLocal {
        PrintToPdfOptionsLocal {
            landscape: pdf_options.landscape,
            display_header_footer: pdf_options.display_header_footer,
            print_background: pdf_options.print_background,
            scale: pdf_options.scale,
            paper_width: pdf_options.paper_width,
            paper_height: pdf_options.paper_height,
            margin_top: pdf_options.margin_top,
            margin_bottom: pdf_options.margin_bottom,
            margin_left: pdf_options.margin_left,
            margin_right: pdf_options.margin_right,
            page_ranges: pdf_options.page_ranges.clone(),
            ignore_invalid_page_ranges: pdf_options.ignore_invalid_page_ranges,
            header_template: pdf_options.header_template.clone(),
            footer_template: pdf_options.footer_template.clone(),
            prefer_css_page_size: pdf_options.prefer_css_page_size,
            transfer_mode: None,
        }
    }

    /// Ignores Transfer mode which is private and not clonable
    pub fn to_print_to_pdf_options(&self) -> PrintToPdfOptions {
        PrintToPdfOptions {
            landscape: self.landscape,
            display_header_footer: self.display_header_footer,
            print_background: self.print_background,
            scale: self.scale,
            paper_width: self.paper_width,
            paper_height: self.paper_height,
            margin_top: self.margin_top,
            margin_bottom: self.margin_bottom,
            margin_left: self.margin_left,
            margin_right: self.margin_right,
            page_ranges: self.page_ranges.clone(),
            ignore_invalid_page_ranges: self.ignore_invalid_page_ranges,
            header_template: self.header_template.clone(),
            footer_template: self.footer_template.clone(),
            prefer_css_page_size: self.prefer_css_page_size,
            transfer_mode: None,
        }
    }
}
#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SendTemplateBody {
    // TODO: Rename this struct
    pub audience_selection: Option<AudienceSelection>,
    pub audience_voter_ids: Option<Vec<String>>,
    pub communication_method: Option<TemplateMethod>,
    pub schedule_now: Option<bool>,
    pub schedule_date: Option<String>,
    pub email: Option<EmailConfig>,
    pub sms: Option<SmsConfig>,
    pub document: Option<String>,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    pub report_options: Option<ReportOptions>,
}

/// Struct for the DEFAULT extra_config JSON file.
#[derive(Serialize, Deserialize, Debug)]
pub struct ReportExtraConfig {
    pub pdf_options: PrintToPdfOptionsLocal,
    pub communication_templates: CommunicationTemplatesExtraConfig,
    pub report_options: ReportOptions,
}

/// Struct for DEFAULT Communication Templates in extra_config JSON file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommunicationTemplatesExtraConfig {
    pub email_config: EmailConfig,
    pub sms_config: SmsConfig,
}

/// Struct for DEFAULT ReportOptions in extra_config JSON file.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ReportOptions {
    pub max_items_per_report: Option<usize>,
    pub max_threads: Option<usize>,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum VoteReceiptPipeType {
    #[strum(serialize = "BALLOT_IMAGES")]
    BALLOT_IMAGES,
}
