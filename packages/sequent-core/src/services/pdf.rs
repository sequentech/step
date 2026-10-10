// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "s3")]
use crate::services::s3;
use crate::util::aws::{
    AWS_S3_PRIVATE_URI_ENV, AWS_S3_PUBLIC_BUCKET_ENV, AWS_S3_PUBLIC_URI_ENV,
};
use crate::util::convert_vec::IntoVec;
use crate::util::retry::retry_with_exponential_backoff;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use headless_chrome::browser::tab::RequestPausedDecision;
use headless_chrome::browser::transport::{SessionId, Transport};
use headless_chrome::protocol::cdp::Fetch::{
    events::RequestPausedEvent, FailRequest, FulfillRequest, HeaderEntry,
    RequestPattern, RequestStage,
};
use headless_chrome::protocol::cdp::Network::{ErrorReason, ResourceType};
pub use headless_chrome::types::{PrintToPdfOptions, TransferMode};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use reqwest;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use strum_macros::{Display, EnumString};
use tempfile::tempdir;
use tokio::runtime::Runtime;
use tracing::{debug, error, event, info, instrument, warn, Level};

/// Directory where the doc renderer image ships the assets that templates
/// reference as `/assets/...`.
const BUNDLED_ASSETS_DIR: &str = "/assets";
const ANY_URL_PATTERN: &str = "*";
const DATA_SCHEME: &str = "data";
const BLOB_SCHEME: &str = "blob";
const FILE_SCHEME: &str = "file";
const HTTP_SCHEME: &str = "http";
const HTTPS_SCHEME: &str = "https";
const DISABLE_SCRIPTS_ARG: &str = "--blink-settings=scriptEnabled=false";
const HOST_RESOLVER_RULES_ARG: &str = "--host-resolver-rules";
const UNRESOLVABLE_HOSTS_RULE: &str = "MAP * ~NOTFOUND";
const EXCLUDE_HOST_RULE: &str = "EXCLUDE";
const DISABLE_NON_PROXIED_UDP_ARG: &str =
    "--force-webrtc-ip-handling-policy=disable_non_proxied_udp";
const HTTP_OK: u32 = 200;
const CONTENT_TYPE_HEADER: &str = "Content-Type";
const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";
const CONTENT_SECURITY_POLICY_HEADER: &str = "Content-Security-Policy";
const CSP_NONE_SOURCE: &str = "'none'";
/// The rendered document reaches Chromium base64 encoded in one DevTools
/// message, which Chromium limits to 100 MiB.
const MAX_SCRIPTED_DOCUMENT_BYTES: usize = 48 * 1024 * 1024;

/// What Chromium may run while it renders HTML to PDF. Both policies load
/// only the resources that `PdfResourceAllowList` accepts.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Display,
    EnumString,
)]
pub enum PdfResourcePolicy {
    /// Scripts run, but the document is served with a content security policy
    /// that confines their connections to the public bucket URLs.
    #[default]
    Restricted,
    /// Same resources as `Restricted`, with JavaScript disabled.
    RestrictedNoScripts,
}

impl PdfResourcePolicy {
    pub fn for_content(contains_sensitive_data: bool) -> Self {
        if contains_sensitive_data {
            PdfResourcePolicy::RestrictedNoScripts
        } else {
            PdfResourcePolicy::Restricted
        }
    }

    fn allows_scripts(self) -> bool {
        match self {
            PdfResourcePolicy::Restricted => true,
            PdfResourcePolicy::RestrictedNoScripts => false,
        }
    }

    /// Documents too large to be served with a content security policy render
    /// without scripts.
    fn for_document_size(self, document_bytes: usize) -> Self {
        if document_bytes > MAX_SCRIPTED_DOCUMENT_BYTES {
            PdfResourcePolicy::RestrictedNoScripts
        } else {
            self
        }
    }
}

