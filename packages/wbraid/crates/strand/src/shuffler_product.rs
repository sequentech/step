#![allow(clippy::type_complexity)]
// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
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
//! use strand::shuffler_product::Shuffler;
//!
//! let ctx = RistrettoCtx;
//! let sk = PrivateKey::gen(&ctx);
//! let pk = sk.get_pk();
//!
//! let es = util::random_product_ciphertexts(10, 3, &ctx);
//! let seed = vec![];
//! let hs = ctx.generators(es.rows().len() + 1, &seed).unwrap();
//! let shuffler = Shuffler::new(
//!    &pk,
//!    &hs,
//!    &ctx,
//! );
//! let (e_primes, rs, perm) = shuffler.gen_shuffle(&es);
//! let proof =
//!    shuffler.gen_proof(&es, &e_primes, rs, &perm, &[]).unwrap();
//! let ok = shuffler.check_proof(&proof, &es, &e_primes, &[]).unwrap();
//!
//! assert!(ok);
//! ```

// r_diamond in haines appears as r_hat in haenni
// r_star in in haines appears as r_prime in haenni
// r_big in haines appears as r_prime (vector form) in haenni
#[cfg(feature = "rayon")]
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use borsh::{BorshDeserialize, BorshSerialize};
use rand::seq::SliceRandom;

use crate::context::{Ctx, Element, Exponent};
use crate::elgamal::{Ciphertext, PublicKey};
use crate::rng::StrandRng;
use crate::serialization::StrandSerialize;
use crate::serialization::StrandVector;
use crate::util::{Par, StrandError};
use crate::zkp::ChallengeInput;

pub(crate) struct YChallengeInput<'a, C: Ctx> {
    pub es: &'a Vec<Vec<Ciphertext<C>>>,
    pub e_primes: &'a Vec<Vec<Ciphertext<C>>>,
    pub cs: &'a [C::E],
    pub c_hats: &'a [C::E],
    pub pk: &'a PublicKey<C>,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
/// Shuffle proof commitments.
pub struct Commitments<C: Ctx> {
    pub t1: C::E,
    pub t2: C::E,
    pub t3: C::E,
    // w-length
    pub t4_1s: Vec<C::E>,
    // w-length
    pub t4_2s: Vec<C::E>,
    pub t_hats: StrandVector<C::E>,
}

#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
/// Shuffle proof responses.
pub struct Responses<C: Ctx> {
    pub(crate) s1: C::X,
    pub(crate) s2: C::X,
    pub(crate) s3: C::X,
    // w-length
    pub(crate) s4s: Vec<C::X>,
    pub(crate) s_hats: StrandVector<C::X>,
    pub(crate) s_primes: StrandVector<C::X>,
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
/// A proof of shuffle.
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

pub(super) struct PermutationData<'a, C: Ctx> {
    pub(crate) permutation: &'a [usize],
    pub(crate) commitments_c: &'a [C::E],
    pub(crate) commitments_r: &'a [C::X],
}

#[derive(Clone, Debug)]
/// A rectangular array, for example to represent a list of product ciphertexts,
/// all of equal width.
pub struct StrandRectangle<T: Send + Sync> {
    rows: Vec<Vec<T>>,
}
impl<T: Send + Sync> StrandRectangle<T> {
    pub fn new(rows: Vec<Vec<T>>) -> Result<Self, StrandError> {
        if !Self::is_rectangular(&rows) {
            Err(StrandError::Generic("Not a rectangular array".to_string()))
        } else {
            Ok(StrandRectangle { rows })
        }
    }

    pub(crate) fn new_unchecked(rows: Vec<Vec<T>>) -> Self {
        StrandRectangle { rows }
    }

    pub fn rows(&self) -> &Vec<Vec<T>> {
        &self.rows
    }

    pub fn width(&self) -> usize {
        if self.rows.len() < 1 {
            0
        } else {
            self.rows[0].len()
        }
    }

