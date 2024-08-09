// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use openssl::symm::{Cipher, Crypter, Mode};
use openssl::hash::MessageDigest;
use openssl::rand::rand_bytes;
use std::fs::File;
use std::io::{Read, Write};


fn encrypt_file(input_file_path: &str, output_file_path: &str, password: &str) -> std::io::Result<()> {
    // Initialize the cipher
    let cipher = Cipher::aes_256_cbc();

    // Generate a random salt
    let mut salt = [0u8; 8];
    rand_bytes(&mut salt).unwrap();

    // Derive the key and IV from the password using MD5
    let key_iv = openssl::pkcs5::bytes_to_key(
        cipher,
        MessageDigest::md5(),
        &salt,
        Some(password.as_bytes()),
        1, // Iteration count
    ).expect("Failed to derive key and IV");

    let key = key_iv.key;
    let iv = key_iv.iv.expect("Failed to derive IV");

    // Create a Crypter for encryption
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, &key, Some(&iv)).unwrap();
    crypter.pad(true);

    // Read the input file
    let mut input_file = File::open(input_file_path)?;
    let mut input_data = Vec::new();
    input_file.read_to_end(&mut input_data)?;

    // Encrypt the data
    let mut output_data = vec![0; input_data.len() + cipher.block_size()];
    let count = crypter.update(&input_data, &mut output_data).unwrap();
    let rest = crypter.finalize(&mut output_data[count..]).unwrap();
    output_data.truncate(count + rest);

    // Write the salt and encrypted data to the output file
    let mut output_file = File::create(output_file_path)?;
    output_file.write_all(b"Salted__")?; // Write OpenSSL's salt header
    output_file.write_all(&salt)?; // Store the salt at the beginning of the file
    output_file.write_all(&output_data)?;

    Ok(())
}
