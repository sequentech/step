// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Free & Fair
// See LICENSE.md for details

//! Fuzz target for the Naor-Yung verify-and-strip boundary: adversarial
//! ciphertext bytes are deserialized and, when accepted, run through the
//! well-formedness verification (`PublicKey::strip`, i.e. the PlEq verifier)
//! under a fixed, deterministically derived key. Verification may accept or
//! reject; only panics are findings.

#![no_main]

use std::sync::LazyLock;

use cryptography::context::{Context, RistrettoCtx};
use cryptography::cryptosystem::{elgamal, naoryung};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::Deserializable;
use libfuzzer_sys::fuzz_target;

const CTX: &[u8] = b"fuzz verify_ny_strip";

static NY_PK: LazyLock<naoryung::PublicKey<RistrettoCtx>> = LazyLock::new(|| {
    let y = <RistrettoCtx as Context>::G::hash_to_element(&[CTX], &[b"fuzz key"])
        .expect("hash_to_element cannot fail on fixed input");
    naoryung::PublicKey::augment(&elgamal::PublicKey::new(y), CTX)
        .expect("auxiliary key derivation cannot fail")
});

fuzz_target!(|data: &[u8]| {
    if let Ok(ciphertext) = naoryung::Ciphertext::<RistrettoCtx, 2>::deser(data) {
        let _ = NY_PK.strip(ciphertext, CTX);
    }
});
