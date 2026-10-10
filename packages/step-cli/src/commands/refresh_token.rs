// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::utils::read_config::{get_config_dir, refresh_and_save_token, CREATE_CONFIG_FILE_NAME};
use clap::Args;
use colored::Colorize;

#[derive(Args, Debug)]
#[command(about = "Refresh auth jwt", long_about = None)]
pub struct Refresh;

impl Refresh {
    pub fn run(&self) {
        match refresh_token() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("Error! Failed to refresh token: {}", err)
            }
        }
    }
}

fn refresh_token() -> Result<(), Box<dyn std::error::Error>> {
    refresh_and_save_token()?;
    let config_file = get_config_dir()?.join(CREATE_CONFIG_FILE_NAME);
    println!(
        "{}",
        format!(
            "Success! Configuration refreshed successfully at {:?}",
            config_file
        )
        .green(),
    );
    Ok(())
}
