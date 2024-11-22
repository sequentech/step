// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use headless_chrome::types::PrintToPdfOptions;
use serde_json::json;
use std::{fs, process::Command, path::PathBuf};
use tracing::{event, instrument, Level};
use reqwest;

pub enum PdfTransport {
    Orare {
        binary_path: String,
        features: Vec<String>,
    },
    OrareOpenWhisk {
        endpoint: String,
    },
    Inplace,
    Console,
}

pub struct PdfRenderer {
    transport: PdfTransport,
}

impl PdfRenderer {
    #[instrument(err)]
    pub async fn new() -> Result<Self> {
        event!(Level::INFO, "PdfRenderer::new() - Starting initialization");
        
        let pdf_transport_name = match std::env::var("PDF_TRANSPORT_NAME") {
            Ok(name) => {
                event!(Level::INFO, "Found PDF_TRANSPORT_NAME: {}", name);
                name
            },
            Err(e) => {
                event!(Level::ERROR, "Failed to get PDF_TRANSPORT_NAME: {}", e);
                return Err(anyhow!("PDF_TRANSPORT_NAME env var missing"));
            }
        };

        event!(Level::INFO, "PdfTransport: {pdf_transport_name}");

        let transport = match pdf_transport_name.as_str() {
            "orare-openwhisk" => {
                PdfTransport::OrareOpenWhisk {
                    endpoint: "http://orare:8082/render".to_string(),
                }
            }
            "orare-inplace" => {
                let binary_path = std::env::var("PDF_LAMBDA_BINARY_PATH")
                    .map_err(|_err| anyhow!("PDF_LAMBDA_BINARY_PATH env var missing"))?;
                PdfTransport::Orare {
                    binary_path,
                    features: vec!["inplace".to_string()],
                }
            }
            "Inplace" => PdfTransport::Inplace,
            _ => PdfTransport::Console,
        };

        Ok(PdfRenderer { transport })
    }

    #[instrument(skip(self, html), err)]
    pub async fn render_pdf(
        &self,
        html: String,
        pdf_options: Option<PrintToPdfOptions>,
    ) -> Result<Vec<u8>> {
        match &self.transport {
            PdfTransport::OrareOpenWhisk { endpoint } => {
                event!(Level::INFO, "Using OpenWhisk endpoint: {}", endpoint);
                let client = reqwest::Client::new();
                let payload = json!({
                    "html": html,
                    "pdf_options": pdf_options,
                });

                let response = client
                    .post(endpoint)
                    .json(&payload)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    let error = response.text().await?;
                    event!(Level::ERROR, "OpenWhisk request failed: {}", error);
                    return Err(anyhow!("OpenWhisk request failed: {}", error));
                }

                let response_json = response.json::<serde_json::Value>().await?;
                let pdf_base64 = response_json["pdf_base64"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing pdf_base64 in response"))?;

                BASE64.decode(pdf_base64).map_err(|e| anyhow!(e))
            }
            PdfTransport::Orare { binary_path, features } => {
                let output_dir = PathBuf::from(binary_path);
                
                // Try different possible target directories
                let possible_target_dirs = [
                    output_dir.join("rust-local-target/debug"),
                    output_dir.join("target/debug"),
                    output_dir.parent().unwrap().join("rust-local-target/debug"),
                    output_dir.parent().unwrap().join("target/debug"),
                ];

                // Find the binary
                let binary_path = possible_target_dirs
                    .iter()
                    .find_map(|dir| {
                        let binary = dir.join("doc_renderer");
                        if binary.exists() {
                            event!(Level::INFO, "Found binary at: {}", binary.display());
                            Some(binary)
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| {
                        event!(Level::ERROR, "Binary not found in any of these locations:");
                        for dir in &possible_target_dirs {
                            event!(Level::ERROR, "  - {}", dir.join("doc_renderer").display());
                        }
                        anyhow!("Binary not found. Please build doc_renderer first.")
                    })?;

                let payload = json!({
                    "html": html,
                    "pdf_options": pdf_options,
                });

                let payload_str = serde_json::to_string(&payload)?;
                event!(Level::INFO, "Executing binary with payload length: {}", payload_str.len());

                let output = Command::new(&binary_path)
                    .current_dir(&output_dir)
                    .arg(&payload_str)
                    .output()
                    .map_err(|e| {
                        event!(Level::ERROR, "Failed to execute command: {}", e);
                        anyhow!("Failed to execute command: {}", e)
                    })?;

                if !output.status.success() {
                    let error = String::from_utf8_lossy(&output.stderr);
                    event!(Level::ERROR, "Binary execution failed: {}", error);
                    return Err(anyhow!("Orare PDF renderer failed: {}", error));
                }

                let stdout = String::from_utf8(output.stdout)?;
                event!(Level::DEBUG, "Orare raw output: {}", stdout);
                
                if let Some(json_start) = stdout.rfind("{\"Ok\":{") {
                    let json_str = &stdout[json_start..];
                    let response: serde_json::Value = serde_json::from_str(json_str)?;
                    let pdf_base64 = response["Ok"]["pdf_base64"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing pdf_base64 in response"))?;

                    BASE64.decode(pdf_base64).map_err(|e| anyhow!(e))
                } else {
                    Err(anyhow!("Could not find JSON response in output"))
                }
            }
            PdfTransport::Inplace => {
                event!(Level::INFO, "Using Inplace transport for PDF rendering");
                let result = sequent_core::services::pdf::html_to_pdf(html, pdf_options)
                    .map_err(|e| {
                        event!(Level::ERROR, "html_to_pdf failed: {}", e);
                        anyhow!("Inplace PDF rendering failed: {}", e)
                    })?;

                if !result.starts_with(b"%PDF") {
                    event!(Level::WARN, "Result is not a valid PDF, checking fallback");
                    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let fallback_path = format!("/tmp/output/fallback_{}.pdf", timestamp);
                    
                    if let Ok(fallback_content) = fs::read(&fallback_path) {
                        if fallback_content.starts_with(b"%PDF") {
                            event!(Level::INFO, "Using fallback PDF from: {}", fallback_path);
                            return Ok(fallback_content);
                        }
                    }
                    event!(Level::ERROR, "No valid PDF found in fallback");
                }
                
                Ok(result)
            }
            PdfTransport::Console => {
                event!(Level::INFO, "PdfTransport::Console: Would render PDF");
                Ok(vec![0x25, 0x50, 0x44, 0x46])
            }
        }
    }
}
