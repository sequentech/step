// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "s3")]
use crate::services::s3;
use crate::util::convert_vec::IntoVec;

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
use tokio::runtime::Runtime;
use tracing::{debug, event, info, instrument, Level};

#[derive(PartialEq)]
pub enum DocRendererBackend {
    AWSLambda,
    OpenWhisk,
    InPlace,
}

pub fn doc_renderer_backend() -> DocRendererBackend {
    match std::env::var("DOC_RENDERER_BACKEND").as_deref() {
        Ok("aws_lambda") => {
            event!(Level::INFO, "Using AWS Lambda doc renderer backend",);
            DocRendererBackend::AWSLambda
        }
        Ok("openwhisk") => {
            event!(Level::INFO, "Using Openwhisk doc renderer backend",);
            DocRendererBackend::OpenWhisk
        }
        Ok("inplace") => {
            event!(Level::INFO, "Using inplace doc renderer backend",);
            DocRendererBackend::InPlace
        }
        Ok(unknown_backend) => {
            event!(
                Level::WARN,
                "Unknown backend {:?} specified in the DOC_RENDERER_BACKEND envvar, defaulting to inplace",
                unknown_backend
            );
            DocRendererBackend::InPlace
        }
        Err(_) => {
            event!(
                Level::WARN,
                "Missing DOC_RENDERER_BACKEND envvar, defaulting to inplace",
            );
            DocRendererBackend::InPlace
        }
    }
}

#[derive(PartialEq)]
pub enum PdfTransport {
    AWSLambda {
        endpoint: String,
    },
    OpenWhisk {
        endpoint: String,
        basic_auth: Option<String>,
    },
    InPlace,
}

pub struct PdfRenderer {
    pub transport: PdfTransport,
}

pub mod sync {
    use super::*;

    pub struct PdfRenderer {
        pub transport: PdfTransport,
    }

    impl PdfRenderer {
        pub fn render_pdf(
            html: String,
            pdf_options: Option<PrintToPdfOptions>,
        ) -> Result<Vec<u8>> {
            let html_sha256 = sha256::digest(&html);
            Ok(PdfRenderer::new()?.do_render_pdf(html, pdf_options)?)
        }

        pub fn new() -> Result<Self> {
            event!(Level::INFO, "PdfRenderer::new() - Starting initialization");

            let doc_renderer_backend = match std::env::var(
                "DOC_RENDERER_BACKEND",
            ) {
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
                        endpoint: std::env::var("AWS_LAMBDA_DOC_RENDERER_ENDPOINT")
                            .map_err(|_| anyhow!("Please, set AWS_LAMBDA_DOC_RENDERER_ENDPOINT pointing to the doc-renderer AWS lambda endpoint"))?
                    }
                },
                "openwhisk" => {
                    let mut openwhisk_endpoint = std::env::var("OPENWHISK_DOC_RENDERER_ENDPOINT");
                    if !openwhisk_endpoint.is_ok() {
                        let openwhisk_api_host = std::env::var("OPENWHISK_API_HOST");
                        if let Ok(openwhisk_api_host) = openwhisk_api_host {
                            openwhisk_endpoint = Ok(
                                format!(
                                    "{openwhisk_api_host}/api/v1/namespaces/_/actions/pdf-tools/doc_renderer?blocking=true&result=true"
                                )
                            );
                        } else {
                            return Err(
                                anyhow!("Please, set OPENWHISK_API_HOST pointing to the OpenWhisk API host and port (http://<ip>:<port>)")
                            )
                        }

                    }

                    PdfTransport::OpenWhisk {
                        endpoint: openwhisk_endpoint?,
                        basic_auth: std::env::var("OPENWHISK_BASIC_AUTH").ok(),
                    }
                },
                "inplace" => PdfTransport::InPlace,
                transport => return Err(anyhow!("Unknown Doc renderer backend: {}", transport)),
            };

