// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Fuzz target for `ProtocolMessage` deserialization — the outermost
//! adversarial boundary: these bytes arrive from the untrusted board before
//! any signature check. A bijection oracle: any accepted byte string must
//! re-serialize to exactly itself (`SERIALIZATION.md` property P2); a panic
//! or a mismatch is a finding.

#![no_main]

use braid::messages::wire::ProtocolMessage;
use cryptography::context::RistrettoCtx;
use cryptography::utils::serialization::{Deserializable, Serializable};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(message) = ProtocolMessage::<RistrettoCtx>::deser(data) {
        assert_eq!(message.ser(), data, "accepted bytes must re-serialize identically");
    }
});
