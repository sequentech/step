// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Free & Fair
// See LICENSE.md for details

//! Fuzz target for the Schnorr verification boundary: adversarial proof bytes
//! are deserialized and, when accepted, verified against a fixed,
//! deterministically derived statement. Verification may accept or reject;
//! only panics are findings.

#![no_main]

use std::sync::LazyLock;

use cryptography::context::{Context, RistrettoCtx};
use cryptography::traits::groups::CryptographicGroup;
use cryptography::utils::serialization::Deserializable;
use cryptography::zkp::schnorr::SchnorrProof;
use libfuzzer_sys::fuzz_target;

type Element = <RistrettoCtx as Context>::Element;

static STATEMENT: LazyLock<(Element, Element)> = LazyLock::new(|| {
    let g = RistrettoCtx::generator();
    let y = <RistrettoCtx as Context>::G::hash_to_element(&[b"fuzz"], &[b"fuzz statement"])
        .expect("hash_to_element cannot fail on fixed input");
    (g, y)
});

fuzz_target!(|data: &[u8]| {
    if let Ok(proof) = SchnorrProof::<RistrettoCtx>::deser(data) {
        let (g, y) = &*STATEMENT;
        let _ = proof.verify(g, y, b"fuzz verify_schnorr");
    }
});
