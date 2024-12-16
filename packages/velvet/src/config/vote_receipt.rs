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

pub const DEFAULT_MCBALLOT_TITLE: &str = "Vote receipts";

impl PipeConfigVoteReceipts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mcballot() -> Self {
        let html = include_str!("../resources/mcballot_receipts.hbs");

        Self {
            template: html.to_string(),
            extra_data: json!({
                "title": DEFAULT_MCBALLOT_TITLE,
                "file_logo": "http://minio:9000/public/public-assets/sequent-logo.svg",
                "file_qrcode_lib": "http://minio:9000/public/public-assets/qrcode.min.js"
            }),
            enable_pdfs: false,
        }
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
