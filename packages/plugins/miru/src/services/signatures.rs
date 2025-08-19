// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sequent_core::std_temp_path::create_temp_file;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use tracing::instrument;

pub const ECIES_TOOL_PATH: &str = "/usr/local/bin/ecies-tool.jar";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EciesKeyPair {
    pub private_key_pem: String,
    pub public_key_pem: String,
}

//TODO: check if working inside the wasm
#[instrument(err, ret)]
pub fn run_shell_command(command: &str) -> Result<String, String> {
    // Run the shell command
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| e.to_string())?;

    // Check if the command was successful
    if !output.status.success() {
        return Err(format!("Shell command failed: {:?}", output));
    }

    // Convert the output to a string
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Return the output
    Ok(stdout.to_string())
}

#[instrument(err)]
pub fn generate_ecies_key_pair() -> Result<EciesKeyPair, String> {
    let (temp_private_pem_file, _filename) = create_temp_file("private_key", ".pem")?;
    let temp_private_pem_file_path = temp_private_pem_file.path.as_path();
    let temp_private_pem_file_string = temp_private_pem_file_path.to_string_lossy().to_string();

    let (temp_public_pem_file, _filename) = create_temp_file("public_key", ".pem")?;
    let temp_public_pem_file_path = temp_public_pem_file.path.as_path();
    let temp_public_pem_file_string = temp_public_pem_file_path.to_string_lossy().to_string();

    let command = format!(
        "java -jar {} create-keys {} {}",
        ECIES_TOOL_PATH, temp_public_pem_file_string, temp_private_pem_file_string
    );
    run_shell_command(&command)?;

    let private_key_pem =
        fs::read_to_string(temp_private_pem_file_path).map_err(|e| e.to_string())?;
    let public_key_pem =
        fs::read_to_string(temp_public_pem_file_string).map_err(|e| e.to_string())?;

    println!("generate_ecies_key_pair(): public_key_pem: {public_key_pem:?}");

    Ok(EciesKeyPair {
        private_key_pem: private_key_pem,
        public_key_pem: public_key_pem,
    })
}
