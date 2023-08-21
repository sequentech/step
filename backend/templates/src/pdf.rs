use anyhow::Result;
use headless_chrome::types::PrintToPdfOptions;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use std::time::Duration;
use std::thread::sleep;

pub fn print_to_pdf(
    file_path: &str,
    pdf_options: PrintToPdfOptions,
    wait: Option<Duration>,
) -> Result<Vec<u8>> {
    let options = LaunchOptionsBuilder::default()
        .sandbox(false)
        .build()
        .expect("Default should not panic");
    let browser = Browser::new(options)?;
    let _tab0 = browser.wait_for_initial_tab()?;
    //let tab = tab.navigate_to(file_path)?.wait_until_navigated()?;
    let tab = browser.new_tab().unwrap();
    println!("path: {}", file_path);
    tab.navigate_to(file_path).unwrap().wait_until_navigated().unwrap();
    //tab.wait_for_element("input#searchInput").unwrap().click();

    if let Some(wait) = wait {
        //info!("Waiting {} before export to PDF", format_duration(wait));
        sleep(wait);
    }

    //debug!("Using PDF options: {:?}", pdf_options);
    let bytes = tab.print_to_pdf(Some(pdf_options))?;

    Ok(bytes)
}