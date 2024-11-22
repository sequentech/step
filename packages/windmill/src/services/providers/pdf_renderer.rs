// SPDX-FileCopyrightText: 2024 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use headless_chrome::types::PrintToPdfOptions;
use serde_json::json;
use std::{fs, path::PathBuf, process::Command};
use tracing::{event, instrument, Level};

pub enum PdfTransport {
    Orare {
        binary_path: String,
        features: Vec<String>,
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
            }
            Err(e) => {
                event!(Level::ERROR, "Failed to get PDF_TRANSPORT_NAME: {}", e);
                return Err(anyhow!("PDF_TRANSPORT_NAME env var missing"));
            }
        };

        event!(Level::INFO, "PdfTransport: {pdf_transport_name}");

        let transport = match pdf_transport_name.as_str() {
            "orare-openwhisk" => {
                let binary_path = std::env::var("PDF_LAMBDA_BINARY_PATH")
                    .map_err(|_err| anyhow!("PDF_LAMBDA_BINARY_PATH env var missing"))?;
                PdfTransport::Orare {
                    binary_path,
                    features: vec!["openwhisk".to_string()],
                }
            }
            "orare-openwhisk-dev" => {
                let binary_path = std::env::var("PDF_LAMBDA_BINARY_PATH")
                    .map_err(|_err| anyhow!("PDF_LAMBDA_BINARY_PATH env var missing"))?;
                PdfTransport::Orare {
                    binary_path,
                    features: vec!["openwhisk-dev".to_string()],
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
            PdfTransport::Orare {
                binary_path,
                features,
            } => {
                let output_dir = PathBuf::from(binary_path);
                event!(
                    Level::INFO,
                    "Building orare in directory: {}",
                    output_dir.display()
                );

                let build_output = Command::new("cargo")
                    .current_dir(&output_dir)
                    .arg("build")
                    .arg("--features")
                    .arg(&features.join(","))
                    .output()?;

                if !build_output.status.success() {
                    let error = String::from_utf8_lossy(&build_output.stderr);
                    event!(Level::ERROR, "Build failed: {}", error);
                    return Err(anyhow!("Failed to build doc_renderer: {}", error));
                }

                // Try different possible target directories
                let possible_target_dirs = [
                    output_dir.join("rust-local-target/debug"),
                    output_dir.join("target/debug"),
                    output_dir.parent().unwrap().join("rust-local-target/debug"),
                    output_dir.parent().unwrap().join("target/debug"),
                    PathBuf::from(
                        "/workspaces/step/packages/orare/doc_renderer/rust-local-target/debug",
                    ),
                    PathBuf::from("/workspaces/step/packages/orare/doc_renderer/target/debug"),
                ];

                for dir in &possible_target_dirs {
                    event!(Level::INFO, "Checking target dir: {}", dir.display());
                    if dir.exists() {
                        event!(Level::INFO, "Found target dir: {}", dir.display());
                        let binary_path = dir.join("doc_renderer");
                        if binary_path.exists() {
                            event!(Level::INFO, "Found binary at: {}", binary_path.display());

                            // Check binary permissions and type
                            let metadata = fs::metadata(&binary_path).map_err(|e| {
                                event!(Level::ERROR, "Failed to get binary metadata: {}", e);
                                anyhow!("Failed to get binary metadata: {}", e)
                            })?;

                            event!(Level::INFO, "Binary metadata:");
                            event!(Level::INFO, "  - Is file: {}", metadata.is_file());
                            event!(Level::INFO, "  - Size: {} bytes", metadata.len());

                            let payload = json!({
                                "html": html,
                                "pdf_options": pdf_options,
                            });

                            let payload_str = serde_json::to_string(&payload)?;
                            event!(Level::INFO, "Executing binary with command:");
                            event!(Level::INFO, "  - Binary: {}", binary_path.display());
                            event!(Level::INFO, "  - Working dir: {}", output_dir.display());
                            event!(Level::INFO, "  - Payload length: {}", payload_str.len());

                            // Try to execute with absolute path
                            let absolute_binary_path =
                                fs::canonicalize(&binary_path).map_err(|e| {
                                    event!(Level::ERROR, "Failed to get absolute path: {}", e);
                                    anyhow!("Failed to get absolute path: {}", e)
                                })?;

                            event!(
                                Level::INFO,
                                "Absolute binary path: {}",
                                absolute_binary_path.display()
                            );

                            let output = Command::new(&absolute_binary_path)
                                .current_dir(&output_dir)
                                .arg(&payload_str)
                                .output()
                                .map_err(|e| {
                                    event!(Level::ERROR, "Failed to execute command: {}", e);
                                    event!(
                                        Level::ERROR,
                                        "Current dir exists: {}",
                                        output_dir.exists()
                                    );
                                    event!(Level::ERROR, "Binary exists: {}", binary_path.exists());
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

                                return BASE64.decode(pdf_base64).map_err(|e| anyhow!(e));
                            }
                            return Err(anyhow!("Could not find JSON response in output"));
                        }
                    }
                }

                event!(
                    Level::ERROR,
                    "No valid target directory found after checking:"
                );
                for dir in &possible_target_dirs {
                    event!(Level::ERROR, "  - {}", dir.display());
                }
                Err(anyhow!("Could not find target directory"))
            }
            PdfTransport::Inplace => {
                event!(Level::INFO, "Using Inplace transport for PDF rendering");
                let result =
                    sequent_core::services::pdf::html_to_pdf(html, pdf_options).map_err(|e| {
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
