// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::{
    ballot::ConsolidatedReportPolicy,
    types::{
        ceremonies::TallyType,
        date_time::{DateFormat, TimeZone},
        templates::PrintToPdfOptionsLocal,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, str::FromStr};
use strum_macros::EnumString;

/// Configuration for the report generation pipeline.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PipeConfigGenerateReports {
    /// Whether to generate PDF outputs from reports.
    pub enable_pdfs: bool,
    /// HTML template for report content rendering.
    pub report_content_template: Option<String>,
    /// PDF printing options configuration.
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    /// Execution metadata and annotations.
    pub execution_annotations: HashMap<String, String>,
    /// System-level template for report processing.
    pub system_template: String,
    /// Additional data passed to report templates.
    pub extra_data: Value,
    /// Type of tally (electoral results or initialization).
    pub tally_type: TallyType,
}

/// Policy for ordering candidates in report output.
#[derive(Serialize, Deserialize, Debug, Default, EnumString)]
pub enum CandidatesOrderPolicy {
    /// Sort candidates by their winning position (default).
    #[default]
    SortByWinningPosition,
    /// Keep candidates in the order they appear on the ballot.
    AsInBallot,
}

/// Per-contest configuration for report generation.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ContestReportConfig {
    /// Order policy for candidates in this contest's report.
    pub candidates_order: CandidatesOrderPolicy,
}

/// Configuration key for storing per-contest report settings.
pub const CONTEST_REPORT_CONFIG: &str = "sequent:velvet:contest-report-config";
