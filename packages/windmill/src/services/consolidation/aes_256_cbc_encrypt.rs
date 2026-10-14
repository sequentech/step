// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Password-based AES-256-CBC encryption of export and archive files.
//!
//! The container is the OpenSSL `enc` format - the ASCII magic `Salted__`,
//! an 8-byte random salt, then the ciphertext with PKCS#7 padding - so that a
//! file can also be opened with the OpenSSL command line:
//!
//! `openssl enc -d -aes-256-cbc -pbkdf2 -iter 600000 -md sha256 -in <file> -out <plain> -pass pass:<password>`
//!
//! Key and IV are derived from the password and the salt with
//! PBKDF2-HMAC-SHA-256 and 600 000 iterations. Files written by earlier
//! releases derived them with OpenSSL's legacy `EVP_BytesToKey` (MD5, one
//! iteration); [`decrypt_file_aes_256_cbc`] still opens them by falling back
//! to that derivation when the PBKDF2 one does not yield valid padding.
//!
//! Everything runs in-process through the OpenSSL library (`openssl` crate):
//! no external program is started and the password never leaves memory.
type KeyDerivation = fn(&str, &[u8]) -> Result<KeyIv>;

use anyhow::{anyhow, Context, Result};
use openssl::hash::MessageDigest;
use openssl::pkcs5::{bytes_to_key, pbkdf2_hmac};
use openssl::rand::rand_bytes;
use openssl::symm::{Cipher, Crypter, Mode};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use tracing::{instrument, warn};

const MAGIC: &[u8; 8] = b"Salted__";
const SALT_LEN: usize = 8;
const KEY_LEN: usize = 32;
const IV_LEN: usize = 16;
const BUFFER_LEN: usize = 64 * 1024;

/// PBKDF2 iteration count, matching `openssl enc -pbkdf2 -iter 600000`.
pub const PBKDF2_ITERATIONS: usize = 600_000;

type KeyIv = ([u8; KEY_LEN], [u8; IV_LEN]);

fn derive_pbkdf2(password: &str, salt: &[u8]) -> Result<KeyIv> {
    let mut derived = [0u8; KEY_LEN + IV_LEN];
    pbkdf2_hmac(
        password.as_bytes(),
        salt,
        PBKDF2_ITERATIONS,
        MessageDigest::sha256(),
        &mut derived,
    )
    .context("PBKDF2 key derivation failed")?;
    let mut key = [0u8; KEY_LEN];
    let mut iv = [0u8; IV_LEN];
    key.copy_from_slice(&derived[..KEY_LEN]);
    iv.copy_from_slice(&derived[KEY_LEN..]);
    Ok((key, iv))
}

/// OpenSSL's legacy `EVP_BytesToKey` derivation (`openssl enc -md md5`
/// without `-pbkdf2`), kept only to read files written by earlier releases.
fn derive_legacy(password: &str, salt: &[u8]) -> Result<KeyIv> {
    let pair = bytes_to_key(
        Cipher::aes_256_cbc(),
        MessageDigest::md5(),
        password.as_bytes(),
        Some(salt),
        1,
    )
    .context("legacy key derivation failed")?;
    let iv_vec = pair
        .iv
        .ok_or_else(|| anyhow!("legacy key derivation produced no IV"))?;
    let mut key = [0u8; KEY_LEN];
    let mut iv = [0u8; IV_LEN];
    key.copy_from_slice(&pair.key);
    iv.copy_from_slice(&iv_vec);
    Ok((key, iv))
}

fn run_cipher<R: Read, W: Write>(
    mode: Mode,
    key_iv: &KeyIv,
    input: &mut R,
    output: &mut W,
) -> Result<()> {
    let cipher = Cipher::aes_256_cbc();
    let mut crypter = Crypter::new(cipher, mode, &key_iv.0, Some(&key_iv.1))
        .context("initialising AES-256-CBC")?;
    crypter.pad(true);
    let mut in_buf = vec![0u8; BUFFER_LEN];
    let mut out_buf = vec![0u8; BUFFER_LEN + cipher.block_size()];
    loop {
        let read = input.read(&mut in_buf).context("reading input")?;
        if read == 0 {
            break;
        }
        let written = crypter
            .update(&in_buf[..read], &mut out_buf)
            .context("AES-256-CBC update")?;
        output
            .write_all(&out_buf[..written])
            .context("writing output")?;
    }
    let written = crypter.finalize(&mut out_buf).map_err(|_| {
        anyhow!("AES-256-CBC finalisation failed: wrong password or corrupted data")
    })?;
    output
        .write_all(&out_buf[..written])
        .context("writing output")?;
    Ok(())
}

/// Encrypts `input_file_path` into `output_file_path` with AES-256-CBC, the
/// key and IV derived from `password` with PBKDF2-HMAC-SHA-256.
#[instrument(skip(password), err)]
pub fn encrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    let mut salt = [0u8; SALT_LEN];
    rand_bytes(&mut salt).context("generating the salt")?;
    let key_iv = derive_pbkdf2(password, &salt)?;

    let mut input = BufReader::new(
        File::open(input_file_path)
            .with_context(|| format!("opening {input_file_path} for encryption"))?,
    );
    let mut output = BufWriter::new(
        File::create(output_file_path).with_context(|| format!("creating {output_file_path}"))?,
    );
    output.write_all(MAGIC)?;
    output.write_all(&salt)?;
    run_cipher(Mode::Encrypt, &key_iv, &mut input, &mut output)
        .with_context(|| format!("encrypting {input_file_path} to {output_file_path}"))?;
    output.flush().context("flushing the encrypted file")?;
    Ok(())
}

