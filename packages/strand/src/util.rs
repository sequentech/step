// SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sha2::{Sha384, Sha512};
use thiserror::Error;

use crate::context::Ctx;
use crate::elgamal::Ciphertext;

cfg_if::cfg_if! {
    if #[cfg(feature = "rayon")] {
        use rayon::iter::IntoParallelIterator;
        use rayon::prelude::*;
        use std::iter::IntoIterator;


        pub(crate) trait Par<I: IntoIterator + IntoParallelIterator> {
            fn par(self) -> <I as rayon::iter::IntoParallelIterator>::Iter;
        }

        impl<I: IntoIterator + IntoParallelIterator> Par<I> for I {
            #[inline(always)]
            fn par(self) -> <I as rayon::iter::IntoParallelIterator>::Iter {
                self.into_par_iter()
            }
        }

    } else {
        pub(crate) trait Par<I: IntoIterator> {
            fn par(self) -> I::IntoIter;
        }

        impl<I: IntoIterator> Par<I> for I {
            #[inline(always)]
            fn par(self) -> I::IntoIter {
                self.into_iter()
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum StrandError {
    #[error("{0}")]
    Generic(String),
    #[cfg(feature = "num_bigint")]
    #[error("bigint parse error: {0}")]
    ParseBigIntError(#[from] num_bigint::ParseBigIntError),
    #[error("io error: {0}")]
    SerializationError(#[from] std::io::Error),
    #[error("decode error: {0}")]
    DecodingError(#[from] base64::DecodeError),
    #[error("ecdsa error: {0}")]
    EcdsaError(#[from] ecdsa::Error),
    #[cfg(feature = "openssl")]
    #[error("openssl error: {0}")]
    OpenSSLError(#[from] openssl::error::ErrorStack),
}

/// Converts a slice into a hash-sized array.
pub fn to_hash_array(input: &[u8]) -> Result<[u8; 64], StrandError> {
    to_u8_array(input)
}

/// Converts a slice into a fixed size array.
pub fn to_u8_array<const N: usize>(
    input: &[u8],
) -> Result<[u8; N], StrandError> {
    if input.len() == N {
        let mut bytes = [0u8; N];
        bytes.copy_from_slice(input);
        Ok(bytes)
    } else {
        Err(StrandError::Generic(
            "Unexpected number of bytes".to_string(),
        ))
    }
}

/// Fast generation of ciphertexts using random group elements.
pub fn random_ciphertexts<C: Ctx>(n: usize, ctx: &C) -> Vec<Ciphertext<C>> {
    (0..n)
        .par()
        .map(|_| Ciphertext {
            mhr: ctx.rnd(),
            gr: ctx.rnd(),
        })
        .collect()
}

/// Size of all hashes.
pub const STRAND_HASH_LENGTH_BYTES: usize = 64;
pub type Hash = [u8; 64];
pub(crate) type Hasher = Sha512;
pub(crate) use sha2::Digest;

/// Single entry point for all hashing, vector version.
pub fn hash(bytes: &[u8]) -> Vec<u8> {
    let mut hasher = hasher();
    hasher.update(bytes);
    hasher.finalize().to_vec()
}
/// Single entry point for all hashing, array version.
pub fn hash_array(bytes: &[u8]) -> Hash {
    let mut hasher = hasher();
    hasher.update(bytes);
    hasher.finalize().into()
}
/// Single access point for all hashing.
pub fn hasher() -> Hasher {
    Sha512::new()
}

// Calling verify_digest on a RustCrypto ecdsa VerifyingKey<P384> fails to
// compile this error unless the digest passed is Sha384:
/*
    the trait `DigestVerifier<CoreWrapper<CtVariableCoreWrapper<Sha256VarCore, UInt<UInt<UInt<UInt<UInt<UInt<UTerm, B1>, B0>, B0>, B0>, B0>, B0>, OidSha256>>, _>` is not implemented for `ecdsa::VerifyingKey<NistP384>`
*/
pub fn rust_crypto_ecdsa_hasher() -> RustCryptoHasher {
    Sha384::new()
}
pub(crate) type RustCryptoHasher = Sha384;

cfg_if::cfg_if! {
    if #[cfg(feature = "openssl")] {
        use openssl::hash::{Hasher as HasherOpenSSL, MessageDigest};

        pub fn hasher_openssl() -> Result<HasherOpenSSL, StrandError> {
            let md = MessageDigest::sha512();
            let hasher = HasherOpenSSL::new(md)?;
            Ok(hasher)
        }

        pub fn hash_openssl(bytes: &[u8]) -> Result<Vec<u8>, StrandError> {
            let mut hasher = hasher_openssl()?;
            hasher.update(bytes)?;
            let result = hasher.finish()?;
            Ok(result.to_vec())
        }
    }
}
