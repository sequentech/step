// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Shuffler for the Terelius-Wikstrom proof of shuffle

use crate::context::Context;
use crate::cryptosystem::elgamal::{self, Ciphertext};
use crate::traits::groups::CryptographicGroup;
use crate::traits::groups::DistGroupOps;
use crate::traits::groups::DistScalarOps;
use crate::traits::groups::GroupElement;
use crate::traits::groups::GroupScalar;
use crate::traits::groups::ReplGroupOps;
use crate::traits::groups::ReplScalarOps;
use crate::utils::error::Error;
use crate::utils::error::ErrorContext;
use crate::utils::hash;
use crate::utils::serialization::VSerializable;

use rand::RngExt;
use sha3::Digest;
use vser_derive::VSerializable as VSer;

use rayon::prelude::*;

/**
 * Shuffler for the Terelius-Wikstrom proof of shuffle
 *
 * Given lists of ciphertexts `w = w_1, w_2 .. w_n` encrypted under public
 * key `pk`, the function [`shuffle`][`Self::shuffle`]:
 *
 * - Re-encrypts and permutes the ciphertexts producing a list `w' = w'_1, w'_2 .. w'_n`
 * - Computes a corresponding proof of shuffle `P`
 *
 * Conversely, given lists of ciphertexts `w = w_1, w_2 .. w_n`,
 * `w' = w'_1, w'_2 .. w'_n` encrypted under public key `pk`, and a proof
 * of shuffle `P`, the function [`verify`][`Self::verify`]:
 *
 * - Verifies the proof of shuffle `P`
 *
 * The computation and verification of a shuffle proof requires `N` independent
 * generators of the group, that prover and verifier must derive
 * independently from some common data. For convenience, these `h_generators` and
 * the public key `pk` are passed to the `Shuffler` constructor.
 *
 * See `EVS`: Protocol 12.3
 *
 * # Examples
 * ```
 * use std::array;
 *
 * use cryptography::context::Context;
 * use cryptography::context::RistrettoCtx as RCtx;
 * use cryptography::groups::ristretto255::RistrettoElement;
 * use cryptography::groups::ristretto255::Ristretto255Group;
 * use cryptography::cryptosystem::elgamal::Ciphertext;
 * use cryptography::cryptosystem::elgamal::KeyPair;
 * use cryptography::traits::groups::CryptographicGroup;
 * use cryptography::zkp::shuffle::Shuffler;
 *
 * const W: usize = 2;
 *
 * let keypair: KeyPair<RCtx> = KeyPair::generate();
 *
 * // generate some random messages
 * let messages: Vec<[RistrettoElement; W]> = (0..3)
 *     .map(|_| array::from_fn(|_| RCtx::random_element()))
 *     .collect();
 * let ciphertexts: Vec<Ciphertext<RCtx, W>> =
 *     messages.iter().map(|m| keypair.encrypt(m)).collect();
 *
 * let generators_context = &[];
 * let generators = Ristretto255Group::ind_generators(3, generators_context).unwrap();
 *
 * let shuffler = Shuffler::<RCtx, W>::new(generators, keypair.pkey);
 * let proof_context = &[];
 * let (shuffled, proof) = shuffler.shuffle(&ciphertexts, proof_context).unwrap();
 *
 * let ok = shuffler.verify(&ciphertexts, &shuffled, &proof, proof_context).unwrap();
 *
 * assert!(ok);
 * ```
 */
pub struct Shuffler<C: Context, const W: usize> {
    /// List of independent generators matching the size of the input ciphertexts
    h_generators: Vec<C::Element>,
    /// Public key under which the input ciphertexts are encrypted
    pk: elgamal::PublicKey<C>,
}

/**
 * Source of the two Fiat-Shamir challenges in a proof of shuffle.
 *
 * The Terelius-Wikstrom proof fixes the *algebra* but not how its challenges are
 * derived from the transcript; any derivation works as long as prover and
 * verifier agree. Factoring the derivation out behind this trait lets the same,
 * tested proof implementation serve a second transcript convention — in
 * particular Verificatum's, so that proofs braid produces can be checked by an
 * independently written verifier (see `VERIFICATUM.md`).
 *
 * [`NativeChallenges`] is the default and reproduces the previous behaviour
 * exactly, so [`Shuffler::shuffle`] and [`Shuffler::verify`] are unchanged.
 *
 * Implementors must derive both challenges deterministically from public data
 * only. Note the batching challenges are handed the generators and the Pedersen
 * commitments as well as the ciphertexts: braid's own derivation ignores them,
 * but Verificatum's commits to them, and they are available at the point the
 * challenge is needed.
 */
pub trait ShuffleChallenges<C: Context, const W: usize> {
    /// The batching vector `e = (e_1, ..., e_N)`.
    ///
    /// Values are reduced into the scalar field. That is sound for any
    /// convention whose challenges are wider than the group order, because these
    /// values are only ever used as exponents and `g^e = g^(e mod q)`.
    ///
    /// # Errors
    ///
    /// Implementation-defined: whatever the convention's derivation can fail
    /// with — typically hashing to a scalar failing in the backend.
    fn batching_challenges(
        &self,
        generators: &[C::Element],
        pedersen_commitments: &[C::Element],
        pk: &elgamal::PublicKey<C>,
        ciphertexts: &[Ciphertext<C, W>],
        permuted_ciphertexts: &[Ciphertext<C, W>],
        context: &[u8],
    ) -> Result<Vec<C::Scalar>, Error>;

    /// The single challenge `v`, derived from the proof commitments.
    ///
    /// # Errors
    ///
    /// Implementation-defined: whatever the convention's derivation can fail
    /// with — typically hashing to a scalar failing in the backend.
    fn challenge(
        &self,
        pk: &elgamal::PublicKey<C>,
        commitments: &ShuffleCommitments<C, W>,
        context: &[u8],
    ) -> Result<C::Scalar, Error>;
}

/// braid's own challenge derivation: hashing with domain-separation tags into
/// full-width scalars. The default, and the behaviour of [`Shuffler::shuffle`]
/// and [`Shuffler::verify`].
pub struct NativeChallenges;

impl<C: Context, const W: usize> ShuffleChallenges<C, W> for NativeChallenges {
    fn batching_challenges(
        &self,
        _generators: &[C::Element],
        _pedersen_commitments: &[C::Element],
        pk: &elgamal::PublicKey<C>,
        ciphertexts: &[Ciphertext<C, W>],
        permuted_ciphertexts: &[Ciphertext<C, W>],
        context: &[u8],
    ) -> Result<Vec<C::Scalar>, Error> {
        let a = [
            pk.ser(),
            ciphertexts.to_vec().ser(),
            permuted_ciphertexts.to_vec().ser(),
            context.to_vec(),
        ];
        let input: Vec<&[u8]> = a.iter().map(Vec::as_slice).collect();

        let mut hasher = C::get_hasher();
        hash::update_hasher(&mut hasher, &input, &Shuffler::<C, W>::DS_TAGS_CHALLENGE_E);
        let bytes = hasher.finalize();

        let mut ret = Vec::with_capacity(ciphertexts.len());
        for i in 0..ciphertexts.len() {
            // Cannot use platform dependent type in random oracle
            let i_u64 = i as u64;
            let prefix = bytes.clone();
            let inputs: &[&[u8]] = &[prefix.as_slice(), &i_u64.to_be_bytes()];
            let ds_tags: &[&[u8]; 2] = &[b"prefix", b"shuffle_proof_challenge_e_counter"];
            ret.push(C::G::hash_to_scalar(inputs, ds_tags)?);
        }
        Ok(ret)
    }

