// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use std::process::Command;
use tracing::{info, instrument};

#[instrument(err, ret)]
pub fn run_shell_command(command: &str) -> Result<String> {
    // Run the shell command
    let output = Command::new("sh").arg("-c").arg(command).output()?;

    // Check if the command was successful
    if !output.status.success() {
        return Err(anyhow::anyhow!("Shell command failed: {:?}", output));
    }

    // Convert the output to a string
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Return the output
    Ok(stdout.to_string())
}
