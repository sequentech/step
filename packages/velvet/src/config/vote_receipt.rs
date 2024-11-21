// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeConfigVoteReceipts {
    pub template: String,
    pub extra_data: Value,
    pub enable_pdfs: bool,
}

impl PipeConfigVoteReceipts {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for PipeConfigVoteReceipts {
    fn default() -> Self {
        let html = include_str!("../resources/vote_receipts.hbs");

        Self {
            template: html.to_string(),
            extra_data: json!("{}"),
            enable_pdfs: false,
        }
    }
}
