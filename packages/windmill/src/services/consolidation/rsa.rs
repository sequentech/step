// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{Context, Result};
use openssl::rsa::{Padding, Rsa};
use sequent_core::signatures::ecies_encrypt::ECIES_TOOL_PATH;
use sequent_core::signatures::shell::run_shell_command;
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

pub fn derive_public_key_from_p12(pk12_file_path_string: &str, password: &str) -> Result<String> {
    let command = format!(
        "java -jar {} public-key {} {}",
        ECIES_TOOL_PATH, pk12_file_path_string, password
    );

    let public_pem = run_shell_command(&command)?.replace("\n\n", "\n");

    info!("public pem: '{}'", public_pem);

    Ok(public_pem)
}

#[instrument(skip_all, err)]
pub fn rsa_sign_data(
    pk12_file_path_string: &str,
    password: &str,
    data_path: &str,
) -> Result<String> {
    let command = format!(
        "java -jar {} sign-rsa {} {} {}",
        ECIES_TOOL_PATH, pk12_file_path_string, data_path, password
    );

    let encrypted_base64 = run_shell_command(&command)?.replace("\n", "");

    info!("ecies_sign_data: '{}'", encrypted_base64);

    Ok(encrypted_base64)
}
