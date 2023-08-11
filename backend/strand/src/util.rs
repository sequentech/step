use num_bigint::ParseBigIntError;
// SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use sha2::{Digest, Sha512};
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
    #[error("bigint parse error: {0}")]
    ParseBigIntError(#[from] ParseBigIntError),
    #[error("io error: {0}")]
    SerializationError(#[from] std::io::Error),
    #[error("decode error: {0}")]
    DecodingError(#[from] base64::DecodeError),
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
pub fn hasher() -> Sha512 {
    Sha512::new()
}
