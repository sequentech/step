// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::types::date_time::{DateFormat, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PipeConfigGenerateReports {
    pub enable_pdfs: bool,
    pub report_content_template: Option<String>,
}
