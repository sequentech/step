// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The signing code: eight characters every signer of a request sees, so
//! the people at a Post can compare it aloud.

use super::canonical::{canonical_json, CanonicalJsonError};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const SIGNING_CODE_DOMAIN: &[u8] = b"step-signing-code";

/// Crockford's base32: no I, L, O or U, so a code read aloud is unambiguous.
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The first 40 bits of
/// `SHA-256("step-signing-code" || request_id || SHA-256(canonical(subject)))`
/// as Crockford base32, `XXXX-XXXX`. `request_id` is hashed as its
/// lowercase hyphenated text (36 bytes) and the inner digest as its 32 raw
/// bytes. The code doesn't depend on the payload, so the payload can hold it.
pub fn signing_code(
    request_id: Uuid,
    subject: &Value,
) -> Result<String, CanonicalJsonError> {
    let subject_digest = Sha256::digest(canonical_json(subject)?.as_bytes());
    let digest = Sha256::new()
        .chain_update(SIGNING_CODE_DOMAIN)
        .chain_update(request_id.hyphenated().to_string().as_bytes())
        .chain_update(subject_digest)
        .finalize();
    let mut leading = [0u8; 5];
    leading.copy_from_slice(&digest[..5]);
    Ok(crockford_code(leading))
}

/// Five bytes as eight Crockford base32 characters, grouped `XXXX-XXXX`.
pub(crate) fn crockford_code(bytes: [u8; 5]) -> String {
    let bits = bytes
        .iter()
        .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte));
    let mut code = String::with_capacity(9);
    for index in 0..8 {
        if index == 4 {
            code.push('-');
        }
        let shift = 35 - 5 * index;
        let symbol = ((bits >> shift) & 0x1f) as usize;
        code.push(char::from(CROCKFORD[symbol]));
    }
    code
}

/// Lowercase hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// `signing_request.payload_sha256`.
pub fn payload_sha256(canonical_payload: &str) -> String {
    sha256_hex(canonical_payload.as_bytes())
}

#[cfg(test)]
#[path = "code_tests.rs"]
mod code_tests;
