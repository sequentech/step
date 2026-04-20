// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::tally::{download_document, get_documents, get_tally_session_execution};
use clap::Args;
use colored::Colorize;
use std::error::Error;
use tracing::{error, info};

#[derive(Args)]
#[command(about = "Download Tally Results", long_about = None)]
/// Download tally results command arguments
pub struct DownloadTallyResults {
    /// ID of the tally session
    #[arg(long)]
    tally_id: String,

    /// Output directory for downloaded files
    #[arg(long, default_value = "output")]
    output_dir: String,

    /// ID of the election event
    #[arg(long)]
    election_event_id: String,
}

impl DownloadTallyResults {
    /// Run the download tally results command
    pub fn run(&self) {
        match download_results(&self.tally_id, &self.output_dir, &self.election_event_id) {
            Ok(()) => {
                info!(
                    "{} {}",
                    "Success! Downloaded tally results to:".green(),
                    self.output_dir.cyan()
                );
            }
            Err(err) => {
                error!("Error! Failed to download tally results: {err}");
            }
        }
    }
}

/// Download tally results
pub fn download_results(
    tally_id: &str,
    output_dir: &str,
    election_event_id: &str,
) -> Result<(), Box<dyn Error>> {
    let config = crate::utils::read_config::read_config()?;

    // Get the results event ID from the tally session execution
    let results_event_id = get_tally_session_execution::get_tally_session_execution(tally_id)?;

    let Some(results_event_id) = results_event_id else {
        return Err(Box::from("No results event ID found for tally session"));
    };

    // Get the documents from the results event
    let documents = get_documents::get_documents(&results_event_id)?;
    info!(
        "Found documents: {}",
        serde_json::to_string_pretty(&documents)?
    );

    // Extract the tar_gz document ID
    let tar_gz_id = documents["tar_gz"]
        .as_str()
        .ok_or_else(|| Box::<dyn Error>::from("No tar_gz document found"))?;

    // Get the document URL and download it
    let document = download_document::fetch_document(election_event_id, tar_gz_id)?;
    let output_path = format!("{output_dir}/tally.tar.gz");
    download_document::download_file(&document.url, &output_path)?;

    Ok(())
}
