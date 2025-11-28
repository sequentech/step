// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use strand::backend::ristretto::RistrettoCtx;
cfg_if::cfg_if! {
    if #[cfg(feature = "rug")] {
        use strand::backend::rug::RugCtx;
        use strand::backend::rug::P2048 as RUGP2048;
    }
}

#[test]
fn test_protocol_memory() {
    braid::util::init_log(true);

    let ctx = RistrettoCtx;
    braid::test::protocol_test_memory::run(1000, 1, ctx);
    cfg_if::cfg_if! {
        if #[cfg(feature = "rug")] {
            let ctx = RugCtx::<RUGP2048>::default();
            braid::test::protocol_test_memory::run(100, 1, ctx);
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_protocol_http() {
    braid::util::init_log(true);
    let ctx = RistrettoCtx;

    braid::test::protocol_test_http::run(1000, 1, ctx).await;
}
