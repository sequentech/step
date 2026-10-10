// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::trustees::get_ceremony_status::get_keys_ceremony_status;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
#[command(about = "Get Key Ceremony Status", long_about = None)]
pub struct GetKeyCeremonyStatus {
    /// Election event id - the election event the key ceremony belongs to
    #[arg(long)]
    election_event_id: String,

    /// Key ceremony id - the key ceremony to check
    #[arg(long)]
    key_ceremony_id: String,
}

impl GetKeyCeremonyStatus {
    /// Execute the command, preserving failures for shell automation.
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match get_keys_ceremony_status(&self.election_event_id, &self.key_ceremony_id) {
            Ok(Some(status)) => {
                println!(
                    "{} {}",
                    "Success! Keys Ceremony status:".green(),
                    status.cyan()
                );
            }
            Ok(None) => {
                return Err(format!("Keys ceremony not found: {}", self.key_ceremony_id).into());
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}
