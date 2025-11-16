// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use openssl::pkcs12::Pkcs12;
use openssl::pkey::PKey;
use sequent_core::signatures::ecies_encrypt::ECIES_TOOL_PATH;
use sequent_core::signatures::shell::run_shell_command;
use sequent_core::util::temp_path::*;
use std::fs;
use std::io::Read;
use tempfile::{tempdir, NamedTempFile, TempPath};
use tracing::{info, instrument};

#[instrument(skip_all, err)]
pub fn get_pk12_id(p12_path: &str, password: &str) -> Result<openssl::pkey::Id> {
    // Read the .p12 file
    let mut file = fs::File::open(p12_path)?;
    let mut p12_data = Vec::new();
    file.read_to_end(&mut p12_data)?;

    // Parse the .p12 file
    let pkcs12 = Pkcs12::from_der(&p12_data)?;
    let parsed = pkcs12.parse2(password)?;
    let pkey = parsed.pkey.ok_or(anyhow!("Can't find pkey"))?;

    // Return the key type
    Ok(pkey.id())
}

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

pub fn get_p12_cert(p12_file: &NamedTempFile, password: &str) -> Result<TempPath> {
    let p12_file_path = p12_file.path().to_string_lossy().to_string();
    let cert_temp_file =
        generate_temp_file("p12", "cert").with_context(|| "Error creating temp file")?;
    let cert_temp_path = cert_temp_file.into_temp_path();
    let cert_temp_path_string = cert_temp_path.to_string_lossy().to_string();

    let cert_command = format!(
        "openssl pkcs12 -in {} -passin pass:{} -nokeys -out {}",
        p12_file_path, password, cert_temp_path_string
    );
    run_shell_command(&cert_command)?;

    Ok(cert_temp_path)
}

#[instrument(err, ret)]
pub fn get_p12_fingerprint(p12_cert_path: &TempPath) -> Result<String> {
    let cert_temp_path_string = p12_cert_path.to_string_lossy().to_string();

    let fingerprint_command = format!(
        "openssl x509 -in {} -noout -fingerprint -sha256",
        cert_temp_path_string
    );

    let fingerprint = run_shell_command(&fingerprint_command)?.replace("\n", "");

    Ok(fingerprint)
}

#[instrument(skip_all, err)]
pub fn check_certificate_cas(
    p12_cert_path: &TempPath,
    root_ca: &str,
    intermediate_cas: &str,
) -> Result<()> {
    // Create a temporary directory
    let temp_dir = tempdir()?;

    // Get the path to the temporary directory
    let temp_dir_path = temp_dir.path().to_path_buf();

    // write root ca
    let root_ca_file_path = temp_dir_path.join("root-ca.cer");
    fs::write(root_ca_file_path.clone(), root_ca)?;

    // write root ca
    let intermediate_ca_file_path = temp_dir_path.join("intermediate-ca.cer");
    fs::write(intermediate_ca_file_path.clone(), intermediate_cas)?;

    let verify_command = format!(
        "openssl verify -CAfile {} -untrusted {} {}",
        root_ca_file_path.to_string_lossy().to_string(),
        intermediate_ca_file_path.to_string_lossy().to_string(),
        p12_cert_path.to_string_lossy().to_string(),
    );
    let verify_result = run_shell_command(&verify_command)?.replace("\n", "");

    if !verify_result.ends_with(": OK") {
        return Err(anyhow!(verify_result));
    }

    Ok(())
}
