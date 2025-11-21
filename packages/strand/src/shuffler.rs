#![allow(clippy::type_complexity)]
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! # Examples
//!
//! ```
//! // This example shows how to shuffle ciphertexts and generate a proof.
//! use strand::context::Ctx;
//! use strand::backend::ristretto::RistrettoCtx;
//! use strand::elgamal::{PrivateKey, PublicKey};
//! use strand::util;
//! use strand::shuffler::Shuffler;
//!
//! let ctx = RistrettoCtx;
//! let sk = PrivateKey::gen(&ctx);
//! let pk = sk.get_pk();
//!
//! let es = util::random_ciphertexts(10, &ctx);
//! let seed = vec![];
//! let hs = ctx.generators(es.len() + 1, &seed).unwrap();
//! let shuffler = Shuffler::new(
//!    &pk,
//!    &ctx,
//! );
//! let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
//! let proof =
//!    shuffler.gen_proof(es.clone(), &e_primes, rs, hs.clone(), perm, &[]).unwrap();
//! let ok = shuffler.check_proof(&proof, es, e_primes, hs, &[]).unwrap();
//!
//! assert!(ok);
//! ```

use borsh::{BorshDeserialize, BorshSerialize};
use rand::seq::SliceRandom;
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
// use std::time::Instant;

use crate::context::{Ctx, Element, Exponent};
use crate::elgamal::{Ciphertext, PublicKey};
use crate::rng::StrandRng;
use crate::serialization::StrandSerialize;
use crate::serialization::StrandVector;
use crate::util::{Par, StrandError};
use crate::zkp::ChallengeInput;

pub(crate) struct YChallengeInput<'a, C: Ctx> {
    // pub es: &'a [Ciphertext<C>],
    // pub e_primes: &'a [Ciphertext<C>],
    pub es: Vec<u8>,
    pub e_primes: Vec<u8>,
    pub cs: &'a [C::E],
    pub c_hats: &'a [C::E],
    pub pk: &'a PublicKey<C>,
}

/// Shuffle proof commitments.
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct Commitments<C: Ctx> {
    pub t1: C::E,
    pub t2: C::E,
    pub t3: C::E,
    pub t4_1: C::E,
    pub t4_2: C::E,
    pub t_hats: StrandVector<C::E>,
}

/// Shuffle proof responses.
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct Responses<C: Ctx> {
    pub(crate) s1: C::X,
    pub(crate) s2: C::X,
    pub(crate) s3: C::X,
    pub(crate) s4: C::X,
    pub(crate) s_hats: StrandVector<C::X>,
    pub(crate) s_primes: StrandVector<C::X>,
}

/// A proof of shuffle.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct ShuffleProof<C: Ctx> {
    // proof commitment
    pub(crate) t: Commitments<C>,
    // proof response
    pub(crate) s: Responses<C>,
    // permutation commitment
    pub(crate) cs: StrandVector<C::E>,
    // commitment chain
    pub(crate) c_hats: StrandVector<C::E>,
}

pub(super) struct PermutationData<C: Ctx> {
    pub(crate) permutation: Vec<usize>,
    pub(crate) commitments_c: Vec<C::E>,
    pub(crate) commitments_r: Vec<C::X>,
}

/// Interface to ciphertext shuffling and verifying.
pub struct Shuffler<'a, C: Ctx> {
    pub(crate) pk: &'a PublicKey<C>,
    // pub(crate) generators: &'a Vec<C::E>,
    pub(crate) ctx: C,
}

