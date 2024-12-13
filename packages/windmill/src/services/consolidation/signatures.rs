// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::shell::run_shell_command;
use crate::services::{
    consolidation::ecies_encrypt::ECIES_TOOL_PATH, temp_path::generate_temp_file,
};
use anyhow::{Context, Result};
use std::io::Write;
use tracing::{info, instrument};

#[instrument(skip_all, err)]
pub fn ecdsa_sign_data(
    pk12_file_path_string: &str,
    password: &str,
    data_path: &str,
) -> Result<String> {
    let command = format!(
        "java -jar {} sign-ec {} {} {}",
        ECIES_TOOL_PATH, pk12_file_path_string, data_path, password
    );

    let encrypted_base64 = run_shell_command(&command)?.replace("\n", "");

    info!("ecdsa_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}

#[instrument(skip_all, err)]
pub fn get_pem_fingerprint(pem: &str) -> Result<String> {
    let mut temp_file =
        generate_temp_file("cert", "pem").with_context(|| "Error creating temp file")?;

    temp_file.write_all(pem.as_bytes())?;

    // Flush to ensure data is physically pushed to the file buffer.
    temp_file.flush()?;

    let temp_path = temp_file.into_temp_path();
    let temp_path_string = temp_path.to_string_lossy().to_string();

    let command = format!(
        "openssl x509 -in {} -noout -fingerprint -sha256",
        temp_path_string
    );

    let fingerprint = run_shell_command(&command)?.replace("\n", "");

    Ok(fingerprint)
}
