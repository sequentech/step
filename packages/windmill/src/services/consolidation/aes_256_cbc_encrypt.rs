// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::shell::run_shell_command;
use anyhow::{anyhow, Context, Result};
use tracing::{info, instrument};

// used to recreate this command:
// openssl enc -aes-256-cbc -e -in $input_file_path -out $output_file_path -pass pass:$password -md md5

// #[instrument(skip(password), err)]
pub fn encrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    let command = format!(
        "openssl enc -aes-256-cbc -e -in {} -out {} -pass pass:{} -md md5",
        input_file_path, output_file_path, password
    );

    run_shell_command(&command).context("Failed to encrypt file")?;

    Ok(())
}

#[instrument(skip(password), err)]
pub fn decrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    let command = format!(
        "openssl enc -aes-256-cbc -d -in \"{}\" -out \"{}\" -pass pass:{} -md md5",
        input_file_path, output_file_path, password
    );

    run_shell_command(&command)
        .context("Failed to decrypt file")
        .map_err(|err| {
            anyhow!("Error decrypting file {input_file_path} to {output_file_path}: {err}")
        })?;

    Ok(())
}