    fn challenge(
        &self,
        pk: &elgamal::PublicKey<C>,
        commitments: &ShuffleCommitments<C, W>,
        context: &[u8],
    ) -> Result<C::Scalar, Error> {
        let a = [
            pk.ser(),
            commitments.big_b_n.ser(),
            commitments.big_a_prime.ser(),
            commitments.big_b_prime_n.ser(),
            commitments.big_c_prime.ser(),
            commitments.big_d_prime.ser(),
            commitments.big_f_prime.ser(),
            context.to_vec(),
        ];
        let input: Vec<&[u8]> = a.iter().map(Vec::as_slice).collect();
        C::G::hash_to_scalar(&input, &Shuffler::<C, W>::DS_TAGS_CHALLENGE_V)
    }
}

impl<C: Context, const W: usize> Shuffler<C, W> {
    /// Construct a Shuffler with the given values.
    pub fn new(h_generators: Vec<C::Element>, pk: elgamal::PublicKey<C>) -> Self {
        Self { h_generators, pk }
    }

    /// Generate random exponents for ciphertext re-encryption.
    ///
    /// Returns a tuple of form (commitment exponents, re-encryption exponents)
    pub(crate) fn gen_private_exponents(size: usize) -> (Vec<C::Scalar>, Vec<[C::Scalar; W]>) {
        #[crate::warning("The following code is not optimized. Parallelize with rayon")]
        (0..size)
            .into_par_iter()
            .map(|_| {
                let mut rng = C::get_rng();
                (
                    C::Scalar::random(&mut rng),
                    <[C::Scalar; W]>::random(&mut rng),
                )
            })
            .collect()
    }

