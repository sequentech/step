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
    pub fn run(&self) {
        match get_keys_ceremony_status(&self.election_event_id, &self.key_ceremony_id) {
            Ok(Some(status)) => {
                println!(
                    "{} {}",
                    "Success! Keys Ceremony status:".green(),
                    status.cyan()
                );
            }
            Ok(None) => {
                eprintln!("Error! Keys ceremony not found: {}", self.key_ceremony_id)
            }
            Err(err) => {
                eprintln!("Error! Failed to get keys ceremony status: {}", err)
            }
        }
    }
}
