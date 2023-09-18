// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sha2::{Sha384, Sha512};

use crate::util::StrandError;

/// Size of all hashes.
pub const STRAND_HASH_LENGTH_BYTES: usize = 64;
pub type Hash = [u8; 64];
pub(crate) type Hasher = Sha512;
pub(crate) use sha2::Digest;

/// Single entry point for all hashing, vector version.
pub fn hash(bytes: &[u8]) -> Result<Vec<u8>, StrandError> {
    let mut hasher = hasher();
    hasher.update(bytes);
    Ok(hasher.finalize().to_vec())
}
/// Single entry point for all hashing, array version.
pub fn hash_array(bytes: &[u8]) -> Result<Hash, StrandError> {
    let mut hasher = hasher();
    hasher.update(bytes);
    let ret: Hash = hasher.finalize().into();
    Ok(ret)
}
/// Single access point for all hashing.
pub fn hasher() -> Hasher {
    Sha512::new()
}

// Calling verify_digest on a RustCrypto ecdsa VerifyingKey<P384> fails to
// compile, unless the digest passed is Sha384:
/*
    the trait `DigestVerifier<CoreWrapper<CtVariableCoreWrapper<Sha256VarCore, UInt<UInt<UInt<UInt<UInt<UInt<UTerm, B1>, B0>, B0>, B0>, B0>, B0>, OidSha256>>, _>` is not implemented for `ecdsa::VerifyingKey<NistP384>`
*/
pub fn rust_crypto_ecdsa_hasher() -> RustCryptoHasher {
    Sha384::new()
}
pub(crate) type RustCryptoHasher = Sha384;
