#![allow(clippy::type_complexity)]
use borsh::{BorshDeserialize, BorshSerialize};
// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rand::seq::SliceRandom;
#[cfg(feature = "rayon")]
use rayon::prelude::*;
// use sha3::{Digest, Sha3_512 as Sha512};
use sha2::Digest;

use crate::context::{Ctx, Element, Exponent};
use crate::elgamal::{Ciphertext, PublicKey};
use crate::rnd::StrandRng;
use crate::serialization::StrandSerialize;
use crate::serialization::{StrandVectorC, StrandVectorE, StrandVectorX};
use crate::util::{Par, StrandError};
use crate::zkp::ChallengeInput;

pub(crate) struct YChallengeInput<'a, C: Ctx> {
    pub es: &'a [Ciphertext<C>],
    pub e_primes: &'a [Ciphertext<C>],
    pub cs: &'a [C::E],
    pub c_hats: &'a [C::E],
    pub pk: &'a PublicKey<C>,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct Commitments<C: Ctx> {
    pub t1: C::E,
    pub t2: C::E,
    pub t3: C::E,
    pub t4_1: C::E,
    pub t4_2: C::E,
    pub t_hats: StrandVectorE<C>,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct Responses<C: Ctx> {
    pub(crate) s1: C::X,
    pub(crate) s2: C::X,
    pub(crate) s3: C::X,
    pub(crate) s4: C::X,
    pub(crate) s_hats: StrandVectorX<C>,
    pub(crate) s_primes: StrandVectorX<C>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct ShuffleProof<C: Ctx> {
    // proof commitment
    pub(crate) t: Commitments<C>,
    // proof response
    pub(crate) s: Responses<C>,
    // permutation commitment
    pub(crate) cs: StrandVectorE<C>,
    // commitment chain
    pub(crate) c_hats: StrandVectorE<C>,
}

pub(super) struct PermutationData<'a, C: Ctx> {
    pub(crate) permutation: &'a [usize],
    pub(crate) commitments_c: &'a [C::E],
    pub(crate) commitments_r: &'a [C::X],
}

pub struct Shuffler<'a, C: Ctx> {
    pub(crate) pk: &'a PublicKey<C>,
    pub(crate) generators: &'a Vec<C::E>,
    pub(crate) ctx: C,
}

