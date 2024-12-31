// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

mod io;
mod openwhisk;

use crate::io::{Input, Output};

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    info!("Starting PDF service");

    // Log environment variables
    for (key, value) in std::env::vars() {
        if key.contains("PDF") || key.contains("OPENWHISK") {
            info!("ENV: {} = {}", key, value);
        }
    }

    match std::env::var("PDF_TRANSPORT_NAME")
        .unwrap_or_default()
        .as_str()
    {
        "aws_lambda" => {
            #[orare::lambda_runtime]
            fn render_pdf(input: Input) -> Result<Output, String> {
                Ok(Output { pdf_base64: String::new() })
            }
        }
        "openwhisk" => {
            info!("Using OpenWhisk mode");
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
        pdf_transport => {
            error!("Unknown PDF transport: {pdf_transport}");
            std::process::exit(1);
        }
    }
}
