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
/// Audience selection for communication templates.
pub enum AudienceSelection {
    #[strum(serialize = "ALL_USERS")]
    /// Template is meant to be sent to all users, regardless of their voting status.
    ALL_USERS,
    #[strum(serialize = "NOT_VOTED")]
    /// Template is meant to be sent only to users who have not voted yet.
    NOT_VOTED,
    #[strum(serialize = "VOTED")]
    /// Template is meant to be sent only to users who have voted.
    VOTED,
    #[strum(serialize = "SELECTED")]
    /// Template is meant to be sent only to selected users.
    SELECTED,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
/// Template types for communication templates.
pub enum TemplateType {
    #[strum(serialize = "CREDENTIALS")]
    /// Template for credentials.
    CREDENTIALS,
    #[strum(serialize = "BALLOT_RECEIPT")]
    /// Template for ballot receipts.
    BALLOT_RECEIPT,
    #[strum(serialize = "PARTICIPATION_REPORT")]
    /// Template for participation reports.
    PARTICIPATION_REPORT,
    #[strum(serialize = "ELECTORAL_RESULTS")]
    /// Template for electoral results.
    ELECTORAL_RESULTS,
    #[strum(serialize = "OTP")]
    /// Template for one-time passwords.
    OTP,
    #[strum(serialize = "TALLY_REPORT")]
    /// Template for tally reports.
    TALLY_REPORT,
    #[strum(serialize = "MANUALLY_VERIFY_VOTER")]
    /// Template for manually verifying voters.
    MANUALLY_VERIFY_VOTER,
    #[strum(serialize = "MANUALLY_VERIFY_APPROVAL")]
    /// Template for manually verifying approvals.
    MANUALLY_VERIFY_APPROVAL,
}

#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
/// Communication methods for communication templates.
pub enum TemplateMethod {
    #[strum(serialize = "EMAIL")]
    /// Template is meant to be sent via email.
    EMAIL,
    #[strum(serialize = "SMS")]
    /// Template is meant to be sent via SMS.
    SMS,
    #[strum(serialize = "DOCUMENT")]
    /// Template is meant to be sent as a document.
    DOCUMENT,
}

#[derive(Deserialize, Debug, Serialize, Clone, Default)]
/// Configuration for email templates.
pub struct EmailConfig {
    /// The subject of the email.
    pub subject: String,
    /// The plaintext body of the email.
    pub plaintext_body: String,
    /// The HTML body of the email.
    pub html_body: Option<String>,
}

#[derive(Deserialize, Debug, Serialize, Clone, Default)]
/// Configuration for SMS templates.
pub struct SmsConfig {
    /// The message of the SMS.
    pub message: String,
}

/// A replica of `headless_chrome::types::PrintToPdfOptions` version = "1.0.12"
/// that implements Clone
#[derive(Deserialize, Debug, Serialize, Clone, Default)]
#[allow(missing_docs)]
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
    pub generate_document_outline: Option<bool>,
    pub generate_tagged_pdf: Option<bool>,
}

impl PrintToPdfOptionsLocal {
    #[must_use]
    /// Creates a `PrintToPdfOptionsLocal` from a `PrintToPdfOptions` by copying all fields except `transfer_mode`.
    pub fn from_pdf_options(
        pdf_options: &PrintToPdfOptions,
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
            generate_document_outline: pdf_options.generate_document_outline,
            generate_tagged_pdf: pdf_options.generate_tagged_pdf,
        }
    }

    /// Ignores Transfer mode which is private and not clonable
    #[must_use]
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
            generate_document_outline: self.generate_document_outline,
            generate_tagged_pdf: self.generate_tagged_pdf,
            transfer_mode: None,
        }
    }
}
#[derive(Deserialize, Debug, Serialize, Clone)]
/// Struct for the body of the `send_template` endpoint.
pub struct SendTemplateBody {
    // TODO: Rename this struct
    /// The users to send the template to
    pub audience_selection: Option<AudienceSelection>,
    /// Voter IDs to send the template to, if `audience_selection` is `SELECTED`
    pub audience_voter_ids: Option<Vec<String>>,
    /// The type of communication method to use for the template
    pub communication_method: Option<TemplateMethod>,
    /// Whether to schedule the template to be sent immediately
    pub schedule_now: Option<bool>,
    /// The date to schedule the template to be sent
    pub schedule_date: Option<String>,
    /// Configuration for email templates (if `communication_method` is `EMAIL`)
    pub email: Option<EmailConfig>,
    /// Configuration for SMS templates (if `communication_method` is `SMS`)
    pub sms: Option<SmsConfig>,
    /// The document to send with the template
    pub document: Option<String>,
    /// The name of the template
    pub name: Option<String>,
    /// The alias of the template
    pub alias: Option<String>,
    /// PDF options for the template
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    /// Report options for the template
    pub report_options: Option<ReportOptions>,
}

/// Struct for the DEFAULT `extra_config` JSON file.
#[derive(Serialize, Deserialize, Debug)]
pub struct ReportExtraConfig {
    /// PDF options for the report.
    pub pdf_options: PrintToPdfOptionsLocal,
    /// Communications configuration for the report.
    pub communication_templates: CommunicationTemplatesExtraConfig,
    /// Report options for the report.
    pub report_options: ReportOptions,
}

/// Struct for DEFAULT Communication Templates in `extra_config` JSON file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommunicationTemplatesExtraConfig {
    /// Configuration for email templates.
    pub email_config: EmailConfig,
    /// Configuration for SMS templates.
    pub sms_config: SmsConfig,
}

/// Struct for DEFAULT `ReportOptions` in `extra_config` JSON file.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ReportOptions {
    /// Maximum number of items to include in the report. If `None`, there is no limit.
    pub max_items_per_report: Option<usize>,
    /// Maximum number of threads to use when generating the report.
    pub max_threads: Option<usize>,
}
