// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Free & Fair
// See LICENSE.md for details

//! Fuzz target for the batched Naor-Yung verify-and-strip boundary
//! (`PublicKey::strip_all`, i.e. `PlEqProof::verify_batch` and its per-item
//! attribution fallback) — a differential oracle against the per-item
//! `strip`.
//!
//! The input describes a list of up to 16 ballots drawn from a fixed pool of
//! valid ones, each optionally corrupted by one byte of its encoding. Every
//! ballot that still deserializes is kept. The oracle asserts that
//! `strip_all` accepts exactly when every per-item `strip` accepts, returns
//! the same ElGamal ciphertexts in the same order when it does, and never
//! reports `BatchVerificationInconsistent` (a batch that rejects a list whose
//! proofs all verify individually).
//!
//! Input layout: byte 0 selects the list length (`% 17`); then, per ballot,
//! three bytes — pool index, byte offset into the encoding, XOR mask (0 leaves
//! the ballot valid).

#![no_main]

use std::sync::LazyLock;

use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::{elgamal, naoryung};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::error::Error;
use cryptography::utils::serialization::{Deserializable, Serializable};
use libfuzzer_sys::fuzz_target;

type C = RistrettoCtx;
const W: usize = 2;
const CTX: &[u8] = b"fuzz verify_ny_strip_all";
const POOL: usize = 8;
const MAX_LIST: usize = 16;

static NY_PK: LazyLock<naoryung::PublicKey<C>> = LazyLock::new(|| {
    let y = <C as Context>::G::hash_to_element(&[CTX], &[b"fuzz key"])
        .expect("hash_to_element cannot fail on fixed input");
    naoryung::PublicKey::augment(&elgamal::PublicKey::new(y), CTX)
        .expect("auxiliary key derivation cannot fail")
});

/// Encodings of valid ballots, derived deterministically so runs reproduce.
static POOL_BYTES: LazyLock<Vec<Vec<u8>>> = LazyLock::new(|| {
    (0..POOL)
        .map(|i| {
            let tag = [u8::try_from(i).expect("pool index fits in u8")];
            let message: [<C as Context>::Element; W] = std::array::from_fn(|w| {
                let w = [u8::try_from(w).expect("width fits in u8")];
                <C as Context>::G::hash_to_element(&[&tag, &w], &[b"fuzz message", b"component"])
                    .expect("hash_to_element cannot fail on fixed input")
            });
            let r: [<C as Context>::Scalar; W] = std::array::from_fn(|w| {
                let w = [u8::try_from(w).expect("width fits in u8")];
                <C as Context>::G::hash_to_scalar(&[&tag, &w], &[b"fuzz randomness", b"component"])
                    .expect("hash_to_scalar cannot fail on fixed input")
            });
            NY_PK
                .encrypt_with_r(&message, &r, CTX)
                .expect("encryption cannot fail")
                .ser()
        })
        .collect()
});

fuzz_target!(|data: &[u8]| {
    let Some((&len_byte, rest)) = data.split_first() else {
        return;
    };
    let len = usize::from(len_byte) % (MAX_LIST + 1);

    let ballots: Vec<naoryung::Ciphertext<C, W>> = rest
        .chunks_exact(3)
        .take(len)
        .filter_map(|spec| {
            let mut bytes = POOL_BYTES[usize::from(spec[0]) % POOL].clone();
            let at = usize::from(spec[1]) % bytes.len();
            bytes[at] ^= spec[2];
            naoryung::Ciphertext::<C, W>::deser(&bytes).ok()
        })
        .collect();

    let per_item: Vec<Result<elgamal::Ciphertext<C, W>, Error>> = ballots
        .iter()
        .cloned()
        .map(|c| NY_PK.strip(c, CTX))
        .collect();
    let all_valid = per_item.iter().all(Result::is_ok);

    match NY_PK.strip_all(ballots, CTX) {
        Ok(stripped) => {
            assert!(all_valid, "strip_all accepted a list with an invalid ballot");
            let expected: Vec<elgamal::Ciphertext<C, W>> =
                per_item.into_iter().map(|r| r.expect("all valid")).collect();
            assert_eq!(stripped, expected, "strip_all and strip disagree on the output");
        }
        Err(Error::NaorYungStripError(_)) => {
            assert!(!all_valid, "strip_all rejected a list whose ballots all verify");
        }
        Err(Error::BatchVerificationInconsistent(msg)) => {
            panic!("batch and per-item verification disagree: {msg}");
        }
        Err(other) => panic!("unexpected error from strip_all: {other:?}"),
    }
});
