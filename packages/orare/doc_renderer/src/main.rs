// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use ::tracing::{error, info};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sequent_core::services::pdf::PrintToPdfOptions;
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
        fn render_pdf(input: Input) -> Result<Output, String> {
            pdf::render_pdf(input)
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
