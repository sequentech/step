// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode,
};

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::shuffler::*;
use strand::util;

pub(crate) fn test_product_shuffle_generic<C: Ctx>(ctx: C, n: usize) {
    let sk = PrivateKey::gen(&ctx);
    let pk = sk.get_pk();

    let es = util::random_product_ciphertexts(n, 1, &ctx);
    let seed = vec![];
    let hs = ctx.generators(es.rows().len() + 1, &seed).unwrap();
    let shuffler = strand::shuffler_product::Shuffler::new(&pk, &hs, &ctx);

    let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
    let proof = shuffler.gen_proof(&es, &e_primes, rs, &perm, &[]).unwrap();

    let ok = shuffler.check_proof(&proof, &es, &e_primes, &[]).unwrap();

    assert!(ok);
}

fn test_shuffle_generic<C: Ctx>(ctx: C, n: usize) {
    let sk = PrivateKey::gen(&ctx);
    let pk = sk.get_pk();

    let es = util::random_ciphertexts(n, &ctx);
    let seed = vec![];
    let hs = ctx.generators(es.len() + 1, &seed).unwrap();
    let shuffler = Shuffler::new(&pk, &ctx);

    let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
    let proof = shuffler
        .gen_proof(es.clone(), &e_primes, rs, hs.clone(), perm, &vec![])
        .unwrap();
    let ok = shuffler
        .check_proof(&proof, es, e_primes, hs, &vec![])
        .unwrap();

    assert!(ok);
}

fn shuffle_ristretto(n: usize) {
    let ctx = RistrettoCtx;
    test_shuffle_generic(ctx, n);
}

fn product_shuffle_ristretto(n: usize) {
    let ctx = RistrettoCtx;
    test_product_shuffle_generic(ctx, n);
}

cfg_if::cfg_if! {
    if #[cfg(feature = "num_bigint")] {
        use strand::backend::num_bigint::{BigintCtx, P2048};
        fn shuffle_bigint(n: usize) {
            let ctx: BigintCtx<P2048> = Default::default();
            test_shuffle_generic(ctx, n);
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "malachite")] {
        use strand::backend::malachite::{MalachiteCtx, P2048 as MP2048};
        fn shuffle_malachite(n: usize) {
            let ctx: MalachiteCtx<MP2048> = Default::default();
            test_shuffle_generic(ctx, n);
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "rug")] {
        use strand::backend::rug::RugCtx;
        use strand::backend::rug::P2048 as RP2048;
        fn shuffle_rug(n: usize) {
            let ctx: RugCtx::<RP2048> = Default::default();
            test_shuffle_generic(ctx, n);
        }
    }
}

fn bench_shuffle(c: &mut Criterion) {
    let mut group = c.benchmark_group("shuffle");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    for i in [100usize].iter() {
        group.bench_with_input(BenchmarkId::new("ristretto", i), i, |b, i| {
            b.iter(|| shuffle_ristretto(*i))
        });
        group.bench_with_input(BenchmarkId::new("ristrettox", i), i, |b, i| {
            b.iter(|| product_shuffle_ristretto(*i))
        });
        #[cfg(feature = "num_bigint")]
        group.bench_with_input(BenchmarkId::new("bigint", i), i, |b, i| {
            b.iter(|| shuffle_bigint(*i))
        });
        #[cfg(feature = "malachite")]
        group.bench_with_input(BenchmarkId::new("malachite", i), i, |b, i| {
            b.iter(|| shuffle_malachite(*i))
        });
        #[cfg(feature = "rug")]
        group.bench_with_input(BenchmarkId::new("rug", i), i, |b, i| {
            b.iter(|| shuffle_rug(*i))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_shuffle);
criterion_main!(benches);
