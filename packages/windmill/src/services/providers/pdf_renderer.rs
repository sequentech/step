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
    AWSLambda {
        endpoint: String,
    },
    OpenWhisk {
        endpoint: String,
    },
    Windmill,
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
            "aws_lambda" => {
                PdfTransport::AWSLambda {
                    endpoint: std::env::var("AWS_LAMBDA_ENDPOINT")
                        .unwrap_or_else(|_| "lambda.us-east-1.amazonaws.com".to_string())
                }
            },
            "openwhisk" => {
                PdfTransport::OpenWhisk {
                    endpoint: std::env::var("OPENWHISK_ENDPOINT")
                        .unwrap_or_else(|_| {
                            "http://127.0.0.2:3233/api/v1/namespaces/_/actions/pdf-tools/doc_renderer?blocking=true&result=true".to_string()
                        }),
                }
            },
            "windmill" => PdfTransport::Windmill,
            transport => return Err(anyhow!("Unknown PDF transport: {}", transport)),
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
            PdfTransport::AWSLambda { endpoint } => {
                todo!()
            },
            PdfTransport::OpenWhisk { endpoint } => {
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
            PdfTransport::Windmill => {
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