impl<'a, C: Ctx> Shuffler<'a, C> {
    pub fn new(
        pk: &'a PublicKey<C>,
        generators: &'a Vec<C::E>,
        ctx: &C,
    ) -> Shuffler<'a, C> {
        Shuffler {
            pk,
            generators,
            ctx: ctx.clone(),
        }
    }

    pub fn gen_shuffle(
        &self,
        ciphertexts: &[Ciphertext<C>],
    ) -> (Vec<Ciphertext<C>>, Vec<C::X>, Vec<usize>) {
        let perm: Vec<usize> = gen_permutation(ciphertexts.len());
        let (result, rs) = self.apply_permutation(&perm, ciphertexts);

        (result, rs, perm)
    }

    pub fn apply_permutation(
        &self,
        perm: &[usize],
        ciphertexts: &[Ciphertext<C>],
    ) -> (Vec<Ciphertext<C>>, Vec<C::X>) {
        assert!(perm.len() == ciphertexts.len());

        let ctx = &self.ctx;

        let (e_primes, rs): (Vec<Ciphertext<C>>, Vec<C::X>) = ciphertexts
            .par()
            .map(|c| {
                let r = ctx.rnd_exp();

                let a =
                    c.mhr.mul(&ctx.emod_pow(&self.pk.element, &r)).modp(ctx);
                let b = c.gr.mul(&ctx.gmod_pow(&r)).modp(ctx);

                let c_ = Ciphertext { mhr: a, gr: b };
                (c_, r)
            })
            .unzip();

        let mut e_primes_permuted: Vec<Ciphertext<C>> = vec![];
        for p in perm {
            e_primes_permuted.push(e_primes[*p].clone());
        }

        (e_primes_permuted, rs)
    }

    pub fn gen_proof(
        &self,
        es: &[Ciphertext<C>],
        e_primes: &[Ciphertext<C>],
        r_primes: &[C::X],
        perm: &[usize],
        label: &[u8],
    ) -> Result<ShuffleProof<C>, StrandError> {
        // let now = Instant::now();
        let (cs, rs) = self.gen_commitments(perm, &self.ctx);
        // println!("gen_commitments {}", now.elapsed().as_millis());

        let perm_data = PermutationData {
            permutation: perm,
            commitments_c: &cs,
            commitments_r: &rs,
        };

        // let now = Instant::now();
        let (proof, _, _) =
            self.gen_proof_ext(es, e_primes, r_primes, &perm_data, label)?;
        // println!("gen_proof_ext {}", now.elapsed().as_millis());

        Ok(proof)
    }

    // gen_proof_ext has support for
    // 1. Returns extra data used for coq test transcript
    // 2. Allows passing in permutation data for multi-shuffling
    pub(super) fn gen_proof_ext(
        &self,
        es: &[Ciphertext<C>],
        e_primes: &[Ciphertext<C>],
        r_primes: &[C::X],
        perm_data: &PermutationData<C>,
        label: &[u8],
    ) -> Result<(ShuffleProof<C>, Vec<C::X>, C::X), StrandError> {
        let ctx = &self.ctx;

        #[allow(non_snake_case)]
        let N = es.len();

        let h_generators = &self.generators[1..];
        let h_initial = &self.generators[0];

        assert!(N == e_primes.len());
        assert!(N == r_primes.len());
        assert!(N == perm_data.permutation.len());
        assert!(N == h_generators.len());
        assert!(N > 0, "cannot shuffle 0 ciphertexts");

        // let gmod = ctx.modulus();

        let (cs, rs) = (perm_data.commitments_c, perm_data.commitments_r);
        let perm = perm_data.permutation;

        // COST
        // let now = Instant::now();
        let us = self.shuffle_proof_us(es, e_primes, cs, N, label)?;
        // println!("shuffle proof us {}", now.elapsed().as_millis());

        let mut u_primes: Vec<&C::X> = Vec::with_capacity(N);
        for &i in perm.iter() {
            u_primes.push(&us[i]);
        }

        // COST
        // let now = Instant::now();

        let (c_hats, r_hats) =
            self.gen_commitment_chain(h_initial, &u_primes, ctx);

        // println!("gen commitment chain {}", now.elapsed().as_millis());

        // 0 cost *
        let mut vs = vec![C::X::mul_identity(); N];
        for i in (0..N - 1).rev() {
            vs[i] = u_primes[i + 1].mul(&vs[i + 1]).modq(ctx);
        }

        let mut r_bar = C::X::add_identity();
        let mut r_hat: C::X = C::X::add_identity();
        let mut r_tilde: C::X = C::X::add_identity();
        let mut r_prime: C::X = C::X::add_identity();

        // let now = Instant::now();
        // 0 cost
        for i in 0..N {
            r_bar = r_bar.add(&rs[i]);
            r_hat = r_hat.add(&r_hats[i].mul(&vs[i]));
            r_tilde = r_tilde.add(&rs[i].mul(&us[i]));
            r_prime = r_prime.add(&r_primes[i].mul(&us[i]));
        }

        // println!("v4 {}", now.elapsed().as_millis());

        r_bar = r_bar.modq(ctx);
        r_hat = r_hat.modq(ctx);
        r_tilde = r_tilde.modq(ctx);
        r_prime = r_prime.modq(ctx);

        let omegas: Vec<C::X> = (0..4).map(|_| ctx.rnd_exp()).collect();
        let omega_hats: Vec<C::X> = (0..N).map(|_| ctx.rnd_exp()).collect();
        let omega_primes: Vec<C::X> = (0..N).map(|_| ctx.rnd_exp()).collect();

        let t1 = ctx.gmod_pow(&omegas[0]);
        let t2 = ctx.gmod_pow(&omegas[1]);

        let mut t3_temp = C::E::mul_identity();
        let mut t4_1_temp = C::E::mul_identity();
        let mut t4_2_temp = C::E::mul_identity();

        let values: Vec<(C::E, C::E, C::E)> = (0..N)
            .par()
            .map(|i| {
                (
                    ctx.emod_pow(&h_generators[i], &omega_primes[i]),
                    ctx.emod_pow(&e_primes[i].mhr, &omega_primes[i]),
                    ctx.emod_pow(&e_primes[i].gr, &omega_primes[i]),
                )
            })
            .collect();

        // ~0 cost *
        for value in values.iter().take(N) {
            t3_temp = t3_temp.mul(&value.0).modp(ctx);
            t4_1_temp = t4_1_temp.mul(&value.1).modp(ctx);
            t4_2_temp = t4_2_temp.mul(&value.2).modp(ctx);
        }

        let t3 = (ctx.gmod_pow(&omegas[2])).mul(&t3_temp).modp(ctx);
        let t4_1 = (ctx.emod_pow(&self.pk.element.invp(ctx), &omegas[3]))
            .mul(&t4_1_temp)
            .modp(ctx);
        let t4_2 = (ctx.emod_pow(&ctx.generator().invp(ctx), &omegas[3]))
            .mul(&t4_2_temp)
            .modp(ctx);

        let t_hats = (0..c_hats.len())
            .par()
            .map(|i| {
                let previous_c =
                    if i == 0 { h_initial } else { &c_hats[i - 1] };

                (ctx.gmod_pow(&omega_hats[i]))
                    .mul(&ctx.emod_pow(previous_c, &omega_primes[i]))
                    .modp(ctx)
            })
            .collect();

        let y = YChallengeInput {
            es,
            e_primes,
            cs,
            c_hats: &c_hats,
            pk: self.pk,
        };

        let t = Commitments {
            t1,
            t2,
            t3,
            t4_1,
            t4_2,
            t_hats: StrandVectorE(t_hats),
        };

        // let now = Instant::now();
        // ~0 cost
        let c: C::X = self.shuffle_proof_challenge(&y, &t, label)?;

        // println!("shuffle proof challenge {}", now.elapsed().as_millis());

        let s1 = omegas[0].add(&c.mul(&r_bar)).modq(ctx);
        let s2 = omegas[1].add(&c.mul(&r_hat)).modq(ctx);
        let s3 = omegas[2].add(&c.mul(&r_tilde)).modq(ctx);
        let s4 = omegas[3].add(&c.mul(&r_prime)).modq(ctx);

        let mut s_hats: Vec<C::X> = Vec::with_capacity(N);
        let mut s_primes: Vec<C::X> = Vec::with_capacity(N);

        // 0 cost
        for i in 0..N {
            let next_s_hat = omega_hats[i].add(&c.mul(&r_hats[i])).modq(ctx);
            let next_s_prime =
                omega_primes[i].add(&c.mul(u_primes[i])).modq(ctx);

            s_hats.push(next_s_hat);
            s_primes.push(next_s_prime);
        }

        let s = Responses {
            s1,
            s2,
            s3,
            s4,
            s_hats: StrandVectorX(s_hats),
            s_primes: StrandVectorX(s_primes),
        };

        let cs = cs.to_vec();

        Ok((
            ShuffleProof {
                t,
                s,
                cs: StrandVectorE(cs),
                c_hats: StrandVectorE(c_hats),
            },
            us,
            c,
        ))
    }

    pub fn check_proof(
        &self,
        proof: &ShuffleProof<C>,
        es: &[Ciphertext<C>],
        e_primes: &[Ciphertext<C>],
        label: &[u8],
    ) -> Result<bool, StrandError> {
        let ctx = &self.ctx;

        #[allow(non_snake_case)]
        let N = es.len();

        let h_generators = &self.generators[1..];
        let h_initial = &self.generators[0];

        assert!(N == e_primes.len());
        assert!(N == h_generators.len());

        // let gmod = ctx.modulus();

        let us: Vec<C::X> =
            self.shuffle_proof_us(es, e_primes, &proof.cs.0, N, label)?;

        let mut c_bar_num: C::E = C::E::mul_identity();
        let mut c_bar_den: C::E = C::E::mul_identity();
        let mut u: C::X = C::X::mul_identity();
        let mut c_tilde: C::E = C::E::mul_identity();
        let mut a_prime: C::E = C::E::mul_identity();
        let mut b_prime: C::E = C::E::mul_identity();

        let mut t_tilde3_temp: C::E = C::E::mul_identity();
        let mut t_tilde41_temp: C::E = C::E::mul_identity();
        let mut t_tilde42_temp: C::E = C::E::mul_identity();

        let values: Vec<(C::E, C::E, C::E, C::E, C::E, C::E)> = (0..N)
            .par()
            .map(|i| {
                (
                    ctx.emod_pow(&proof.cs.0[i], &us[i]),
                    ctx.emod_pow(&es[i].mhr, &us[i]),
                    ctx.emod_pow(&es[i].gr, &us[i]),
                    ctx.emod_pow(&h_generators[i], &proof.s.s_primes.0[i]),
                    ctx.emod_pow(&e_primes[i].mhr, &proof.s.s_primes.0[i]),
                    ctx.emod_pow(&e_primes[i].gr, &proof.s.s_primes.0[i]),
                )
            })
            .collect();

        // let now = Instant::now();

        for i in 0..N {
            c_bar_num = c_bar_num.mul(&proof.cs.0[i]).modp(ctx);
            c_bar_den = c_bar_den.mul(&h_generators[i]).modp(ctx);
            u = u.mul(&us[i]).modq(ctx);

            c_tilde = c_tilde.mul(&values[i].0).modp(ctx);
            a_prime = a_prime.mul(&values[i].1).modp(ctx);
            b_prime = b_prime.mul(&values[i].2).modp(ctx);
            t_tilde3_temp = t_tilde3_temp.mul(&values[i].3).modp(ctx);
            t_tilde41_temp = t_tilde41_temp.mul(&values[i].4).modp(ctx);
            t_tilde42_temp = t_tilde42_temp.mul(&values[i].5).modp(ctx);
        }

        // println!("v1 {}", now.elapsed().as_millis());

        let c_bar = c_bar_num.divp(&c_bar_den, ctx).modp(ctx);

        let c_hat = proof.c_hats.0[N - 1]
            .divp(&ctx.emod_pow(h_initial, &u), ctx)
            .modp(ctx);

        let y = YChallengeInput {
            es,
            e_primes,
            cs: &proof.cs.0,
            c_hats: &proof.c_hats.0,
            pk: self.pk,
        };

        let c = self.shuffle_proof_challenge(&y, &proof.t, label)?;

        let t_prime1 = (ctx.emod_pow(&c_bar.invp(ctx), &c))
            .mul(&ctx.gmod_pow(&proof.s.s1))
            .modp(ctx);

        let t_prime2 = (ctx.emod_pow(&c_hat.invp(ctx), &c))
            .mul(&ctx.gmod_pow(&proof.s.s2))
            .modp(ctx);

        let t_prime3 = (ctx.emod_pow(&c_tilde.invp(ctx), &c))
            .mul(&ctx.gmod_pow(&proof.s.s3))
            .mul(&t_tilde3_temp)
            .modp(ctx);

        let t_prime41 = (ctx.emod_pow(&a_prime.invp(ctx), &c))
            .mul(&ctx.emod_pow(&self.pk.element.invp(ctx), &proof.s.s4))
            .mul(&t_tilde41_temp)
            .modp(ctx);

        let t_prime42 = (ctx.emod_pow(&b_prime.invp(ctx), &c))
            .mul(&ctx.emod_pow(&ctx.generator().invp(ctx), &proof.s.s4))
            .mul(&t_tilde42_temp)
            .modp(ctx);

        let t_hat_primes: Vec<C::E> = (0..N)
            .par()
            .map(|i| {
                let c_term = if i == 0 {
                    h_initial
                } else {
                    &proof.c_hats.0[i - 1]
                };

                let inverse = proof.c_hats.0[i].invp(ctx);
                (ctx.emod_pow(&inverse, &c))
                    .mul(&ctx.gmod_pow(&proof.s.s_hats.0[i]))
                    .mul(&ctx.emod_pow(c_term, &proof.s.s_primes.0[i]))
                    .modp(ctx)
            })
            .collect();

        let mut checks = Vec::with_capacity(5 + N);
        checks.push(proof.t.t1.eq(&t_prime1));
        checks.push(proof.t.t2.eq(&t_prime2));
        checks.push(proof.t.t3.eq(&t_prime3));
        checks.push(proof.t.t4_1.eq(&t_prime41));
        checks.push(proof.t.t4_2.eq(&t_prime42));

        for (i, t_hat) in proof.t.t_hats.0.iter().enumerate().take(N) {
            checks.push(t_hat.eq(&t_hat_primes[i]));
        }
        Ok(!checks.contains(&false))
    }

    pub(crate) fn gen_commitments(
        &self,
        perm: &[usize],
        ctx: &C,
    ) -> (Vec<C::E>, Vec<C::X>) {
        let generators = &self.generators[1..];

        assert!(generators.len() == perm.len());

        let (cs, rs): (Vec<C::E>, Vec<C::X>) = generators
            .par()
            .map(|h| {
                let r = ctx.rnd_exp();
                let c = h.mul(&ctx.gmod_pow(&r)).modp(ctx);

                (c, r)
            })
            .unzip();

        let mut cs_permuted = vec![C::E::mul_identity(); perm.len()];
        let mut rs_permuted = vec![C::X::mul_identity(); perm.len()];

        for i in 0..perm.len() {
            cs_permuted[perm[i]] = cs[i].clone();
            rs_permuted[perm[i]] = rs[i].clone();
        }

        (cs_permuted, rs_permuted)
    }

    fn gen_commitment_chain(
        &self,
        initial: &C::E,
        us: &[&C::X],
        ctx: &C,
    ) -> (Vec<C::E>, Vec<C::X>) {
        let mut cs: Vec<C::E> = Vec::with_capacity(us.len());

        let (firsts, rs): (Vec<C::E>, Vec<C::X>) = (0..us.len())
            .par()
            .map(|_| {
                let r = ctx.rnd_exp();
                // let first = ctx.gmod_pow(&r).modulo(ctx.modulus());
                let first = ctx.gmod_pow(&r);

                (first, r)
            })
            .unzip();

        // let now = Instant::now();

        for i in 0..us.len() {
            let c_temp = if i == 0 { initial } else { &cs[i - 1] };

            let second = ctx.emod_pow(c_temp, us[i]);
            let c = firsts[i].mul(&second).modp(ctx);

            cs.push(c);
        }

        // println!("v9 {}", now.elapsed().as_millis());

        (cs, rs)
    }

    fn shuffle_proof_us(
        &self,
        es: &[Ciphertext<C>],
        e_primes: &[Ciphertext<C>],
        cs: &[C::E],
        n: usize,
        label: &[u8],
    ) -> Result<Vec<C::X>, StrandError> {
        let mut prefix_challenge_input = ChallengeInput::from(&[
            ("es", &StrandVectorC(es.to_vec())),
            ("e_primes", &StrandVectorC(e_primes.to_vec())),
        ])?;
        prefix_challenge_input.add("cs", &StrandVectorE::<C>(cs.to_vec()))?;
        prefix_challenge_input.add("label", &label.to_vec())?;

        let prefix_bytes = prefix_challenge_input.strand_serialize()?;

        // optimization: instead of calculating u = H(prefix || i),
        // we do u = H(H(prefix) || i)
        // that way we avoid allocating prefix-size bytes n times
        let mut hasher = crate::util::hasher();
        hasher.update(prefix_bytes);
        let prefix_hash = hasher.finalize().to_vec();

        let us: Result<Vec<C::X>, StrandError> = (0..n)
            .par()
            .map(|i| {
                let next = ChallengeInput::from_bytes(vec![
                    ("prefix", prefix_hash.clone()),
                    ("counter", i.to_le_bytes().to_vec()),
                ]);

                let bytes = next.get_bytes();
                let z: Result<C::X, StrandError> = match bytes {
                    Err(e) => Err(e),
                    Ok(b) => Ok(self.ctx.hash_to_exp(&b)),
                };
                z
            })
            .collect();

        us
    }

    fn shuffle_proof_challenge(
        &self,
        y: &YChallengeInput<C>,
        t: &Commitments<C>,
        label: &[u8],
    ) -> Result<C::X, StrandError> {
        let mut challenge_input = ChallengeInput::from(&[
            ("t1", &t.t1),
            ("t2", &t.t2),
            ("t3", &t.t3),
            ("t4_1", &t.t4_1),
            ("t4_2", &t.t4_2),
        ])?;

        challenge_input
            .add_bytes("es", StrandVectorC(y.es.to_vec()).strand_serialize()?);
        challenge_input.add_bytes(
            "e_primes",
            StrandVectorC(y.e_primes.to_vec()).strand_serialize()?,
        );
        challenge_input.add_bytes(
            "cs",
            StrandVectorE::<C>(y.cs.to_vec()).strand_serialize()?,
        );
        challenge_input.add_bytes(
            "c_hats",
            StrandVectorE::<C>(y.c_hats.to_vec()).strand_serialize()?,
        );
        challenge_input
            .add_bytes("pk.element", y.pk.element.strand_serialize()?);
        challenge_input.add_bytes("t_hats", t.t_hats.strand_serialize()?);
        challenge_input.add_bytes("label", label.to_vec());

        let bytes = challenge_input.get_bytes()?;

        Ok(self.ctx.hash_to_exp(&bytes))
    }
}

pub(crate) fn gen_permutation(size: usize) -> Vec<usize> {
    let mut rng = StrandRng;

    let mut ret: Vec<usize> = (0..size).collect();
    ret.shuffle(&mut rng);

    ret
}