    pub fn is_rectangular(rows: &Vec<Vec<T>>) -> bool {
        let width: HashSet<usize> = rows.par().map(|r| r.len()).collect();

        width.len() == 1
    }
}

/// Interface to ciphertext shuffling and verifying.
pub struct Shuffler<'a, C: Ctx> {
    pub(crate) pk: &'a PublicKey<C>,
    pub(crate) generators: &'a Vec<C::E>,
    pub(crate) ctx: C,
}

impl<'a, C: Ctx> Shuffler<'a, C> {
    /// Constructs a new shuffler.
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

    /// Generates a shuffle of the given ciphertexts, returning the resulting
    /// shuffle, the random re-encryption factors and the applied
    /// permutation. NOTE: the second and third returned parameters are SECRETS.
    pub fn gen_shuffle(
        &self,
        ciphertexts: &StrandRectangle<Ciphertext<C>>,
    ) -> (StrandRectangle<Ciphertext<C>>, Vec<Vec<C::X>>, Vec<usize>) {
        let perm: Vec<usize> = gen_permutation(ciphertexts.rows().len());
        let (result, rs) = self.apply_permutation(&perm, &ciphertexts);

        (result, rs, perm)
    }

    pub(crate) fn apply_permutation(
        &self,
        perm: &[usize],
        product_ciphertexts: &StrandRectangle<Ciphertext<C>>,
    ) -> (StrandRectangle<Ciphertext<C>>, Vec<Vec<C::X>>) {
        assert!(perm.len() == product_ciphertexts.rows().len());

        let ctx = &self.ctx;
        let rng = Arc::new(Mutex::new(ctx.get_rng()));

        let (e_primes, rs): (Vec<Vec<Ciphertext<C>>>, Vec<Vec<C::X>>) =
            product_ciphertexts
                .rows()
                .par()
                .map(|cs| {
                    let mut rs = vec![];

                    let e_primes_: Vec<Ciphertext<C>> = cs
                        .iter()
                        .map(|c| {
                            // It is idiomatic to unwrap on lock
                            let mut rng_ = rng.lock().unwrap();
                            let r = ctx.rnd_exp(&mut rng_);
                            drop(rng_);

                            let a = c
                                .mhr
                                .mul(&ctx.emod_pow(&self.pk.element, &r))
                                .modp(ctx);
                            let b = c.gr.mul(&ctx.gmod_pow(&r)).modp(ctx);

                            rs.push(r);
                            Ciphertext { mhr: a, gr: b }
                        })
                        .collect::<Vec<Ciphertext<C>>>();

                    (e_primes_, rs)
                })
                .unzip();

        let mut e_primes_permuted: Vec<Vec<Ciphertext<C>>> = vec![];
        for p in perm {
            e_primes_permuted.push(e_primes[*p].clone());
        }

        (StrandRectangle::new_unchecked(e_primes_permuted), rs)
    }

    /// Computes a proof of shuffle given the original ciphertexts, shuffled
    /// ciphertexts, and shuffling secrets. Called after gen_shuffle.
    pub fn gen_proof(
        &self,
        es: &StrandRectangle<Ciphertext<C>>,
        e_primes: &StrandRectangle<Ciphertext<C>>,
        // r_big in haines appears as r_prime (vector form) in haenni
        r_big: Vec<Vec<C::X>>,
        perm: &[usize],
        label: &[u8],
    ) -> Result<ShuffleProof<C>, StrandError> {
        assert_eq!(self.generators.len(), perm.len() + 1);
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
            self.gen_proof_ext(es, e_primes, r_big, perm_data, label)?;
        // println!("gen_proof_ext {}", now.elapsed().as_millis());

        Ok(proof)
    }

