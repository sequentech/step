// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use openssl::rsa::{Padding, Rsa};
use sequent_core::signatures::ecies_encrypt::ecies_tool_command;
use sequent_core::signatures::shell::run_command;
use std::process::Command;
use tracing::{info, instrument};

// Function to generate RSA public/private key pair in PEM format
#[instrument(skip_all, err)]
pub fn generate_rsa_keys() -> Result<(String, String)> {
    // Generate a 2048-bit RSA key pair
    let rsa = Rsa::generate(2048).context("Failed to generate RSA key pair")?;

    // Extract private key in PEM format
    let private_key_pem = rsa
        .private_key_to_pem()
        .context("Failed to convert private key to PEM format")?;
    let private_key_pem = String::from_utf8(private_key_pem)
        .context("Failed to convert private key PEM to string")?;

    // Extract public key in PEM format
    let public_key_pem = rsa
        .public_key_to_pem()
        .context("Failed to convert public key to PEM format")?;
    let public_key_pem =
        String::from_utf8(public_key_pem).context("Failed to convert public key PEM to string")?;

    Ok((public_key_pem, private_key_pem))
}

// Function to encrypt data using the RSA private key extracted from a private key PEM string
#[instrument(skip_all, err)]
pub fn encrypt_with_rsa_private_key(private_key_pem: &str, data: &[u8]) -> Result<Vec<u8>> {
    // Parse the private key PEM string to get the RSA structure
    let rsa = Rsa::private_key_from_pem(private_key_pem.as_bytes())
        .context("Failed to parse private key from PEM format")?;

    // Create a buffer to hold the encrypted data
    let mut encrypted_data = vec![0; rsa.size() as usize];

    // Encrypt the data using the RSA private key
    let encrypted_len = rsa
        .private_encrypt(data, &mut encrypted_data, Padding::PKCS1)
        .context("Failed to encrypt data using the private key")?;

    // Trim the encrypted data buffer to the actual size of the encrypted data
    encrypted_data.truncate(encrypted_len);

    Ok(encrypted_data)
}

fn public_key_command(pk12_file_path_string: &str, password: &str) -> Command {
    ecies_tool_command(&["public-key", pk12_file_path_string, password])
}

#[instrument(skip(password), err)]
pub fn derive_public_key_from_p12(pk12_file_path_string: &str, password: &str) -> Result<String> {
    let public_pem =
        run_command(public_key_command(pk12_file_path_string, password))?.replace("\n\n", "\n");

    info!("public pem: '{}'", public_pem);

    Ok(public_pem)
}

fn rsa_sign_command(pk12_file_path_string: &str, password: &str, data_path: &str) -> Command {
    ecies_tool_command(&["sign-rsa", pk12_file_path_string, data_path, password])
}

#[instrument(skip_all, err)]
pub fn rsa_sign_data(
    pk12_file_path_string: &str,
    password: &str,
    data_path: &str,
) -> Result<String> {
    let encrypted_base64 =
        run_command(rsa_sign_command(pk12_file_path_string, password, data_path))?
            .replace("\n", "");

    info!("ecies_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::signatures::ecies_encrypt::ECIES_TOOL_PATH;

    const HOSTILE_PASSWORD: &str = "pass word; rm -rf / $(touch pwned) `id` | cat";

    #[test]
    fn test_rsa_sign_command_passes_password_as_single_argument() {
        let command = rsa_sign_command("/tmp/key.p12", HOSTILE_PASSWORD, "/tmp/data");
        assert_eq!(command.get_program(), "java");
        let args: Vec<&std::ffi::OsStr> = command.get_args().collect();
        assert_eq!(
            args,
            vec![
                "-jar",
                ECIES_TOOL_PATH,
                "sign-rsa",
                "/tmp/key.p12",
                "/tmp/data",
                HOSTILE_PASSWORD
            ]
        );
    }

    #[test]
    fn test_public_key_command_passes_password_as_single_argument() {
        let command = public_key_command("/tmp/key.p12", HOSTILE_PASSWORD);
        assert_eq!(command.get_program(), "java");
        let args: Vec<&std::ffi::OsStr> = command.get_args().collect();
        assert_eq!(
            args,
            vec![
                "-jar",
                ECIES_TOOL_PATH,
                "public-key",
                "/tmp/key.p12",
                HOSTILE_PASSWORD
            ]
        );
    }
}