    /// Shuffle the input ciphertexts and computes a corresponding proof.
    ///
    /// The input ciphertexts are re-encrypted with random (private) exponents, and permuted
    /// with a random (private) permutation. A corresponding proof of shuffle is computed.
    ///
    /// # Parameters
    ///
    /// - `ciphertexts`: The input ciphertexts to be shuffled, of width `W`
    /// - `context`: proof context label (ZKP CONTEXT)
    ///
    /// # Errors
    ///
    /// - `EmptyShuffle` if the input ciphertexts are zero length
    /// - `MismatchedShuffleLength` if there is a length mismatch between ciphertexts and generators
    ///
    /// # Panics
    ///
    /// This function will panic if the length of the generated permutation does
    /// not match the ciphertexts length, which should be impossible.
    ///
    /// Returns the shuffled ciphertexts of width `W` and the proof of shuffle.
    #[crate::warning(
        "The following function is not optimized. Parallelize with rayon. Error handling wrt generators length is suboptimal"
    )]
    pub fn shuffle(
        &self,
        ciphertexts: &[Ciphertext<C, W>],
        context: &[u8],
    ) -> Result<(Vec<Ciphertext<C, W>>, ShuffleProof<C, W>), Error> {
        self.shuffle_with(ciphertexts, context, &NativeChallenges)
    }

    /// As [`shuffle`](Self::shuffle), but deriving the two Fiat-Shamir
    /// challenges through `challenges` instead of braid's own convention.
    ///
    /// The proof algebra is identical; only the transcript differs. This is what
    /// lets braid emit a proof another implementation's verifier will accept
    /// (see [`ShuffleChallenges`]).
    ///
    /// # Errors
    ///
    /// - `EmptyShuffle` if the input ciphertexts are zero length
    /// - `MismatchedShuffleLength` if there is a length mismatch between ciphertexts and generators
    /// - Any error the [`ShuffleChallenges`] implementation returns while deriving
    ///   the challenges
    ///
    /// # Panics
    ///
    /// This function will panic if the length of the generated permutation does
    /// not match the ciphertexts length, which should be impossible.
    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::similar_names)]
    #[allow(clippy::too_many_lines)]
    pub fn shuffle_with<X: ShuffleChallenges<C, W>>(
        &self,
        ciphertexts: &[Ciphertext<C, W>],
        context: &[u8],
        challenges: &X,
    ) -> Result<(Vec<Ciphertext<C, W>>, ShuffleProof<C, W>), Error> {
        if ciphertexts.is_empty() {
            return Err(Error::EmptyShuffle);
        }
        if ciphertexts.len() != self.h_generators.len() {
            return Err(Error::MismatchedShuffleLength).with_context("Mismatched length between ciphertexts and h_generators when mixing");
        }

        let big_n = ciphertexts.len();
        let permutation = Permutation::generate::<C>(big_n);
        let permutation_data = self.apply_permutation(&permutation, ciphertexts);
        let permutation_data = permutation_data.expect("permutation.len() == ciphertexts.len()");
        let permuted_ciphertexts = permutation_data.permuted_ciphertexts;
        let commitment_exponents = permutation_data.commitment_exponents;
        let encryption_exponents = permutation_data.encryption_exponents;
        let pedersen_commitments = permutation_data.pedersen_commitments;

        let g = C::generator();

        ///////////////// Step 1 /////////////////

        // Challenge e
        let e_n = challenges.batching_challenges(
            &self.h_generators,
            &pedersen_commitments,
            &self.pk,
            ciphertexts,
            &permuted_ciphertexts,
            context,
        )?;
        // the calculation of A and F is moved to Step 5

        ///////////////// Step 2 /////////////////

        // a) Bridging commitments
        let e_prime_n = permutation
            .apply_inverse(&e_n)
            .expect("permutation.len() == e_n.len()");
        let b_n: Vec<C::Scalar> = (0..big_n).map(|_| C::random_scalar()).collect();
        // h_1 is at index 0
        let mut big_b_previous = &self.h_generators[0];
        let mut big_b_n = vec![];
        let g_b_n: Vec<C::Element> = b_n.clone().into_par_iter().map(|b| g.exp(&b)).collect();
        for (i, g_b) in g_b_n.iter().enumerate() {
            let big_b_factor = big_b_previous.exp(e_prime_n[i]);
            let big_b_i = g_b.mul(&big_b_factor);
            big_b_n.push(big_b_i);
            big_b_previous = &big_b_n[i];
        }

        // b) Proof commitments
        let alpha = C::random_scalar();
        let (beta_n, epsilon_n): (Vec<C::Scalar>, Vec<C::Scalar>) = (0..big_n)
            .into_par_iter()
            .map(|_| (C::random_scalar(), C::random_scalar()))
            .collect();
        let gamma = C::random_scalar();
        let delta = C::random_scalar();
        let mut rng = C::get_rng();
        let phi = <[C::Scalar; W]>::random(&mut rng);

        // A'
        let h_n_epsilon_n = self
            .h_generators
            .clone()
            .into_par_iter()
            .zip(epsilon_n.clone().into_par_iter());
        let h_n_epsilon_n = h_n_epsilon_n.map(|(h, e)| h.exp(&e));
        let h_n_epsilon_n_fold = h_n_epsilon_n
            .into_par_iter()
            .reduce(C::Element::one, |acc, next| acc.mul(&next));
        // let h_n_epsilon_n_fold: C::Element = h_n_epsilon_n_fold.collect();
        let big_a_prime = g.exp(&alpha);
        let big_a_prime = big_a_prime.mul(&h_n_epsilon_n_fold);

        // B'
        // We need to start this calculation at big_b_0, which is = h_1
        let h_1_iter = rayon::iter::once(&self.h_generators[0]);

        // the last value of big_b_0_n, B_N, is not used in this calculation, it is used later when computing big_d
        // cannot underflow, ciphertexts.len() > 0
        #[allow(clippy::arithmetic_side_effects)]
        let except_last = &big_b_n[0..big_b_n.len() - 1];
        let big_b_0_n_minus_1 = h_1_iter.chain(except_last.into_par_iter());

        let big_b_n_epsilon_n = big_b_0_n_minus_1
            .into_par_iter()
            .zip(epsilon_n.clone().into_par_iter());
        let big_b_n_epsilon_n_beta_n = big_b_n_epsilon_n.zip(beta_n.clone().into_par_iter());
        let big_b_prime_n: Vec<C::Element> = big_b_n_epsilon_n_beta_n
            .into_par_iter()
            .map(|((big_b, e), beta)| {
                let g_beta = g.exp(&beta);
                let big_b_epsilon = big_b.exp(&e);

                g_beta.mul(&big_b_epsilon)
            })
            .collect();

        // F'
        let w_prime_n_epsilon_n = permuted_ciphertexts
            .clone()
            .into_par_iter()
            .zip(epsilon_n.clone().into_par_iter());
        let w_prime_n_epsilon_n = w_prime_n_epsilon_n
            .into_par_iter()
            .map(|(w, e)| w.map_ref(|uv| uv.dist_exp(&e)));
        let w_prime_n_epsilon_n = fold_values(
            w_prime_n_epsilon_n,
            <[[C::Element; W]; 2]>::one,
            |acc, next| acc.mul(next),
        );
        let big_f_prime = Ciphertext::<C, W>(w_prime_n_epsilon_n);
        let big_f_prime: Ciphertext<C, W> = big_f_prime.re_encrypt(&phi.neg(), &self.pk.y);

        // C'
        let big_c_prime = g.exp(&gamma);

        // D'
        let big_d_prime = g.exp(&delta);

        let commitments = ShuffleCommitments::new(
            big_b_n,
            big_a_prime,
            big_b_prime_n,
            big_c_prime,
            big_d_prime,
            big_f_prime,
            pedersen_commitments,
        );

        ///////////////// Step 3 /////////////////

        // Challenge v
        let v = challenges.challenge(&self.pk, &commitments, context)?;

        ///////////////// Step 4 /////////////////

        // a
        let r_n_e_prime_n = commitment_exponents.par_iter().zip(e_prime_n.par_iter());
        let r_n_e_prime_n = r_n_e_prime_n.map(|(r, e)| r.mul(e));
        let a = r_n_e_prime_n.reduce(C::Scalar::zero, |acc, next| acc.add(&next));

        // c
        let c = commitment_exponents
            .iter()
            .fold(C::Scalar::zero(), |acc, next| acc.add(next));

        // f
        let s_n_e_n = encryption_exponents.par_iter().zip(e_n.par_iter());
        let s_n_e_n = s_n_e_n.map(|(s, e)| s.dist_mul(e));
        let f = fold_values(s_n_e_n, <[C::Scalar; W]>::zero, |acc, next| acc.add(next));

        // d_n
        // "sets d1 = b1 and computes di = bi + e′i*di−1 for i ∈ [N]"
        // This means we start the computation at i = 1 (which is i = 2 in EVS)
        // and our vector d_n has d_n[0] = b_n[0] (d1 = b1 in EVS)
        let mut d_n = vec![b_n[0].clone()];
        #[crate::warning("Figure out how this skip(1) behaves")]
        for (i, b) in b_n.iter().enumerate().skip(1) {
            // cannot underflow, skip(1) starts at 1
            #[allow(clippy::arithmetic_side_effects)]
            let e_prime_d = e_prime_n[i].mul(&d_n[i - 1]);
            let sum = b.add(&e_prime_d);
            d_n.push(sum);
        }
        // d
        // cannot underflow, d_n.len() > 0
        #[allow(clippy::arithmetic_side_effects)]
        let d = &d_n[d_n.len() - 1];

        // k_a
        let k_a = v.mul(&a).add(&alpha);

        // k_b
        let b_n_beta_n = b_n.par_iter().zip(beta_n.par_iter());
        let k_b_n: Vec<C::Scalar> = b_n_beta_n
            .map(|(b, beta)| {
                let vb = v.mul(b);
                vb.add(beta)
            })
            .collect();

        // k_e_n
        let e_prime_n_epsilon_n = e_prime_n.par_iter().zip(epsilon_n.par_iter());
        let k_e_n: Vec<C::Scalar> = e_prime_n_epsilon_n
            .map(|(e, epsilon)| {
                let ve = v.mul(e);
                ve.add(epsilon)
            })
            .collect();

        // k_c
        let k_c = v.mul(&c).add(&gamma);

        // k_d
        let k_d = v.mul(d).add(&delta);

        // k_f
        let k_f = v.repl_mul(&f).add(&phi);

        let responses = Responses::<C, W>::new(k_a, k_b_n, k_c, k_d, k_e_n, k_f);
        let proof = ShuffleProof::new(commitments, responses);

        Ok((permuted_ciphertexts, proof))
    }

    /// Verify the given proof of shuffle with respect to the original and shuffled ciphertexts.
    ///
    /// # Parameters
    ///
    /// - `ciphertexts`: The original ciphertexts, of width `W`
    /// - `permuted_ciphertexts`: The shuffled ciphertexts, of width `W`
    /// - `proof`: The proof of shuffle
    /// - `context`: proof context label (ZKP CONTEXT)
    ///
    /// # Errors
    ///
    /// - `EmptyShuffle` if the input ciphertexts are zero length
    /// - `MismatchedShuffleLength` if there is a length mismatch between ciphertexts
    /// - `MismatchedShuffleLength` if there is a length mismatch between ciphertexts and generators
    /// - `MismatchedShuffleLength` if there is a length mismatch between proof commitments and ciphertexts
    ///
    /// Returns `true` if the proof is valid, `false` otherwise.
    #[crate::warning(
        "The following function is not optimized. Parallelize with rayon. Error handling wrt generators length is suboptimal"
    )]
    pub fn verify(
        &self,
        ciphertexts: &[Ciphertext<C, W>],
        permuted_ciphertexts: &[Ciphertext<C, W>],
        proof: &ShuffleProof<C, W>,
        context: &[u8],
    ) -> Result<bool, Error> {
        self.verify_with(ciphertexts, permuted_ciphertexts, proof, context, &NativeChallenges)
    }

    /// As [`verify`](Self::verify), but deriving the challenges through
    /// `challenges`. Must be paired with the matching
    /// [`shuffle_with`](Self::shuffle_with) convention.
    ///
    /// # Errors
    ///
    /// - `EmptyShuffle` if the input ciphertexts are zero length
    /// - `MismatchedShuffleLength` if there is a length mismatch between the
    ///   ciphertext lists, the generators, or the proof commitments
    /// - Any error the [`ShuffleChallenges`] implementation returns while deriving
    ///   the challenges
    #[allow(clippy::similar_names)]
    #[allow(clippy::too_many_lines)]
    pub fn verify_with<X: ShuffleChallenges<C, W>>(
        &self,
        ciphertexts: &[Ciphertext<C, W>],
        permuted_ciphertexts: &[Ciphertext<C, W>],
        proof: &ShuffleProof<C, W>,
        context: &[u8],
        challenges: &X,
    ) -> Result<bool, Error> {
        if ciphertexts.is_empty() {
            return Err(Error::EmptyShuffle);
        }
        if ciphertexts.len() != permuted_ciphertexts.len() {
            return Err(Error::MismatchedShuffleLength).with_context("Mismatched length between ciphertexts and permuted ciphertexts");
        }
        if ciphertexts.len() != self.h_generators.len() {
            return Err(Error::MismatchedShuffleLength).with_context("Mismatched length between ciphertexts and h_generators");
        }
        if proof.commitments.big_b_n.len() != ciphertexts.len() {
            return Err(Error::MismatchedShuffleLength).with_context("Mismatched length between proof commitments big_b_n and ciphertexts");
        }
        if proof.commitments.big_b_prime_n.len() != ciphertexts.len() {
            return Err(Error::MismatchedShuffleLength).with_context("Mismatched length between proof commitments big_b_prime_n and ciphertexts");
        }
        if proof.commitments.u_n.len() != ciphertexts.len() {
            return Err(Error::MismatchedShuffleLength).with_context("Mismatched length between proof commitments u_n and ciphertexts");
        }

        let commitments = &proof.commitments;
        let responses = &proof.responses;
        let g = C::generator();

        let e_n = challenges.batching_challenges(
            &self.h_generators,
            &commitments.u_n,
            &self.pk,
            ciphertexts,
            permuted_ciphertexts,
            context,
        )?;
        let v = challenges.challenge(&self.pk, commitments, context)?;

        ///////////////// Step 5 /////////////////

        // A (comes from Step 1 in evs)
        let e_n_u_n = e_n.par_iter().zip(commitments.u_n.par_iter());
        let big_a_n = e_n_u_n.map(|(e, u)| u.exp(e));
        let big_a: C::Element = big_a_n.reduce(C::Element::one, |acc, next| acc.mul(&next));

        // F (comes from Step 1 in evs)
        let e_n_w_n = e_n.par_iter().zip(ciphertexts.par_iter());
        let big_f_n = e_n_w_n.map(|(e, w)| w.map_ref(|uv| uv.dist_exp(e)));
        // let big_f_n = e_n_w_n.map(|(e, w)| array::from_fn(|i| w.0[i].dist_exp(&e)));
        let big_f: [[C::Element; W]; 2] =
            fold_values(big_f_n, <[[C::Element; W]; 2]>::one, |acc, next| {
                acc.mul(next)
            });

        // C
        let u_n_fold = commitments
            .u_n
            .iter()
            .fold(C::Element::one(), |acc, next| acc.mul(next));
        let h_n_fold = self
            .h_generators
            .iter()
            .fold(C::Element::one(), |acc, next| acc.mul(next));
        let big_c = u_n_fold.mul(&h_n_fold.inv());

        // D
        let e_n_fold = e_n
            .into_par_iter()
            .reduce(C::Scalar::one, |acc, next| acc.mul(&next));
        let h1_e_n_fold = self.h_generators[0].exp(&e_n_fold);
        // this is B_N
        // cannot underflow, ciphertexts.len() > 0
        #[allow(clippy::arithmetic_side_effects)]
        let big_b_last = &commitments.big_b_n[commitments.big_b_n.len() - 1];
        let big_d = big_b_last.mul(&h1_e_n_fold.inv());

        // B_0
        let big_b_0 = &self.h_generators[0];

        ////// Verification 1 //////

        let h_n_k_e_n = self.h_generators.par_iter().zip(responses.k_e_n.par_iter());
        let h_n_k_e_n = h_n_k_e_n.map(|(h, k)| h.exp(k));
        let h_n_k_e_n_fold = h_n_k_e_n.reduce(C::Element::one, |acc, next| acc.mul(&next));
        let g_k_a = g.exp(&responses.k_a);
        let lhs_1 = big_a.exp(&v).mul(&commitments.big_a_prime);
        let rhs_1 = g_k_a.mul(&h_n_k_e_n_fold);

        ////// Verification 2 //////

        // We need to start this calculation at big_b_0, which is = h_1
        let h_1_iter = rayon::iter::once(big_b_0);
        // the last value of big_b_0_n, B_N, is not used in this calculation, it is used later when computing big_d
        let big_b_n = &commitments.big_b_n;
        // cannot underflow, ciphertexts.len() > 0
        #[allow(clippy::arithmetic_side_effects)]
        let except_last = &big_b_n[0..big_b_n.len() - 1];
        let big_b_0_n_minus_1 = h_1_iter.chain(except_last.into_par_iter());

        let big_b_0_n_minus_1_k_e_n = big_b_0_n_minus_1.zip(responses.k_e_n.par_iter());
        let big_b_0_n_minus_1_k_e_n_k_b_n = big_b_0_n_minus_1_k_e_n.zip(responses.k_b_n.par_iter());

        let rhs_2: Vec<C::Element> = big_b_0_n_minus_1_k_e_n_k_b_n
            .map(|((b, k_e), k_b)| {
                let b_k_e = b.exp(k_e);
                let g_k_b = g.exp(k_b);

                g_k_b.mul(&b_k_e)
            })
            .collect();

        let big_b_prime_n = &commitments.big_b_prime_n;
        let big_b_n_big_b_prime_n = big_b_n.par_iter().zip(big_b_prime_n.par_iter());
        let lhs_2: Vec<C::Element> = big_b_n_big_b_prime_n
            .map(|(big_b, big_b_prime)| {
                let big_b_v = big_b.exp(&v);
                big_b_v.mul(big_b_prime)
            })
            .collect();

        ////// Verification 3 //////

        let big_c_v = big_c.exp(&v);
        let lhs_3 = big_c_v.mul(&commitments.big_c_prime);
        let rhs_3 = g.exp(&responses.k_c);

        ////// Verification 4 //////

        let big_d_v = big_d.exp(&v);
        let lhs_4 = big_d_v.mul(&commitments.big_d_prime);
        let rhs_4 = g.exp(&responses.k_d);

        ////// Verification 5 //////

        let big_f_prime = &commitments.big_f_prime;
        let big_f_v = big_f.map(|uv| uv.dist_exp(&v));
        let lhs_5 = big_f_v.mul(&big_f_prime.0);

        let w_prime_n = permuted_ciphertexts;
        let w_prime_n_k_e_n = w_prime_n.par_iter().zip(responses.k_e_n.par_iter());
        let w_prime_n_k_e_n = w_prime_n_k_e_n.map(|(w, k)| w.map_ref(|uv| uv.dist_exp(k)));
        let w_prime_n_k_e_n_fold = fold_values(
            w_prime_n_k_e_n,
            <[[C::Element; W]; 2]>::one,
            |acc, next| acc.mul(next),
        );

        let one = [g, self.pk.y.clone()].map(|gy| gy.repl_exp(&responses.k_f.neg()));
        let rhs_5 = one.mul(&w_prime_n_k_e_n_fold);

        let ret =
            lhs_1 == rhs_1 && lhs_2 == rhs_2 && lhs_3 == rhs_3 && lhs_4 == rhs_4 && lhs_5 == rhs_5;

        Ok(ret)
    }

    /// Re-encrypt and permute the input ciphertexts with the given permutation data.
    ///
    /// See `EVS`: Protocol 12.3, Satisfying clause
    ///
    /// # Params
    ///
    /// - `permutation`: The permutation to apply, obtained through [`Permutation::generate`]
    /// - `ciphertexts`: The input ciphertexts to shuffle
    ///
    /// Returns the [`PermutationData`] applied to the input ciphertexts
    pub(crate) fn apply_permutation(
        &self,
        permutation: &Permutation,
        ciphertexts: &[Ciphertext<C, W>],
    ) -> Result<PermutationData<C, W>, Error> {
        let (r_n, s_n) = Self::gen_private_exponents(ciphertexts.len());

        let r_permuted = permutation.apply(&r_n)?;
        let h_permuted = permutation.apply(&self.h_generators)?;
        let w_permuted = permutation.apply_inverse(ciphertexts)?;
        let s_permuted = permutation.apply_inverse(&s_n)?;

        let r_h_permuted = r_permuted.into_par_iter().zip(h_permuted.into_par_iter());
        #[crate::warning("The following code is not optimized. Parallelize with rayon")]
        let u_n: Vec<C::Element> = r_h_permuted
            .into_par_iter()
            .map(|(r, h)| {
                let g = C::generator();
                let g_r = g.exp(r);
                g_r.mul(h)
            })
            .collect();

        let s_w_permuted = w_permuted.into_par_iter().zip(s_permuted.into_par_iter());

        #[crate::warning("The following code is not optimized. Parallelize with rayon")]
        let w_prime_n: Vec<Ciphertext<C, W>> = s_w_permuted
            .into_par_iter()
            .map(|(c, s)| c.re_encrypt(s, &self.pk.y))
            .collect();

        let ret = PermutationData::new(r_n, s_n, u_n, w_prime_n);
        Ok(ret)
    }

    /// Domain separation tags for the e-challenge input
    #[crate::warning(
        "Challenge inputs are incomplete. Also add generators and pedersen commitments"
    )]
    const DS_TAGS_CHALLENGE_E: [&[u8]; 4] = [
        b"pk",
        b"w_n",
        b"w_prime_n",
        b"shuffle_proof_challenge_e_context",
    ];

    /// Domain separation tags for the v-challenge input
    #[crate::warning(
        "Challenge inputs are incomplete. Also add generators, pedersen commitments, pk, and ciphertexts"
    )]
    const DS_TAGS_CHALLENGE_V: [&[u8]; 8] = [
        b"pk",
        b"big_b_n",
        b"big_a_prime",
        b"big_b_prime_n",
        b"big_c_prime",
        b"big_d_prime",
        b"big_f_prime_n",
        b"shuffle_challenge_input_v_context",
    ];

}

