// SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode,
};
use strand::backend::malachite::{MalachiteCtx, P2048 as MP2048};
use strand::backend::num_bigint::{BigintCtx, P2048};
use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::shuffler::*;
use strand::util;

fn test_shuffle_generic<C: Ctx>(ctx: C, n: usize) {
    let sk = PrivateKey::gen(&ctx);
    let pk = sk.get_pk();

    let es = util::random_ciphertexts(n, &ctx);
    let seed = vec![];
    let hs = ctx.generators(es.len() + 1, &seed);
    let shuffler = Shuffler::new(&pk, &hs, &ctx);

    let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
    let proof = shuffler
        .gen_proof(&es, &e_primes, &rs, &perm, &vec![])
        .unwrap();
    let ok = shuffler
        .check_proof(&proof, &es, &e_primes, &vec![])
        .unwrap();

    assert!(ok);
}

fn shuffle_ristretto(n: usize) {
    let ctx = RistrettoCtx;
    test_shuffle_generic(ctx, n);
}

fn shuffle_bigint(n: usize) {
    let ctx: BigintCtx<P2048> = Default::default();
    test_shuffle_generic(ctx, n);
}

fn shuffle_malachite(n: usize) {
    let ctx: MalachiteCtx<MP2048> = Default::default();
    test_shuffle_generic(ctx, n);
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
            b.iter(|| shuffle_ristretto(*i * 10))
        });
        group.bench_with_input(BenchmarkId::new("bigint", i), i, |b, i| {
            b.iter(|| shuffle_bigint(*i))
        });
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
