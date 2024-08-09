// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use openssl::hash::MessageDigest;
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};
use std::fs::File;
use std::io::{Read, Write};

const OPENSSL_ENCRYPT_ITERATION_COUNT: i32 = 10000;
const OPENSSL_SALT_BYTES: usize = 8;
const OPENSSL_SALT_HEADER: &[u8; 8] = b"Salted__";

fn encrypt_file(input_file_path: &str, output_file_path: &str, password: &str) -> Result<()> {
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
