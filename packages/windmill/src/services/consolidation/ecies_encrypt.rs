// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use ecies::encrypt;
use openssl::ec::{EcGroup, EcKey};
use openssl::nid::Nid;
use openssl::pkey::PKey;
use strand::hash::hash_sha256;

pub fn ecies_encrypt_string(public_key_pem: &str, password: &[u8]) -> Result<String> {
    // Parse the PEM file and extract the public key
    let public_key = PKey::public_key_from_pem(public_key_pem.as_bytes())
        .context("Failed to parse PEM and extract public key")?;

    let public_key_der = public_key.public_key_to_der()?;

    // Encrypt the password
    let encrypted_data =
        encrypt(&public_key_der, password).context("Failed to encrypt the password")?;

    // Encode the encrypted data in base64
    let encrypted_base64 = STANDARD.encode(&encrypted_data);

    Ok(encrypted_base64)
}

pub fn generate_ecies_key_pair() -> Result<(String, String)> {
    // Create an elliptic curve group using the secp256k1 curve
    let group = EcGroup::from_curve_name(Nid::SECP256K1)
        .with_context(|| "Failed to create elliptic curve group for secp256k1")?;

    // Generate an EC key pair
    let ec_key = EcKey::generate(&group).with_context(|| "Failed to generate EC key pair")?;

    // Convert the private key to PEM format
    let private_key_pem = ec_key
        .private_key_to_pem()
        .with_context(|| "Failed to convert private key to PEM format")?;
    let private_key_pem_str = String::from_utf8(private_key_pem)
        .with_context(|| "Failed to convert private key PEM to UTF-8 string")?;

    // Convert the public key to PEM format
    let public_key_pem = ec_key
        .public_key_to_pem()
        .with_context(|| "Failed to convert public key to PEM format")?;
    let public_key_pem_str = String::from_utf8(public_key_pem)
        .with_context(|| "Failed to convert public key PEM to UTF-8 string")?;

    Ok((private_key_pem_str, public_key_pem_str))
}

pub fn ecies_sign_data(public_key_pem_str: &str, data: &[u8]) -> Result<(String, String)> {
    let hash_bytes = hash_sha256(data)?;
    let sha256_hash_base64 = STANDARD.encode(hash_bytes.clone());

    let encrypted_data = ecies_encrypt_string(public_key_pem_str, &hash_bytes)?;
    // Encode the encrypted data in base64
    let encrypted_base64 = STANDARD.encode(&encrypted_data);

    Ok((sha256_hash_base64, encrypted_base64))
}
