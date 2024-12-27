// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::shell::run_shell_command;
use crate::services::{
    consolidation::ecies_encrypt::ECIES_TOOL_PATH, temp_path::generate_temp_file,
};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::Write;
use tempfile::{tempdir, NamedTempFile};
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

#[instrument(skip_all, err)]
pub fn check_certificate_cas(
    p12_file: &NamedTempFile,
    password: &str,
    root_ca: &str,
    intermediate_cas: &str,
) -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;

    // Get the path to the temporary directory
    let temp_dir_path = temp_dir.path().to_path_buf();

    // Get path to p12 file
    let pk12_file_path = p12_file.path();

    // write password to file
    let password_file_path = temp_dir_path.join("password.txt");
    fs::write(password_file_path.clone(), password)?;

    // write root ca
    let root_ca_file_path = temp_dir_path.join("root-ca.cer");
    fs::write(root_ca_file_path.clone(), root_ca)?;

    // write root ca
    let intermediate_ca_file_path = temp_dir_path.join("intermediate-ca.cer");
    fs::write(intermediate_ca_file_path.clone(), intermediate_cas)?;

    let extracted_ca_file_path = temp_dir_path.join("extracted.crt");

    let extract_command = format!(
        "openssl pkcs12 -in {} -passin file:{} -nokeys -out {}",
        pk12_file_path.to_string_lossy().to_string(),
        password_file_path.to_string_lossy().to_string(),
        extracted_ca_file_path.to_string_lossy().to_string(),
    );
    run_shell_command(&extract_command)?.replace("\n", "");
    let verify_command = format!(
        "openssl verify -CAfile {} -untrusted {} {}",
        root_ca_file_path.to_string_lossy().to_string(),
        intermediate_ca_file_path.to_string_lossy().to_string(),
        extracted_ca_file_path.to_string_lossy().to_string(),
    );
    let verify_result = run_shell_command(&verify_command)?.replace("\n", "");

    if !verify_result.ends_with(": OK") {
        return Err(anyhow!(verify_result));
    }

    Ok(())
}
