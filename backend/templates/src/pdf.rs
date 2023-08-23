// SPDX-FileCopyrightText: 2023 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::thread::sleep;
use std::time::Duration;

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
    let _tab0 = browser.wait_for_initial_tab()?;
    let tab = browser.new_tab()?;
    //tab.set_default_timeout(Duration::from_secs(99999999));
    println!("path: {}", file_path);
    tab.navigate_to(file_path)?.wait_until_navigated()?;

    if let Some(wait) = wait {
        sleep(wait);
    }

    //debug!("Using PDF options: {:?}", pdf_options);
    let bytes = tab.print_to_pdf(Some(pdf_options))?;

    Ok(bytes)
}
