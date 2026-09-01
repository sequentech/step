// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The full protocol over P-256.
//!
//! braid's default context is Ristretto255, but P-256 is the curve an
//! independent Verificatum verifier can check (see the repo-root
//! `VERIFICATUM.md`), so the whole DKG → mix → threshold-decrypt round needs to
//! work over it. Until the P-256 byte/scalar encoding existed this panicked in
//! the DKG, because share transport encrypts a scalar to a group element.

#![cfg(feature = "native")]

use cryptography::context::P256Ctx;

#[test]
fn test_protocol_memory_p256() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_memory::run::<P256Ctx>(200, 1, 2);
}

/// The board-union path over P-256 as well, so the curve is exercised across a
/// DKG reused by a separate tally rather than a single session only.
#[test]
fn test_protocol_memory_union_p256() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_memory_union::run::<P256Ctx>(50, 2, 2);
}
