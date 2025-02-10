// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::io::{Input, Output};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sequent_core::services::s3;
use sequent_core::util::aws::get_region;
use sequent_core::{services::pdf::PrintToPdfOptions, util::convert_vec::IntoVec};
use tokio::runtime::Runtime;
use tracing::{info, instrument};

cfg_if::cfg_if! {
    if #[cfg(feature = "aws_lambda")] {
        fn uri(input: &Input) -> Result<String, String> {
            let bucket = input.clone().bucket.ok_or_else(|| format!("missing bucket"))?;
            Ok(
                format!(
                    "https://{}.s3.{}.amazonaws.com/{}/{}",
                    bucket,
                    std::env::var("AWS_REGION")
                        .map_err(|err| format!("AWS_REGION env var missing: {err}"))?,
                    bucket,
                    input.clone().html_path.ok_or_else(|| format!("missing html_path"))?,
                )
            )
        }
    } else if #[cfg(feature = "openwhisk")] {
        fn uri(input: &Input) -> String {
            todo!()
        }
    } else {
        fn uri(input: &Input) -> String {
            unreachable!()
        }
    }
}

pub fn render_pdf(input: Input) -> Result<Output, String> {
    let html = if let Some(html) = input.html {
        html
    } else if let Some(ref html_path) = input.html_path {
        let rt =
            Runtime::new().map_err(|err| format!("error creating Tokio runtime: {:?}", err))?;

        let bucket = input
            .bucket
            .clone()
            .ok_or_else(|| format!("missing bucket"))?;
        let html_path = input
            .html_path
            .clone()
            .ok_or_else(|| format!("missing html_path"))?;

        let uri = uri(&input)?;
        rt.block_on(async {
            String::from_utf8(
                s3::get_file_from_s3(bucket, html_path)
                    .await
                    .expect("could not retrieve file from S3"),
            )
        })
        .map_err(|err| format!("error downloading HTML to render from s3: {:?}", err))?
    } else {
        return Err(format!("no html or html_path were provided"));
    };
    let bytes = sequent_core::services::pdf::html_to_pdf(html, input.pdf_options)
        .map_err(|e| format!("error generating PDF: {e:?}"))?;

    info!("PDF generation completed");

    Ok(Output {
        pdf: Some(bytes),
        ..Default::default()
    })
}
