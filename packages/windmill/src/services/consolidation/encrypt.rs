// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use ecies::encrypt;
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};
use std::fs::File;
use std::io::{Read, Write};

const OPENSSL_ENCRYPT_ITERATION_COUNT: i32 = 10000;
const OPENSSL_SALT_BYTES: usize = 8;
const OPENSSL_SALT_HEADER: &[u8; 8] = b"Salted__";

// used to recreate this command:
// openssl enc -aes-256-cbc -e -in $input_file_path -out $output_file_path -pass pass:$password -md md5
pub fn encrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    // Initialize the cipher
    let cipher = Cipher::aes_256_cbc();

    // Generate a random salt
    let mut salt = [0u8; OPENSSL_SALT_BYTES];
    rand_bytes(&mut salt).context("Failed to generate random salt")?;

    // Derive the key and IV from the password using MD5
    let key_iv = openssl::pkcs5::bytes_to_key(
        cipher,
        MessageDigest::md5(),
        &salt,
        Some(password.as_bytes()),
        OPENSSL_ENCRYPT_ITERATION_COUNT, // Iteration count
    )
    .context("Failed to derive key and IV")?;

    let key = key_iv.key;
    let iv = key_iv.iv.context("Failed to derive IV")?;

    if key.len() != cipher.key_len() {
        return Err(anyhow!(
            "key len {} doesn't match cipher key len {}",
            key.len(),
            cipher.key_len()
        ));
    }

    if Some(iv.len()) != cipher.iv_len() {
        return Err(anyhow!(
            "iv len {} doesn't match cipher iv len {:?}",
            iv.len(),
            cipher.iv_len()
        ));
    }

    // Create a Crypter for encryption
    let mut crypter =
        Crypter::new(cipher, Mode::Encrypt, &key, Some(&iv)).context("Failed to create Crypter")?;
    crypter.pad(true);

    // Read the input file
    let mut input_file = File::open(input_file_path).context("Failed to open input file")?;
    let mut input_data = Vec::new();
    input_file
        .read_to_end(&mut input_data)
        .context("Failed to read input file")?;

    // Encrypt the data
    let mut output_data = vec![0; input_data.len() + cipher.block_size()];
    let count = crypter
        .update(&input_data, &mut output_data)
        .context("Failed to encrypt data")?;
    let rest = crypter
        .finalize(&mut output_data[count..])
        .context("Failed to finalize encryption")?;
    output_data.truncate(count + rest);

    // Write the salt and encrypted data to the output file
    let mut output_file = File::create(output_file_path).context("Failed to create output file")?;
    output_file
        .write_all(OPENSSL_SALT_HEADER)
        .context("Failed to write salt header")?; // Write OpenSSL's salt header
    output_file
        .write_all(&salt)
        .context("Failed to write salt")?; // Store the salt at the beginning of the file
    output_file
        .write_all(&output_data)
        .context("Failed to write encrypted data")?;

    Ok(())
}

pub fn encrypt_password(public_key_pem: &str, password: &str) -> Result<String> {
    // Parse the PEM file and extract the public key
    let public_key = PKey::public_key_from_pem(public_key_pem.as_bytes())
        .context("Failed to parse PEM and extract public key")?;

    let public_key_der = public_key.public_key_to_der()?;

    // Encrypt the password
    let encrypted_data =
        encrypt(&public_key_der, password.as_bytes()).context("Failed to encrypt the password")?;

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
