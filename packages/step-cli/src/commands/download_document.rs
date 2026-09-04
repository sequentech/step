// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::tally::download_document::{download_file_plain, fetch_document_url};
use clap::Args;
use colored::Colorize;

#[derive(Args)]
#[command(about = "Download a document (not tied to an election event) to a local file", long_about = None)]
pub struct DownloadDocument {
    /// Document id - as returned by e.g. export-tenant-config
    #[arg(long)]
    document_id: String,

    /// Output path to write the downloaded file to
    #[arg(long)]
    output: String,
}

impl DownloadDocument {
    pub fn run(&self) {
        match download_document(&self.document_id, &self.output) {
            Ok(()) => {
                println!(
                    "{} {}",
                    "Success! Downloaded document to:".green(),
                    self.output.cyan()
                );
            }
            Err(err) => {
                eprintln!("Error! Failed to download document: {}", err)
            }
        }
    }
}

fn download_document(document_id: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let document = fetch_document_url(document_id)?;
    download_file_plain(&document.url, output)
}
