// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use ecies::encrypt;
use openssl::bn::BigNum;
use openssl::bn::BigNumContext;
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcKey, PointConversionForm};
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::pkey::Private;
use openssl::symm::{Cipher, Crypter, Mode};
use strand::hash::hash_sha256;
use tracing::{info, instrument};
use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::process::Command;
use crate::services::temp_path::generate_temp_file;

#[derive(Debug, Clone)]
pub struct EciesKeyPair {
    pub private_key: EcKey<Private>,
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

    let plaintext = "6n0NWbzB1KhHMhbiW13QgSgiaEP8DYueYUJOru4HOhtqbWU7iGwjkULU1Zh8UPW";//std::str::from_utf8(password)?;
    info!("plaintext: '{}'", plaintext);

    let command = format!("java -jar /app/windmill/external-bin/ecies-tool.jar encrypt {} {}", temp_pem_file_string, plaintext);

    let result = run_shell_command(&command)?;

    info!("ecies_encrypt_string: '{}'", result);

    Ok(result)
/*
    
    // Parse the public key from PEM
    let public_key = EcKey::public_key_from_pem(public_key_pem.as_bytes())
        .context("Failed to parse PEM and extract EC key")?;

    // Convert the private key to a PKey<Private> type
    let ephemeral_pkey = PKey::from_ec_key(acm_key_pair.private_key.clone())
        .context("Failed to convert ephemeral private key to PKey")?;

    // Convert the public key to a PKey<Public> type
    let public_pkey =
        PKey::from_ec_key(public_key).context("Failed to convert public key to PKey")?;

    // Derive the shared secret using ECDH via Deriver
    let mut deriver = Deriver::new(&ephemeral_pkey).context("Failed to create Deriver")?;
    deriver
        .set_peer(&public_pkey)
        .context("Failed to set peer key for Deriver")?;
    let shared_secret = deriver
        .derive_to_vec()
        .context("Failed to derive shared secret")?;

    // Hash the shared secret to derive a symmetric key (e.g., using SHA-256)
    let derived_key = openssl::sha::sha256(&shared_secret);

    // Encrypt the password using AES-256-CBC
    let cipher = Cipher::aes_256_cbc();
    let iv = vec![0u8; cipher.iv_len().unwrap_or(16)];
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, &derived_key, Some(&iv))
        .context("Failed to create AES crypter")?;

    let mut ciphertext = vec![0; password.len() + cipher.block_size()];
    let mut count = crypter
        .update(password, &mut ciphertext)
        .context("Failed to encrypt data")?;
    count += crypter
        .finalize(&mut ciphertext[count..])
        .context("Failed to finalize encryption")?;
    ciphertext.truncate(count);

    // Encode the encrypted data in base64
    let encrypted_base64 = STANDARD.encode(&ciphertext);

    Ok(encrypted_base64)
*/
}

#[instrument(err)]
pub fn generate_ecies_key_pair() -> Result<EciesKeyPair> {
    // Create an elliptic curve group using the secp256r1 curve
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)
        .with_context(|| "Failed to create elliptic curve group for secp256r1")?;

    // Generate an EC key pair
    let ec_key = EcKey::generate(&group).with_context(|| "Failed to generate EC key pair")?;

    // Convert the public key to PEM format
    let public_key_pem = ec_key
        .public_key_to_pem()
        .with_context(|| "Failed to convert public key to PEM format")?;
    let public_key_pem_str = String::from_utf8(public_key_pem)
        .with_context(|| "Failed to convert public key PEM to UTF-8 string")?;

    Ok(EciesKeyPair {
        private_key: ec_key,
        public_key_pem: public_key_pem_str,
    })
}

#[instrument(skip(data), err)]
pub fn ecies_sign_data(
    public_key_pem_str: &str,
    acm_key_pair: &EciesKeyPair,
    data: &[u8],
) -> Result<(String, String)> {
    let hash_bytes = hash_sha256(data)?;
    let sha256_hash_base64 = STANDARD.encode(hash_bytes.clone());

    let encrypted_base64 = "".to_string();//ecies_encrypt_string(public_key_pem_str, acm_key_pair, &hash_bytes)?;

    Ok((sha256_hash_base64, encrypted_base64))
}