/// Fold the values of a parallel iterator into one, combining with `combine`
/// from `identity()`.
///
/// This is the seam between two fold strategies, selected at compile time by
/// the `bounded-combine` feature so they can be benchmarked against each
/// other. This definition is the default: rayon's recursive `reduce`, fused
/// with the upstream `map` (nothing is materialized). Its stack use grows
/// with input length, `W` and run-time work stealing, because each split
/// dispatches a stack frame carrying `T`-sized accumulators -- measured on
/// Windows x64 at `W = 100`, pool threads need 4 MiB at `N = 100`, 8 MiB at
/// `N = 1,000` and 16 MiB at `N = 10,000`.
///
/// Both strategies combine the operands in their original order, so for the
/// associative operations used here the result -- and therefore any proof
/// derived from it -- is identical across the two.
#[cfg(not(feature = "bounded-combine"))]
fn fold_values<T, I, Ident, Combine>(items: I, identity: Ident, combine: Combine) -> T
where
    I: IntoParallelIterator<Item = T>,
    T: Send + Sync,
    Ident: Fn() -> T + Send + Sync,
    Combine: Fn(T, &T) -> T + Send + Sync,
{
    items
        .into_par_iter()
        .reduce(identity, |acc, next| combine(acc, &next))
}

/// Fold the values of a parallel iterator into one, combining with `combine`
/// from `identity()`.
///
/// This is the seam between two fold strategies, selected at compile time by
/// the `bounded-combine` feature so they can be benchmarked against each
/// other. This definition is the `bounded-combine` strategy: the values are
/// materialized and handed to [`bounded_combine`], whose stack use is bounded
/// by a small constant independent of input length and scheduling. The cost
/// is the materialization itself, `len * size_of::<T>()` bytes of heap held
/// for the duration of the fold.
///
/// Both strategies combine the operands in their original order, so for the
/// associative operations used here the result -- and therefore any proof
/// derived from it -- is identical across the two.
#[cfg(feature = "bounded-combine")]
fn fold_values<T, I, Ident, Combine>(items: I, identity: Ident, combine: Combine) -> T
where
    I: IntoParallelIterator<Item = T>,
    T: Send + Sync,
    Ident: Fn() -> T + Send + Sync,
    Combine: Fn(T, &T) -> T + Send + Sync,
{
    let values: Vec<T> = items.into_par_iter().collect();
    bounded_combine(&values, identity, combine)
}

