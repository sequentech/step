// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::Private;
use strand::hash::hash_sha256;
use tracing::{info, instrument};
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::process::Command;
use crate::services::temp_path::generate_temp_file;
use std::fs;

#[derive(Debug, Clone)]
pub struct EciesKeyPair {
    pub private_key_pem: String,
    pub public_key_pem: String,
}

#[instrument(err)]
fn run_shell_command(command: &str) -> Result<String> {
    // Run the shell command
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()?;

    // Check if the command was successful
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Shell command failed: {}", stderr));
    }

    // Convert the output to a string
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Return the output
    Ok(stdout.to_string())
}

#[instrument(skip(password, acm_key_pair), err)]
pub fn ecies_encrypt_string(
    public_key_pem: &str,
    acm_key_pair: &EciesKeyPair,
    password: &[u8],
) -> Result<String> {
    let temp_pem_file = generate_temp_file("public_key", ".pem")?;
    let temp_pem_file_path = temp_pem_file.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    {
        let mut output_file = File::create(temp_pem_file_path).context("Failed to create file")?;
        output_file
            .write_all(public_key_pem.as_bytes())
            .context("Failed to write file")?;
    }
    // Encode the &[u8] to a Base64 string
    let plaintext_b64 = STANDARD.encode(password);

    info!("plaintext b64: '{}'", plaintext_b64);

    let command = format!("java -jar /app/windmill/external-bin/ecies-tool.jar encrypt {} {}", temp_pem_file_string, plaintext_b64);

    let result = run_shell_command(&command)?.replace("\n", "");

    info!("ecies_encrypt_string: '{}'", result);

    Ok(result)
}

#[instrument(err)]
pub fn generate_ecies_key_pair() -> Result<EciesKeyPair> {
    let temp_private_pem_file = generate_temp_file("private_key", ".pem")?;
    let temp_private_pem_file_path = temp_private_pem_file.path();
    let temp_private_pem_file_string = temp_private_pem_file_path.to_string_lossy().to_string();

    let temp_public_pem_file = generate_temp_file("public_key", ".pem")?;
    let temp_public_pem_file_path = temp_public_pem_file.path();
    let temp_public_pem_file_string = temp_public_pem_file_path.to_string_lossy().to_string();

    let command = format!("java -jar /app/windmill/external-bin/ecies-tool.jar create-keys {} {}", temp_public_pem_file_string, temp_private_pem_file_string);
    run_shell_command(&command)?;

    let private_key_pem = fs::read_to_string(temp_private_pem_file_path)?;
    let public_key_pem = fs::read_to_string(temp_private_pem_file_path)?;

    Ok(EciesKeyPair {
        private_key_pem: private_key_pem,
        public_key_pem: public_key_pem,
    })
}

#[instrument(skip(data), err)]
pub fn ecies_sign_data(
    acm_key_pair: &EciesKeyPair,
    data: &[u8],
) -> Result<(String, String)> {
    // Retrieve the PEM as a string
    info!("pem: {}", acm_key_pair.private_key_pem);

    let temp_pem_file = generate_temp_file("private_key", ".pem")?;
    let temp_pem_file_path = temp_pem_file.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    {
        let mut output_file = File::create(temp_pem_file_path).context("Failed to create file")?;
        output_file
            .write_all(acm_key_pair.private_key_pem.as_bytes())
            .context("Failed to write file")?;
    }
    let temp_data_file = generate_temp_file("data", ".exz")?;
    let temp_data_file_path = temp_data_file.path();
    let temp_data_file_string = temp_data_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    {
        let mut output_file = File::create(temp_data_file_path).context("Failed to create file")?;
        output_file
            .write_all(data)
            .context("Failed to write file")?;
    }

    let command = format!("java -jar /app/windmill/external-bin/ecies-tool.jar sign {} {}", temp_pem_file_string, temp_data_file_string);

    let encrypted_base64 = run_shell_command(&command)?.replace("\n", "");

    info!("ecies_sign_data: '{}'", encrypted_base64);

    let hash_bytes = hash_sha256(data)?;
    let hex_string: String = hash_bytes.iter().map(|byte| format!("{:02X}", byte)).collect();

    Ok((hex_string, encrypted_base64))
}
