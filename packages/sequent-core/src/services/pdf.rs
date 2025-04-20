// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::fs::File;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;
use tracing::{debug, info, instrument};

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

    let browser =
        Browser::new(options).with_context(|| "Error obtaining the browser")?;
    let tab = browser
        .new_tab()
        .with_context(|| "Error obtaining the tab")?;

    tab.navigate_to(file_path)?
        .wait_until_navigated()
        .with_context(|| "Error navigaring to file")?;

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

#[instrument(skip_all, err)]
pub fn html_to_pdf(html: String) -> Result<Vec<u8>> {
    // Create temp html file
    let dir = tempdir()?;
    let file_path = dir.path().join("index.html");
    let mut file = File::create(file_path.clone())?;
    let file_path_str = file_path.to_str().unwrap();
    file.write_all(html.as_bytes())?;
    let url_path = format!("file://{}", file_path_str);

    info!("html_to_pdf: {url_path}");

    print_to_pdf(
        url_path.as_str(),
        PrintToPdfOptions {
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
            generate_document_outline: None,
            generate_tagged_pdf: None,
        },
        None,
    )
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
        let bytes =
            html_to_pdf("<body><h1>Hello, world!</h1></body>".to_string())
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
