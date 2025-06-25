// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use sequent_core::services::pdf::{PrintToPdfOptions, TransferMode};

/// Lambda Inputs are of two forms: raw, which receives the full HTML
/// and returns the base64 PDF.
///
/// Some platforms such as AWS Lambda have limitations (or extra
/// costs) when dealing with big parameters or return values (e.g. >
/// 6MB on buffered mode, uncapped on streaming mode, but with extra
/// costs.
///
/// This leads to two workflows: raw, that will primarily be used by
/// OpenWhisk locally, unbounded input and output size, and S3, which
/// uses S3 as a intermediate store: the calling code uploads the
/// lambda input to S3, the lambda reads it, renders, and writes back
/// the result to the S3 store, which in turn will be read by the code
/// that invoked the lambda, finally returning the PDF to the user.
#[derive(Debug, Serialize, Deserialize)]
pub enum Input {
    #[serde(rename = "raw")]
    Raw {
        html: String,
        #[serde(default)]
        pdf_options: Option<PrintToPdfOptions>,
    },
    #[serde(rename = "s3")]
    S3 {
        // The bucket name.
        bucket: String,
        // Where the lambda within the provided bucket will read the
        // document to render.
        input_path: String,
        // Where the lambda within the provided bucket will write the
        // rendered document.
        output_path: String,
        #[serde(default)]
        pdf_options: Option<PrintToPdfOptions>,
    },
}

impl Clone for Input {
    fn clone(&self) -> Self {
        match self {
            Input::Raw { html, pdf_options } => Input::Raw {
                html: html.clone(),
                pdf_options: pdf_options.as_ref().map(|pdf_options| PrintToPdfOptions {
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
            },
            Input::S3 {
                bucket,
                input_path,
                output_path,
                pdf_options,
            } => Input::S3 {
                bucket: bucket.clone(),
                input_path: input_path.clone(),
                output_path: output_path.clone(),
                pdf_options: pdf_options.as_ref().map(|pdf_options| PrintToPdfOptions {
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
            },
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Output {
    // If the Input was provided via the S3 mechanism, the pdf_base64
    // will not be present on the HTTP response, but a success HTTP
    // return code will be provided on success, and the PDF can be
    // found on the `output_path` provided in the lambda input at the
    // provided bucket.
    pub pdf_base64: Option<String>,
}