    pub(super) fn gen_proof_ext(
        &self,
        es: &StrandRectangle<Ciphertext<C>>,
        e_primes: &StrandRectangle<Ciphertext<C>>,
        r_big: Vec<Vec<C::X>>,
        perm_data: PermutationData<C>,
        label: &[u8],
    ) -> Result<(ShuffleProof<C>, Vec<C::X>, C::X), StrandError> {
        let ctx = &self.ctx;
        let mut rng = ctx.get_rng();

        #[allow(non_snake_case)]
        let N = es.rows().len();
        let width = es.width();

        let h_generators = &self.generators[1..];
        let h_initial = &self.generators[0];

        if N != e_primes.rows().len() {
            return Err(StrandError::Generic(
                "N != e_primes.rows().len()".to_string(),
            ));
        }
        if N != r_big.len() {
            return Err(StrandError::Generic("N != r_big.len()".to_string()));
        }
        if N != perm_data.permutation.len() {
            return Err(StrandError::Generic(
                "N != perm_data.permutation.len()".to_string(),
            ));
        }
        if N != h_generators.len() {
            return Err(StrandError::Generic(
                "N != h_generators.len()".to_string(),
            ));
        }
        if N <= 0 {
            return Err(StrandError::Generic(
                "Cannot shuffle 0 ciphertexts".to_string(),
            ));
        }
        if width <= 0 {
            return Err(StrandError::Generic(
                "Cannot shuffle 0-width ciphertexts".to_string(),
            ));
        }

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
        // r_diamond in haines appears as r_hat in haenni
        let mut r_diamond: C::X = C::X::add_identity();
        let mut r_tilde: C::X = C::X::add_identity();
        // r_star in in haines appears as r_prime (scalar form) in haenni
        let mut r_star: Vec<C::X> = vec![C::X::add_identity(); width];

        // let now = Instant::now();
        // 0 cost
        for w in 0..width {
            for i in 0..N {
                if w == 0 {
                    r_bar = r_bar.add(&rs[i]);
                    r_diamond = r_diamond.add(&r_hats[i].mul(&vs[i]));
                    r_tilde = r_tilde.add(&rs[i].mul(&us[i]));
                    // r_prime = r_prime.add(&r_primes[i].mul(&us[i]));
                }
                r_star[w] = r_star[w].add(&r_big[i][w].mul(&us[i]));
            }
        }

        // println!("v4 {}", now.elapsed().as_millis());

        r_bar = r_bar.modq(ctx);
        r_diamond = r_diamond.modq(ctx);
        r_tilde = r_tilde.modq(ctx);
        // r_prime = r_prime.modq(ctx);

        for w in 0..width {
            r_star[w] = r_star[w].modq(ctx);
        }

        let omegas: Vec<C::X> = (0..3).map(|_| ctx.rnd_exp(&mut rng)).collect();
        let omega_4: Vec<C::X> =
            (0..width).map(|_| ctx.rnd_exp(&mut rng)).collect();

        let omega_hats: Vec<C::X> =
            (0..N).map(|_| ctx.rnd_exp(&mut rng)).collect();
        let omega_primes: Vec<C::X> =
            (0..N).map(|_| ctx.rnd_exp(&mut rng)).collect();

        let t1 = ctx.gmod_pow(&omegas[0]);
        let t2 = ctx.gmod_pow(&omegas[1]);

        let mut t3_temp = C::E::mul_identity();
        let mut t4_1_temp = vec![C::E::mul_identity(); width];
        let mut t4_2_temp = vec![C::E::mul_identity(); width];

        let values: Vec<(C::E, Vec<C::E>, Vec<C::E>)> = (0..N)
            .par()
            .map(|i| {
                let mut mhrs = vec![];
                let mut grs = vec![];

                for c in &e_primes.rows()[i] {
                    mhrs.push(ctx.emod_pow(&c.mhr, &omega_primes[i]));
                    grs.push(ctx.emod_pow(&c.gr, &omega_primes[i]));
                }

                (
                    ctx.emod_pow(&h_generators[i], &omega_primes[i]),
                    mhrs,
                    grs,
                    // ctx.emod_pow(&e_primes[i].mhr, &omega_primes[i]),
                    // ctx.emod_pow(&e_primes[i].gr, &omega_primes[i]),
                )
            })
            .collect();

        // ~0 cost *
        for value in values.iter() {
            t3_temp = t3_temp.mul(&value.0).modp(ctx);

            for w in 0..width {
                t4_1_temp[w] = t4_1_temp[w].mul(&value.1[w]).modp(ctx);
                t4_2_temp[w] = t4_2_temp[w].mul(&value.2[w]).modp(ctx);
            }
            // t4_1_temp = t4_1_temp.mul(&value.1).modp(ctx);
            // t4_2_temp = t4_2_temp.mul(&value.2).modp(ctx);
        }

        let t3 = (ctx.gmod_pow(&omegas[2])).mul(&t3_temp).modp(ctx);
        let mut t4_1s = vec![C::E::mul_identity(); width];
        let mut t4_2s = vec![C::E::mul_identity(); width];

        for w in 0..width {
            t4_1s[w] = (ctx.emod_pow(&self.pk.element.invp(ctx), &omega_4[w]))
                .mul(&t4_1_temp[w])
                .modp(ctx);

            t4_2s[w] = (ctx.emod_pow(&ctx.generator().invp(ctx), &omega_4[w]))
                .mul(&t4_2_temp[w])
                .modp(ctx);
        }

        /*let t4_1 = (ctx.emod_pow(&self.pk.element.invp(ctx), &omegas[3]))
            .mul(&t4_1_temp)
            .modp(ctx);
        let t4_2 = (ctx.emod_pow(&ctx.generator().invp(ctx), &omegas[3]))
            .mul(&t4_2_temp)
            .modp(ctx);*/

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
            es: es.rows(),
            e_primes: e_primes.rows(),
            cs,
            c_hats: &c_hats,
            pk: self.pk,
        };

