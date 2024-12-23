// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use orare::lambda_runtime;
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::{info, error};

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

    match std::env::var("PDF_TRANSPORT_NAME").unwrap_or_default().as_str() {
        "orare-openwhisk" => {
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
        _ => {
            info!("Using Inplace mode");
            // Only use lambda_runtime in non-OpenWhisk mode
            #[lambda_runtime]
            fn render_pdf(input: Input) -> Result<Output, String> {
                let bytes = sequent_core::services::pdf::html_to_pdf(input.html.unwrap_or_default(), input.pdf_options)
                    .map_err(|e| e.to_string())?;

                let pdf_base64 = BASE64.encode(bytes);
                info!("PDF generation completed");
                Ok(Output { pdf_base64 })
            }

            // For inplace mode, we need input
            let input = std::env::args().nth(1).unwrap_or_else(|| "{}".to_string());
            let input: Input = serde_json::from_str(&input).unwrap_or(Input {
                html: Some("".to_string()),
                pdf_options: None,
            });

            render_pdf(input).unwrap();
        }
    }
}