impl<'a, C: Ctx> Shuffler<'a, C> {
    /// Constructs a new shuffler.
    pub fn new(
        pk: &'a PublicKey<C>,
        // generators: &'a Vec<C::E>,
        ctx: &C,
    ) -> Shuffler<'a, C> {
        Shuffler {
            pk,
            // generators,
            ctx: ctx.clone(),
        }
    }

    /// Generates a shuffle of the given ciphertexts, returning the resulting
    /// shuffle, the random re-encryption factors and the applied
    /// permutation. NOTE: the second and third returned parameters are SECRETS.
    pub fn gen_shuffle(
        &self,
        ciphertexts: &[Ciphertext<C>],
    ) -> (Vec<Ciphertext<C>>, Vec<C::X>, Vec<usize>) {
        let perm: Vec<usize> = gen_permutation(ciphertexts.len());

        let (result, rs) = self.apply_permutation(&perm, ciphertexts);
        (result, rs, perm)
    }

    pub(crate) fn apply_permutation(
        &self,
        perm: &[usize],
        ciphertexts: &[Ciphertext<C>],
    ) -> (Vec<Ciphertext<C>>, Vec<C::X>) {
        assert!(perm.len() == ciphertexts.len());

        let ctx = &self.ctx;
        let rng = Arc::new(Mutex::new(ctx.get_rng()));
        // let now = Instant::now(); println!("apply perm par..");
        let (e_primes, rs): (Vec<Ciphertext<C>>, Vec<C::X>) = ciphertexts
            .par()
            .map(|c| {
                // It is idiomatic to unwrap on lock
                let mut rng_ = rng.lock().unwrap();

                let r = ctx.rnd_exp(&mut rng_);
                drop(rng_);

                let a =
                    c.mhr.mul(&ctx.emod_pow(&self.pk.element, &r)).modp(ctx);
                let b = c.gr.mul(&ctx.gmod_pow(&r)).modp(ctx);

                let c_ = Ciphertext { mhr: a, gr: b };
                (c_, r)
            })
            .unzip();

        // println!("apply perm par: {}", now.elapsed().as_millis());

        let mut e_primes_permuted: Vec<Ciphertext<C>> = vec![];
        for p in perm {
            e_primes_permuted.push(e_primes[*p].clone());
        }

        (e_primes_permuted, rs)
    }

    /// Computes a proof of shuffle given the original ciphertexts, shuffled
    /// ciphertexts, and shuffling secrets. Called after gen_shuffle.
    pub fn gen_proof(
        &self,
        es: Vec<Ciphertext<C>>,
        e_primes: &[Ciphertext<C>],
        r_primes: Vec<C::X>,
        generators: Vec<C::E>,
        perm: Vec<usize>,
        label: &[u8],
    ) -> Result<ShuffleProof<C>, StrandError> {
        // let now = Instant::now(); println!("gen_commitments..");
        let (cs, rs) = self.gen_commitments(&perm, &generators, &self.ctx);
        // println!("gen_commitments {}", now.elapsed().as_millis());

        let perm_data = PermutationData {
            permutation: perm,
            commitments_c: cs,
            commitments_r: rs,
        };

        // let now = Instant::now();
        let (proof, _) = self.gen_proof_ext(
            es, e_primes, r_primes, generators, perm_data, label,
        )?;
        // println!("gen_proof_ext {}", now.elapsed().as_millis());

        Ok(proof)
    }

    // gen_proof_ext has support for
    // 1. Returns extra data used for coq test transcript >> UPDATE: removed
    //    will break
    // test_gen_coq_data() in rug.rs
    pub(super) fn gen_proof_ext(
        &self,
        es: Vec<Ciphertext<C>>,
        e_primes: &[Ciphertext<C>],
        r_primes: Vec<C::X>,
        generators: Vec<C::E>,
        perm_data: PermutationData<C>,
        label: &[u8],
    ) -> Result<(ShuffleProof<C>, /* Vec<C::X> , */ C::X), StrandError> {
        let ctx = &self.ctx;
        let mut rng = ctx.get_rng();

        #[allow(non_snake_case)]
        let N = es.len();

        let h_generators = &generators[1..];
        let h_initial = &generators[0].clone();

        assert!(N == e_primes.len());
        assert!(N == r_primes.len());
        assert!(N == perm_data.permutation.len());
        assert!(N == h_generators.len());
        assert!(N > 0, "cannot shuffle 0 ciphertexts");

        let (cs, rs) = (perm_data.commitments_c, perm_data.commitments_r);
        let perm = perm_data.permutation;

        let es_bytes = serialize_flatten(&es)?;
        drop(es);
        let e_primes_bytes = serialize_flatten(e_primes)?;

        // COST
        // let now = Instant::now(); println!("shuffle proof us..");
        let us = self.shuffle_proof_us(
            es_bytes.clone(),
            e_primes_bytes.clone(),
            &cs,
            N,
            label,
        )?;
        // println!("shuffle proof us {}", now.elapsed().as_millis());

        let mut u_primes: Vec<&C::X> = Vec::with_capacity(N);
        for &i in perm.iter() {
            u_primes.push(&us[i]);
        }

        drop(perm);

        // COST
        // let now = Instant::now(); println!("gen_commitment_chain..");

        let (c_hats, r_hats) =
            self.gen_commitment_chain(h_initial, &u_primes, ctx);

        // println!("gen_commitment_chain {}", now.elapsed().as_millis());

        // let now = Instant::now();  println!("block 1..");

        let mut vs = vec![C::X::mul_identity(); N];
        for i in (0..N - 1).rev() {
            vs[i] = u_primes[i + 1].mul(&vs[i + 1]).modq(ctx);
        }

        let mut r_bar = C::X::add_identity();
        let mut r_hat: C::X = C::X::add_identity();
        let mut r_tilde: C::X = C::X::add_identity();
        let mut r_prime: C::X = C::X::add_identity();

        for i in 0..N {
            r_bar = r_bar.add(&rs[i]);
            r_hat = r_hat.add(&r_hats[i].mul(&vs[i]));
            r_tilde = r_tilde.add(&rs[i].mul(&us[i]));
            r_prime = r_prime.add(&r_primes[i].mul(&us[i]));
        }

        drop(vs);
        drop(rs);
        drop(r_primes);

        r_bar = r_bar.modq(ctx);
        r_hat = r_hat.modq(ctx);
        r_tilde = r_tilde.modq(ctx);
        r_prime = r_prime.modq(ctx);

        let omegas: Vec<C::X> = (0..4).map(|_| ctx.rnd_exp(&mut rng)).collect();
        let omega_hats: Vec<C::X> =
            (0..N).map(|_| ctx.rnd_exp(&mut rng)).collect();
        let omega_primes: Vec<C::X> =
            (0..N).map(|_| ctx.rnd_exp(&mut rng)).collect();

        let t1 = ctx.gmod_pow(&omegas[0]);
        let t2 = ctx.gmod_pow(&omegas[1]);

        let mut t3_temp = C::E::mul_identity();
        let mut t4_1_temp = C::E::mul_identity();
        let mut t4_2_temp = C::E::mul_identity();
        // println!("block 1 {}", now.elapsed().as_millis());

        // let now = Instant::now();  println!("par 1..");
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

        drop(generators);
        // println!("par 1 {}", now.elapsed().as_millis());

        // let now = Instant::now();  println!("block 2..");

        for value in values.iter().take(N) {
            t3_temp = t3_temp.mul(&value.0).modp(ctx);
            t4_1_temp = t4_1_temp.mul(&value.1).modp(ctx);
            t4_2_temp = t4_2_temp.mul(&value.2).modp(ctx);
        }

        drop(values);

        let t3 = (ctx.gmod_pow(&omegas[2])).mul(&t3_temp).modp(ctx);
        let t4_1 = (ctx.emod_pow(&self.pk.element.invp(ctx), &omegas[3]))
            .mul(&t4_1_temp)
            .modp(ctx);
        let t4_2 = (ctx.emod_pow(&ctx.generator().invp(ctx), &omegas[3]))
            .mul(&t4_2_temp)
            .modp(ctx);

        // println!("block 2 {}", now.elapsed().as_millis());

        // let now = Instant::now();  println!("par 2..");
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
        // println!("par 2 {}", now.elapsed().as_millis());

        let y = YChallengeInput {
            es: es_bytes,
            e_primes: e_primes_bytes,
            cs: &cs,
            c_hats: &c_hats,
            pk: self.pk,
        };

        let t = Commitments {
            t1,
            t2,
            t3,
            t4_1,
            t4_2,
            t_hats: StrandVector(t_hats),
        };

        // COST
        let c: C::X = self.shuffle_proof_challenge(y, &t, label)?;

        // println!("shuffle proof challenge {}", now.elapsed().as_millis());

        // let now = Instant::now(); println!("block 3..");

        let s1 = omegas[0].add(&c.mul(&r_bar)).modq(ctx);
        let s2 = omegas[1].add(&c.mul(&r_hat)).modq(ctx);
        let s3 = omegas[2].add(&c.mul(&r_tilde)).modq(ctx);
        let s4 = omegas[3].add(&c.mul(&r_prime)).modq(ctx);

        let mut s_hats: Vec<C::X> = Vec::with_capacity(N);
        let mut s_primes: Vec<C::X> = Vec::with_capacity(N);

        for i in 0..N {
            let next_s_hat = omega_hats[i].add(&c.mul(&r_hats[i])).modq(ctx);
            let next_s_prime =
                omega_primes[i].add(&c.mul(u_primes[i])).modq(ctx);

            s_hats.push(next_s_hat);
            s_primes.push(next_s_prime);
        }

        drop(u_primes);
        drop(us);
        drop(omega_hats);
        drop(omega_primes);

        // println!("block 3 {}", now.elapsed().as_millis());

        let s = Responses {
            s1,
            s2,
            s3,
            s4,
            s_hats: StrandVector(s_hats),
            s_primes: StrandVector(s_primes),
        };

        Ok((
            ShuffleProof {
                t,
                s,
                cs: StrandVector(cs),
                c_hats: StrandVector(c_hats),
            },
            // us,
            c,
        ))
    }

    /// Checks a proof against the original ciphertexts and permuted
    /// ciphertexts. Returns true if verification passes.
    pub fn check_proof(
        &self,
        proof: &ShuffleProof<C>,
        es: Vec<Ciphertext<C>>,
        e_primes: Vec<Ciphertext<C>>,
        generators: Vec<C::E>,
        label: &[u8],
    ) -> Result<bool, StrandError> {
        let ctx = &self.ctx;

        #[allow(non_snake_case)]
        let N = es.len();

        let h_generators = &generators[1..];
        let h_initial = &generators[0].clone();

        assert!(N == e_primes.len());
        assert!(N == h_generators.len());

        let es_bytes = serialize_flatten(&es)?;
        let e_primes_bytes = serialize_flatten(&e_primes)?;

        let us: Vec<C::X> = self.shuffle_proof_us(
            es_bytes.clone(),
            e_primes_bytes.clone(),
            &proof.cs.0,
            N,
            label,
        )?;

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

        drop(es);
        drop(e_primes);

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

        drop(generators);
        drop(us);
        drop(values);
        // println!("v1 {}", now.elapsed().as_millis());

        let c_bar = c_bar_num.divp(&c_bar_den, ctx).modp(ctx);

        let c_hat = proof.c_hats.0[N - 1]
            .divp(&ctx.emod_pow(h_initial, &u), ctx)
            .modp(ctx);

        let y = YChallengeInput {
            es: es_bytes,
            e_primes: e_primes_bytes,
            cs: &proof.cs.0,
            c_hats: &proof.c_hats.0,
            pk: self.pk,
        };

        let c = self.shuffle_proof_challenge(y, &proof.t, label)?;

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
        generators: &[C::E],
        ctx: &C,
    ) -> (Vec<C::E>, Vec<C::X>) {
        let rng = Arc::new(Mutex::new(ctx.get_rng()));
        // let generators = &self.generators[1..];
        let generators = &generators[1..];

        assert!(generators.len() == perm.len());

        let (mut cs, mut rs): (Vec<C::E>, Vec<C::X>) = generators
            .par()
            .map(|h| {
                // It is idiomatic to unwrap on lock
                let mut rng_ = rng.lock().unwrap();
                let r = ctx.rnd_exp(&mut rng_);
                drop(rng_);
                let c = h.mul(&ctx.gmod_pow(&r)).modp(ctx);

                (c, r)
            })
            .unzip();

        let mut cs_permuted = vec![C::E::mul_identity(); perm.len()];
        let mut rs_permuted = vec![C::X::mul_identity(); perm.len()];

        for i in (0..perm.len()).rev() {
            cs_permuted[perm[i]] = cs.remove(i);
            rs_permuted[perm[i]] = rs.remove(i);
        }

        /* let mut cs_permuted = vec![C::E::mul_identity(); perm.len()];
        let mut rs_permuted = vec![C::X::mul_identity(); perm.len()];

        for i in 0..perm.len() {
            cs_permuted[perm[i]] = cs[i].clone();
            rs_permuted[perm[i]] = rs[i].clone();
        }*/

        (cs_permuted, rs_permuted)
    }

    fn gen_commitment_chain(
        &self,
        initial: &C::E,
        us: &[&C::X],
        ctx: &C,
    ) -> (Vec<C::E>, Vec<C::X>) {
        let rng = Arc::new(Mutex::new(ctx.get_rng()));

        let mut cs: Vec<C::E> = Vec::with_capacity(us.len());

        let (firsts, rs): (Vec<C::E>, Vec<C::X>) = (0..us.len())
            .par()
            .map(|_| {
                // It is idiomatic to unwrap on lock
                let mut rng_ = rng.lock().unwrap();
                let r = ctx.rnd_exp(&mut rng_);
                drop(rng_);

                let first = ctx.gmod_pow(&r);

                (first, r)
            })
            .unzip();

        // let now = Instant::now();
        // COST
        println!("strand: gen_commitment_chain..");
        for i in 0..us.len() {
            let c_temp = if i == 0 { initial } else { &cs[i - 1] };

            let second = ctx.emod_pow(c_temp, us[i]);
            let c = firsts[i].mul(&second).modp(ctx);

            cs.push(c);
        }
        println!("strand: gen_commitment_chain: complete");

        // println!("v9 {}", now.elapsed().as_millis());

        (cs, rs)
    }

    fn shuffle_proof_us(
        &self,
        // es: &[Ciphertext<C>],
        // e_primes: &[Ciphertext<C>],
        es: Vec<u8>,
        e_primes: Vec<u8>,
        cs: &[C::E],
        n: usize,
        label: &[u8],
    ) -> Result<Vec<C::X>, StrandError> {
        /* let es = serialize_flatten(&es)?;
        let e_primes = serialize_flatten(&e_primes)?;*/

        let mut prefix_challenge_input = ChallengeInput::from_bytes(vec![
            ("es", es),
            ("e_primes", e_primes),
        ]);

        // Copying
        // prefix_challenge_input.add("cs",
        // &StrandVector::<C::E>(cs.to_vec()))?;
        let cs = serialize_flatten(&cs)?;
        prefix_challenge_input.add("cs", &cs)?;
        prefix_challenge_input.add("label", &label.to_vec())?;

        let prefix_bytes = prefix_challenge_input.strand_serialize()?;

        // optimization: instead of calculating u = H(prefix || i),
        // we do u = H(H(prefix) || i)
        // that way we avoid allocating prefix-size bytes n times
        let prefix_hash = crate::hash::hash_to_array(&prefix_bytes)?;

        let us: Result<Vec<C::X>, StrandError> = (0..n)
            .par()
            .map(|i| {
                let next = [
                    ("prefix", &prefix_hash[0..]),
                    ("counter", &i.to_le_bytes()[0..]),
                ];
                /*let next = ChallengeInput::from_bytes(vec![
                    ("prefix", prefix_hash.clone()),
                    ("counter", i.to_le_bytes().to_vec()),
                ]);
                // let bytes = next.get_bytes();*/
                let bytes = borsh::to_vec(&next).map_err(|e| e.into());

                let z: Result<C::X, StrandError> = match bytes {
                    Err(e) => Err(e),
                    Ok(b) => Ok(self.ctx.hash_to_exp(&b)?),
                };
                z
            })
            .collect();

        us
    }

    fn shuffle_proof_challenge(
        &self,
        y: YChallengeInput<C>,
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

        /*challenge_input
            .add_bytes("es", serialize_flatten(y.es)?);
        challenge_input.add_bytes(
            "e_primes",
            serialize_flatten(y.e_primes)?,
        );*/
        challenge_input.add_bytes("es", y.es);
        challenge_input.add_bytes("e_primes", y.e_primes);

        challenge_input.add_bytes("cs", serialize_flatten(y.cs)?);
        challenge_input.add_bytes("c_hats", serialize_flatten(y.c_hats)?);

        challenge_input
            .add_bytes("pk.element", y.pk.element.strand_serialize()?);
        challenge_input.add_bytes("t_hats", t.t_hats.strand_serialize()?);
        challenge_input.add_bytes("label", label.to_vec());

        let bytes = challenge_input.get_bytes()?;

        Ok(self.ctx.hash_to_exp(&bytes)?)
    }
}

