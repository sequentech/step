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
pub enum HashFileVerifyError {
    #[strum(serialize = "io-error")]
    IoError(String, std::io::Error), // Error reading voters file
    #[strum(serialize = "hash-mismatch")]
    HashMismatch(String, String), // Voters file hash does not match
    #[strum(serialize = "hash-computing-error")]
    HashComputingError(String, StrandError), // Error computing the hash
}

impl std::error::Error for HashFileVerifyError {}

pub fn integrity_check(
    temp_file_path: &NamedTempFile,
    sha256: String,
) -> Result<(), HashFileVerifyError> {
    let sha256 = sha256.to_lowercase();
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
