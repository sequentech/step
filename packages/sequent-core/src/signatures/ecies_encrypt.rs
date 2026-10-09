// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::signatures::shell::{build_command, run_command};
use crate::util::temp_path::generate_temp_file;
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::path::PathBuf;
use std::process::Command;
use strand::hash::hash_sha256;
use tempfile::tempdir;
use tracing::{info, instrument};

pub const ECIES_TOOL_PATH: &str = "/usr/local/bin/ecies-tool.jar";
#[derive(Clone, Serialize, Deserialize)]
pub struct EciesKeyPair {
    pub private_key_pem: String,
    pub public_key_pem: String,
}

impl fmt::Debug for EciesKeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EciesKeyPair")
            .field("private_key_pem", &"<redacted>")
            .field("public_key_pem", &self.public_key_pem)
            .finish()
    }
}

/// Builds the command that runs the ECIES tool with the given arguments,
/// without a shell.
pub fn ecies_tool_command<S: AsRef<OsStr>>(args: &[S]) -> Command {
    let mut command = build_command(
        "java",
        &[OsStr::new("-jar"), OsStr::new(ECIES_TOOL_PATH)],
    );
    command.args(args);
    command
}

fn run_ecies_tool<S: AsRef<OsStr>>(args: &[S]) -> Result<String> {
    run_command(ecies_tool_command(args))
}

#[instrument(skip(password), err)]
pub fn ecies_encrypt_string(
    public_key_pem: &str,
    password: &str,
) -> Result<String> {
    let temp_pem_file = generate_temp_file("public_key", ".pem")?;
    let temp_pem_file_path = temp_pem_file.path();
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

    let result = run_ecies_tool(&[
        OsStr::new("encrypt"),
        temp_pem_file_path.as_os_str(),
        OsStr::new(password),
    ])?
    .replace("\n", "");

    info!("ecies_encrypt_string: '{}'", result);

    Ok(result)
}

#[instrument(err)]
pub fn generate_ecies_key_pair() -> Result<EciesKeyPair> {
    let temp_private_pem_file = generate_temp_file("private_key", ".pem")?;
    let temp_private_pem_file_path = temp_private_pem_file.path();

    let temp_public_pem_file = generate_temp_file("public_key", ".pem")?;
    let temp_public_pem_file_path = temp_public_pem_file.path();

    run_ecies_tool(&[
        OsStr::new("create-keys"),
        temp_public_pem_file_path.as_os_str(),
        temp_private_pem_file_path.as_os_str(),
    ])?;

    let private_key_pem = fs::read_to_string(temp_private_pem_file_path)?;
    let public_key_pem = fs::read_to_string(temp_public_pem_file_path)?;

    info!("generate_ecies_key_pair(): public_key_pem: {public_key_pem:?}");

    Ok(EciesKeyPair {
        private_key_pem: private_key_pem,
        public_key_pem: public_key_pem,
    })
}

#[instrument(skip(acm_key_pair, data), err)]
pub fn ecies_sign_data(
    acm_key_pair: &EciesKeyPair,
    data: &str,
) -> Result<String> {
    let temp_pem_file = generate_temp_file("private_key", ".pem")?;
    let temp_pem_file_path = temp_pem_file.path();
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
    // Write the salt and encrypted data to the output file
    {
        let mut output_file = File::create(temp_data_file_path)
            .context("Failed to create file")?;
        output_file
            .write_all(data.as_bytes())
            .context("Failed to write file")?;
    }

    let encrypted_base64 = run_ecies_tool(&[
        OsStr::new("sign"),
        temp_pem_file_path.as_os_str(),
        temp_data_file_path.as_os_str(),
    ])?
    .replace("\n", "");

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

    // 4. Run sign-bulk in one call: sign-bulk <private_key_file>
    //    <folder_to_sign>
    //
    //    The second parameter is just the directory path;
    //    the Java tool will sign every file that doesn't end with .sign
    run_ecies_tool(&[
        OsStr::new("sign-bulk"),
        private_key_path.as_os_str(),
        tmp_dir.path().as_os_str(),
    ])?;

    // 5. After sign-bulk finishes, we collect the results: For each
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

    // 6. Return all signatures in a HashMap keyed by the "id"
    Ok(signature_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_pair_debug_redacts_private_key() {
        let key_pair = EciesKeyPair {
            private_key_pem: "PRIVATE-KEY-PEM-CONTENT".to_string(),
            public_key_pem: "PUBLIC-KEY-PEM-CONTENT".to_string(),
        };
        let debug = format!("{:?}", key_pair);
        assert!(!debug.contains("PRIVATE-KEY-PEM-CONTENT"));
        assert!(debug.contains("PUBLIC-KEY-PEM-CONTENT"));
    }

    #[test]
    fn test_ecies_tool_command_passes_password_as_single_argument() {
        let password = "pass word; rm -rf / $(touch pwned) `id` | cat";
        let command =
            ecies_tool_command(&["encrypt", "/tmp/key.pem", password]);
        assert_eq!(command.get_program(), "java");
        let args: Vec<&std::ffi::OsStr> = command.get_args().collect();
        assert_eq!(
            args,
            vec!["-jar", ECIES_TOOL_PATH, "encrypt", "/tmp/key.pem", password]
        );
    }
}