/// Chunk-count multiplier for [`bounded_combine`]. This is used to
/// create a number of chunks proportional to the available thread count.
#[cfg(feature = "bounded-combine")]
const BOUNDED_COMBINE_CHUNKS_PER_THREAD: usize = 4;

/// Combine `items` via `combine`, starting from `identity()`.
///
/// Splits `items` into a number of chunks proportional to the available
/// thread count (not to `items.len()`), folds each chunk with a plain
/// sequential loop, and combines the resulting handful of partial values
/// with another plain sequential loop. Rayon's own recursive `reduce`
/// dispatches roughly one stack frame per split, and how many splits stay
/// unstolen (and so execute nested, rather than unwinding first) depends on
/// thread contention at run time, not just on `items.len()`; when combined
/// values are large, that recursion's stack cost is not bounded independent
/// of scheduling. Pre-chunking here bounds the recursive dispatch depth to
/// a small constant, and each chunk's own fold uses a loop, not recursion,
/// so no combined value is ever carried through a deep call stack.
#[cfg(feature = "bounded-combine")]
fn bounded_combine<T, Ident, Combine>(items: &[T], identity: Ident, combine: Combine) -> T
where
    T: Sync + Send,
    Ident: Fn() -> T + Sync,
    Combine: Fn(T, &T) -> T + Sync,
{
    if items.is_empty() {
        return identity();
    }
    let num_chunks = rayon::current_num_threads()
        .max(1)
        .saturating_mul(BOUNDED_COMBINE_CHUNKS_PER_THREAD)
        .min(items.len());
    let chunk_size = items.len().div_ceil(num_chunks);
    let partials: Vec<T> = items
        .par_chunks(chunk_size)
        .map(|chunk| {
            let mut acc = identity();
            for next in chunk {
                acc = combine(acc, next);
            }
            acc
        })
        .collect();
    let mut result = identity();
    for partial in &partials {
        result = combine(result, partial);
    }
    result
}

