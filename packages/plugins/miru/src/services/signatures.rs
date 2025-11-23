// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_core::plugins::{get_plugin_shared_dir, Plugins};
use sequent_core::std_temp_path::create_temp_file;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
use tracing::instrument;

use crate::bindings::plugins_manager::documents_manager::documents::{
    run_shell_command_ecies_encrypt_string, run_shell_command_ecies_sign_data,
    run_shell_command_generate_ecies_key_pair,
};

pub const ECIES_TOOL_PATH: &str = "/usr/local/bin/ecies-tool.jar";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EciesKeyPair {
    pub private_key_pem: String,
    pub public_key_pem: String,
}

#[instrument(err)]
pub fn generate_ecies_key_pair() -> Result<EciesKeyPair, String> {
    let base_path = get_plugin_shared_dir(&Plugins::MIRU);

    let (temp_private_pem_file, temp_private_pem_file_name) =
        create_temp_file("private_key", ".pem", &base_path)?;
    let temp_private_pem_file_path = temp_private_pem_file.path.as_path();

    let (temp_public_pem_file, temp_public_pem_file_name) =
        create_temp_file("public_key", ".pem", &base_path)?;
    let temp_public_pem_file_path = temp_public_pem_file.path.as_path();

    run_shell_command_generate_ecies_key_pair(
        ECIES_TOOL_PATH,
        &temp_public_pem_file_name,
        &temp_private_pem_file_name,
    );

    let private_key_pem =
        fs::read_to_string(temp_private_pem_file_path).map_err(|e| e.to_string())?;
    let public_key_pem =
        fs::read_to_string(temp_public_pem_file_path).map_err(|e| e.to_string())?;

    println!("generate_ecies_key_pair(): public_key_pem: {public_key_pem:?}");

    Ok(EciesKeyPair {
        private_key_pem: private_key_pem,
        public_key_pem: public_key_pem,
    })
}

#[instrument(skip(password), err)]
pub fn ecies_encrypt_string(
    public_key_pem: &str,
    password: &str,
    base_path: &str,
) -> Result<String, String> {
    let (temp_pem_file, temp_pem_file_name) = create_temp_file("public_key", ".pem", base_path)?;
    let temp_pem_file_path = temp_pem_file.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    // Using brackets: let it drop out of scope so that all bytes are written
    {
        let mut output_file = File::create(temp_pem_file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        output_file
            .write_all(public_key_pem.as_bytes())
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }
    // Encode the &[u8] to a Base64 string
    let result =
        run_shell_command_ecies_encrypt_string(ECIES_TOOL_PATH, &temp_pem_file_name, password)?;

    println!("ecies_encrypt_string: '{}'", result);

    Ok(result)
}

#[instrument(skip(data), err)]
pub fn ecies_sign_data(
    acm_key_pair: &EciesKeyPair,
    data: &str,
    base_path: &str,
) -> Result<String, String> {
    // Retrieve the PEM as a string
    println!("pem: {}", acm_key_pair.private_key_pem);

    let (temp_pem_file, temp_pem_file_name) = create_temp_file("private_key", ".pem", base_path)?;
    let temp_pem_file_path = temp_pem_file.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    // Using brackets: let it drop out of scope so that all bytes are written
    {
        let mut output_file = File::create(temp_pem_file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        output_file
            .write_all(acm_key_pair.private_key_pem.as_bytes())
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }
    let (temp_data_file, temp_data_file_name) = create_temp_file("data", ".eml", base_path)?;
    let temp_data_file_path = temp_data_file.path();
    let temp_data_file_string = temp_data_file_path.to_string_lossy().to_string();
    // Write the salt and encrypted data to the output file
    {
        let mut output_file = File::create(temp_data_file_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        output_file
            .write_all(data.as_bytes())
            .map_err(|e| format!("Failed to write file: {}", e))?;
    }

    let encrypted_base64 = run_shell_command_ecies_sign_data(
        ECIES_TOOL_PATH,
        &temp_pem_file_name,
        &temp_data_file_name,
    )?;

    println!("ecies_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}
