// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "s3")]
use crate::services::s3;
use crate::util::convert_vec::IntoVec;
use crate::util::retry::retry_with_exponential_backoff;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
pub use headless_chrome::types::{PrintToPdfOptions, TransferMode};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use reqwest;
use serde_json::json;
use sha256;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;
use tokio::runtime::Runtime;
use tracing::{debug, error, event, info, instrument, warn, Level};

#[derive(PartialEq)]
pub enum DocRendererBackend {
    AWSLambda,
    OpenWhisk,
    InPlace,
}

pub fn doc_renderer_backend() -> DocRendererBackend {
    match std::env::var("DOC_RENDERER_BACKEND").as_deref() {
        Ok("aws_lambda") => {
            info!("Using AWS Lambda doc renderer backend");
            DocRendererBackend::AWSLambda
        }
        Ok("openwhisk") => {
            info!("Using OpenWhisk doc renderer backend");
            DocRendererBackend::OpenWhisk
        }
        Ok("inplace") => {
            info!("Using InPlace doc renderer backend");
            DocRendererBackend::InPlace
        }
        Ok(unknown_backend) => {
            warn!("Unknown backend {:?} specified in the DOC_RENDERER_BACKEND envvar, defaulting to InPlace", unknown_backend);
            DocRendererBackend::InPlace
        }
        Err(_) => {
            warn!("Missing DOC_RENDERER_BACKEND envvar, defaulting to InPlace");
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

/// --- SYNC VERSION ---
pub mod sync {
    use super::*;
    use std::thread;
    use std::time::Duration;

    pub struct PdfRenderer {
        pub transport: PdfTransport,
    }

    impl PdfRenderer {
        pub fn render_pdf(
            html: String,
            pdf_options: Option<PrintToPdfOptions>,
        ) -> Result<Vec<u8>> {
            let _html_sha256 = sha256::digest(&html);
            // We call our synchronous do_render_pdf
            Ok(PdfRenderer::new()?.do_render_pdf(html, pdf_options)?)
        }

        pub fn new() -> Result<Self> {
            info!("PdfRenderer::new() [sync] - Starting initialization");

            let doc_renderer_backend = match std::env::var(
                "DOC_RENDERER_BACKEND",
            ) {
                Ok(name) => {
                    info!("Found DOC_RENDERER_BACKEND: {name:?}");
                    name
                }
                Err(e) => {
                    error!("Failed to get DOC_RENDERER_BACKEND: {e:?}; defaulting to InPlace");
                    "inplace".to_string()
                }
            };

            let transport = match doc_renderer_backend.as_str() {
                "aws_lambda" => {
                    PdfTransport::AWSLambda {
                        endpoint: std::env::var("AWS_LAMBDA_DOC_RENDERER_ENDPOINT")
                            .map_err(|_| anyhow!("Please, set AWS_LAMBDA_DOC_RENDERER_ENDPOINT pointing to the doc-renderer AWS lambda endpoint"))?
                    }
                }
                "openwhisk" => {
                    let mut openwhisk_endpoint = std::env::var("OPENWHISK_DOC_RENDERER_ENDPOINT");
                    if !openwhisk_endpoint.is_ok() {
                        let openwhisk_api_host = std::env::var("OPENWHISK_API_HOST");
                        if let Ok(host) = openwhisk_api_host {
                            openwhisk_endpoint = Ok(format!("{host}/api/v1/namespaces/_/actions/pdf-tools/doc_renderer?blocking=true&result=true"));
                        } else {
                            return Err(anyhow!("Please, set OPENWHISK_API_HOST pointing to the OpenWhisk API host and port (http://<ip>:<port>)"))
                        }
                    }
                    PdfTransport::OpenWhisk {
                        endpoint: openwhisk_endpoint?,
                        basic_auth: std::env::var("OPENWHISK_BASIC_AUTH").ok(),
                    }
                }
                "inplace" => PdfTransport::InPlace,
                transport => return Err(anyhow!("Unknown Doc renderer backend: {transport:?}")),
            };

            Ok(PdfRenderer { transport })
        }

        /// Synchronous send_request using reqwest::blocking and our own retry
        /// loop.
        fn send_request(
            &self,
            endpoint: &str,
            payload: serde_json::Value,
            basic_auth: Option<String>,
        ) -> Result<reqwest::blocking::Response> {
            let client = reqwest::blocking::Client::builder()
                .pool_idle_timeout(None)
                .build()?;
            let mut retries = 3;
            let mut delay = Duration::from_millis(100);

            loop {
                let mut builder = client.post(endpoint.clone()).json(&payload);
                if let Some(ref basic_auth) = basic_auth {
                    let parts: Vec<&str> = basic_auth.split(':').collect();
                    if parts.len() != 2 {
                        return Err(anyhow!("Invalid basic auth provided"));
                    }
                    builder = builder.basic_auth(parts[0], Some(parts[1]));
                }

                match builder.send() {
                    Ok(response) => break Ok(response),
                    Err(e) => {
                        if retries == 1 {
                            break Err(anyhow!("error sending request: {e:?}"));
                        }
                        error!(
                            "Request failed: {e:?}. Retrying in {delay:?}..."
                        );
                        thread::sleep(delay);
                        delay *= 2;
                        retries -= 1;
                    }
                }
            }
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
                        endpoint: endpoint.clone(),
                    }) == self.transport
                    {
                        info!("Using AWS Lambda endpoint: {endpoint:?}");
                        let html_sha256 = sha256::digest(&html);
                        let input_filename = format!("input-{html_sha256:?}");
                        let output_filename = format!("output-{html_sha256:?}");

                        #[cfg(feature = "s3")]
                        {
                            let rt = Runtime::new()?;
                            rt.block_on(async {
                                s3::upload_data_to_s3(
                                    html.clone().into_bytes().into(),
                                    input_filename.clone(),
                                    false,
                                    s3_private_bucket().ok_or_else(|| anyhow!("missing bucket"))?,
                                    "text/plain".to_string(),
                                    None,
                                    None,
                                )
                                .await
                                .map_err(|err| {
                                    anyhow!("error uploading input document to S3: {err:?}")
                                })
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
                        info!("Using OpenWhisk endpoint: {endpoint:?}");
                        json!({
                            "raw": {
                                "html": html,
                                "pdf_options": pdf_options,
                            }
                        })
                    };

                    let response =
                        self.send_request(&endpoint, payload, basic_auth)?;

                    if !response.status().is_success() {
                        let error = response.text()?;
                        if (PdfTransport::AWSLambda {
                            endpoint: endpoint.clone(),
                        }) == self.transport
                        {
                            error!("AWS Lambda request failed: {error:?}");
                            return Err(anyhow!(
                                "AWS Lambda request failed: {error:?}"
                            ));
                        } else {
                            error!("OpenWhisk request failed: {error:?}");
                            return Err(anyhow!(
                                "OpenWhisk request failed: {error:?}"
                            ));
                        }
                    }

                    match &self.transport {
                        PdfTransport::AWSLambda { .. } => {
                            let html_sha256 = sha256::digest(&html);
                            let output_filename =
                                format!("output-{html_sha256:?}");
                            let rt = Runtime::new()?;
                            if cfg!(feature = "s3") {
                                rt.block_on(get_file_from_s3(
                                    s3_private_bucket().ok_or_else(|| {
                                        anyhow!("missing bucket")
                                    })?,
                                    output_filename,
                                ))
                            } else {
                                Err(anyhow!("cannot read result from s3 as this component was built without s3 support"))
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
                            BASE64
                                .decode(pdf_base64)
                                .map_err(|e| anyhow!("{e:?}"))
                        }
                        _ => unreachable!(),
                    }
                }
                PdfTransport::InPlace => {
                    info!("Using InPlace backend for PDF rendering");
                    let result =
                        html_to_pdf(html, pdf_options).map_err(|e| {
                            warn!("html_to_pdf failed: {e:?}");
                            anyhow!("InPlace PDF rendering failed: {e:?}")
                        })?;

                    if !result.starts_with(b"%PDF") {
                        warn!("Result is not a valid PDF, checking fallback");
                        let timestamp =
                            chrono::Local::now().format("%Y%m%d_%H%M%S");
                        let fallback_path =
                            format!("/tmp/output/fallback_{timestamp:?}.pdf");

                        if let Ok(fallback_content) = fs::read(&fallback_path) {
                            if fallback_content.starts_with(b"%PDF") {
                                info!("Using fallback PDF from: {fallback_path:?}");
                                return Ok(fallback_content);
                            }
                        }
                        error!("No valid PDF found in fallback");
                    }

                    Ok(result)
                }
            }
        }
    }
}

/// --- ASYNC VERSION ---
impl PdfRenderer {
    /// Public async render_pdf that preserves the async signature.
    pub async fn render_pdf(
        html: String,
        pdf_options: Option<PrintToPdfOptions>,
    ) -> Result<Vec<u8>> {
        Ok(PdfRenderer::new()?.do_render_pdf(html, pdf_options).await?)
    }

    /// Creates a new PdfRenderer based on environment configuration.
    pub fn new() -> Result<Self> {
        info!("PdfRenderer::new() [async] - Starting initialization");

        let doc_renderer_backend = match std::env::var("DOC_RENDERER_BACKEND") {
            Ok(name) => {
                info!("Found DOC_RENDERER_BACKEND: {name:?}");
                name
            }
            Err(e) => {
                error!("Failed to get DOC_RENDERER_BACKEND: {e:?}; defaulting to InPlace");
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
                    if let Ok(host) = openwhisk_api_host {
                        openwhisk_endpoint = Ok(format!("{host}/api/v1/namespaces/_/actions/pdf-tools/doc_renderer?blocking=true&result=true"));
                    } else {
                        return Err(anyhow!("Please, set OPENWHISK_API_HOST pointing to the OpenWhisk API host and port (http://<ip>:<port>)"))
                    }
                }

                PdfTransport::OpenWhisk {
                    endpoint: openwhisk_endpoint?,
                    basic_auth: std::env::var("OPENWHISK_BASIC_AUTH").ok(),
                }
            },
            "inplace" => PdfTransport::InPlace,
            transport => return Err(anyhow!("Unknown Doc renderer backend: {transport:?}")),
        };

        Ok(PdfRenderer { transport })
    }

    /// Async do_render_pdf uses retry_with_exponential_backoff for the HTTP
    /// request.
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
                    endpoint: endpoint.clone(),
                }) == self.transport
                {
                    info!("Using AWS Lambda endpoint: {endpoint:?}");
                    let html_sha256 = sha256::digest(&html);
                    let input_filename = format!("input-{html_sha256:?}");
                    let output_filename = format!("output-{html_sha256:?}");

                    #[cfg(feature = "s3")]
                    {
                        retry_with_exponential_backoff(
                            || async {
                                s3::upload_data_to_s3(
                                    html.clone().into_bytes().into(),
                                    input_filename.clone(),
                                    false,
                                    s3_private_bucket().ok_or_else(|| {
                                        anyhow!("missing bucket")
                                    })?,
                                    "text/plain".to_string(),
                                    None,
                                    None,
                                )
                                .await
                            },
                            3,
                            Duration::from_millis(100),
                        )
                        .await
                        .map_err(|err| {
                            anyhow!(
                                "error uploading input document to S3: {err:?}"
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
                        "Using OpenWhisk endpoint: {endpoint:?}"
                    );
                    json!({
                        "raw": {
                            "html": html,
                            "pdf_options": pdf_options,
                        }
                    })
                };

                let client = reqwest::Client::builder()
                    .pool_idle_timeout(None)
                    .build()?;
                let mut request_builder =
                    client.post(endpoint.clone()).json(&payload);
                if let Some(basic_auth) = basic_auth {
                    let parts: Vec<&str> = basic_auth.split(':').collect();
                    if parts.len() != 2 {
                        return Err(anyhow!("Invalid basic auth provided"));
                    }
                    request_builder =
                        request_builder.basic_auth(parts[0], Some(parts[1]));
                }

                let response = retry_with_exponential_backoff(
                    || async {
                        info!("Sending the request with client={client:#?}");
                        let output = request_builder
                            .try_clone()
                            .expect("failed to clone request builder")
                            .send()
                            .await;
                        info!("Request sent!");
                        output
                    },
                    3,
                    Duration::from_millis(100),
                )
                .await
                .map_err(|e| anyhow!("error sending async request: {e:?}"))?;

                if !response.status().is_success() {
                    let error = response.text().await.map_err(|e| {
                        anyhow!(
                            "error obtaining error text from request: {e:?}"
                        )
                    })?;

                    if (PdfTransport::AWSLambda {
                        endpoint: endpoint.clone(),
                    }) == self.transport
                    {
                        error!("AWS Lambda request failed: {error:?}");
                        return Err(anyhow!(
                            "AWS Lambda request failed: {error:?}"
                        ));
                    } else {
                        error!("OpenWhisk request failed: {error:?}");
                        return Err(anyhow!(
                            "OpenWhisk request failed: {error:?}"
                        ));
                    }
                }

                match &self.transport {
                    PdfTransport::AWSLambda { .. } => {
                        let html_sha256 = sha256::digest(&html);
                        let output_filename = format!("output-{html_sha256:?}");

                        retry_with_exponential_backoff(
                            || async {
                                get_file_from_s3(
                                    s3_private_bucket().ok_or_else(|| {
                                        anyhow!("missing bucket")
                                    })?,
                                    output_filename.clone(),
                                )
                                .await
                            },
                            3,
                            Duration::from_millis(100),
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
                        BASE64.decode(pdf_base64).map_err(|e| anyhow!("{e:?}"))
                    }
                    _ => unreachable!(),
                }
            }
            PdfTransport::InPlace => {
                info!("Using InPlace backend for PDF rendering");
                let result = html_to_pdf(html, pdf_options).map_err(|e| {
                    error!("html_to_pdf failed: {e:?}");
                    anyhow!("InPlace PDF rendering failed: {e:?}")
                })?;

                if !result.starts_with(b"%PDF") {
                    warn!("Result is not a valid PDF, checking fallback");
                    let timestamp =
                        chrono::Local::now().format("%Y%m%d_%H%M%S");
                    let fallback_path =
                        format!("/tmp/output/fallback_{timestamp:?}.pdf");

                    if let Ok(fallback_content) = fs::read(&fallback_path) {
                        if fallback_content.starts_with(b"%PDF") {
                            info!("Using fallback PDF from: {fallback_path:?}");
                            return Ok(fallback_content);
                        }
                    }
                    error!("No valid PDF found in fallback");
                }

                Ok(result)
            }
        }
    }
}

/// S3 helper functions.
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
                .map_err(|err| anyhow!("could not retrieve file from S3: {err:?}"))
        }
    } else {
        fn s3_private_bucket() -> Option<String> {
            None
        }
        fn s3_bucket_path(path: String) -> Option<String> {
            None
        }
        async fn get_file_from_s3(_bucket: String, _output_filename: String) -> Result<Vec<u8>> {
            unimplemented!()
        }
    }
}

/// Converts HTML to PDF using headless_chrome.
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

    info!("html_to_pdf: {url_path:?}");
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
        generate_document_outline: None,
        generate_tagged_pdf: None,
        transfer_mode: None,
    });

    print_to_pdf(url_path.as_str(), pdf_options, None)
}

/// Uses headless_chrome to print the file to PDF.
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

    debug!("Sleeping {wait:?}..");
    if let Some(wait) = wait {
        sleep(wait);
    }
    debug!("Awake! After {wait:?}");
    info!("4. Printing");

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
