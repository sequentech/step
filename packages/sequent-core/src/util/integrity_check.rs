// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use strand::hash::hash_sha256;
use strand::util::StrandError;
use strum_macros::Display;
use tempfile::NamedTempFile;

#[derive(Debug, Display)]
/// Errors that can occur during the integrity check of a file.
pub enum HashFileVerifyError {
    #[strum(serialize = "io-error")]
    /// Error reading voters file
    IoError(String, std::io::Error),
    #[strum(serialize = "hash-mismatch")]
    /// Voters file hash does not match
    HashMismatch(String, String),
    #[strum(serialize = "hash-computing-error")]
    /// Error computing the hash
    HashComputingError(String, StrandError),
}

impl std::error::Error for HashFileVerifyError {}

/// Checks the integrity of a file by comparing its SHA-256 hash.
///
/// # Errors
/// Returns an error if the file cannot be opened, read, or if the hash does not match.
pub fn integrity_check(
    temp_file_path: &NamedTempFile,
    sha256: String,
) -> Result<(), HashFileVerifyError> {
    let mut sha256 = sha256;
    sha256.make_ascii_lowercase();
    let mut file = File::open(temp_file_path).map_err(|err| {
        HashFileVerifyError::IoError(
            "Error opening the temp file.".to_string(),
            err,
        )
    })?;

    let mut file_buffer: Vec<u8> = vec![];
    file.read_to_end(&mut file_buffer).map_err(|err| {
        HashFileVerifyError::IoError(
            "Error reading the temp file.".to_string(),
            err,
        )
    })?;

    let calculated_hash_result = hash_sha256(file_buffer.as_slice());
    match calculated_hash_result {
        Ok(hash) => {
            // Get lowercase hex representation.
            let hash_str: String =
                hash.iter().fold(String::new(), |mut output, b| {
                    let _ = write!(output, "{b:02x}");
                    output
                });
            if !hash_str.eq(sha256.as_str()) {
                return Err(HashFileVerifyError::HashMismatch(
                    sha256, hash_str,
                ));
            }
        }
        Err(err) => {
            return Err(HashFileVerifyError::HashComputingError(
                "Error computing the hash from file.".to_string(),
                err,
            ));
        }
    }
    Ok(())
}
