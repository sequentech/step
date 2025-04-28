// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ::tracing::{error, info};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sequent_core::serialization::base64::Base64Deserialize;
use sequent_core::services::pdf::PrintToPdfOptions;
use sequent_core::util::aws::{
    get_fetch_expiration_secs, get_region, get_s3_aws_config, get_upload_expiration_secs,
};
use serde::{Deserialize, Serialize};
use tokio::task;

mod io;
mod openwhisk;
mod pdf;

#[cfg(feature = "aws_lambda")]
use sequent_core::services::s3;

use crate::io::{Input, Output};

cfg_if::cfg_if! {
    if #[cfg(all(feature = "aws_lambda", feature = "openwhisk"))] {
        fn main() {
            compile_error!("Either feature \"openwhisk\" or \"aws_lambda\" has to be provided, but not both");
        }
    } else if #[cfg(feature = "aws_lambda")] {
        #[orare::lambda_runtime]
        async fn render_pdf(input: Input) -> Result<(), String> {
            match input {
                Input::Raw { .. } => {
                    return Err("unsupported mode for an AWS Lambda build. Default limits on the AWS Lambda platform don't allow to have inputs/outputs greater than 6MB in buffered mode. Please, provide the input document through S3; the rendered document will be uploaded to S3 as well by the lambda".to_string())
                },
                Input::S3 { bucket, input_path, output_path, pdf_options } => {
                    let html = String::from_utf8(
                        s3::get_file_from_s3(bucket.clone(), input_path.clone())
                            .await
                            .map_err(|err| format!("could not retrieve file {} (bucket: {}) from S3: {:?}", input_path.clone(), bucket.clone(), err))?
                    ).map_err(|_| format!("provided document is not valid UTF-8"))?;
                    let pdf = pdf::render_pdf(html, pdf_options)
                        .map_err(|err| format!("could not render PDF due to error: {:?}", err))?;
                    s3::upload_data_to_s3(
                        pdf.into(),
                        output_path,
                        false,
                        bucket.clone(),
                        "application/pdf".to_string(),
                        None,
                    ).await
                        .map_err(|err| format!("could not upload PDF to S3: {:?}", err))?;
                }
            }

            Ok(())
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
                    error!("Failed to create Tokio runtime: {:?}", e);
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
