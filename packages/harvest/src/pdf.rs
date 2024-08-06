// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::thread::sleep;
use std::time::Duration;
use tracing::instrument;

#[instrument(skip_all)]
pub fn print_to_pdf(
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
        .build()
        .expect("Default should not panic");
    let browser = Browser::new(options)?;
    let tab = browser.new_tab()?;
    println!("path: {}", file_path);
    tab.navigate_to(file_path)?.wait_until_navigated()?;

    if let Some(wait) = wait {
        sleep(wait);
    }

    //debug!("Using PDF options: {:?}", pdf_options);
    let bytes = tab.print_to_pdf(Some(pdf_options))?;

    Ok(bytes)
}
