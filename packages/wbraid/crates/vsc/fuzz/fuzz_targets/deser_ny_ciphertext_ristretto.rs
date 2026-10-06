// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Free & Fair
// See LICENSE.md for details

//! Fuzz target for Naor-Yung ciphertext deserialization: a bijection oracle. Any accepted byte string must re-serialize to
//! exactly itself (`SERIALIZATION.md` property P2); a panic or a mismatch is
//! a finding.

#![no_main]

use cryptography::context::RistrettoCtx;
use cryptography::cryptosystem::naoryung::Ciphertext;
use cryptography::utils::serialization::{Deserializable, Serializable};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = Ciphertext::<RistrettoCtx, 2>::deser(data) {
        assert_eq!(value.ser(), data, "accepted bytes must re-serialize identically");
    }
});
