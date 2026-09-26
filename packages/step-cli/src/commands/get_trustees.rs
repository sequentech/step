// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::trustees::get::GetTrustees;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
#[command(about = "List Trustees", long_about = None)]
pub struct ListTrustees;

impl ListTrustees {
    /// Execute the command, preserving failures for shell automation.
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        match GetTrustees::get_all() {
            Ok(trustees) => {
                for (name, public_key) in &trustees {
                    println!("Trustee: name={} public_key={}", name, public_key);
                }
                println!(
                    "{} {}",
                    "Success! Listed trustees, count:".green(),
                    trustees.len().to_string().cyan()
                );
            }
            Err(err) => return Err(err),
        }
        Ok(())
    }
}
