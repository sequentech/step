#![allow(non_upper_case_globals)]
// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

extern crate lazy_static;

use anyhow::Result;
use sequent_core::util::init_log::init_log;
use sequent_core::services::pdf::html_to_pdf;
use dotenv::dotenv;
use structopt::StructOpt;
use tracing::{event, Level};
use windmill::services::celery_app::*;
use std::fs;
extern crate chrono;


#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_log(true);
    let html_template = r#"
        <!DOCTYPE html>
        <html>
        <body>
        
        <h1>My First Heading</h1>
        <p>My first paragraph.</p>
        
        </body>
        </html> 
    "#;
    loop {
        let local_pdf = html_to_pdf(html_template.to_string())?;
        fs::write("rust.pdf", local_pdf)?;
    }

    Ok(())
}