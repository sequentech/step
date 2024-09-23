// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use base64::{engine::general_purpose, Engine as _};
use openssl::hash::{hash_xof as hxof, Hasher as HasherOpenSSL, MessageDigest};

use crate::util::StrandError;

/// Sha-512 hashes are 64 bytes.
pub const STRAND_HASH_LENGTH_BYTES: usize = 64;
/// Sha-512 hashes are 64 byte arrays: [u8; 64].
pub type Hash = [u8; STRAND_HASH_LENGTH_BYTES];

pub(crate) type Hasher = HasherOpenSSL;

pub fn hasher() -> Result<Hasher, StrandError> {
    let md = MessageDigest::sha512();
    let hasher = HasherOpenSSL::new(md)?;
    Ok(hasher)
}

pub fn hash(bytes: &[u8]) -> Result<Vec<u8>, StrandError> {
    let mut hasher = hasher()?;
    hasher.update(bytes)?;
    let result = hasher.finish()?;
    Ok(result.to_vec())
}
pub fn hash_to_array(bytes: &[u8]) -> Result<Hash, StrandError> {
    let mut hasher = hasher()?;
    hasher.update(bytes)?;
    let result = hasher.finish()?;
    let bytes = result.to_vec();
    Ok(crate::util::to_hash_array(&bytes)?)
}
/// Hash and base 64 encode resulting bytes.
pub fn hash_b64(bytes: &[u8]) -> Result<String, StrandError> {
    let bytes = hash(bytes)?;
    let ret = general_purpose::STANDARD_NO_PAD.encode(&bytes);
    Ok(ret)
}

pub fn hash_xof(
    length_bytes: usize,
    prefix: &[u8],
) -> Result<Vec<u8>, StrandError> {
    let mut buf = vec![0; length_bytes];
    hxof(MessageDigest::shake_256(), prefix, &mut buf)?;

    Ok(buf)
}

pub(crate) fn rust_crypto_ecdsa_hasher() -> Result<RustCryptoHasher, StrandError>
{
    let md = MessageDigest::sha384();
    let hasher = HasherOpenSSL::new(md)?;
    Ok(hasher)
}
pub(crate) type RustCryptoHasher = HasherOpenSSL;

pub fn info() -> String {
    format!("{}, FIPS_ENABLED: TRUE", module_path!())
}