/// Resources Chromium may load while it renders HTML to PDF: the rendered
/// document, the bundled assets directory, the public bucket and inline
/// `data:`/`blob:` URLs.
struct PdfResourceAllowList {
    resource_policy: PdfResourcePolicy,
    document: PathBuf,
    document_html: String,
    assets_dir: PathBuf,
    public_bucket_urls: Vec<Url>,
}

impl PdfResourceAllowList {
    fn allows(&self, url: &str) -> bool {
        let Ok(url) = Url::parse(url) else {
            return false;
        };
        match url.scheme() {
            DATA_SCHEME | BLOB_SCHEME => true,
            FILE_SCHEME => {
                url.to_file_path().is_ok_and(|path| self.allows_file(&path))
            }
            HTTP_SCHEME | HTTPS_SCHEME => self
                .public_bucket_urls
                .iter()
                .any(|base| is_under_url(&url, base)),
            _ => false,
        }
    }

    fn allows_file(&self, path: &Path) -> bool {
        let is_normalized = path.components().all(|component| {
            matches!(component, Component::RootDir | Component::Normal(_))
        });
        is_normalized
            && (path == self.document || path.starts_with(&self.assets_dir))
    }

    /// Documents load only from the rendered file, so frames and navigations
    /// never load anything else.
    fn allows_request(&self, url: &str, resource_type: &ResourceType) -> bool {
        match resource_type {
            ResourceType::Document => Url::parse(url)
                .ok()
                .filter(|url| url.scheme() == FILE_SCHEME)
                .and_then(|url| url.to_file_path().ok())
                .is_some_and(|path| path == self.document),
            _ => self.allows(url),
        }
    }

    fn decide(&self, event: RequestPausedEvent) -> RequestPausedDecision {
        let params = event.params;
        if self.allows_request(&params.request.url, &params.resource_Type) {
            if params.resource_Type == ResourceType::Document
                && self.resource_policy.allows_scripts()
            {
                return self.serve_document(params.request_id);
            }
            return RequestPausedDecision::Continue(None);
        }
        warn!(
            "Blocked a PDF resource request to {}",
            describe_origin(&params.request.url)
        );
        RequestPausedDecision::Fail(FailRequest {
            request_id: params.request_id,
            error_reason: ErrorReason::BlockedByClient,
        })
    }

