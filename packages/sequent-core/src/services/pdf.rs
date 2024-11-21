// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;
use tracing::{debug, info, instrument, warn};

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
        .build()
        .expect("Default should not panic");

    match Browser::new(options) {
        Ok(browser) => {
            let tab = browser
                .new_tab()
                .with_context(|| "Error obtaining the tab")?;

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
        Err(e) => {
            warn!("Browser initialization failed: {}. Falling back to file writing.", e);
            fallback_to_file(file_path, &pdf_options)
        }
    }
}

#[cfg(feature = "pdf-inplace")]
#[instrument(skip_all, err)]
fn fallback_to_file(file_path: &str, pdf_options: &PrintToPdfOptions) -> Result<Vec<u8>> {
    use std::process::Command;
    use std::fs;
    use std::path::PathBuf;
    use tracing::info;

    let actual_path = file_path.trim_start_matches("file://");
    let output_dir = PathBuf::from("/tmp/output");
    fs::create_dir_all(&output_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let output_path = output_dir.join(format!("fallback_{}.pdf", timestamp));

    // Prepare wkhtmltopdf command with options
    let mut command = Command::new("wkhtmltopdf");

    if pdf_options.landscape.unwrap_or(false) {
        command.arg("--orientation").arg("Landscape");
    }

    if let Some(width) = pdf_options.paper_width {
        command.arg("--page-width").arg(width.to_string());
    }

    if let Some(height) = pdf_options.paper_height {
        command.arg("--page-height").arg(height.to_string());
    }

    if let Some(margin_top) = pdf_options.margin_top {
        command.arg("--margin-top").arg(margin_top.to_string());
    }

    if let Some(margin_bottom) = pdf_options.margin_bottom {
        command.arg("--margin-bottom").arg(margin_bottom.to_string());
    }

    if let Some(margin_left) = pdf_options.margin_left {
        command.arg("--margin-left").arg(margin_left.to_string());
    }

    if let Some(margin_right) = pdf_options.margin_right {
        command.arg("--margin-right").arg(margin_right.to_string());
    }

    if pdf_options.print_background.unwrap_or(false) {
        command.arg("--print-media-type");
    }

    command.arg(actual_path).arg(output_path.to_str().unwrap());

    // Execute the command
    let status = command.status()?;

    if !status.success() {
        anyhow::bail!("Failed to generate PDF using wkhtmltopdf");
    }

    let pdf_bytes = fs::read(&output_path)?;
    info!("PDF saved to: {}", output_path.display());

    Ok(pdf_bytes)
}

#[cfg(not(feature = "pdf-inplace"))]
#[instrument(skip_all, err)]
fn fallback_to_file(file_path: &str, pdf_options: &PrintToPdfOptions) -> Result<Vec<u8>> {
    let actual_path = file_path.trim_start_matches("file://");
    let html = std::fs::read_to_string(actual_path)?;

    let output_dir = PathBuf::from("/tmp/output");
    std::fs::create_dir_all(&output_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let output_path = output_dir.join(format!("fallback_{}.pdf", timestamp));
    let mut file = File::create(&output_path)?;
    file.write_all(html.as_bytes())?;

    info!("Fallback PDF saved to: {}", output_path.display());

    Ok(html.into_bytes())
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
pub fn html_to_text(html: String) -> Result<Vec<u8>> {
    let output_dir = PathBuf::from("/tmp/output");
    std::fs::create_dir_all(&output_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let file_path = output_dir.join(format!("test_{}.txt", timestamp));
    let mut file = File::create(&file_path)?;
    file.write_all(html.as_bytes())?;

    info!("html_to_text: saved to {}", file_path.display());

    Ok(html.into_bytes())
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
