// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Communication templates and PDF generation settings for notifications and reports.

use headless_chrome::types::PrintToPdfOptions;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

/// Which voters receive a scheduled communication template.
#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum AudienceSelection {
    /// Every voter in scope.
    #[strum(serialize = "ALL_USERS")]
    ALL_USERS,
    /// Voters who have not yet cast a ballot.
    #[strum(serialize = "NOT_VOTED")]
    NOT_VOTED,
    /// Voters who have already cast a ballot.
    #[strum(serialize = "VOTED")]
    VOTED,
    /// A manually chosen subset of voters.
    #[strum(serialize = "SELECTED")]
    SELECTED,
}

/// Category of notification or report template.
#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum TemplateType {
    /// Voter login credentials.
    #[strum(serialize = "CREDENTIALS")]
    CREDENTIALS,
    /// Confirmation after a ballot is cast.
    #[strum(serialize = "BALLOT_RECEIPT")]
    BALLOT_RECEIPT,
    /// Turnout summary report.
    #[strum(serialize = "PARTICIPATION_REPORT")]
    PARTICIPATION_REPORT,
    /// Published election results report.
    #[strum(serialize = "ELECTORAL_RESULTS")]
    ELECTORAL_RESULTS,
    /// One-time password for authentication.
    #[strum(serialize = "OTP")]
    OTP,
    /// Post-tally audit report.
    #[strum(serialize = "TALLY_REPORT")]
    TALLY_REPORT,
    /// Notification after manual voter verification.
    #[strum(serialize = "MANUALLY_VERIFY_VOTER")]
    MANUALLY_VERIFY_VOTER,
    /// Notification after manual application approval.
    #[strum(serialize = "MANUALLY_VERIFY_APPROVAL")]
    MANUALLY_VERIFY_APPROVAL,
}

/// Delivery channel for a communication template.
#[allow(non_camel_case_types)]
#[derive(
    Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, EnumString,
)]
pub enum TemplateMethod {
    /// Email delivery.
    #[strum(serialize = "EMAIL")]
    EMAIL,
    /// SMS text message delivery.
    #[strum(serialize = "SMS")]
    SMS,
    /// Generated PDF or other document.
    #[strum(serialize = "DOCUMENT")]
    DOCUMENT,
}

/// Email subject and body for a template send.
#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub struct EmailConfig {
    /// Email subject line.
    pub subject: String,
    /// Plain-text body.
    pub plaintext_body: String,
    /// Optional HTML body.
    pub html_body: Option<String>,
}

/// SMS message body for a template send.
#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub struct SmsConfig {
    /// SMS message text.
    pub message: String,
}

/// A replica of `headless_chrome::types::PrintToPdfOptions` version = "1.0.12"
/// that implements Clone
#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub struct PrintToPdfOptionsLocal {
    /// Print pages in landscape orientation.
    pub landscape: Option<bool>,
    /// Include browser header and footer in the PDF.
    pub display_header_footer: Option<bool>,
    /// Render background colors and images.
    pub print_background: Option<bool>,
    /// Scale factor applied to page content.
    pub scale: Option<f64>,
    /// Paper width in inches.
    pub paper_width: Option<f64>,
    /// Paper height in inches.
    pub paper_height: Option<f64>,
    /// Top margin in inches.
    pub margin_top: Option<f64>,
    /// Bottom margin in inches.
    pub margin_bottom: Option<f64>,
    /// Left margin in inches.
    pub margin_left: Option<f64>,
    /// Right margin in inches.
    pub margin_right: Option<f64>,
    /// Page ranges to include (e.g. `"1-3,5"`).
    pub page_ranges: Option<String>,
    /// Silently skip invalid entries in `page_ranges`.
    pub ignore_invalid_page_ranges: Option<bool>,
    /// HTML template for the page header.
    pub header_template: Option<String>,
    /// HTML template for the page footer.
    pub footer_template: Option<String>,
    /// Use CSS `@page` size instead of `paper_width` / `paper_height`.
    pub prefer_css_page_size: Option<bool>,
    /// Chrome transfer mode (not populated on conversion; field kept for JSON round-trip).
    pub transfer_mode: Option<String>,
    /// Generate a PDF document outline (bookmarks).
    pub generate_document_outline: Option<bool>,
    /// Generate a tagged (accessible) PDF.
    pub generate_tagged_pdf: Option<bool>,
}

impl PrintToPdfOptionsLocal {
    /// Converts from the non-cloneable `headless_chrome` options struct.
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
            generate_document_outline: pdf_options.generate_document_outline,
            generate_tagged_pdf: pdf_options.generate_tagged_pdf,
        }
    }

    /// Converts back to `headless_chrome` options.
    ///
    /// `transfer_mode` is omitted because it is private and not clonable in the upstream type.
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

/// Request body for sending or scheduling a communication template.
/// TODO: Rename this struct
#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SendTemplateBody {
    /// Which voters receive the message.
    pub audience_selection: Option<AudienceSelection>,
    /// Explicit voter identifiers when `audience_selection` is `SELECTED`.
    pub audience_voter_ids: Option<Vec<String>>,
    /// Delivery channel (email, SMS, or document).
    pub communication_method: Option<TemplateMethod>,
    /// When true, send immediately instead of scheduling.
    pub schedule_now: Option<bool>,
    /// ISO 8601 date/time for a deferred send.
    pub schedule_date: Option<String>,
    /// Email content when sending via email.
    pub email: Option<EmailConfig>,
    /// SMS content when sending via SMS.
    pub sms: Option<SmsConfig>,
    /// Document template identifier when generating a PDF.
    pub document: Option<String>,
    /// Human-readable name for this send operation.
    pub name: Option<String>,
    /// Template alias to render.
    pub alias: Option<String>,
    /// PDF rendering options for document templates.
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    /// Report generation limits and threading settings.
    pub report_options: Option<ReportOptions>,
}

/// Struct for the DEFAULT `extra_config` JSON file.
#[derive(Serialize, Deserialize, Debug)]
pub struct ReportExtraConfig {
    /// Default PDF rendering options.
    pub pdf_options: PrintToPdfOptionsLocal,
    /// Default email and SMS template bodies.
    pub communication_templates: CommunicationTemplatesExtraConfig,
    /// Default report generation settings.
    pub report_options: ReportOptions,
}

/// Struct for DEFAULT Communication Templates in `extra_config` JSON file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommunicationTemplatesExtraConfig {
    /// Default email template content.
    pub email_config: EmailConfig,
    /// Default SMS template content.
    pub sms_config: SmsConfig,
}

/// Struct for DEFAULT `ReportOptions` in `extra_config` JSON file.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ReportOptions {
    /// Maximum rows included in a single report chunk.
    pub max_items_per_report: Option<usize>,
    /// Worker thread count for parallel report generation.
    pub max_threads: Option<usize>,
}
