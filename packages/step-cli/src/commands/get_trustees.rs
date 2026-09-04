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
    pub fn run(&self) {
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
            Err(err) => {
                eprintln!("Error! Failed to list trustees: {}", err)
            }
        }
    }
}
