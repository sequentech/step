// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::{
    signatures::ecies_encrypt::EciesKeyPair,
    types::templates::{PrintToPdfOptionsLocal, ReportOptions, VoteReceiptPipeType},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeConfigVoteReceipts {
    pub template: String,
    pub system_template: String,
    pub extra_data: Value,
    pub enable_pdfs: bool,
    pub pipe_type: VoteReceiptPipeType,
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    pub report_options: Option<ReportOptions>,
    pub execution_annotations: Option<HashMap<String, String>>,
    pub acm_key: Option<EciesKeyPair>,
}

pub const DEFAULT_MCBALLOT_TITLE: &str = "Vote receipts";

impl PipeConfigVoteReceipts {
    #[instrument(skip_all, name = "PipeConfigVoteReceipts::new")]
    pub fn new() -> Self {
        Self::default()
    }

    #[instrument(skip_all, name = "PipeConfigVoteReceipts::mcballot")]
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
            pipe_type: pipe_type.unwrap_or(VoteReceiptPipeType::BALLOT_IMAGES),
            pdf_options: None,
            report_options: None,
            execution_annotations: None,
            acm_key: None,
        }
    }
}

impl Default for PipeConfigVoteReceipts {
    #[instrument(skip_all, name = "PipeConfigVoteReceipts::default")]
    fn default() -> Self {
        let html: &str = include_str!("../resources/vote_receipt_user.hbs");
        let system_html = include_str!("../resources/vote_receipt_system.hbs");

        Self {
            template: html.to_string(),
            system_template: system_html.to_string(),
            extra_data: json!("{}"),
            enable_pdfs: true,
            pipe_type: VoteReceiptPipeType::BALLOT_IMAGES,
            pdf_options: None,
            report_options: None,
            execution_annotations: None,
            acm_key: None,
        }
    }
}
