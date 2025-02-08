// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use sequent_core::services::pdf::{PrintToPdfOptions, TransferMode};

#[derive(Debug, Deserialize)]
pub struct Input {
    #[serde(default)]
    pub html: Option<String>,
    #[serde(default)]
    pub html_path: Option<String>,
    #[serde(default)]
    pub pdf_options: Option<PrintToPdfOptions>,
    #[serde(default)]
    pub bucket: Option<String>,
    #[serde(default)]
    pub result_path: Option<String>,
}

impl Clone for Input {
    fn clone(&self) -> Self {
        Input {
            html: self.html.clone(),
            html_path: self.html_path.clone(),
            pdf_options: self
                .pdf_options
                .as_ref()
                .map(|pdf_options| PrintToPdfOptions {
                    landscape: pdf_options.landscape.clone(),
                    display_header_footer: pdf_options.display_header_footer.clone(),
                    print_background: pdf_options.print_background.clone(),
                    scale: pdf_options.scale.clone(),
                    paper_width: pdf_options.paper_width.clone(),
                    paper_height: pdf_options.paper_height.clone(),
                    margin_top: pdf_options.margin_top.clone(),
                    margin_bottom: pdf_options.margin_bottom.clone(),
                    margin_left: pdf_options.margin_left.clone(),
                    margin_right: pdf_options.margin_right.clone(),
                    page_ranges: pdf_options.page_ranges.clone(),
                    ignore_invalid_page_ranges: pdf_options.ignore_invalid_page_ranges.clone(),
                    header_template: pdf_options.header_template.clone(),
                    footer_template: pdf_options.footer_template.clone(),
                    prefer_css_page_size: pdf_options.prefer_css_page_size.clone(),
                    transfer_mode: None,
                }),
            bucket: self.bucket.clone(),
            result_path: self.result_path.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Output {
    pub pdf: Option<Vec<u8>>,
}