/// Convenience structure to hold re-encryption and permutation data
pub(crate) struct PermutationData<C: Context, const W: usize> {
    /// Commitment exponents, private
    commitment_exponents: Vec<C::Scalar>,
    /// Re-encryption exponents, private
    encryption_exponents: Vec<[C::Scalar; W]>,
    /// Pedersen commitments, public
    pedersen_commitments: Vec<C::Element>,
    /// Permuted ciphertexts, public
    permuted_ciphertexts: Vec<Ciphertext<C, W>>,
}

impl<C: Context, const W: usize> PermutationData<C, W> {
    /// Construct a new `PermutationData` instance with given values.
    ///
    /// This structure is returned by the [`Shuffler::apply_permutation`] function.
    pub fn new(
        commitment_exponents: Vec<C::Scalar>,
        encryption_exponents: Vec<[C::Scalar; W]>,
        pedersen_commitments: Vec<C::Element>,
        permuted_ciphertexts: Vec<Ciphertext<C, W>>,
    ) -> Self {
        Self {
            commitment_exponents,
            encryption_exponents,
            pedersen_commitments,
            permuted_ciphertexts,
        }
    }
}

/**
 * Terelius-Wikstrom proof of shuffle.
 *
 * Given lists of ciphertexts `w = w_1, w_2 .. w_n` and `w' = w'_1, w'_2 .. w'_n`
 * encrypted under public key `pk` proves that `w'` is a permutation of re-encryptions
 * of `w`. Equivalently, the list of plaintexts corresponding to `w'` is a permutation of
 * the plaintexts corresponding to `w`.
 *
 * See `EVS`: Protocol 12.3
 */
#[crate::warning("Remove clone, only requried for sandbox/sr")]
#[derive(Debug, VSer, PartialEq, Clone)]
pub struct ShuffleProof<C: Context, const W: usize> {
    /// Proof shuffle commitments
    pub commitments: ShuffleCommitments<C, W>,
    /// Challenge responses
    pub responses: Responses<C, W>,
}

impl<C: Context, const W: usize> ShuffleProof<C, W> {
    /// Construct a `ShuffleProof` with the given values.
    ///
    /// The standard way to obtain a `ShuffleProof` is through the [`Shuffler::shuffle`] function.
    pub fn new(commitments: ShuffleCommitments<C, W>, responses: Responses<C, W>) -> Self {
        Self {
            commitments,
            responses,
        }
    }
}

/// Commitments for the shuffle proof
///
/// Includes bridging commitments, proof commitments and
/// pedersen commitments.
#[derive(Debug, VSer, PartialEq, Clone)]
pub struct ShuffleCommitments<C: Context, const W: usize> {
    /// Bridging commitments
    big_b_n: Vec<C::Element>,

    /// Proof commitment `big_a_prime`
    big_a_prime: C::Element,

    /// Proof commitment `big_b_prime_n`
    big_b_prime_n: Vec<C::Element>,

    /// Proof commitment `big_c_prime`
    big_c_prime: C::Element,

    /// Proof commitment `big_d_prime`
    big_d_prime: C::Element,

    /// Proof commitments `big_f_prime`
    big_f_prime: Ciphertext<C, W>,

    /// Pedersen commitments
    u_n: Vec<C::Element>,
}

impl<C: Context, const W: usize> ShuffleCommitments<C, W> {
    /// Construct a new `ShuffleCommitments` instance with given values.
    #[allow(clippy::similar_names)]
    pub fn new(
        big_b_n: Vec<C::Element>,
        big_a_prime: C::Element,
        big_b_prime_n: Vec<C::Element>,
        big_c_prime: C::Element,
        big_d_prime: C::Element,
        big_f_prime: Ciphertext<C, W>,
        u_n: Vec<C::Element>,
    ) -> Self {
        Self {
            big_b_n,
            big_a_prime,
            big_b_prime_n,
            big_c_prime,
            big_d_prime,
            big_f_prime,
            u_n,
        }
    }

    /// Bridging commitments `B`.
    #[must_use]
    pub fn big_b_n(&self) -> &[C::Element] {
        &self.big_b_n
    }

    /// Proof commitment `A'`.
    #[must_use]
    pub fn big_a_prime(&self) -> &C::Element {
        &self.big_a_prime
    }

    /// Proof commitments `B'`.
    #[must_use]
    pub fn big_b_prime_n(&self) -> &[C::Element] {
        &self.big_b_prime_n
    }

    /// Proof commitment `C'`.
    #[must_use]
    pub fn big_c_prime(&self) -> &C::Element {
        &self.big_c_prime
    }

    /// Proof commitment `D'`.
    #[must_use]
    pub fn big_d_prime(&self) -> &C::Element {
        &self.big_d_prime
    }

    /// Proof commitment `F'`.
    #[must_use]
    pub fn big_f_prime(&self) -> &Ciphertext<C, W> {
        &self.big_f_prime
    }

    /// The Pedersen commitments to the permutation, `u`.
    ///
    /// Carried inside this struct, but note other implementations may store it
    /// as a separate artifact alongside the proof commitments.
    #[must_use]
    pub fn u_n(&self) -> &[C::Element] {
        &self.u_n
    }
}

/**
 * Responses to the challenge in the shuffle proof
 */
#[derive(Debug, VSer, PartialEq, Clone)]
pub struct Responses<C: Context, const W: usize> {
    /// Response `k_a`
    pub k_a: C::Scalar,

    /// Responses `k_b_n`
    pub k_b_n: Vec<C::Scalar>,

    /// Response `k_c`
    pub k_c: C::Scalar,

    /// Response `k_d`
    pub k_d: C::Scalar,

    /// Responses `k_e_n`
    pub k_e_n: Vec<C::Scalar>,

    /// Responses `k_f`
    pub k_f: [C::Scalar; W],
}

impl<C: Context, const W: usize> Responses<C, W> {
    /// Construct a new `Responses` instance with given values.
    #[allow(clippy::similar_names)]
    pub fn new(
        k_a: C::Scalar,
        k_b_n: Vec<C::Scalar>,
        k_c: C::Scalar,
        k_d: C::Scalar,
        k_e_n: Vec<C::Scalar>,
        k_f: [C::Scalar; W],
    ) -> Self {
        Self {
            k_a,
            k_b_n,
            k_c,
            k_d,
            k_e_n,
            k_f,
        }
    }
}

