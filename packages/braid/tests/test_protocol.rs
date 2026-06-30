// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::util::init_log::init_log;
use strand::backend::ristretto::RistrettoCtx;

#[test]
fn test_protocol_memory() {
    init_log(true);

    let ctx = RistrettoCtx;
    braid::native::test::protocol_test_memory::run(1000, 1, ctx);
}

#[tokio::test]
#[ignore]
async fn test_protocol_http() {
    init_log(true);
    let ctx = RistrettoCtx;

    braid::native::test::protocol_test_http::run(1000, 1, ctx).await;
}
