// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::signatures::shell::run_shell_command;
use crate::util::temp_path::generate_temp_file;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use strand::hash::hash_sha256;
use tracing::{info, instrument};

pub const ECIES_TOOL_PATH: &str = "/usr/local/bin/ecies-tool.jar";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EciesKeyPair {
    pub private_key_pem: String,
    pub public_key_pem: String,
}

#[instrument(skip(password), err)]
pub fn ecies_encrypt_string(
    public_key_pem: &str,
    password: &str,
) -> Result<String> {
    let temp_pem_file = generate_temp_file("public_key", ".pem")?;
    let temp_pem_file_path = temp_pem_file.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    // Using brackets: let it drop out of scope so that all bytes are written
    {
        let mut output_file = File::create(temp_pem_file_path)
            .context("Failed to create file")?;
        output_file
            .write_all(public_key_pem.as_bytes())
            .context("Failed to write file")?;
    }
    // Encode the &[u8] to a Base64 string

    let command = format!(
        "java -jar {} encrypt {} {}",
        ECIES_TOOL_PATH, temp_pem_file_string, password
    );
    info!("command: '{}'", command);

    let result = run_shell_command(&command)?.replace("\n", "");

    info!("ecies_encrypt_string: '{}'", result);

    Ok(result)
}

#[instrument(err)]
pub fn generate_ecies_key_pair() -> Result<EciesKeyPair> {
    let temp_private_pem_file = generate_temp_file("private_key", ".pem")?;
    let temp_private_pem_file_path = temp_private_pem_file.path();
    let temp_private_pem_file_string =
        temp_private_pem_file_path.to_string_lossy().to_string();

    let temp_public_pem_file = generate_temp_file("public_key", ".pem")?;
    let temp_public_pem_file_path = temp_public_pem_file.path();
    let temp_public_pem_file_string =
        temp_public_pem_file_path.to_string_lossy().to_string();

    let command = format!(
        "java -jar {} create-keys {} {}",
        ECIES_TOOL_PATH,
        temp_public_pem_file_string,
        temp_private_pem_file_string
    );
    run_shell_command(&command)?;

    let private_key_pem = fs::read_to_string(temp_private_pem_file_path)?;
    let public_key_pem = fs::read_to_string(temp_public_pem_file_string)?;

    info!("generate_ecies_key_pair(): public_key_pem: {public_key_pem:?}");

    Ok(EciesKeyPair {
        private_key_pem: private_key_pem,
        public_key_pem: public_key_pem,
    })
}

#[instrument(skip(data), err)]
pub fn ecies_sign_data(
    acm_key_pair: &EciesKeyPair,
    data: &str,
) -> Result<String> {
    // Retrieve the PEM as a string
    info!("pem: {}", acm_key_pair.private_key_pem);

    let temp_pem_file = generate_temp_file("private_key", ".pem")?;
    let temp_pem_file_path = temp_pem_file.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    // Using brackets: let it drop out of scope so that all bytes are written
    {
        let mut output_file = File::create(temp_pem_file_path)
            .context("Failed to create file")?;
        output_file
            .write_all(acm_key_pair.private_key_pem.as_bytes())
            .context("Failed to write file")?;
    }
    let temp_data_file = generate_temp_file("data", ".eml")?;
    let temp_data_file_path = temp_data_file.path();
    let temp_data_file_string =
        temp_data_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    {
        let mut output_file = File::create(temp_data_file_path)
            .context("Failed to create file")?;
        output_file
            .write_all(data.as_bytes())
            .context("Failed to write file")?;
    }

    let command = format!( 
        ECIES_TOOL_PATH, temp_pem_file_string, temp_data_file_string
    );

    let encrypted_base64 = run_shell_command(&command)?.replace("\n", "");

    info!("ecies_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}