    /// Content security policy for documents that run scripts. It confines the
    /// connections that request interception does not see, such as
    /// WebSockets, workers and other windows, to the public bucket URLs.
    fn content_security_policy(&self) -> String {
        let connect_sources = if self.public_bucket_urls.is_empty() {
            CSP_NONE_SOURCE.to_string()
        } else {
            self.public_bucket_urls
                .iter()
                .map(Url::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        };
        format!(
            "connect-src {connect_sources}; worker-src {CSP_NONE_SOURCE}; \
             frame-src {CSP_NONE_SOURCE}; object-src {CSP_NONE_SOURCE}; \
             form-action {CSP_NONE_SOURCE}; \
             sandbox allow-scripts allow-same-origin"
        )
    }

    fn serve_document(&self, request_id: String) -> RequestPausedDecision {
        RequestPausedDecision::Fulfill(FulfillRequest {
            request_id,
            response_code: HTTP_OK,
            response_headers: Some(vec![
                HeaderEntry {
                    name: CONTENT_TYPE_HEADER.to_string(),
                    value: HTML_CONTENT_TYPE.to_string(),
                },
                HeaderEntry {
                    name: CONTENT_SECURITY_POLICY_HEADER.to_string(),
                    value: self.content_security_policy(),
                },
            ]),
            binary_response_headers: None,
            body: Some(BASE64.encode(self.document_html.as_bytes())),
            response_phrase: None,
        })
    }

    /// Chromium arguments that make every host except the public bucket's
    /// unresolvable and keep WebRTC off UDP. They cover the connections that
    /// request interception does not see, such as WebSockets, workers and
    /// other windows.
    fn chrome_network_args(&self) -> Vec<String> {
        let mut rules = vec![UNRESOLVABLE_HOSTS_RULE.to_string()];
        for host in self.public_bucket_urls.iter().filter_map(Url::host_str) {
            let rule = format!("{EXCLUDE_HOST_RULE} {host}");
            if !rules.contains(&rule) {
                rules.push(rule);
            }
        }
        vec![
            format!("{HOST_RESOLVER_RULES_ARG}={}", rules.join(", ")),
            DISABLE_NON_PROXIED_UDP_ARG.to_string(),
        ]
    }
}

fn is_under_url(url: &Url, base: &Url) -> bool {
    url.scheme() == base.scheme()
        && url.host() == base.host()
        && url.port_or_known_default() == base.port_or_known_default()
        && url.username().is_empty()
        && url.password().is_none()
        && url.path().starts_with(base.path())
}

fn describe_origin(url: &str) -> String {
    match Url::parse(url) {
        Ok(url) => {
            format!("{}://{}", url.scheme(), url.host_str().unwrap_or_default())
        }
        Err(_) => "an invalid URL".to_string(),
    }
}

/// Base URLs of the public bucket through the private and the public S3
/// endpoints, see `s3::get_minio_url` and `s3::get_minio_public_url`.
fn public_bucket_urls() -> Vec<Url> {
    let Ok(bucket) = std::env::var(AWS_S3_PUBLIC_BUCKET_ENV) else {
        return vec![];
    };
    [AWS_S3_PRIVATE_URI_ENV, AWS_S3_PUBLIC_URI_ENV]
        .into_iter()
        .filter_map(|endpoint_env| std::env::var(endpoint_env).ok())
        .filter_map(|endpoint| {
            Url::parse(&format!("{endpoint}/{bucket}/")).ok()
        })
        .collect()
}

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
pub fn html_to_pdf(
    html: String,
    options: Option<PrintToPdfOptions>,
) -> Result<Vec<u8>> {
    html_to_pdf_with_policy(html, options, PdfResourcePolicy::default())
}

/// Converts HTML to PDF using headless_chrome under `resource_policy`.
pub fn html_to_pdf_with_policy(
    html: String,
    options: Option<PrintToPdfOptions>,
    resource_policy: PdfResourcePolicy,
) -> Result<Vec<u8>> {
    render_html_to_pdf(
        html,
        options,
        resource_policy,
        Path::new(BUNDLED_ASSETS_DIR),
        public_bucket_urls(),
    )
}

#[instrument(skip_all, err)]
fn render_html_to_pdf(
    html: String,
    options: Option<PrintToPdfOptions>,
    resource_policy: PdfResourcePolicy,
    assets_dir: &Path,
    public_bucket_urls: Vec<Url>,
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
        transfer_mode: None,
    });

    let resource_policy = resource_policy.for_document_size(html.len());
    let allow_list = Arc::new(PdfResourceAllowList {
        resource_policy,
        document: file_path.clone(),
        document_html: html,
        assets_dir: assets_dir.to_path_buf(),
        public_bucket_urls,
    });

    print_to_pdf(url_path.as_str(), pdf_options, None, &allow_list)
}

