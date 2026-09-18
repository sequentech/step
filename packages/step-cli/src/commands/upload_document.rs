// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::upload_file::GetUploadUrl;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
#[command(about = "Upload a local file as a document owned by the current tenant", long_about = None)]
pub struct UploadDocument {
    /// Path of the local file to upload
    #[arg(long)]
    file_path: String,
}

impl UploadDocument {
    /// Execute the command, preserving failures for shell automation.
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match GetUploadUrl::upload(self.file_path.clone(), true) {
            Ok(document_id) => {
                println!(
                    "{} {}",
                    "Success! Uploaded document. ID:".green(),
                    document_id.cyan()
                );
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}
