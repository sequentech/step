// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::shell::run_shell_command;
use crate::services::temp_path::generate_temp_file;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::Private;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use strand::hash::hash_sha256;
use tracing::{info, instrument};

const ECIES_TOOL_PATH: &str = "/usr/local/bin/ecies-tool.jar";
#[derive(Debug, Clone)]
pub struct EciesKeyPair {
    pub private_key_pem: String,
    pub public_key_pem: String,
}

#[instrument(skip(password), err)]
pub fn ecies_encrypt_string(public_key_pem: &str, password: &[u8]) -> Result<String> {
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

    let command = format!(
        "java -jar {} encrypt {} {}",
        ECIES_TOOL_PATH, temp_pem_file_string, plaintext_b64
    );

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

    let command = format!(
        "java -jar {} create-keys {} {}",
        ECIES_TOOL_PATH, temp_public_pem_file_string, temp_private_pem_file_string
    );
    run_shell_command(&command)?;

    let private_key_pem = fs::read_to_string(temp_private_pem_file_path)?;
    let public_key_pem = fs::read_to_string(temp_private_pem_file_path)?;

    Ok(EciesKeyPair {
        private_key_pem: private_key_pem,
        public_key_pem: public_key_pem,
    })
}

#[instrument(skip(data), err)]
pub fn ecies_sign_data(acm_key_pair: &EciesKeyPair, data: &[u8]) -> Result<(String, String)> {
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

    let command = format!(
        "java -jar {} sign {} {}",
        ECIES_TOOL_PATH, temp_pem_file_string, temp_data_file_string
    );

    let encrypted_base64 = run_shell_command(&command)?.replace("\n", "");

    info!("ecies_sign_data: '{}'", encrypted_base64);

    let hash_bytes = hash_sha256(data)?;
    let hex_string: String = hash_bytes
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect();

    Ok((hex_string, encrypted_base64))
}
