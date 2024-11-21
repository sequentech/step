// SPDX-FileCopyrightText: 2022 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode,
};
use rand::rngs::OsRng;
use rand::RngCore;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::zkp::Zkp;

fn encrypt<C: Ctx>(ctx: &C, pk: &PublicKey<C>, data: C::P, n: usize) {
    let mut rng = ctx.get_rng();
    let zkp = Zkp::new(ctx);

    for _ in 0..n {
        let plaintext = ctx.encode(&data).unwrap();
        let randomness = ctx.rnd_exp(&mut rng);
        let c = pk.encrypt_with_randomness(&plaintext, &randomness);

        let _proof = zkp.encryption_popk(&randomness, c.mhr(), c.gr(), &vec![]);
    }
}

fn encrypt_ristretto(
    ctx: &RistrettoCtx,
    pk: &PublicKey<RistrettoCtx>,
    n: usize,
) {
    let mut csprng = OsRng;
    let mut fill = [0u8; 30];
    csprng.fill_bytes(&mut fill);
    let plaintext = to_ristretto_plaintext_array(&fill.to_vec());
    encrypt(ctx, pk, plaintext, n);
}

cfg_if::cfg_if! {
if #[cfg(feature = "num_bigint")] {
    use strand::backend::num_bigint::{BigintCtx, P2048};
    fn encrypt_bigint(
        ctx: &BigintCtx<P2048>,
        pk: &PublicKey<BigintCtx<P2048>>,
        n: usize,
    ) {
        let mut rng = ctx.get_rng();
        let plaintext = ctx.rnd_plaintext(&mut rng);
        encrypt(ctx, pk, plaintext, n);
    }
}
}

cfg_if::cfg_if! {
if #[cfg(feature = "malachite")] {
    use strand::backend::malachite::{MalachiteCtx, P2048 as MP2048};
    fn encrypt_malachite(
        ctx: &MalachiteCtx<MP2048>,
        pk: &PublicKey<MalachiteCtx<MP2048>>,
        n: usize,
    ) {
        let plaintext = ctx.rnd_plaintext();
        encrypt(ctx, pk, plaintext, n);
    }
}
}

cfg_if::cfg_if! {
if #[cfg(feature = "rug")] {
    use strand::backend::rug::RugCtx;
    use strand::backend::rug::P2048 as RP2048;
    fn encrypt_rug(ctx: &RugCtx<RP2048>, pk: &PublicKey<RugCtx<RP2048>>, n: usize) {
        let plaintext = ctx.rnd_plaintext();
        encrypt(ctx, pk, plaintext, n);
    }
}
}

fn bench_encrypt(c: &mut Criterion) {
    let rctx = RistrettoCtx;
    let rsk = PrivateKey::gen(&rctx);
    let rpk = rsk.get_pk();

    cfg_if::cfg_if! {
        if #[cfg(feature = "num_bigint")] {
            let bctx: BigintCtx<P2048> = Default::default();
            let bsk = PrivateKey::gen(&bctx);
            let bpk = bsk.get_pk();
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "malachite")] {
        let mctx: MalachiteCtx<MP2048> = Default::default();
        let msk = PrivateKey::gen(&mctx);
        let mpk = msk.get_pk();
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(feature = "rug")] {
            use strand::backend::rug::P2048 as RP2048;
            let gctx: RugCtx::<RP2048> = Default::default();
            let gsk = PrivateKey::gen(&gctx);
            let gpk = gsk.get_pk();
        }
    }

    let mut group = c.benchmark_group("encrypt");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    for i in [10usize].iter() {
        group.bench_with_input(BenchmarkId::new("ristretto", i), i, |b, i| {
            b.iter(|| encrypt_ristretto(&rctx, &rpk, *i))
        });
        #[cfg(feature = "num_bigint")]
        group.bench_with_input(BenchmarkId::new("bigint", i), i, |b, i| {
            b.iter(|| encrypt_bigint(&bctx, &bpk, *i))
        });
        #[cfg(feature = "malachite")]
        group.bench_with_input(BenchmarkId::new("malachite", i), i, |b, i| {
            b.iter(|| encrypt_malachite(&mctx, &mpk, *i))
        });
        #[cfg(feature = "rug")]
        group.bench_with_input(BenchmarkId::new("rug", i), i, |b, i| {
            b.iter(|| encrypt_rug(&gctx, &gpk, *i))
        });
    }
    group.finish();
}

pub(crate) fn to_ristretto_plaintext_array(input: &[u8]) -> [u8; 30] {
    strand::util::to_u8_array(input).unwrap()
}

criterion_group!(benches, bench_encrypt);
criterion_main!(benches);