            Ok(PdfRenderer { transport })
        }

        pub fn do_render_pdf(
            &self,
            html: String,
            pdf_options: Option<PrintToPdfOptions>,
        ) -> Result<Vec<u8>> {
            let (endpoint, basic_auth) = match &self.transport {
                PdfTransport::AWSLambda { endpoint } => {
                    (endpoint.clone(), None)
                }
                PdfTransport::OpenWhisk {
                    endpoint,
                    basic_auth,
                } => (endpoint.clone(), basic_auth.clone()),
                PdfTransport::InPlace => (String::new(), None),
            };

            match &self.transport {
                PdfTransport::AWSLambda { .. }
                | PdfTransport::OpenWhisk { .. } => {
                    let payload = if (PdfTransport::AWSLambda {
                        endpoint: endpoint.to_string(),
                    }) == self.transport
                    {
                        event!(
                            Level::INFO,
                            "Using AWS Lambda endpoint: {}",
                            endpoint
                        );
                        let html_sha256 = sha256::digest(&html);
                        let output_filename = format!("output-{}", html_sha256);

                        #[cfg(feature = "s3")]
                        {
                            let rt = Runtime::new()?;
                            rt.block_on(async {
                                s3::upload_data_to_s3(
                                    html.clone().into_bytes().into(),
                                    s3_bucket_path(format!("input-{}", html_sha256)).ok_or_else(|| anyhow!("missing bucket path"))?,
                                    false,
                                    s3_private_bucket().ok_or_else(|| anyhow!("missing bucket"))?,
                                    "text/plain".to_string(),
                                    None,
                                )
                                    .await
                                    .map_err(|err| {
                                        anyhow!(
                                            "error uploading input document to S3: {:?}",
                                            err
                                        )
                                    })
                            })?;
                        }
                        json!({
                            "s3": {
                                "bucket": s3_private_bucket().ok_or_else(|| anyhow!("missing bucket"))?,
                                "input_path": format!("input-{}", html_sha256),
                                "output_path": output_filename,
                                "pdf_options": pdf_options,
                            }
                        })
                    } else {
                        event!(
                            Level::INFO,
                            "Using OpenWhisk endpoint: {}",
                            endpoint
                        );
                        json!({
                            "raw": {
                                "html": html,
                                "pdf_options": pdf_options,
                            }
                        })
                    };
                    let client = reqwest::blocking::Client::new();
                    let mut request_builder =
                        client.post(endpoint.clone()).json(&payload);
                    if let Some(basic_auth) = basic_auth {
                        let basic_auth: Vec<&str> =
                            basic_auth.split(":").collect();
                        if basic_auth.len() != 2 {
                            return Err(anyhow!("Invalid basic auth provided"));
                        }
                        request_builder = request_builder
                            .basic_auth(basic_auth[0], Some(basic_auth[1]))
                    }
                    let response = request_builder.send()?;

                    if !response.status().is_success() {
                        let error = response.text()?;
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

                    match &self.transport {
                        PdfTransport::AWSLambda { .. } => {
                            let html_sha256 = sha256::digest(&html);
                            let output_filename =
                                format!("output-{}", html_sha256);
                            let rt = Runtime::new()?;
                            if cfg!(feature = "s3") {
                                rt.block_on(async {
                                    get_file_from_s3(
                                        s3_private_bucket().ok_or_else(
                                            || anyhow!("missing bucket"),
                                        )?,
                                        output_filename,
                                    )
                                    .await
                                })
                            } else {
                                return Err(anyhow!("cannot read result from s3 as this component was built without s3 support"));
                            }
                        }
                        PdfTransport::OpenWhisk { .. } => {
                            let response_json =
                                response.json::<serde_json::Value>()?;

                            let pdf_base64 = response_json["pdf_base64"]
                                .as_str()
                                .ok_or_else(|| {
                                    anyhow!("Missing pdf_base64 in response")
                                })?;

                            BASE64.decode(pdf_base64).map_err(|e| anyhow!(e))
                        }
                        _ => {
                            unreachable!()
                        }
                    }
                }
                PdfTransport::InPlace => {
                    event!(
                        Level::INFO,
                        "Using InPlace backend for PDF rendering"
                    );
                    let result =
                        html_to_pdf(html, pdf_options).map_err(|e| {
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
}

cfg_if::cfg_if! {
    if #[cfg(feature = "s3")] {
        fn s3_private_bucket() -> Option<String> {
            s3::get_private_bucket().ok()
        }
        fn s3_bucket_path(path: String) -> Option<String> {
            Some(path)
        }
        async fn get_file_from_s3(bucket: String, output_filename: String) -> Result<Vec<u8>> {
            s3::get_file_from_s3(bucket, output_filename)
                .await
                .map_err(|err| {
                    anyhow!(
                        "could not retrieve file from S3: {:?}",
                        err
                    )
                })
        }
    } else {
        fn s3_private_bucket() -> Option<String> {
            None
        }
        fn s3_bucket_path(path: String) -> Option<String> {
            None
        }
        async fn get_file_from_s3(bucket: String, output_filename: String) -> Result<Vec<u8>> {
            unimplemented!()
        }
    }
}

impl PdfRenderer {
    pub async fn render_pdf(
        html: String,
        pdf_options: Option<PrintToPdfOptions>,
    ) -> Result<Vec<u8>> {
        Ok(PdfRenderer::new()?.do_render_pdf(html, pdf_options).await?)
    }

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
                    endpoint: std::env::var("AWS_LAMBDA_DOC_RENDERER_ENDPOINT")
                        .map_err(|_| anyhow!("Please, set AWS_LAMBDA_DOC_RENDERER_ENDPOINT pointing to the doc-renderer AWS lambda endpoint"))?
                }
            },
            "openwhisk" => {
                let mut openwhisk_endpoint = std::env::var("OPENWHISK_DOC_RENDERER_ENDPOINT");
                if !openwhisk_endpoint.is_ok() {
                    let openwhisk_api_host = std::env::var("OPENWHISK_API_HOST");
                    if let Ok(openwhisk_api_host) = openwhisk_api_host {
                        openwhisk_endpoint = Ok(
                            format!(
                                "{openwhisk_api_host}/api/v1/namespaces/_/actions/pdf-tools/doc_renderer?blocking=true&result=true"
                            )
                        );
                    } else {
                        return Err(
                            anyhow!("Please, set OPENWHISK_API_HOST pointing to the OpenWhisk API host and port (http://<ip>:<port>)")
                        )
                    }

                }

                PdfTransport::OpenWhisk {
                    endpoint: openwhisk_endpoint?,
                    basic_auth: std::env::var("OPENWHISK_BASIC_AUTH").ok(),
                }
            },
            "inplace" => PdfTransport::InPlace,
            transport => return Err(anyhow!("Unknown Doc renderer backend: {}", transport)),
        };

        Ok(PdfRenderer { transport })
    }

    pub async fn do_render_pdf(
        &self,
        html: String,
        pdf_options: Option<PrintToPdfOptions>,
    ) -> Result<Vec<u8>> {
        let (endpoint, basic_auth) = match &self.transport {
            PdfTransport::AWSLambda { endpoint } => (endpoint.clone(), None),
            PdfTransport::OpenWhisk {
                endpoint,
                basic_auth,
            } => (endpoint.clone(), basic_auth.clone()),
            PdfTransport::InPlace => (String::new(), None),
        };

        match &self.transport {
            PdfTransport::AWSLambda { .. } | PdfTransport::OpenWhisk { .. } => {
                let payload = if (PdfTransport::AWSLambda {
                    endpoint: endpoint.to_string(),
                }) == self.transport
                {
                    event!(
                        Level::INFO,
                        "Using AWS Lambda endpoint: {}",
                        endpoint
                    );
                    let html_sha256 = sha256::digest(&html);
                    let input_filename = format!("input-{}", html_sha256);
                    let output_filename = format!("output-{}", html_sha256);

                    #[cfg(feature = "s3")]
                    {
                        s3::upload_data_to_s3(
                            html.clone().into_bytes().into(),
                            input_filename.clone(),
                            false,
                            s3_private_bucket()
                                .ok_or_else(|| anyhow!("missing bucket"))?,
                            "text/plain".to_string(),
                            None,
                        )
                        .await
                        .map_err(|err| {
                            anyhow!(
                                "error uploading input document to S3: {:?}",
                                err
                            )
                        })?;
                    }

                    json!({
                        "s3": {
                            "bucket": s3_private_bucket().ok_or_else(|| anyhow!("missing bucket"))?,
                            "input_path": input_filename,
                            "output_path": output_filename,
                            "pdf_options": pdf_options,
                        }
                    })
                } else {
                    event!(
                        Level::INFO,
                        "Using OpenWhisk endpoint: {}",
                        endpoint
                    );
                    json!({
                        "raw": {
                            "html": html,
                            "pdf_options": pdf_options,
                        }
                    })
                };
                let client = reqwest::Client::new();

                let mut request_builder =
                    client.post(endpoint.clone()).json(&payload);
                if let Some(basic_auth) = basic_auth {
                    let basic_auth: Vec<&str> = basic_auth.split(":").collect();
                    if basic_auth.len() != 2 {
                        return Err(anyhow!("Invalid basic auth provided"));
                    }
                    request_builder = request_builder
                        .basic_auth(basic_auth[0], Some(basic_auth[1]))
                }
                let response = request_builder.send().await?;

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

                match &self.transport {
                    PdfTransport::AWSLambda { .. } => {
                        let html_sha256 = sha256::digest(&html);
                        let output_filename = format!("output-{}", html_sha256);
                        get_file_from_s3(
                            s3_private_bucket()
                                .ok_or_else(|| anyhow!("missing bucket"))?,
                            output_filename,
                        )
                        .await
                    }
                    PdfTransport::OpenWhisk { .. } => {
                        let response_json =
                            response.json::<serde_json::Value>().await?;

                        let pdf_base64 =
                            response_json["pdf_base64"].as_str().ok_or_else(
                                || anyhow!("Missing pdf_base64 in response"),
                            )?;

                        BASE64.decode(pdf_base64).map_err(|e| anyhow!(e))
                    }
                    _ => {
                        unreachable!()
                    }
                }
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
        .enable_logging(true)
        .idle_browser_timeout(Duration::from_secs(99999999))
        .args(vec![
            std::ffi::OsStr::new("--disable-setuid-sandbox"),
            std::ffi::OsStr::new("--disable-dev-shm-usage"),
            std::ffi::OsStr::new("--single-process"),
            std::ffi::OsStr::new("--no-zygote"),
        ])
        .build()
        .expect("Default should not panic");

    info!("1. Opening browser");
    let browser =
        Browser::new(options).with_context(|| "Error obtaining the browser")?;

    info!("2. Opening tab");
    let tab = browser.new_tab()?;

    tab.set_default_timeout(Duration::from_secs(99999999));
    info!("3. Navigating to tab");
    tab.navigate_to(file_path)?
        .wait_until_navigated()
        .with_context(|| "Error navigating to file")?;

    debug!("Sleeping {wait:#?}..");
    if let Some(wait) = wait {
        sleep(wait);
    }
    debug!("Awake! After {wait:#?}");
    info!("4. printing");

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
