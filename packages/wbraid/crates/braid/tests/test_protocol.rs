// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Native-only: drives the tokio/rayon harnesses in `braid::native`.
#![cfg(feature = "native")]

use cryptography::context::RistrettoCtx;

#[test]
fn test_protocol_memory() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_memory::run::<RistrettoCtx>(1000, 1, 2); // W=2 (default)
}

// Board union (§8.2): one DKG, one tally over a child board unioned with it.
#[test]
fn test_protocol_memory_union() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_memory_union::run::<RistrettoCtx>(100, 1, 2);
}

// Board union as the batch mechanism (§8.2): one DKG reused by several tallies,
// each with a different ciphertext set on its own child board.
#[test]
fn test_protocol_memory_union_batches() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_memory_union::run::<RistrettoCtx>(100, 3, 2);
}

#[tokio::test]
#[ignore]
async fn test_protocol_http() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_http::run::<RistrettoCtx>(1000, 1, 2).await;
    // W=2 (default)
}

// Board union (§8.2) over live b4 + SQLite persistence: one DKG reused by several
// tallies, each on its own child board. Requires a running b4 + S3/LocalStack.
#[tokio::test]
#[ignore]
async fn test_protocol_http_union() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_http_union::run::<RistrettoCtx>(1000, 2, 2).await;
}
