// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode,
};
use rand::rngs::OsRng;
use rand::RngCore;

use strand::backend::ristretto::RistrettoCtx;
use strand::context::Ctx;
use strand::elgamal::*;
use strand::serialization::StrandSerialize;
use strand::signature::{
    StrandSignature, StrandSignaturePk, StrandSignatureSk,
};
use strand::zkp::{Schnorr, Zkp};

#[derive(BorshSerialize, BorshDeserialize)]
struct Ballot<C: Ctx> {
    ciphertext: Ciphertext<C>,
    popk: Schnorr<C>,
    signature: StrandSignature,
}

fn ballots<C: Ctx>(
    ctx: &C,
    pk: &PublicKey<C>,
    data: C::P,
    n: usize,
    sk: &StrandSignatureSk,
) -> Vec<Ballot<C>> {
    let mut rng = ctx.get_rng();
    let zkp = Zkp::new(ctx);

    let mut ballots = vec![];

    for _ in 0..n {
        let plaintext = ctx.encode(&data).unwrap();
        let randomness = ctx.rnd_exp(&mut rng);
        let c = pk.encrypt_with_randomness(&plaintext, &randomness);
        let c_bytes = c.strand_serialize().unwrap();
        let signature = sk.sign(&c_bytes).unwrap();
        let proof = zkp
            .encryption_popk(&randomness, c.mhr(), c.gr(), &vec![])
            .unwrap();
        let ballot = Ballot {
            ciphertext: c,
            popk: proof,
            signature,
        };
        ballots.push(ballot);
    }

    ballots
}

fn verify<C: Ctx>(ctx: &C, ballots: &Vec<Ballot<C>>, vk: &StrandSignaturePk) {
    let zkp = Zkp::new(ctx);
    let label = vec![];
    for b in ballots {
        let ok = zkp
            .encryption_popk_verify(
                &b.ciphertext.mhr,
                &b.ciphertext.gr,
                &b.popk,
                &label,
            )
            .unwrap();
        assert!(ok);
        let bytes = b.ciphertext.strand_serialize().unwrap();
        vk.verify(&b.signature, &bytes).unwrap();
    }
}

fn verify_ristretto(
    ctx: &RistrettoCtx,
    ballots: &Vec<Ballot<RistrettoCtx>>,
    vk: &StrandSignaturePk,
) {
    verify(ctx, ballots, vk)
}

fn ballots_ristretto(
    ctx: &RistrettoCtx,
    pk: &PublicKey<RistrettoCtx>,
    n: usize,
) -> (Vec<Ballot<RistrettoCtx>>, StrandSignaturePk) {
    let mut csprng = OsRng;
    let mut fill = [0u8; 30];
    csprng.fill_bytes(&mut fill);
    let sk = StrandSignatureSk::gen().unwrap();

    (
        ballots(ctx, pk, fill, n, &sk),
        StrandSignaturePk::from_sk(&sk).unwrap(),
    )
}

cfg_if::cfg_if! {
if #[cfg(feature = "num_bigint")] {
    use strand::backend::num_bigint::{BigintCtx, P2048};
    fn ballots_bigint(
        ctx: &BigintCtx<P2048>,
        pk: &PublicKey<BigintCtx<P2048>>,
        n: usize,
    ) {

    }
}
}

cfg_if::cfg_if! {
if #[cfg(feature = "malachite")] {
    use strand::backend::malachite::{MalachiteCtx, P2048 as MP2048};
    fn ballots_malachite(
        ctx: &MalachiteCtx<MP2048>,
        pk: &PublicKey<MalachiteCtx<MP2048>>,
        n: usize,
    ) {

    }
}
}

cfg_if::cfg_if! {
if #[cfg(feature = "rug")] {
    use strand::backend::rug::RugCtx;
    use strand::backend::rug::P2048 as RP2048;
    fn ballots_rug(ctx: &RugCtx<RP2048>, pk: &PublicKey<RugCtx<RP2048>>, n: usize) {

    }
}
}

fn bench_verify(c: &mut Criterion) {
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

    let mut group = c.benchmark_group("verify_ballot");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(10);

    for i in [1000usize].iter() {
        let (ballots, vk) = ballots_ristretto(&rctx, &rpk, *i);
        group.bench_with_input(BenchmarkId::new("ristretto", i), i, |b, _i| {
            b.iter(|| verify_ristretto(&rctx, &ballots, &vk))
        });
        /* TODO

        #[cfg(feature = "num_bigint")]
        let (ballots, vk) = ballots_bigint(&rctx, &rpk, *i);
        group.bench_with_input(BenchmarkId::new("bigint", i), i, |b, i| {
            b.iter(|| verify_bigint(&bctx, &bpk, *i))
        });
        #[cfg(feature = "malachite")]
        let (ballots, vk) = ballots_malachite(&rctx, &rpk, *i);
        group.bench_with_input(BenchmarkId::new("malachite", i), i, |b, i| {
            b.iter(|| verify_malachite(&mctx, &mpk, *i))
        });
        #[cfg(feature = "rug")]
        let (ballots, vk) = ballots_rug(&rctx, &rpk, *i);
        group.bench_with_input(BenchmarkId::new("rug", i), i, |b, i| {
            b.iter(|| verify_rug(&gctx, &gpk, *i))
        });*/
    }
    group.finish();
}

criterion_group!(benches, bench_verify);
criterion_main!(benches);
