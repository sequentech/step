// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::{
    signatures::ecies_encrypt::EciesKeyPair,
    types::templates::{PrintToPdfOptionsLocal, ReportOptions},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeConfigBallotImages {
    pub template: String,
    pub system_template: String,
    pub extra_data: Value,
    pub enable_pdfs: bool,
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    pub report_options: Option<ReportOptions>,
    pub execution_annotations: Option<HashMap<String, String>>,
    pub acm_key: Option<EciesKeyPair>,
}

pub const DEFAULT_MCBALLOT_TITLE: &str = "Ballot images";

impl PipeConfigBallotImages {
    #[instrument(skip_all, name = "PipeConfigBallotImages::new")]
    pub fn new() -> Self {
        Self::default()
    }

    #[instrument(skip_all, name = "PipeConfigBallotImages::mcballot")]
    pub fn mcballot() -> Self {
        let html: &str = include_str!("../resources/ballot_images_user.hbs");
        let system_html = include_str!("../resources/ballot_images_system.hbs");

        Self {
            template: html.to_string(),
            system_template: system_html.to_string(),
            extra_data: json!({
                "title": DEFAULT_MCBALLOT_TITLE,
                "file_logo": "http://minio:9000/public/public-assets/sequent-logo.svg",
                "file_qrcode_lib": "http://minio:9000/public/public-assets/qrcode.min.js"
            }),
            enable_pdfs: true,
            pdf_options: None,
            report_options: None,
            execution_annotations: None,
            acm_key: None,
        }
    }
}

impl Default for PipeConfigBallotImages {
    #[instrument(skip_all, name = "PipeConfigBallotImages::default")]
    fn default() -> Self {
        let html: &str = include_str!("../resources/ballot_images_user.hbs");
        let system_html = include_str!("../resources/ballot_images_system.hbs");

        Self {
            template: html.to_string(),
            system_template: system_html.to_string(),
            extra_data: json!("{}"),
            enable_pdfs: true,
            pdf_options: None,
            report_options: None,
            execution_annotations: None,
            acm_key: None,
        }
    }
}
