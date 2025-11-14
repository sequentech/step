use std::sync::{Arc, Mutex};

// SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
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
    #[error("chacha20poly1305 error: {0}")]
    Chacha20Error(chacha20poly1305::Error),
    #[error("rcgen error: {0}")]
    RCGenError(#[from] rcgen::RcgenError),
    #[error("x509_parser error: {0}")]
    X509ParserError(
        #[from] x509_parser::nom::Err<x509_parser::error::X509Error>,
    ),
    #[cfg(any(feature = "openssl_core", feature = "openssl_full"))]
    #[error("openssl error: {0}")]
    OpenSSLError(#[from] openssl::error::ErrorStack),
    #[error("ed25519 error: {0}")]
    Ed25519Error(#[from] ed25519_dalek::ed25519::Error),
    #[error("Invalid symmetric key length: {0}")]
    InvalidSymmetricKeyLength(String)
}

/// Converts a slice into a hash-sized array.
pub fn to_hash_array(input: &[u8]) -> Result<crate::hash::Hash, StrandError> {
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
    let rng = Arc::new(Mutex::new(ctx.get_rng()));
    (0..n)
        .par()
        .map(|_| {
            let mut rng_ = rng.lock().unwrap();
            Ciphertext {
                mhr: ctx.rnd(&mut rng_),
                gr: ctx.rnd(&mut rng_),
            }
        })
        .collect()
}

cfg_if::cfg_if! {
if #[cfg(not(feature = "wasm"))] {
use crate::shuffler_product::StrandRectangle;

/// Fast generation of product ciphertexts using random group elements.
pub fn random_product_ciphertexts<C: Ctx>(
    n: usize,
    width: usize,
    ctx: &C,
) -> StrandRectangle<Ciphertext<C>> {
    let rng = Arc::new(Mutex::new(ctx.get_rng()));

    let rows: Vec<Vec<Ciphertext<C>>> = (0..n)
        .par()
        .map(|_| {
            let mut rng_ = rng.lock().unwrap();

            let ret: Vec<Ciphertext<C>> = (0..width)
                .map(|_| Ciphertext {
                    mhr: ctx.rnd(&mut rng_),
                    gr: ctx.rnd(&mut rng_),
                })
                .collect();

            ret
        })
        .collect();

    StrandRectangle::new_unchecked(rows)
}

}}