        let t = Commitments {
            t1,
            t2,
            t3,
            t4_1s,
            t4_2s,
            t_hats: StrandVector(t_hats),
        };

        // let now = Instant::now();
        // ~0 cost
        let c: C::X = self.shuffle_proof_challenge(&y, &t, label)?;

        // println!("shuffle proof challenge {}", now.elapsed().as_millis());

        let s1 = omegas[0].add(&c.mul(&r_bar)).modq(ctx);
        let s2 = omegas[1].add(&c.mul(&r_diamond)).modq(ctx);
        let s3 = omegas[2].add(&c.mul(&r_tilde)).modq(ctx);

        let mut s4s = vec![C::X::add_identity(); width];
        for w in 0..width {
            s4s[w] = omega_4[w].add(&c.mul(&r_star[w])).modq(ctx);
        }
        // let s4 = omegas[3].add(&c.mul(&r_prime)).modq(ctx);

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
            s4s,
            s_hats: StrandVector(s_hats),
            s_primes: StrandVector(s_primes),
        };

        let cs = cs.to_vec();

        // FIXME zeroize perm_data.perm and r_primes

        Ok((
            ShuffleProof {
                t,
                s,
                cs: StrandVector(cs),
                c_hats: StrandVector(c_hats),
            },
            us,
            c,
        ))
    }

    /// Checks a proof against the original ciphertexts and permuted
    /// ciphertexts. Returns true if verification passes.
    pub fn check_proof(
        &self,
        proof: &ShuffleProof<C>,
        es: &StrandRectangle<Ciphertext<C>>,
        e_primes: &StrandRectangle<Ciphertext<C>>,
        label: &[u8],
    ) -> Result<bool, StrandError> {
        let ctx = &self.ctx;

        #[allow(non_snake_case)]
        let N = es.rows().len();
        let width = es.width();

        let h_generators = &self.generators[1..];
        let h_initial = &self.generators[0];

        if N != e_primes.rows().len() {
            return Err(StrandError::Generic(
                "N != e_primes.rows().len()".to_string(),
            ));
        }
        if N != h_generators.len() {
            return Err(StrandError::Generic(
                "N != h_generators.len()".to_string(),
            ));
        }
        if N <= 0 {
            return Err(StrandError::Generic(
                "Cannot check proof on 0 ciphertexts".to_string(),
            ));
        }
        if width <= 0 {
            return Err(StrandError::Generic(
                "Cannot check proof on 0-width ciphertexts".to_string(),
            ));
        }

        let us: Vec<C::X> =
            self.shuffle_proof_us(es, e_primes, &proof.cs.0, N, label)?;

        let mut c_bar_num: C::E = C::E::mul_identity();
        let mut c_bar_den: C::E = C::E::mul_identity();
        let mut u: C::X = C::X::mul_identity();
        let mut c_tilde: C::E = C::E::mul_identity();
        let mut a_primes: Vec<C::E> = vec![C::E::mul_identity(); width];
        let mut b_primes: Vec<C::E> = vec![C::E::mul_identity(); width];

        let mut t_tilde3_temp: C::E = C::E::mul_identity();
        let mut t_tilde41_temps: Vec<C::E> = vec![C::E::mul_identity(); width];
        let mut t_tilde42_temps: Vec<C::E> = vec![C::E::mul_identity(); width];

        let es = es.rows();
        let e_primes = e_primes.rows();

        let values: Vec<(
            C::E,
            Vec<C::E>,
            Vec<C::E>,
            C::E,
            Vec<C::E>,
            Vec<C::E>,
        )> = (0..N)
            .par()
            .map(|i| {
                let mut e_mhrs = vec![C::E::mul_identity(); width];
                let mut e_grs = vec![C::E::mul_identity(); width];
                let mut e_prime_mhrs = vec![C::E::mul_identity(); width];
                let mut e_prime_grs = vec![C::E::mul_identity(); width];

                for w in 0..width {
                    e_mhrs[w] = ctx.emod_pow(&es[i][w].mhr, &us[i]);
                    e_grs[w] = ctx.emod_pow(&es[i][w].gr, &us[i]);
                    e_prime_mhrs[w] = ctx
                        .emod_pow(&e_primes[i][w].mhr, &proof.s.s_primes.0[i]);
                    e_prime_grs[w] = ctx
                        .emod_pow(&e_primes[i][w].gr, &proof.s.s_primes.0[i]);
                }
                (
                    ctx.emod_pow(&proof.cs.0[i], &us[i]),
                    e_mhrs,
                    e_grs,
                    ctx.emod_pow(&h_generators[i], &proof.s.s_primes.0[i]),
                    e_prime_mhrs,
                    e_prime_grs,
                )

                /*
                (
                    ctx.emod_pow(&proof.cs.0[i], &us[i]),
                    ctx.emod_pow(&es[i].mhr, &us[i]),
                    ctx.emod_pow(&es[i].gr, &us[i]),
                    ctx.emod_pow(&h_generators[i], &proof.s.s_primes.0[i]),
                    ctx.emod_pow(&e_primes[i].mhr, &proof.s.s_primes.0[i]),
                    ctx.emod_pow(&e_primes[i].gr, &proof.s.s_primes.0[i]),
                )
                */
            })
            .collect();

        // let now = Instant::now();

        for i in 0..N {
            c_bar_num = c_bar_num.mul(&proof.cs.0[i]).modp(ctx);
            c_bar_den = c_bar_den.mul(&h_generators[i]).modp(ctx);
            u = u.mul(&us[i]).modq(ctx);

            c_tilde = c_tilde.mul(&values[i].0).modp(ctx);

            t_tilde3_temp = t_tilde3_temp.mul(&values[i].3).modp(ctx);

            for w in 0..width {
                a_primes[w] = a_primes[w].mul(&values[i].1[w]).modp(ctx);
                b_primes[w] = b_primes[w].mul(&values[i].2[w]).modp(ctx);
                t_tilde41_temps[w] =
                    t_tilde41_temps[w].mul(&values[i].4[w]).modp(ctx);
                t_tilde42_temps[w] =
                    t_tilde42_temps[w].mul(&values[i].5[w]).modp(ctx);
            }

            /* c_tilde = c_tilde.mul(&values[i].0).modp(ctx);
            a_prime = a_prime.mul(&values[i].1).modp(ctx);
            b_prime = b_prime.mul(&values[i].2).modp(ctx);
            t_tilde3_temp = t_tilde3_temp.mul(&values[i].3).modp(ctx);
            t_tilde41_temp = t_tilde41_temp.mul(&values[i].4).modp(ctx);
            t_tilde42_temp = t_tilde42_temp.mul(&values[i].5).modp(ctx);*/
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

        let mut t_prime_41s = vec![C::E::mul_identity(); width];
        let mut t_prime_42s = vec![C::E::mul_identity(); width];

        for w in 0..width {
            t_prime_41s[w] = (ctx.emod_pow(&a_primes[w].invp(ctx), &c))
                .mul(&ctx.emod_pow(&self.pk.element.invp(ctx), &proof.s.s4s[w]))
                .mul(&t_tilde41_temps[w])
                .modp(ctx);

            t_prime_42s[w] = (ctx.emod_pow(&b_primes[w].invp(ctx), &c))
                .mul(&ctx.emod_pow(&ctx.generator().invp(ctx), &proof.s.s4s[w]))
                .mul(&t_tilde42_temps[w])
                .modp(ctx);
        }

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
        for w in 0..width {
            checks.push(proof.t.t4_1s[w].eq(&t_prime_41s[w]));
            checks.push(proof.t.t4_2s[w].eq(&t_prime_42s[w]));
        }

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
        let rng = Arc::new(Mutex::new(ctx.get_rng()));
        let generators = &self.generators[1..];

        let (cs, rs): (Vec<C::E>, Vec<C::X>) = generators
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
        let rng = Arc::new(Mutex::new(ctx.get_rng()));

        let mut cs: Vec<C::E> = Vec::with_capacity(us.len());

        let (firsts, rs): (Vec<C::E>, Vec<C::X>) = (0..us.len())
            .par()
            .map(|_| {
                // It is idiomatic to unwrap on lock
                let mut rng_ = rng.lock().unwrap();
                let r = ctx.rnd_exp(&mut rng_);
                drop(rng_);
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
        es: &StrandRectangle<Ciphertext<C>>,
        e_primes: &StrandRectangle<Ciphertext<C>>,
        cs: &[C::E],
        n: usize,
        label: &[u8],
    ) -> Result<Vec<C::X>, StrandError> {
        let mut prefix_challenge_input =
            ChallengeInput::from(&[("es", es), ("e_primes", e_primes)])?;
        // FIXME unnecessary copy of cs
        // prefix_challenge_input.add_bytes("cs",
        // StrandVector::<C::E>(cs.to_vec()).strand_serialize()?);
        let cbytes: Result<Vec<u8>, StrandError> = serialize_flatten(cs);
        prefix_challenge_input.add_bytes("cs", cbytes?);
        prefix_challenge_input.add("label", &label.to_vec())?;

        let prefix_bytes = prefix_challenge_input.strand_serialize()?;

        // optimization: instead of calculating u = H(prefix || i),
        // we do u = H(H(prefix) || i)
        // that way we avoid allocating prefix-size bytes n times
        let prefix_hash = crate::hash::hash(&prefix_bytes)?;

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
                    Ok(b) => Ok(self.ctx.hash_to_exp(&b)?),
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
        ])?;

        challenge_input.add_bytes("t4_1s", t.t4_1s.strand_serialize()?);

        challenge_input.add_bytes("t4_2s", t.t4_2s.strand_serialize()?);

        challenge_input.add_bytes("es", serialize_flatten(y.es)?);
        challenge_input.add_bytes("e_primes", serialize_flatten(y.e_primes)?);
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

    Ok(bytes?.into_iter().flatten().collect())
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