// "The resulting permutation is picked uniformly from the set of all possible
// permutations." https://rust-random.github.io/rand/rand/seq/trait.SliceRandom.html
pub(crate) fn gen_permutation(size: usize) -> Vec<usize> {
    let mut rng = StrandRng;

    let mut ret: Vec<usize> = (0..size).collect();
    ret.shuffle(&mut rng);

    ret
}

// Helper to avoid copying data to use StrandVector serialization
fn serialize_flatten<T: Send + Sync + StrandSerialize>(
    v: &[T],
) -> Result<Vec<u8>, StrandError> {
    let bytes: Result<Vec<Vec<u8>>, StrandError> =
        v.par().map(|v| v.strand_serialize()).collect();

    // Ok(bytes?.into_iter().flatten().collect())
    Ok(bytes?.strand_serialize()?)
}
/*
// For some reason, deriving these does not work
impl<C: Ctx> BorshSerialize for ShuffleProof<C> {
    fn serialize<W: std::io::Write>(
        &self,
        writer: &mut W,
    ) -> std::io::Result<()> {
        let mut bytes: Vec<Vec<u8>> = vec![];
        bytes.push(borsh::to_vec(&self.t)?);
        bytes.push(borsh::to_vec(&self.s)?);
        bytes.push(borsh::to_vec(&self.cs)?);
        bytes.push(borsh::to_vec(&self.c_hats)?);

        bytes.serialize(writer)
    }
}

impl<C: Ctx> BorshDeserialize for ShuffleProof<C> {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let bytes = <Vec<Vec<u8>>>::deserialize_reader(reader)?;
        let t = Commitments::<C>::try_from_slice(&bytes[0])?;
        let s = Responses::<C>::try_from_slice(&bytes[1])?;
        let cs = StrandVector::<C::E>::try_from_slice(&bytes[2])?;
        let c_hats = StrandVector::<C::E>::try_from_slice(&bytes[3])?;


        Ok(ShuffleProof {
            t,
            s,
            cs,
            c_hats
        })
    }
}*/