/**
 * A permutation and its inverse in vector form
 *
 * The vector values corresponds to values in [one-line
 * notation](https://en.wikipedia.org/wiki/Permutation#One-line_notation).
 *
 * # Examples
 * ```
 * use cryptography::context::Context;
 * use cryptography::context::RistrettoCtx;
 * use cryptography::zkp::shuffle::Permutation;
 *
 * let data = vec!['A', 'B', 'C', 'D', 'E'];
 * let perm_data = Permutation::generate::<RistrettoCtx>(data.len());
 *
 * let permuted_refs = perm_data.apply(&data).unwrap();
 * let permuted_refs: Vec<char> = permuted_refs.into_iter().copied().collect();
 *
 * let inversed_refs = perm_data.apply_inverse(&permuted_refs).unwrap();
 * let inversed_refs: Vec<char> = inversed_refs.into_iter().copied().collect();
 *
 * assert_eq!(data, inversed_refs);
 *
 * ```
 */
pub struct Permutation {
    /// The permutation vector.
    pub permutation: Vec<usize>,

    /// The inverse permutation vector.
    pub inverse: Vec<usize>,
}

impl Permutation {
    /// Generate a random permutation and its inverse.
    ///
    /// Returns a new `Permutation` instance containing the generated permutation and its inverse.
    #[must_use]
    pub fn generate<C: Context>(size: usize) -> Self {
        let mut rng = C::get_rng();

        let mut permutation: Vec<usize> = (0..size).collect();
        Self::shuffle::<C>(&mut permutation, &mut rng);

        let mut inverse = vec![0usize; size];

        for (i, v) in permutation.iter().enumerate() {
            inverse[*v] = i;
        }

        Self {
            permutation,
            inverse,
        }
    }

    /// Shuffle the given integers in place using the Fisher-Yates algorithm.
    fn shuffle<C: Context>(data: &mut [usize], rng: &mut C::Rng) {
        for i in (1..data.len()).rev() {
            let j = rng.random_range(0..=i);
            data.swap(i, j);
        }
    }

    /// Shuffle the given integers in place, using `SliceRandom` from the rand crate.
    /// 
    /// This function uses the [`SliceRandom`](https://rust-random.github.io/rand/rand/seq/trait.SliceRandom.html#tymethod.shuffle) trait generate the permutation, according
    /// to which
    ///
    /// "The resulting permutation is picked uniformly from the set of all possible
    /// permutations."
    fn shuffle_slice_random<C: Context>(data: &mut [usize], rng: &mut C::Rng) {
        use rand::seq::SliceRandom;
        data.shuffle(rng);
    }

    /// The length of the permutation and inverse permutation
    #[must_use]
    pub fn len(&self) -> usize {
        // does not matter which field we choose, they are of equal size
        self.permutation.len()
    }

    /// Check if this is the empty permutation.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.permutation.is_empty()
    }

    /// Apply the permutation to the given slice.
    ///
    /// # Errors
    ///
    /// - `MismatchedPermutationLength` if the target slice length does not match the permutation length
    ///
    /// Returns a new vector with the permuted elements.
    pub fn apply<'a, T>(&self, target: &'a [T]) -> Result<Vec<&'a T>, Error> {
        let size = self.permutation.len();

        if target.len() != size {
            return Err(Error::MismatchedPermutationLength);
        }

        let mut permuted = vec![];
        permuted.resize_with(size, || {
            // Safe due to the above check ensuring target is not empty if size > 0
            &target[0]
        });

        // The element at original index `i` (target[i]) moves to the position `self.permutation[i]`.
        for (i, v_ref) in target.iter().enumerate() {
            permuted[self.permutation[i]] = v_ref;
        }

        Ok(permuted)
    }

    /// Apply the inverse permutation to the given slice.
    ///
    /// # Errors
    ///
    /// - `MismatchedPermutationLength` if the target slice length does not match the permutation length
    ///
    /// Returns a new vector with the permuted elements.
    pub fn apply_inverse<'a, T>(&self, target: &'a [T]) -> Result<Vec<&'a T>, Error> {
        let size = self.inverse.len();

        if target.len() != size {
            return Err(Error::MismatchedPermutationLength);
        }

        let mut permuted = vec![];
        permuted.resize_with(size, || {
            // Safe due to the above check ensuring target is not empty if size > 0
            &target[0]
        });

        // The element at original index `i` (target[i]) moves to the position `self.inverse[i]`.
        for (i, v_ref) in target.iter().enumerate() {
            permuted[self.inverse[i]] = v_ref;
        }

        Ok(permuted)
    }
}

#[cfg(test)]
mod tests {
    use std::array;

    use crate::context::Context;
    use crate::context::P256Ctx as PCtx;
    use crate::context::RistrettoCtx as RCtx;
    use crate::cryptosystem::elgamal::Ciphertext;
    use crate::cryptosystem::elgamal::KeyPair;
    use crate::traits::groups::CryptographicGroup;
    use crate::utils::serialization::{VDeserializable, VSerializable};
    use crate::zkp::shuffle::Permutation;
    use crate::zkp::shuffle::ShuffleProof;
    use crate::zkp::shuffle::Shuffler;

