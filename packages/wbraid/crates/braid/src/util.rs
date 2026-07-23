// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};

use b4::CryptographicHash as Hash;
use cryptography::utils::error::Error as CryptographyError;

/// Returns a truncated hex encoding of the given hash bytes.
///
/// Used when displaying hashes in debug messages.
pub(crate) fn dbg_hash(h: &Hash) -> String {
    hex::encode(h)[0..10].to_string()
}

/// Returns a fixed-size array Hash from the given vector.
pub fn hash_from_vec(bytes: &[u8]) -> Result<Hash, CryptographyError> {
    if bytes.len() == 64 {
        Ok(Hash::try_from(bytes)?)
    } else {
        Err(CryptographyError::DeserializationError(format!(
            "Expected 64 bytes, got {}",
            bytes.len()
        )))
    }
}

/// Returns base64 no pad decode.
pub fn decode_base64(s: &String) -> Result<Vec<u8>> {
    general_purpose::STANDARD_NO_PAD
        .decode(s)
        .map_err(|error| anyhow!(error))
}
