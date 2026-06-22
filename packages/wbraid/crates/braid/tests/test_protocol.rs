// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use cryptography::context::RistrettoCtx;

#[test]
fn test_protocol_memory() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_memory::run::<RistrettoCtx>(1000, 1, 2); // W=2 (default)
}

#[tokio::test]
#[ignore]
async fn test_protocol_http() {
    braid::native::logging::init_log(true);

    braid::native::test::protocol_test_http::run::<RistrettoCtx>(1000, 1, 2).await; // W=2 (default)
}