/// Uses headless_chrome to print the file to PDF, with retry on transient
/// failures.
#[instrument(skip_all, err)]
fn print_to_pdf(
    file_path: &str,
    pdf_options: PrintToPdfOptions,
    wait: Option<Duration>,
    allow_list: &Arc<PdfResourceAllowList>,
) -> Result<Vec<u8>> {
    // When multiple Rayon threads generate PDF batches concurrently (workers
    // 29, 30, 31), each spawns its own headless Chrome process. Chrome can
    // crash mid-print (due to --single-process flag instability or memory
    // pressure from concurrent instances), closing the WebSocket connection and
    // causing tab.print_to_pdf() to fail with ConnectionClosed: Unable to make
    // method calls because underlying connection is closed
    const MAX_RETRIES: u32 = 5;
    let mut delay = Duration::from_secs(1);
    // PrintToPdfOptions doesn't derive Clone, so serialize once and deserialize
    // per attempt to get independent owned copies.
    let pdf_options_json = serde_json::to_value(&pdf_options)
        .with_context(|| "Error serializing pdf_options for retry")?;

    for attempt in 1..=MAX_RETRIES {
        let opts: PrintToPdfOptions =
            serde_json::from_value(pdf_options_json.clone())
                .with_context(|| "Error deserializing pdf_options for retry")?;

        match print_to_pdf_once(file_path, opts, wait, Arc::clone(allow_list)) {
            Ok(bytes) => return Ok(bytes),
            Err(e) if attempt < MAX_RETRIES => {
                warn!(
                    "print_to_pdf attempt {attempt}/{MAX_RETRIES} failed: {e:?}, \
                     retrying in {delay:?}"
                );
                sleep(delay);
                delay *= 2;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}

/// One attempt at printing via headless Chrome.
#[instrument(skip_all, err)]
fn print_to_pdf_once(
    file_path: &str,
    pdf_options: PrintToPdfOptions,
    wait: Option<Duration>,
    allow_list: Arc<PdfResourceAllowList>,
) -> Result<Vec<u8>> {
    let network_args = allow_list.chrome_network_args();
    let mut args = vec![
        std::ffi::OsStr::new("--disable-setuid-sandbox"),
        std::ffi::OsStr::new("--disable-dev-shm-usage"),
        std::ffi::OsStr::new("--single-process"),
        std::ffi::OsStr::new("--no-zygote"),
    ];
    args.extend(network_args.iter().map(std::ffi::OsStr::new));
    if !allow_list.resource_policy.allows_scripts() {
        args.push(std::ffi::OsStr::new(DISABLE_SCRIPTS_ARG));
    }
    let options = LaunchOptionsBuilder::default()
        .sandbox(false)
        // <WTF> Why? well this:
        // https://github.com/rust-headless-chrome/rust-headless-chrome/issues/500
        .devtools(false)
        .headless(true)
        // </WTF>
        .enable_logging(true)
        .idle_browser_timeout(Duration::from_secs(99999999))
        .args(args)
        .build()
        .expect("Default should not panic");

    info!("1. Opening browser");
    let browser =
        Browser::new(options).with_context(|| "Error obtaining the browser")?;

    info!("2. Opening tab");
    let tab = browser.new_tab()?;

    tab.enable_request_interception(Arc::new(
        move |_transport: Arc<Transport>,
              _session_id: SessionId,
              event: RequestPausedEvent| allow_list.decide(event),
    ))?;
    tab.enable_fetch(
        Some(&[RequestPattern {
            url_pattern: Some(ANY_URL_PATTERN.to_string()),
            resource_Type: None,
            request_stage: Some(RequestStage::Request),
        }]),
        None,
    )?;

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
        io::{BufRead, BufReader},
        net::TcpListener,
        path::Path,
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc, Mutex,
        },
        thread,
    };

    use super::*;
    use anyhow::Result;

    const PDF_PAGE_MARKER: &[u8] = b"/Type /Page\n";
    const TALL_PAGE_STYLE: &str = "body { height: 5000px; }";

    fn pdf_page_count(pdf: &[u8]) -> usize {
        pdf.windows(PDF_PAGE_MARKER.len())
            .filter(|window| *window == PDF_PAGE_MARKER)
            .count()
    }

    struct RecordingServer {
        port: u16,
        paths: Arc<Mutex<Vec<String>>>,
        connections: Arc<AtomicUsize>,
    }

    impl RecordingServer {
        fn start(body: &'static str) -> Result<Self> {
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let port = listener.local_addr()?.port();
            let paths = Arc::new(Mutex::new(Vec::new()));
            let connections = Arc::new(AtomicUsize::new(0));
            let recorded = Arc::clone(&paths);
            let counted = Arc::clone(&connections);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { continue };
                    counted.fetch_add(1, Ordering::SeqCst);
                    let mut reader = BufReader::new(&stream);
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).is_err() {
                        continue;
                    }
                    let mut header = String::new();
                    while reader.read_line(&mut header).is_ok()
                        && !header.trim().is_empty()
                    {
                        header.clear();
                    }
                    let path = request_line
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or_default()
                        .to_string();
                    if let Ok(mut paths) = recorded.lock() {
                        paths.push(path);
                    }
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                }
            });
            Ok(RecordingServer {
                port,
                paths,
                connections,
            })
        }

        fn url(&self, path: &str) -> String {
            format!("http://127.0.0.1:{}{}", self.port, path)
        }

        fn localhost_url(&self, path: &str) -> String {
            format!("http://localhost:{}{}", self.port, path)
        }

        fn socket_url(&self, path: &str) -> String {
            format!("ws://127.0.0.1:{}{}", self.port, path)
        }

        fn localhost_socket_url(&self, path: &str) -> String {
            format!("ws://localhost:{}{}", self.port, path)
        }

        fn paths(&self) -> Vec<String> {
            self.paths
                .lock()
                .map(|paths| paths.clone())
                .unwrap_or_default()
        }

        fn connection_count(&self) -> usize {
            self.connections.load(Ordering::SeqCst)
        }
    }

    fn scripts_reaching(server: &RecordingServer) -> String {
        format!(
            r#"<script>
                new WebSocket("{socket}");
                const worker = 'fetch("{worker}");';
                new Worker(URL.createObjectURL(
                    new Blob([worker], {{ type: "text/javascript" }})
                ));
                window.open("{window}");
            </script>"#,
            socket = server.socket_url("/socket"),
            worker = server.url("/worker"),
            window = server.url("/window"),
        )
    }

    #[test]
    fn html_to_pdf_does_not_request_other_hosts() -> Result<()> {
        let server = RecordingServer::start("<p>remote</p>")?;
        let html = format!(
            r#"<body>
                <img src="{img}">
                <iframe src="{frame}"></iframe>
                <link rel="stylesheet" href="{style}">
                <script>fetch("{fetch}");</script>
                {scripts}
            </body>"#,
            img = server.url("/img"),
            frame = server.url("/frame"),
            style = server.url("/style"),
            fetch = server.url("/fetch"),
            scripts = scripts_reaching(&server),
        );

        let bytes = html_to_pdf(html, None)?;

        assert!(bytes.starts_with(b"%PDF"));
        assert_eq!(server.paths(), Vec::<String>::new());
        assert_eq!(server.connection_count(), 0);
        Ok(())
    }

    #[test]
    fn html_to_pdf_does_not_load_other_local_files() -> Result<()> {
        let dir = tempdir()?;
        let style_path = dir.path().join("tall.css");
        fs::write(&style_path, TALL_PAGE_STYLE)?;
        let html = format!(
            r#"<head><link rel="stylesheet" href="file://{}"></head>
            <body><p>local</p></body>"#,
            style_path.display()
        );

        let bytes = html_to_pdf(html, None)?;

        assert_eq!(pdf_page_count(&bytes), 1);
        Ok(())
    }

    #[test]
    fn html_to_pdf_does_not_load_local_files_in_frames() -> Result<()> {
        let server = RecordingServer::start("")?;
        let dir = tempdir()?;
        let frame_path = dir.path().join("frame.html");
        fs::write(
            &frame_path,
            format!(r#"<img src="{}">"#, server.url("/from-frame")),
        )?;
        let html = format!(
            r#"<body><iframe src="file://{}"></iframe></body>"#,
            frame_path.display()
        );

        html_to_pdf(html, None)?;

        assert_eq!(server.paths(), Vec::<String>::new());
        Ok(())
    }

    #[test]
    fn render_loads_bundled_assets_and_public_bucket() -> Result<()> {
        let server = RecordingServer::start("")?;
        let assets_dir = tempdir()?;
        fs::write(assets_dir.path().join("tall.css"), TALL_PAGE_STYLE)?;
        let html = format!(
            r#"<head><link rel="stylesheet" href="file://{assets}/tall.css"></head>
            <body>
                <img src="{public}">
                <img src="{private}">
            </body>"#,
            assets = assets_dir.path().display(),
            public = server.url("/public/public-assets/logo.png"),
            private = server.url("/private/document.pdf"),
        );

        let bytes = render_html_to_pdf(
            html,
            None,
            PdfResourcePolicy::Restricted,
            assets_dir.path(),
            Url::parse(&server.url("/public/")).into_iter().collect(),
        )?;

        assert!(pdf_page_count(&bytes) > 1);
        assert_eq!(server.paths(), vec!["/public/public-assets/logo.png"]);
        Ok(())
    }

    #[test]
    fn render_does_not_load_frames_from_the_public_bucket() -> Result<()> {
        let server =
            RecordingServer::start(r#"<img src="/public/frame-image.png">"#)?;
        let assets_dir = tempdir()?;
        let html = format!(
            r#"<body>
                <img src="{logo}">
                <iframe src="{frame}"></iframe>
            </body>"#,
            logo = server.url("/public/logo.png"),
            frame = server.url("/public/frame.html"),
        );

        render_html_to_pdf(
            html,
            None,
            PdfResourcePolicy::Restricted,
            assets_dir.path(),
            Url::parse(&server.url("/public/")).into_iter().collect(),
        )?;

        assert_eq!(server.paths(), vec!["/public/logo.png"]);
        Ok(())
    }

    #[test]
    fn render_resolves_only_the_public_bucket_host() -> Result<()> {
        let bucket = RecordingServer::start("")?;
        let other = RecordingServer::start("")?;
        let assets_dir = tempdir()?;
        let html = format!(
            r#"<body><img src="{logo}">{scripts}</body>"#,
            logo = bucket.localhost_url("/public/logo.png"),
            scripts = scripts_reaching(&other),
        );

        render_html_to_pdf(
            html,
            None,
            PdfResourcePolicy::Restricted,
            assets_dir.path(),
            Url::parse(&bucket.localhost_url("/public/"))
                .into_iter()
                .collect(),
        )?;

        assert_eq!(bucket.paths(), vec!["/public/logo.png"]);
        assert_eq!(other.connection_count(), 0);
        Ok(())
    }

    #[test]
    fn render_confines_script_connections_to_the_public_bucket_urls(
    ) -> Result<()> {
        let bucket = RecordingServer::start("")?;
        let assets_dir = tempdir()?;
        let html = format!(
            r#"<body>
                <img src="{logo}">
                <form id="form" method="post" action="{form}"></form>
                <script>
                    fetch("{data}");
                    fetch("{other}");
                    new WebSocket("{socket}");
                    new Worker(URL.createObjectURL(
                        new Blob(['fetch("{worker}");'], {{ type: "text/javascript" }})
                    ));
                    window.open("{popup}");
                    document.getElementById("form").submit();
                </script>
            </body>"#,
            logo = bucket.localhost_url("/public/logo.png"),
            data = bucket.localhost_url("/public/data.json"),
            other = bucket.localhost_url("/other/data.json"),
            socket = bucket.localhost_socket_url("/socket"),
            worker = bucket.localhost_url("/public/worker"),
            popup = bucket.localhost_url("/public/popup"),
            form = bucket.localhost_url("/public/form"),
        );

        render_html_to_pdf(
            html,
            None,
            PdfResourcePolicy::Restricted,
            assets_dir.path(),
            Url::parse(&bucket.localhost_url("/public/"))
                .into_iter()
                .collect(),
        )?;

        let mut paths = bucket.paths();
        paths.sort();
        assert_eq!(paths, vec!["/public/data.json", "/public/logo.png"]);
        Ok(())
    }

    const PRIVATE_BUCKET_URL: &str = "http://minio:9000/public/";
    const PUBLIC_BUCKET_URL: &str = "https://s3.example.com/public/";

    const SCRIPT_TALL_PAGE_HTML: &str = r#"<body><script>document.body.style.height = "5000px";</script></body>"#;

    fn render_script_height(policy: PdfResourcePolicy) -> Result<usize> {
        let assets_dir = tempdir()?;
        let bytes = render_html_to_pdf(
            SCRIPT_TALL_PAGE_HTML.to_string(),
            None,
            policy,
            assets_dir.path(),
            vec![],
        )?;
        Ok(pdf_page_count(&bytes))
    }

    #[test]
    fn restricted_render_runs_scripts() -> Result<()> {
        assert!(render_script_height(PdfResourcePolicy::Restricted)? > 1);
        Ok(())
    }

    #[test]
    fn restricted_no_scripts_render_does_not_run_scripts() -> Result<()> {
        assert_eq!(
            render_script_height(PdfResourcePolicy::RestrictedNoScripts)?,
            1
        );
        Ok(())
    }

    #[test]
    fn resource_policy_defaults_to_restricted_and_disables_scripts_for_sensitive_content(
    ) {
        assert_eq!(PdfResourcePolicy::default(), PdfResourcePolicy::Restricted);
        assert_eq!(
            PdfResourcePolicy::for_content(false),
            PdfResourcePolicy::Restricted
        );
        assert_eq!(
            PdfResourcePolicy::for_content(true),
            PdfResourcePolicy::RestrictedNoScripts
        );
        assert!(PdfResourcePolicy::Restricted.allows_scripts());
        assert!(!PdfResourcePolicy::RestrictedNoScripts.allows_scripts());
    }

    #[test]
    fn resource_policy_round_trips_through_strings_and_json() -> Result<()> {
        for policy in [
            PdfResourcePolicy::Restricted,
            PdfResourcePolicy::RestrictedNoScripts,
        ] {
            assert_eq!(
                policy.to_string().parse::<PdfResourcePolicy>()?,
                policy
            );
            let json = serde_json::to_value(policy)?;
            assert_eq!(
                serde_json::from_value::<PdfResourcePolicy>(json)?,
                policy
            );
        }
        assert!("Unrestricted".parse::<PdfResourcePolicy>().is_err());
        Ok(())
    }

    fn test_allow_list(public_bucket_urls: &[&str]) -> PdfResourceAllowList {
        PdfResourceAllowList {
            resource_policy: PdfResourcePolicy::Restricted,
            document: PathBuf::from("/tmp/.tmpRender/index.html"),
            document_html: String::new(),
            assets_dir: PathBuf::from(BUNDLED_ASSETS_DIR),
            public_bucket_urls: public_bucket_urls
                .iter()
                .filter_map(|url| Url::parse(url).ok())
                .collect(),
        }
    }

    #[test]
    fn allow_list_accepts_document_assets_public_bucket_and_inline_urls() {
        let allow_list =
            test_allow_list(&[PRIVATE_BUCKET_URL, PUBLIC_BUCKET_URL]);

        for url in [
            "file:///tmp/.tmpRender/index.html",
            "file:///assets/qrcode.min.js",
            "file:///assets/logos/commissioner-logo.jpg",
            "http://minio:9000/public/public-assets/sequent-logo.svg",
            "http://MINIO:9000/public/public-assets/qrcode.min.js",
            "https://s3.example.com/public/tenant-1/document-2/logo.png",
            "data:image/png;base64,iVBORw0KGgo=",
            "blob:file:///0d4c6f3e-1f2a-4b1e-9a43-5f1f1d1c2b3a",
        ] {
            assert!(allow_list.allows(url), "{url} should be allowed");
        }
    }

    #[test]
    fn allow_list_rejects_other_files_hosts_and_schemes() {
        let allow_list =
            test_allow_list(&[PRIVATE_BUCKET_URL, PUBLIC_BUCKET_URL]);

        for url in [
            "file:///etc/passwd",
            "file:///tmp/",
            "file:///tmp/.tmpOther/index.html",
            "file:///tmp/.tmpRender/other.html",
            "file:///assets/../etc/passwd",
            "file:///assets/..%2Fetc%2Fpasswd",
            "file:///assets-copy/qrcode.min.js",
            "file://remote-host/assets/qrcode.min.js",
            "file:///etc/hosts",
            "http://10.0.0.5/status",
            "http://internal-service:8080/status",
            "http://127.0.0.1:8081/status",
            "http://minio:9000/sequent/private-document.pdf",
            "http://minio:9000/publicity/logo.png",
            "http://minio:9001/public/logo.png",
            "https://minio:9000/public/logo.png",
            "http://s3.example.com/public/logo.png",
            "https://s3.example.com/other-bucket/logo.png",
            "http://user:pass@minio:9000/public/logo.png",
            "http://minio:9000@remote.example/public/logo.png",
            "ws://minio:9000/public/socket",
            "ftp://minio/public/logo.png",
            "javascript:alert(1)",
            "not a url",
        ] {
            assert!(!allow_list.allows(url), "{url} should be blocked");
        }
    }

    #[test]
    fn allow_list_without_public_bucket_rejects_http() {
        let allow_list = test_allow_list(&[]);

        assert!(!allow_list.allows("http://minio:9000/public/logo.png"));
        assert!(allow_list.allows("file:///assets/qrcode.min.js"));
    }

    #[test]
    fn allow_list_loads_only_the_rendered_document_as_a_document() {
        let allow_list =
            test_allow_list(&[PRIVATE_BUCKET_URL, PUBLIC_BUCKET_URL]);

        assert!(allow_list.allows_request(
            "file:///tmp/.tmpRender/index.html",
            &ResourceType::Document
        ));
        assert!(allow_list.allows_request(
            "http://minio:9000/public/frame.html",
            &ResourceType::Image
        ));
        for url in [
            "file:///assets/frame.html",
            "file:///tmp/.tmpRender/other.html",
            "http://minio:9000/public/frame.html",
            "data:text/html,<p>frame</p>",
            "not a url",
        ] {
            assert!(
                !allow_list.allows_request(url, &ResourceType::Document),
                "{url} should not load as a document"
            );
        }
    }

    #[test]
    fn content_security_policy_confines_connections_to_the_public_bucket_urls()
    {
        let policy = test_allow_list(&[PRIVATE_BUCKET_URL, PUBLIC_BUCKET_URL])
            .content_security_policy();

        assert!(policy.starts_with(
            "connect-src http://minio:9000/public/ \
             https://s3.example.com/public/; "
        ));
        for directive in [
            "worker-src 'none'",
            "frame-src 'none'",
            "object-src 'none'",
            "form-action 'none'",
            "sandbox allow-scripts allow-same-origin",
        ] {
            assert!(policy.contains(directive), "{directive} is missing");
        }
        assert!(test_allow_list(&[])
            .content_security_policy()
            .starts_with("connect-src 'none'; "));
    }

    #[test]
    fn large_documents_render_without_scripts() {
        assert_eq!(
            PdfResourcePolicy::Restricted
                .for_document_size(MAX_SCRIPTED_DOCUMENT_BYTES),
            PdfResourcePolicy::Restricted
        );
        assert_eq!(
            PdfResourcePolicy::Restricted
                .for_document_size(MAX_SCRIPTED_DOCUMENT_BYTES + 1),
            PdfResourcePolicy::RestrictedNoScripts
        );
        assert_eq!(
            PdfResourcePolicy::RestrictedNoScripts.for_document_size(0),
            PdfResourcePolicy::RestrictedNoScripts
        );
    }

    #[test]
    fn chrome_network_args_resolve_only_the_public_bucket_hosts() {
        assert_eq!(
            test_allow_list(&[PRIVATE_BUCKET_URL, PUBLIC_BUCKET_URL])
                .chrome_network_args(),
            vec![
                "--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE minio, \
                 EXCLUDE s3.example.com"
                    .to_string(),
                DISABLE_NON_PROXIED_UDP_ARG.to_string(),
            ]
        );
        assert_eq!(
            test_allow_list(&[]).chrome_network_args(),
            vec![
                "--host-resolver-rules=MAP * ~NOTFOUND".to_string(),
                DISABLE_NON_PROXIED_UDP_ARG.to_string(),
            ]
        );
    }

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
