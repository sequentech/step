// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Fuzz target for `Predicate` deserialization — the persistence boundary:
//! these bytes reload the anti-rewrite commitments across restarts. A
//! bijection oracle: any accepted byte string must re-serialize to exactly
//! itself (`SERIALIZATION.md` property P2); a panic or a mismatch is a
//! finding.

#![no_main]

use braid::messages::predicate::Predicate;
use cryptography::utils::serialization::{Deserializable, Serializable};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(predicate) = Predicate::deser(data) {
        assert_eq!(predicate.ser(), data, "accepted bytes must re-serialize identically");
    }
});
