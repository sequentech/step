// SPDX-FileCopyrightText: 2024 JP Laurel <jlaurel@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use headless_chrome::Browser;
use headless_chrome::types::PrintToPdfOptions;
use serde_json::json;
use tracing::instrument;

pub trait PdfRenderer {
    fn render_to_pdf(&self, html: String, options: Option<PrintToPdfOptions>) -> Result<Vec<u8>>;
}

#[cfg(feature = "pdf-inplace")]
mod inplace {
    use super::*;

    pub struct InPlacePdfRenderer;

    impl InPlacePdfRenderer {
        pub fn new() -> Self {
            Self
        }
    }

    impl PdfRenderer for InPlacePdfRenderer {
        #[instrument(skip_all)]
        fn render_to_pdf(&self, html: String, _options: Option<PrintToPdfOptions>) -> Result<Vec<u8>> {
            super::super::pdf::html_to_pdf(html, _options)
        }
    }
}

#[cfg(feature = "pdf-openwhisk")]
mod openwhisk {
    use super::*;
    use std::process::Command;
    use std::fs;
    use std::path::PathBuf;
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
    use tracing::info;

    pub struct OpenWhiskRenderer {
        binary_path: String,
        save_output: bool,
        output_dir: Option<PathBuf>,
    }

    impl OpenWhiskRenderer {
        pub fn new(binary_path: String) -> Self {
            Self { 
                binary_path,
                save_output: false,
                output_dir: None,
            }
        }

        pub fn with_output_dir(binary_path: String, output_dir: impl Into<PathBuf>) -> Self {
            Self { 
                binary_path,
                save_output: true,
                output_dir: Some(output_dir.into()),
            }
        }

        fn save_pdf(&self, pdf_bytes: &[u8], html_content: &str) -> Result<PathBuf> {
            let output_dir = self.output_dir.clone().unwrap_or_else(|| PathBuf::from("."));
            
            fs::create_dir_all(&output_dir)?;

            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            
            let html_path = output_dir.join(format!("test_{}.html", timestamp));
            fs::write(&html_path, html_content)?;
            info!("Saved HTML to: {}", html_path.display());

            let pdf_path = output_dir.join(format!("test_{}.pdf", timestamp));
            fs::write(&pdf_path, pdf_bytes)?;
            info!("Saved PDF to: {}", pdf_path.display());

            Ok(pdf_path)
        }
    }

    

    impl PdfRenderer for OpenWhiskRenderer {
        #[instrument(skip_all)]
        fn render_to_pdf(&self, html: String, options: Option<PrintToPdfOptions>) -> Result<Vec<u8>> {
            let payload = json!({
                "html": html,
                "pdf_options": options,
            });

            let output = Command::new("cargo")
                .current_dir(&self.binary_path)
                .arg("run")
                .arg("--features")
                .arg("openwhisk")
                .arg("--")
                .arg(serde_json::to_string(&payload)?)
                .output()?;
            
            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("OpenWhisk CLI failed: {}", error);
            }
    
            let stdout = String::from_utf8(output.stdout)?;
            let response: serde_json::Value = serde_json::from_str(&stdout.trim())?;
    
            let pdf_base64 = response["pdf_base64"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing pdf_base64 in response"))?;
    
            let pdf_bytes = BASE64.decode(pdf_base64)?;
    
            if self.save_output {
                let saved_path = self.save_pdf(&pdf_bytes, &html)?;
                info!("Development files saved. PDF at: {}", saved_path.display());
            }
    
            Ok(pdf_bytes)
        }
    }
}

#[cfg(feature = "pdf-openwhisk-dev")]
mod openwhisk_dev {
    use super::*;

    pub struct OpenWhiskDevRenderer;

    impl OpenWhiskDevRenderer {
        pub fn new() -> Self {
            Self
        }
    }

    impl PdfRenderer for OpenWhiskDevRenderer {
        #[instrument(skip_all)]
        fn render_to_pdf(&self, html: String, _options: Option<PrintToPdfOptions>) -> Result<Vec<u8>> {
            super::super::pdf::html_to_text(html)
        }
    }
}

pub struct PdfService {
    renderer: Box<dyn PdfRenderer>,
}

impl PdfService {
    pub fn new(renderer: Box<dyn PdfRenderer>) -> Self {
        Self { renderer }
    }

    #[cfg(feature = "pdf-inplace")]
    pub fn with_inplace() -> Self {
        Self::new(Box::new(inplace::InPlacePdfRenderer::new()))
    }
    
    #[cfg(feature = "pdf-openwhisk")]
    pub fn with_openwhisk(binary_path: String) -> Self {
        Self::new(Box::new(openwhisk::OpenWhiskRenderer::new(binary_path)))
    }

    #[cfg(feature = "pdf-openwhisk-dev")]
    pub fn with_openwhisk_dev() -> Self {
        Self::new(Box::new(openwhisk_dev::OpenWhiskDevRenderer::new()))
    }

    #[instrument(skip_all)]
    pub fn render_pdf(&self, html: String, options: Option<PrintToPdfOptions>) -> Result<Vec<u8>> {
        self.renderer.render_to_pdf(html, options)
    }
}
