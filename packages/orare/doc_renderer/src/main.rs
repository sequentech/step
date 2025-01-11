// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ::tracing::{error, info};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sequent_core::serialization::base64::Base64Deserialize;
use sequent_core::services::pdf::PrintToPdfOptions;
use sequent_core::services::s3;
use sequent_core::util::aws::{
    get_fetch_expiration_secs, get_s3_aws_config, get_upload_expiration_secs,
};
use serde::{Deserialize, Serialize};

mod io;
mod openwhisk;
mod pdf;

use crate::io::{Input, Output};

cfg_if::cfg_if! {
    if #[cfg(all(feature = "aws_lambda", feature = "openwhisk"))] {
        fn main() {
            compile_error!("Either feature \"openwhisk\" or \"aws_lambda\" has to be provided, but not both");
        }
    } else if #[cfg(feature = "aws_lambda")] {
        #[orare::lambda_runtime]
        async fn render_pdf(input: Input) -> Result<Output, String> {
            let pdf = pdf::render_pdf(input.clone())?;
            let bucket = input.bucket;
            if let Some(bucket) = bucket {
                let bucket_path = input.bucket_path.ok_or_else(|| format!("missing path in bucket for PDF"))?;
                let raw_pdf = BASE64.decode(pdf.clone().pdf_base64)
                    .map_err(|e| format!("error deserializing PDF in base64 encoding: {e:?}"))?;
                s3::upload_file_to_s3(
                    sha256::digest(raw_pdf),
                    true,
                    bucket,
                    "application/pdf".to_string(),
                    bucket_path,
                    None,
                ).await.map_err(|e| format!("error uploading PDF file to S3: {e:?}"))?;
            }
            Ok(pdf)
        }
    } else if #[cfg(feature = "openwhisk")] {
        fn main() {
            tracing_subscriber::fmt::init();

            // Create a new tokio runtime for the server
            match tokio::runtime::Runtime::new() {
                Ok(rt) => {
                    info!("Created Tokio runtime successfully");
                    // Block on the server - this should run forever
                    rt.block_on(async {
                        info!("Starting OpenWhisk server on port 8080...");
                        openwhisk::start_server().await;
                    });
                }
                Err(e) => {
                    error!("Failed to create Tokio runtime: {}", e);
                    std::process::exit(1);
                }
            }
        }
    } else {
        fn main() {
            compile_error!("Either feature \"openwhisk\" or \"aws_lambda\" has to be provided");
        }
    }
}