/// Decrypts a file written by [`encrypt_file_aes_256_cbc`] (or by
/// `openssl enc -aes-256-cbc -pbkdf2 -iter 600000 -md sha256`) into
/// `output_file_path`. Files written by earlier releases with the legacy
/// derivation are decrypted as well.
#[instrument(skip(password), err)]
pub fn decrypt_file_aes_256_cbc(
    input_file_path: &str,
    output_file_path: &str,
    password: &str,
) -> Result<()> {
    let mut input = File::open(input_file_path)
        .with_context(|| format!("opening {input_file_path} for decryption"))?;
    let mut header = [0u8; MAGIC.len() + SALT_LEN];
    input
        .read_exact(&mut header)
        .with_context(|| format!("{input_file_path} is too short to be an encrypted file"))?;
    if &header[..MAGIC.len()] != MAGIC {
        return Err(anyhow!(
            "{input_file_path} is not an AES-256-CBC encrypted file"
        ));
    }
    let salt = &header[MAGIC.len()..];

    let attempts: [(&str, KeyDerivation); 2] = [
        ("PBKDF2-HMAC-SHA-256", derive_pbkdf2),
        ("legacy EVP_BytesToKey", derive_legacy),
    ];
    let mut last_error = None;
    for (index, (name, derive)) in attempts.iter().enumerate() {
        let key_iv = derive(password, salt)?;
        input
            .seek(SeekFrom::Start(header.len() as u64))
            .context("seeking to the ciphertext")?;
        let mut reader = BufReader::new(&mut input);
        let mut output = BufWriter::new(
            File::create(output_file_path)
                .with_context(|| format!("creating {output_file_path}"))?,
        );
        match run_cipher(Mode::Decrypt, &key_iv, &mut reader, &mut output)
            .and_then(|_| output.flush().context("flushing the decrypted file"))
        {
            Ok(()) => {
                if index > 0 {
                    warn!(
                        "{input_file_path} was encrypted with the {name} derivation of an earlier release; re-export it to obtain the current format"
                    );
                }
                return Ok(());
            }
            Err(err) => last_error = Some(err),
        }
    }
    let _ = std::fs::remove_file(output_file_path);
    Err(last_error.unwrap_or_else(|| anyhow!("decryption failed")))
        .with_context(|| format!("decrypting {input_file_path}: wrong password or corrupted file"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const PLAINTEXT: &[u8] = b"The quick brown fox jumps over the lazy dog";
    const PASSWORD: &str = "correct-horse";
    const SALT: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    // openssl enc -aes-256-cbc -e -pbkdf2 -iter 600000 -md sha256 -S 0102030405060708 -pass pass:correct-horse
    const CT_PBKDF2: &str = "cb6a2ad1a7ef6c3e139df954fc17d73cb91a0d45285b22482d8bddcaf0f589105cc565a036e0e9ef77e65b6550450d4b";
    // openssl enc -aes-256-cbc -e -md md5 -S 0102030405060708 -pass pass:correct-horse
    const CT_LEGACY: &str = "c9913e143dd0e2e212cf0210f169d86cd3afd06619409461eff541470cd17d6fdb67205ba90f41ca131e16aa00ddb66c";

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
            .collect()
    }

    fn write_container(ct_hex: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(MAGIC).unwrap();
        f.write_all(&SALT).unwrap();
        f.write_all(&unhex(ct_hex)).unwrap();
        f.flush().unwrap();
        f
    }

    fn path(f: &tempfile::NamedTempFile) -> String {
        f.path().to_string_lossy().to_string()
    }

    #[test]
    fn decrypts_files_written_by_openssl_enc_with_pbkdf2() {
        let enc = write_container(CT_PBKDF2);
        let out = tempfile::NamedTempFile::new().unwrap();
        decrypt_file_aes_256_cbc(&path(&enc), &path(&out), PASSWORD).unwrap();
        assert_eq!(std::fs::read(out.path()).unwrap(), PLAINTEXT);
    }

    #[test]
    fn decrypts_files_written_by_earlier_releases_with_the_legacy_derivation() {
        let enc = write_container(CT_LEGACY);
        let out = tempfile::NamedTempFile::new().unwrap();
        decrypt_file_aes_256_cbc(&path(&enc), &path(&out), PASSWORD).unwrap();
        assert_eq!(std::fs::read(out.path()).unwrap(), PLAINTEXT);
    }

    #[test]
    fn round_trip_and_wrong_password() {
        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let mut plain = tempfile::NamedTempFile::new().unwrap();
        plain.write_all(&data).unwrap();
        plain.flush().unwrap();
        let enc = tempfile::NamedTempFile::new().unwrap();
        let dec = tempfile::NamedTempFile::new().unwrap();
        encrypt_file_aes_256_cbc(&path(&plain), &path(&enc), PASSWORD).unwrap();
        let ciphertext = std::fs::read(enc.path()).unwrap();
        assert_eq!(&ciphertext[..8], MAGIC);
        assert_eq!(ciphertext.len(), 16 + data.len() + (16 - data.len() % 16));
        decrypt_file_aes_256_cbc(&path(&enc), &path(&dec), PASSWORD).unwrap();
        assert_eq!(std::fs::read(dec.path()).unwrap(), data);
        assert!(decrypt_file_aes_256_cbc(&path(&enc), &path(&dec), "wrong").is_err());
    }

    #[test]
    fn pbkdf2_derivation_matches_openssl() {
        let (key, iv) = derive_pbkdf2(PASSWORD, &SALT).unwrap();
        let out = openssl::symm::decrypt(Cipher::aes_256_cbc(), &key, Some(&iv), &unhex(CT_PBKDF2))
            .unwrap();
        assert_eq!(out, PLAINTEXT);
    }
}
