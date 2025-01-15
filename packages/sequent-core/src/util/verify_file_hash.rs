// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::fmt::Write;
use std::io::Read;
use strand::hash::hash_sha256;
use strand::util::StrandError;
use tempfile::NamedTempFile;

#[derive(Debug)]
pub enum HashFileVerifyError {
    IoError(String, std::io::Error), // Error reading voters file
    HashMismatch,                    // Voters file hash does not match
    HashComputingError(String, StrandError), // Error computing the hash
}

impl core::fmt::Display for HashFileVerifyError {
    fn fmt(
        &self,
        fmt: &mut core::fmt::Formatter,
    ) -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{self:?}")
    }
}

impl std::error::Error for HashFileVerifyError {}

pub fn verify_file_sha256(
    voters_file: &NamedTempFile,
    sha256: String,
) -> Result<(), HashFileVerifyError> {
    // convert f into bytes &[u8]
    let mut f = voters_file.as_file();
    let mut file_buffer: Vec<u8> = vec![];
    f.read_to_end(&mut file_buffer).map_err(|err| {
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
            if hash_str.eq(sha256.to_lowercase().as_str()) {
                return Err(HashFileVerifyError::HashMismatch);
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
