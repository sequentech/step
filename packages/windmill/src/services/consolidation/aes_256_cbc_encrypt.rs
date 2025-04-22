// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use sequent_core::signatures::shell::run_shell_command;
use std::process::Command;
use tracing::{info, instrument};

// used to recreate this command:
// openssl enc -aes-256-cbc -e -in $input_file_path -out $output_file_path -pass pass:$password -md md5

#[instrument(skip(password), err)]
pub fn encrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    let output = Command::new("openssl")
        .args(["enc", "-aes-256-cbc", "-e"])
        .arg("-in")
        .arg(input_file_path)
        .arg("-out")
        .arg(output_file_path)
        // use pass:... to avoid shell interpolation
        .arg("-pass")
        .arg(format!("pass:{password}"))
        .args(["-md", "md5"])
        .output()
        .map_err(|err| {
            anyhow!("Error encrypting file {input_file_path} to {output_file_path}: {err}")
        })?;

    // Check if the command was successful
    if !output.status.success() {
        return Err(anyhow::anyhow!("Command failed: {:?}", output));
    }

    Ok(())
}

#[instrument(skip(password), err)]
pub fn decrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    let output = Command::new("openssl")
        .args(["enc", "-aes-256-cbc", "-d"])
        .arg("-in")
        .arg(input_file_path)
        .arg("-out")
        .arg(output_file_path)
        // use pass:... to avoid shell interpolation
        .arg("-pass")
        .arg(format!("pass:{password}"))
        .args(["-md", "md5"])
        .output()
        .map_err(|err| {
            anyhow!("Error decrypting file {input_file_path} to {output_file_path}: {err}")
        })?;

    // Check if the command was successful
    if !output.status.success() {
        return Err(anyhow::anyhow!("Command failed: {:?}", output));
    }

    Ok(())
}
