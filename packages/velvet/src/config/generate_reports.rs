// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::types::{
    date_time::{DateFormat, TimeZone},
    templates::PrintToPdfOptionsLocal,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, str::FromStr};
use strum_macros::EnumString;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PipeConfigGenerateReports {
    pub enable_pdfs: bool,
    pub report_content_template: Option<String>,
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    pub execution_annotations: HashMap<String, String>,
    pub system_template: String,
    pub extra_data: Value,
}

#[derive(Serialize, Deserialize, Debug, Default, EnumString)]
pub enum CandidatesOrderPolicy {
    #[default]
    SortByWinningPosition,
    AsInBallot,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ContestReportConfig {
    pub candidates_order: CandidatesOrderPolicy,
}

pub const CONTEST_REPORT_CONFIG: &'static str = "sequent:velvet:contest-report-config";