    /// [`bounded_combine`](super::bounded_combine) must agree with a plain
    /// sequential fold -- including on an empty input (identity) and on inputs
    /// shorter than the chunk count.
    #[cfg(feature = "bounded-combine")]
    #[test]
    fn test_bounded_combine_matches_sequential() {
        use crate::traits::groups::GroupScalar;
        type Scalar = <RCtx as Context>::Scalar;

        for len in [0usize, 1, 3, 100, 1000] {
            let values: Vec<Scalar> = (0..len)
                .map(|i| {
                    let i: u32 = i.try_into().expect("len < u32::MAX");
                    Scalar::from(i)
                })
                .collect();

            let expected = values
                .iter()
                .fold(Scalar::zero(), |acc, next| acc.add(next));
            let actual =
                super::bounded_combine(&values, Scalar::zero, |acc, next| acc.add(next));

            assert_eq!(expected, actual, "mismatch at len {len}");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_ristretto() {
        test_shuffle::<RCtx, 2>();
        test_shuffle::<RCtx, 3>();
        test_shuffle::<RCtx, 4>();
        test_shuffle::<RCtx, 5>();
        test_shuffle::<RCtx, 5>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_invalid_ristretto() {
        test_shuffle_invalid::<RCtx, 2>();
        test_shuffle_invalid::<RCtx, 3>();
        test_shuffle_invalid::<RCtx, 4>();
        test_shuffle_invalid::<RCtx, 5>();
        test_shuffle_invalid::<RCtx, 5>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_p256() {
        test_shuffle::<PCtx, 2>();
        test_shuffle::<PCtx, 3>();
        test_shuffle::<PCtx, 4>();
        test_shuffle::<PCtx, 5>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_invalid_p256() {
        test_shuffle_invalid::<PCtx, 2>();
        test_shuffle_invalid::<PCtx, 3>();
        test_shuffle_invalid::<PCtx, 4>();
        test_shuffle_invalid::<PCtx, 5>();
        test_shuffle_invalid::<PCtx, 5>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_label_ristretto() {
        test_shuffle_label::<RCtx>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_label_p256() {
        test_shuffle_label::<PCtx>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_serialization_ristretto() {
        test_shuffle_serialization::<RCtx>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_serialization_p256() {
        test_shuffle_serialization::<PCtx>();
    }

    fn test_shuffle<C: Context, const W: usize>() {
        let count = 10;
        let keypair: KeyPair<C> = KeyPair::generate();

        let messages: Vec<[C::Element; W]> = (0..count)
            .map(|_| array::from_fn(|_| C::random_element()))
            .collect();

        let ciphertexts: Vec<Ciphertext<C, W>> =
            messages.iter().map(|m| keypair.encrypt(m)).collect();

        let generators = C::G::ind_generators(count, &vec![]).unwrap();
        let shuffler = Shuffler::<C, W>::new(generators, keypair.pkey);

        let (pciphertexts, proof) = shuffler.shuffle(&ciphertexts, &vec![]).unwrap();
        let ok = shuffler.verify(&ciphertexts, &pciphertexts, &proof, &vec![]);

        assert!(ok.unwrap());
    }

    fn test_shuffle_invalid<C: Context, const W: usize>() {
        let count = 10;
        let keypair: KeyPair<C> = KeyPair::generate();

        let messages: Vec<[C::Element; W]> = (0..count)
            .map(|_| array::from_fn(|_| C::random_element()))
            .collect();

        let ciphertexts: Vec<Ciphertext<C, W>> =
            messages.iter().map(|m| keypair.encrypt(m)).collect();

        let generators = C::G::ind_generators(count, &vec![]).unwrap();
        let shuffler = Shuffler::<C, W>::new(generators, keypair.pkey.clone());

        let (pciphertexts, proof) = shuffler.shuffle(&ciphertexts, &vec![]).unwrap();
        let ok = shuffler.verify(&ciphertexts, &pciphertexts, &proof, &vec![]);
        assert!(ok.unwrap());

        let messages: Vec<[C::Element; W]> = (0..count)
            .map(|_| array::from_fn(|_| C::random_element()))
            .collect();
        let ciphertexts: Vec<Ciphertext<C, W>> =
            messages.iter().map(|m| keypair.encrypt(m)).collect();
        let not_ok = shuffler.verify(&ciphertexts, &pciphertexts, &proof, &vec![]);

        assert!(!not_ok.unwrap());

        let messages: Vec<[C::Element; W]> = (0..count)
            .map(|_| array::from_fn(|_| C::random_element()))
            .collect();
        let ciphertexts: Vec<Ciphertext<C, W>> =
            messages.iter().map(|m| keypair.encrypt(m)).collect();
        let not_ok = shuffler.verify(&ciphertexts[1..].to_vec(), &pciphertexts, &proof, &vec![]);

        assert!(not_ok.is_err());
    }

    fn test_shuffle_label<C: Context>() {
        const W: usize = 3;
        let count = 10;
        let keypair: KeyPair<C> = KeyPair::generate();

        let messages: Vec<[C::Element; W]> = (0..count)
            .map(|_| array::from_fn(|_| C::random_element()))
            .collect();

        let ciphertexts: Vec<Ciphertext<C, W>> =
            messages.iter().map(|m| keypair.encrypt(m)).collect();

        let generators = C::G::ind_generators(count, &vec![]).unwrap();
        let shuffler = Shuffler::<C, W>::new(generators, keypair.pkey);

        let (pciphertexts, proof) = shuffler.shuffle(&ciphertexts, &vec![1u8]).unwrap();
        let ok = shuffler.verify(&ciphertexts, &pciphertexts, &proof, &vec![2u8]);

        assert!(!ok.unwrap());
    }

    fn test_shuffle_serialization<C: Context>() {
        const W: usize = 3;
        let count = 10;
        let keypair: KeyPair<C> = KeyPair::generate();

        let messages: Vec<[C::Element; W]> = (0..count)
            .map(|_| array::from_fn(|_| C::random_element()))
            .collect();

        let ciphertexts: Vec<Ciphertext<C, W>> =
            messages.iter().map(|m| keypair.encrypt(m)).collect();

        let generators = C::G::ind_generators(count, &vec![]).unwrap();
        let shuffler = Shuffler::<C, W>::new(generators, keypair.pkey);

        let (pciphertexts, proof) = shuffler.shuffle(&ciphertexts, &vec![1u8]).unwrap();
        let s_proof = proof.ser();
        let s_pciphertexts = pciphertexts.ser();
        let s_ciphertexts = ciphertexts.ser();

        let proof = ShuffleProof::<C, W>::deser(&s_proof).unwrap();
        let ciphertexts = Vec::<Ciphertext<C, W>>::deser(&s_ciphertexts).unwrap();
        let pciphertexts = Vec::<Ciphertext<C, W>>::deser(&s_pciphertexts).unwrap();

        let ok = shuffler.verify(&ciphertexts, &pciphertexts, &proof, &vec![2u8]);

        assert!(!ok.unwrap());
    }

    #[test]
    fn test_permutation_ristretto() {
        test_permutation_generation_and_inverse::<RCtx>();
        test_empty_permutation::<RCtx>();
        test_mismatched_length::<RCtx>();
    }

    #[test]
    fn test_permutation_p256() {
        test_permutation_generation_and_inverse::<PCtx>();
        test_empty_permutation::<PCtx>();
        test_mismatched_length::<PCtx>();
    }

    fn test_permutation_generation_and_inverse<C: Context>() {
        let size = 10;
        let perm = Permutation::generate::<C>(size);

        // Test that all numbers from 0 to size-1 are present exactly once in permutation
        let mut p_sorted = perm.permutation.clone();
        p_sorted.sort_unstable();
        let expected_p_sorted: Vec<usize> = (0..size).collect();
        assert_eq!(
            p_sorted, expected_p_sorted,
            "Permutation values are not unique or complete."
        );

        // Test that all numbers from 0 to size-1 are present exactly once in inverse
        let mut inv_sorted = perm.inverse.clone();
        inv_sorted.sort_unstable();
        let expected_inv_sorted: Vec<usize> = (0..size).collect();
        assert_eq!(
            inv_sorted, expected_inv_sorted,
            "Inverse permutation values are not unique or complete."
        );

        // Verify inverse property: perm[inverse[i]] == i
        for i in 0..size {
            assert_eq!(
                perm.permutation[perm.inverse[i]], i,
                "Inverse property failed at index {}",
                i
            );
        }

        // Verify inverse property: inverse[perm[i]] == i
        for i in 0..size {
            assert_eq!(
                perm.inverse[perm.permutation[i]], i,
                "Inverse property failed at index {}",
                i
            );
        }
    }

    fn test_empty_permutation<C: Context>() {
        let perm = Permutation::generate::<C>(0);
        assert_eq!(perm.len(), 0);
        assert!(perm.is_empty());
        assert!(perm.permutation.is_empty());
        assert!(perm.inverse.is_empty());

        let empty_vec: Vec<i32> = vec![];
        let applied_empty = perm.apply(&empty_vec).unwrap();
        assert!(applied_empty.is_empty());
        let applied_inverse_empty = perm.apply_inverse(&empty_vec).unwrap();
        assert!(applied_inverse_empty.is_empty());
    }

    fn test_mismatched_length<C: Context>() {
        let perm = Permutation::generate::<C>(5);
        let small_data = vec![1, 2, 3];
        let err = perm.apply(&small_data);
        assert!(err.is_err());
        let err = perm.apply_inverse(&small_data);
        assert!(err.is_err());
    }
}
