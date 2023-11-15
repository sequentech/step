// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::fs::File;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;
use tracing::instrument;

#[instrument(skip_all)]
pub fn print_to_pdf(
    file_path: &str,
    pdf_options: PrintToPdfOptions,
    wait: Option<Duration>,
) -> Result<Vec<u8>> {
    let options = LaunchOptionsBuilder::default()
        //.idle_browser_timeout(Duration::from_secs(99999999))
        .sandbox(false)
        .build()
        .expect("Default should not panic");
    let browser = Browser::new(options)?;
    //browser.wait_for_initial_tab()?;
    let tab = browser.new_tab()?;
    //tab.set_default_timeout(Duration::from_secs(99999999));
    println!("path: {}", file_path);
    let bytes = tab
        .navigate_to(file_path)?
        .wait_until_navigated()?
        .print_to_pdf(Some(pdf_options))?;

    /*if let Some(wait) = wait {
        sleep(wait);
    }*/

    //debug!("Using PDF options: {:?}", pdf_options);

    Ok(bytes)
}

#[instrument(skip_all)]
pub fn html_to_pdf(html: String) -> Result<Vec<u8>> {
    // Create temp html file
    let dir = tempdir()?;
    let file_path = dir.path().join("index.html");
    let mut file = File::create(file_path.clone())?;
    let file_path_str = file_path.to_str().unwrap();
    file.write_all(html.as_bytes())?;
    let url_path = format!("file://{}", file_path_str);

    print_to_pdf(
        url_path.as_str(),
        PrintToPdfOptions {
            landscape: None,
            display_header_footer: None,
            print_background: None,
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
        },
        Some(Duration::new(10, 0)),
    )
}
