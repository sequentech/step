// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::io::{Input, Output};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
#[cfg(feature = "aws_lambda")]
use sequent_core::services::s3;
use sequent_core::util::aws::get_region;
use sequent_core::{services::pdf::PrintToPdfOptions, util::convert_vec::IntoVec};
use tokio::runtime::Runtime;
use tracing::{info, instrument};

cfg_if::cfg_if! {
    if #[cfg(feature = "aws_lambda")] {
        pub fn uri(input: &Input) -> Result<String, String> {
            match input {
                Input::Raw { .. } => {
                    Err(format!("invalid input for AWS Lambda: use S3 input instead"))
                },
                Input::S3 { bucket, input_path, .. } => {
                    Ok(
                        format!(
                            "https://{}.s3.{}.amazonaws.com/{}/{}",
                            bucket,
                            std::env::var("AWS_REGION")
                                .map_err(|err| format!("AWS_REGION env var missing: {err}"))?,
                            bucket,
                            input_path,
                        )
                    )
                },
            }
        }
    } else if #[cfg(feature = "openwhisk")] {
        pub fn uri(input: &Input) -> String {
            match input {
                Input::Raw { .. } => {
                    todo!()
                },
                Input::S3 { bucket, input_path, .. } => {
                    panic!("invalid input for OpenWhisk lambda: use raw input instead")
                },
            }
        }
    } else {
        pub fn uri(input: &Input) -> String {
            // Unreachable because the lambda has to be built either
            // with aws_lambda or openwhisk crate features. It will
            // fail to build otherwise.
            unreachable!()
        }
    }
}

pub fn render_pdf(html: String, pdf_options: Option<PrintToPdfOptions>) -> Result<Vec<u8>, String> {
    let bytes = sequent_core::services::pdf::html_to_pdf(html, pdf_options)
        .map_err(|e| format!("error generating PDF: {e:?}"))?;

    info!("PDF generation completed");

    Ok(bytes)
}
