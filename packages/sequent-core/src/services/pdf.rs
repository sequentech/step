// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
pub use headless_chrome::types::{PrintToPdfOptions, TransferMode};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use reqwest;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, path::PathBuf, process::Command};
use tempfile::tempdir;
use tracing::{debug, event, info, instrument, Level};

#[derive(PartialEq)]
pub enum PdfTransport {
    AWSLambda { endpoint: String },
    OpenWhisk { endpoint: String },
    InPlace,
}

pub struct PdfRenderer {
    pub transport: PdfTransport,
}

impl PdfRenderer {
    #[instrument(skip(html), err)]
    pub async fn render_pdf(
        html: String,
        pdf_options: Option<PrintToPdfOptions>,
    ) -> Result<Vec<u8>> {
        Ok(PdfRenderer::new()?.do_render_pdf(html, pdf_options).await?)
    }

    #[instrument(err)]
    pub fn new() -> Result<Self> {
        event!(Level::INFO, "PdfRenderer::new() - Starting initialization");

        let doc_renderer_backend = match std::env::var("DOC_RENDERER_BACKEND") {
            Ok(name) => {
                event!(Level::INFO, "Found DOC_RENDERER_BACKEND: {}", name);
                name
            }
            Err(e) => {
                event!(
                    Level::ERROR,
                    "Failed to get DOC_RENDERER_BACKEND: {}; defaulting to inplace",
                    e
                );
                "inplace".to_string()
            }
        };

        let transport = match doc_renderer_backend.as_str() {
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
            "inplace" => PdfTransport::InPlace,
            transport => return Err(anyhow!("Unknown Doc renderer backend: {}", transport)),
        };

        Ok(PdfRenderer { transport })
    }

    #[instrument(skip(self, html), err)]
    pub async fn do_render_pdf(
        &self,
        html: String,
        pdf_options: Option<PrintToPdfOptions>,
    ) -> Result<Vec<u8>> {
        match &self.transport {
            PdfTransport::AWSLambda { endpoint }
            | PdfTransport::OpenWhisk { endpoint } => {
                if (PdfTransport::AWSLambda {
                    endpoint: endpoint.to_string(),
                }) == self.transport
                {
                    event!(
                        Level::INFO,
                        "Using AWS Lambda endpoint: {}",
                        endpoint
                    );
                } else {
                    event!(
                        Level::INFO,
                        "Using OpenWhisk endpoint: {}",
                        endpoint
                    );
                }
                let client = reqwest::Client::new();
                let payload = json!({
                    "html": html,
                    "pdf_options": pdf_options,
                });

                let response =
                    client.post(endpoint).json(&payload).send().await?;

                if !response.status().is_success() {
                    let error = response.text().await?;
                    if (PdfTransport::AWSLambda {
                        endpoint: endpoint.to_string(),
                    }) == self.transport
                    {
                        event!(
                            Level::ERROR,
                            "AWS Lambda request failed: {}",
                            error
                        );
                        return Err(anyhow!(
                            "AWS Lambda request failed: {}",
                            error
                        ));
                    } else {
                        event!(
                            Level::ERROR,
                            "OpenWhisk request failed: {}",
                            error
                        );
                        return Err(anyhow!(
                            "OpenWhisk request failed: {}",
                            error
                        ));
                    }
                }

                let response_json =
                    response.json::<serde_json::Value>().await?;
                let pdf_base64 = response_json["pdf_base64"]
                    .as_str()
                    .ok_or_else(|| anyhow!("Missing pdf_base64 in response"))?;

                BASE64.decode(pdf_base64).map_err(|e| anyhow!(e))
            }
            PdfTransport::InPlace => {
                event!(Level::INFO, "Using InPlace backend for PDF rendering");
                let result = html_to_pdf(html, pdf_options).map_err(|e| {
                    event!(Level::ERROR, "html_to_pdf failed: {}", e);
                    anyhow!("Inplace PDF rendering failed: {}", e)
                })?;

                if !result.starts_with(b"%PDF") {
                    event!(
                        Level::WARN,
                        "Result is not a valid PDF, checking fallback"
                    );
                    let timestamp =
                        chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let fallback_path =
                        format!("/tmp/output/fallback_{}.pdf", timestamp);

                    if let Ok(fallback_content) = fs::read(&fallback_path) {
                        if fallback_content.starts_with(b"%PDF") {
                            event!(
                                Level::INFO,
                                "Using fallback PDF from: {}",
                                fallback_path
                            );
                            return Ok(fallback_content);
                        }
                    }
                    event!(Level::ERROR, "No valid PDF found in fallback");
                }

                Ok(result)
            }
        }
    }
}

#[instrument(skip_all, err)]
pub fn html_to_pdf(
    html: String,
    options: Option<PrintToPdfOptions>,
) -> Result<Vec<u8>> {
    // Create temp html file
    let dir = tempdir()?;
    let file_path = dir.path().join("index.html");
    let mut file = File::create(file_path.clone())?;
    let file_path_str = file_path.to_str().unwrap();
    file.write_all(html.as_bytes())?;
    let url_path = format!("file://{}", file_path_str);

    info!("html_to_pdf: {url_path}");
    debug!("options: {options:#?}");

    let pdf_options = options.unwrap_or_else(|| PrintToPdfOptions {
        landscape: None,
        display_header_footer: None,
        print_background: Some(true),
        scale: None,
        paper_width: None,
        paper_height: None,
        margin_top: None,
        margin_bottom: None,
        margin_left: None,
        margin_right: None,
        page_ranges: None,
        ignore_invalid_page_ranges: None,
        header_template: None,
        footer_template: None,
        prefer_css_page_size: None,
        transfer_mode: None,
    });

    print_to_pdf(url_path.as_str(), pdf_options, None)
}

#[instrument(skip_all, err)]
fn print_to_pdf(
    file_path: &str,
    pdf_options: PrintToPdfOptions,
    wait: Option<Duration>,
) -> Result<Vec<u8>> {
    let options = LaunchOptionsBuilder::default()
        .sandbox(false)
        // <WTF> Why? well this:
        // https://github.com/rust-headless-chrome/rust-headless-chrome/issues/500
        .devtools(false)
        .headless(true)
        // </WTF>
        .args(vec![
            std::ffi::OsStr::new("--disable-setuid-sandbox"),
            std::ffi::OsStr::new("--disable-dev-shm-usage"),
            std::ffi::OsStr::new("--single-process"),
            std::ffi::OsStr::new("--no-zygote"),
        ])
        .build()
        .expect("Default should not panic");

    let browser =
        Browser::new(options).with_context(|| "Error obtaining the browser")?;

    let tab = browser.new_tab()?;

    tab.navigate_to(file_path)?
        .wait_until_navigated()
        .with_context(|| "Error navigating to file")?;

    debug!("Sleeping {wait:#?}..");
    if let Some(wait) = wait {
        sleep(wait);
    }
    debug!("Awake! After {wait:#?}");

    let bytes = tab
        .print_to_pdf(Some(pdf_options))
        .with_context(|| "Error printing to pdf")?;

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, OpenOptions},
        path::Path,
    };

    use super::*;
    use anyhow::Result;

    #[test]
    fn test_pdf_generation() -> Result<()> {
        let bytes = html_to_pdf(
            "<body><h1>Hello, world!</h1></body>".to_string(),
            None,
        )
        .unwrap();

        let file_path = Path::new("./res.pdf");
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&file_path)?;

        file.write_all(&bytes)?;

        assert!(bytes.len() > 0);
        assert!(file_path.exists());

        fs::remove_file(file_path)?;

        Ok(())
    }
}
