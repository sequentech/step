// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::types::templates::VoteReceiptPipeType;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeConfigVoteReceipts {
    pub template: String,
    pub system_template: String,
    pub extra_data: Value,
    pub enable_pdfs: bool,
    pub pipe_type: VoteReceiptPipeType,
}

pub const DEFAULT_MCBALLOT_TITLE: &str = "Vote receipts";

impl PipeConfigVoteReceipts {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mcballot(pipe_type: Option<VoteReceiptPipeType>) -> Self {
        let html: &str = include_str!("../resources/vote_receipt_user.hbs");
        let system_html = include_str!("../resources/vote_receipt_system.hbs");

        Self {
            template: html.to_string(),
            system_template: system_html.to_string(),
            extra_data: json!({
                "title": DEFAULT_MCBALLOT_TITLE,
                "file_logo": "http://minio:9000/public/public-assets/sequent-logo.svg",
                "file_qrcode_lib": "http://minio:9000/public/public-assets/qrcode.min.js"
            }),
            enable_pdfs: true,
            pipe_type: pipe_type.unwrap_or(VoteReceiptPipeType::VOTE_RECEIPT),
        }
    }
}

impl Default for PipeConfigVoteReceipts {
    fn default() -> Self {
        let html: &str = include_str!("../resources/vote_receipt_user.hbs");
        let system_html = include_str!("../resources/vote_receipt_system.hbs");

        Self {
            template: html.to_string(),
            system_template: system_html.to_string(),
            extra_data: json!("{}"),
            enable_pdfs: true,
            pipe_type: VoteReceiptPipeType::VOTE_RECEIPT,
        }
    }
}
