// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::signatures::shell::run_shell_command;
use crate::util::temp_path::generate_temp_file;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;
use strand::hash::hash_sha256;
use tempfile::tempdir;
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
        "java -jar {} sign {} {}",
        ECIES_TOOL_PATH, temp_pem_file_string, temp_data_file_string
    );

    let encrypted_base64 = run_shell_command(&command)?.replace("\n", "");

    info!("ecies_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}

// A struct you can use to keep track of each item you want to sign
pub struct SignRequest {
    pub id: String,   // or any key you want, to correlate back
    pub data: String, // the sign_data string
}

#[instrument(skip_all, err)]
pub fn ecies_sign_data_bulk(
    acm_key_pair: &EciesKeyPair,
    requests: &[SignRequest],
) -> Result<HashMap<String, String>> {
    // If there are no requests, just return an empty map
    if requests.is_empty() {
        return Ok(HashMap::new());
    }

    // 1. Create a temporary directory (folder) for bulk-signing
    let tmp_dir = tempdir().context("Failed to create temporary directory")?;

    // 2. Write the private key into that directory, e.g. private_key.pem
    let private_key_path = tmp_dir.path().join("private_key.pem");
    {
        let mut key_file = File::create(&private_key_path)
            .context("Failed to create private_key.pem")?;
        key_file
            .write_all(acm_key_pair.private_key_pem.as_bytes())
            .context("Failed to write private_key.pem")?;
    }

    // 3. For each request, create a file with the sign_data content E.g.
    //    "sign_0001.txt", "sign_0002.txt", etc. We'll store a small structure
    //    to track (id -> filename).
    let mut file_map: HashMap<String, PathBuf> = HashMap::new();
    for (i, req) in requests.iter().enumerate() {
        let filename = format!("sign_{:04}.txt", i);
        let path = tmp_dir.path().join(&filename);

        {
            let mut f = File::create(&path)
                .with_context(|| format!("Failed to create {}", filename))?;
            f.write_all(req.data.as_bytes())
                .with_context(|| format!("Failed to write {}", filename))?;
        }

        file_map.insert(req.id.clone(), path);
    }

    // 4. Build the sign-bulk command (one call). sign-bulk <private_key_file>
    //    <folder_to_sign>
    //
    //    The second parameter is just the directory path;
    //    the Java tool will sign every file that doesn't end with .sign
    let cmd = format!(
        "java -jar {ecies_tool_path} sign-bulk {key} {folder}",
        ecies_tool_path = ECIES_TOOL_PATH,
        key = private_key_path.to_string_lossy(),
        folder = tmp_dir.path().to_string_lossy(),
    );
    info!("Running sign-bulk => {}", cmd);

    // 5. Execute the shell command (similar to your run_shell_command).
    run_shell_command(&cmd)?;

    // 6. After sign-bulk finishes, we collect the results: For each
    //    sign_xxxx.txt, the tool should have produced sign_xxxx.txt.sign We'll
    //    read them into a map of (id -> signature_base64)
    let mut signature_map = HashMap::new();
    for (id, path) in file_map.iter() {
        // the Java tool will create the file with .sign appended
        let sign_file = path.with_extension("txt.sign");
        if !sign_file.exists() {
            return Err(anyhow::anyhow!(
                "Expected signature file not found: {}",
                sign_file.display()
            ));
        }
        let signature_b64 = std::fs::read_to_string(&sign_file)
            .with_context(|| {
                format!("Failed to read signature file {}", sign_file.display())
            })?
            .trim()
            .to_string();

        signature_map.insert(id.clone(), signature_b64);
    }

    // 7. Return all signatures in a HashMap keyed by the "id"
    Ok(signature_map)
}
