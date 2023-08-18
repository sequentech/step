use strand::backend::malachite::{MalachiteCtx, P2048 as MP2048};
use strand::backend::num_bigint::{BigintCtx, P2048};
use strand::backend::ristretto::RistrettoCtx;
cfg_if::cfg_if! {
    if #[cfg(feature = "rug")] {
        use strand::backend::rug::RugCtx;
        use strand::backend::rug::P2048 as RUGP2048;
    }
}

#[test]
fn test_protocol() {
    braid::util::init_log(true);

    let ctx = RistrettoCtx;
    braid::test::protocol_test::run(1000, 1, ctx);
    let ctx = BigintCtx::<P2048>::default();
    braid::test::protocol_test::run(20, 1, ctx);
    let ctx = MalachiteCtx::<MP2048>::default();
    braid::test::protocol_test::run(20, 1, ctx);
    cfg_if::cfg_if! {
        if #[cfg(feature = "rug")] {
            let ctx = RugCtx::<RUGP2048>::default();
            braid::test::protocol_test::run(100, 1, ctx);
        }
    }
}
