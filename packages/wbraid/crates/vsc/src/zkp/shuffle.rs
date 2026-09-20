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
use crate::utils::serialization::Serializable;

use canonical_derive::Canonical;
use rand::RngExt;
use sha3::Digest;

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
 * [`NativeChallenges`] is the default, used by [`Shuffler::shuffle`] and
 * [`Shuffler::verify`].
 *
 * Implementors must derive both challenges deterministically from public data
 * only, and must follow strong Fiat-Shamir: the batching seed commits to the
 * full statement (generators, Pedersen permutation commitments, public key,
 * both ciphertext lists, context), and the final challenge chains that seed,
 * so it transitively binds the statement and the batching vector.
 */
pub trait ShuffleChallenges<C: Context, const W: usize> {
    /// The batching seed and vector `e = (e_1, ..., e_N)`.
    ///
    /// The seed is the digest committing to the full statement, from which the
    /// batching vector is expanded; the caller passes it on to
    /// [`challenge`](Self::challenge).
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
    ) -> Result<(Vec<u8>, Vec<C::Scalar>), Error>;

    /// The single challenge `v`, derived from the batching seed and the proof
    /// commitments.
    ///
    /// `seed` is the value [`batching_challenges`](Self::batching_challenges)
    /// returned for this statement. Binding it binds — transitively — the
    /// statement and the batching vector `e`, so `v` cannot be fixed before
    /// either.
    ///
    /// # Errors
    ///
    /// Implementation-defined: whatever the convention's derivation can fail
    /// with — typically hashing to a scalar failing in the backend.
    fn challenge(
        &self,
        seed: &[u8],
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
        generators: &[C::Element],
        pedersen_commitments: &[C::Element],
        pk: &elgamal::PublicKey<C>,
        ciphertexts: &[Ciphertext<C, W>],
        permuted_ciphertexts: &[Ciphertext<C, W>],
        context: &[u8],
    ) -> Result<(Vec<u8>, Vec<C::Scalar>), Error> {
        let a = [
            C::generator().ser(),
            generators.to_vec().ser(),
            pedersen_commitments.to_vec().ser(),
            pk.ser(),
            ciphertexts.to_vec().ser(),
            permuted_ciphertexts.to_vec().ser(),
            context.to_vec(),
        ];
        let input: Vec<&[u8]> = a.iter().map(Vec::as_slice).collect();

        let mut hasher = C::get_hasher();
        hash::update_hasher(&mut hasher, &input, &Shuffler::<C, W>::DS_TAGS_CHALLENGE_E);
        let bytes = hasher.finalize();

        // Independent per-index derivations; parallelism cannot change the
        // per-index transcript, so the output matches the sequential loop.
        let ret = (0..ciphertexts.len())
            .into_par_iter()
            .map(|i| {
                // Cannot use platform dependent type in random oracle
                let i_u64 = i as u64;
                let inputs: &[&[u8]] = &[bytes.as_slice(), &i_u64.to_be_bytes()];
                let ds_tags: &[&[u8]; 2] = &[b"prefix", b"shuffle_proof_challenge_e_counter"];
                C::G::hash_to_scalar(inputs, ds_tags)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((bytes.to_vec(), ret))
    }

    fn challenge(
        &self,
        seed: &[u8],
        commitments: &ShuffleCommitments<C, W>,
        context: &[u8],
    ) -> Result<C::Scalar, Error> {
        let a = [
            seed.to_vec(),
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
        #[cfg_attr(
            feature = "custom-warnings",
            crate::warning("The following code is not optimized. Parallelize with rayon")
        )]
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
            return Err(Error::MismatchedShuffleLength).with_context(
                "Mismatched length between ciphertexts and h_generators when mixing",
            );
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
        let (seed, e_n) = challenges.batching_challenges(
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
        // Serial: sampling N scalars is too cheap for rayon to pay for — ~8 ms
        // saved at N = 1e5, under 0.1% of proving (benches/parallel_tradeoff.rs).
        let b_n: Vec<C::Scalar> = (0..big_n).map(|_| C::random_scalar()).collect();
        // Bridging commitments via the closed form (§5.3), replacing the serial
        // recurrence. d_n (the discrete-log recurrence, reused as the Step-4
        // response `d`) and p_n (prefix products, reused for B') come back with
        // it. h_1 is at generator index 0.
        let e_prime_scalars: Vec<C::Scalar> = e_prime_n.iter().map(|&e| e.clone()).collect();
        let BridgingChain { big_b_n, d_n, p_n } =
            bridging_commitments::<C>(&g, &self.h_generators[0], &b_n, &e_prime_scalars);

        // b) Proof commitments
        let alpha = C::random_scalar();
        // Serial: scalar sampling, see `b_n` above (benches/parallel_tradeoff.rs).
        let (beta_n, epsilon_n): (Vec<C::Scalar>, Vec<C::Scalar>) = (0..big_n)
            .map(|_| (C::random_scalar(), C::random_scalar()))
            .collect();
        let gamma = C::random_scalar();
        let delta = C::random_scalar();
        let mut rng = C::get_rng();
        let phi = <[C::Scalar; W]>::random(&mut rng);

        // A' = g^alpha · ∏ h_i^{epsilon_i}. epsilon is secret, so the multi-exp
        // is the constant-time path; g^alpha uses the fixed-base table.
        let h_refs: Vec<&C::Element> = self.h_generators.iter().collect();
        let h_n_epsilon_n_fold = C::Element::multi_exp(&h_refs, &epsilon_n)?;
        let big_a_prime = C::G::g_exp(&alpha).mul(&h_n_epsilon_n_fold);

        // B' = g^{beta_i + d_{i-1}·epsilon_i} · h_1^{p_{i-1}·epsilon_i}, the
        // closed-form follow-on of the bridging chain (§5.4), with d_0 = 0 and
        // p_0 = 1. Two fixed-base batches, replacing the per-element
        // variable-base B_{i-1}^{epsilon_i}. All exponents are secret, and
        // `exp_many` is constant-time.
        let g_exps: Vec<C::Scalar> = (0..big_n)
            .map(|i| {
                let d_prev = if i == 0 {
                    C::Scalar::zero()
                } else {
                    // cannot underflow, i > 0
                    #[allow(clippy::arithmetic_side_effects)]
                    d_n[i - 1].clone()
                };
                beta_n[i].add(&d_prev.mul(&epsilon_n[i]))
            })
            .collect();
        let h1_exps: Vec<C::Scalar> = (0..big_n)
            .map(|i| {
                let p_prev = if i == 0 {
                    C::Scalar::one()
                } else {
                    // cannot underflow, i > 0
                    #[allow(clippy::arithmetic_side_effects)]
                    p_n[i - 1].clone()
                };
                p_prev.mul(&epsilon_n[i])
            })
            .collect();
        let g_part = C::generator().exp_many(&g_exps);
        let h1_part = self.h_generators[0].exp_many(&h1_exps);
        let big_b_prime_n: Vec<C::Element> = g_part
            .iter()
            .zip(h1_part.iter())
            .map(|(a, b)| a.mul(b))
            .collect();

        // F' = Enc(1; -phi) · ∏ w'_i^{epsilon_i}. epsilon is secret, so the
        // componentwise product is the constant-time dist multi-exp.
        let fp_u_bases: Vec<[C::Element; W]> =
            permuted_ciphertexts.iter().map(|w| w.u().clone()).collect();
        let fp_v_bases: Vec<[C::Element; W]> =
            permuted_ciphertexts.iter().map(|w| w.v().clone()).collect();
        let big_f_prime_prod: [[C::Element; W]; 2] = [
            <[C::Element; W]>::dist_multi_exp(&fp_u_bases, &epsilon_n)?,
            <[C::Element; W]>::dist_multi_exp(&fp_v_bases, &epsilon_n)?,
        ];
        let big_f_prime: Ciphertext<C, W> =
            Ciphertext::<C, W>(big_f_prime_prod).re_encrypt(&phi.neg(), &self.pk.y);

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
        let v = challenges.challenge(&seed, &commitments, context)?;

        ///////////////// Step 4 /////////////////

        // a
        // Serial: scalar inner product, too cheap for rayon to pay
        // (benches/parallel_tradeoff.rs).
        let a = commitment_exponents
            .iter()
            .zip(e_prime_n.iter())
            .map(|(r, e)| r.mul(e))
            .fold(C::Scalar::zero(), |acc, next| acc.add(&next));

        // c
        let c = commitment_exponents
            .iter()
            .fold(C::Scalar::zero(), |acc, next| acc.add(next));

        // f
        let s_n_e_n = encryption_exponents.par_iter().zip(e_n.par_iter());
        let s_n_e_n = s_n_e_n.map(|(s, e)| s.dist_mul(e));
        let f = fold_values(s_n_e_n, <[C::Scalar; W]>::zero, |acc, next| acc.add(next));

        // d = d_N: the last of the recurrence `bridging_commitments` already
        // computed (d_1 = b_1, d_i = b_i + e'_i·d_{i-1}), reused rather than
        // recomputed here.
        // cannot underflow, d_n.len() > 0
        #[allow(clippy::arithmetic_side_effects)]
        let d = &d_n[d_n.len() - 1];

        // k_a
        let k_a = v.mul(&a).add(&alpha);

        // k_b
        // Serial: scalar mul+add, too cheap for rayon to pay
        // (benches/parallel_tradeoff.rs). Same for k_e_n below.
        let k_b_n: Vec<C::Scalar> = b_n
            .iter()
            .zip(beta_n.iter())
            .map(|(b, beta)| {
                let vb = v.mul(b);
                vb.add(beta)
            })
            .collect();

        // k_e_n
        let e_prime_n_epsilon_n = e_prime_n.iter().zip(epsilon_n.iter());
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
        self.verify_with(
            ciphertexts,
            permuted_ciphertexts,
            proof,
            context,
            &NativeChallenges,
        )
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
            return Err(Error::MismatchedShuffleLength)
                .with_context("Mismatched length between ciphertexts and permuted ciphertexts");
        }
        if ciphertexts.len() != self.h_generators.len() {
            return Err(Error::MismatchedShuffleLength)
                .with_context("Mismatched length between ciphertexts and h_generators");
        }
        if proof.commitments.big_b_n.len() != ciphertexts.len() {
            return Err(Error::MismatchedShuffleLength).with_context(
                "Mismatched length between proof commitments big_b_n and ciphertexts",
            );
        }
        if proof.commitments.big_b_prime_n.len() != ciphertexts.len() {
            return Err(Error::MismatchedShuffleLength).with_context(
                "Mismatched length between proof commitments big_b_prime_n and ciphertexts",
            );
        }
        if proof.commitments.u_n.len() != ciphertexts.len() {
            return Err(Error::MismatchedShuffleLength)
                .with_context("Mismatched length between proof commitments u_n and ciphertexts");
        }

        let commitments = &proof.commitments;
        let responses = &proof.responses;
        let g = C::generator();

        let (seed, e_n) = challenges.batching_challenges(
            &self.h_generators,
            &commitments.u_n,
            &self.pk,
            ciphertexts,
            permuted_ciphertexts,
            context,
        )?;
        let v = challenges.challenge(&seed, commitments, context)?;

        ///////////////// Step 5 /////////////////

        // A (comes from Step 1 in evs). Vartime multi-exp: `e_n` is public.
        let u_refs: Vec<&C::Element> = commitments.u_n.iter().collect();
        let big_a = C::Element::vartime_multi_exp(&u_refs, &e_n)?;

        // F (comes from Step 1 in evs). Componentwise vartime multi-exp over
        // the 2W ciphertext columns; `e_n` is public.
        let f_u_bases: Vec<[C::Element; W]> = ciphertexts.iter().map(|w| w.u().clone()).collect();
        let f_v_bases: Vec<[C::Element; W]> = ciphertexts.iter().map(|w| w.v().clone()).collect();
        let big_f: [[C::Element; W]; 2] = [
            <[C::Element; W]>::dist_vartime_multi_exp(&f_u_bases, &e_n)?,
            <[C::Element; W]>::dist_vartime_multi_exp(&f_v_bases, &e_n)?,
        ];

        // C
        // Serial: an N-element point product folds in ~12 ms at N = 1e5, and
        // rayon's ~10 ms saving is under 0.1% of verification
        // (benches/parallel_tradeoff.rs); a plain fold is clearer.
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
        // Serial: scalar product, see the point product above.
        let e_n_fold = e_n.iter().fold(C::Scalar::one(), |acc, next| acc.mul(next));
        let h1_e_n_fold = self.h_generators[0].exp(&e_n_fold);
        // this is B_N
        // cannot underflow, ciphertexts.len() > 0
        #[allow(clippy::arithmetic_side_effects)]
        let big_b_last = &commitments.big_b_n[commitments.big_b_n.len() - 1];
        let big_d = big_b_last.mul(&h1_e_n_fold.inv());

        // B_0
        let big_b_0 = &self.h_generators[0];

        ////// Verification 1 //////

        // Vartime multi-exp: the responses `k_e_n` are public (part of the proof).
        let h_refs: Vec<&C::Element> = self.h_generators.iter().collect();
        let h_n_k_e_n_fold = C::Element::vartime_multi_exp(&h_refs, &responses.k_e_n)?;
        let g_k_a = g.exp(&responses.k_a);
        let lhs_1 = big_a.exp(&v).mul(&commitments.big_a_prime);
        let rhs_1 = g_k_a.mul(&h_n_k_e_n_fold);

        ////// Verification 2 (batched) //////
        //
        // The N elementwise checks `B_i^v · B'_i == g^{k_b_i} · B_{i-1}^{k_e_i}`
        // (with B_0 = h_1) are combined into one random-weighted check --
        // Bellare-Garay-Rabin small-exponent batching:
        //
        //   ∏ B_i^{v·t_i} · ∏ B'_i^{t_i} == g^{Σ t_i·k_b_i} · ∏ B_{i-1}^{t_i·k_e_i}
        //
        // The t_i are the verifier's OWN randomness -- not part of the
        // transcript -- so this needs no prover coordination and no
        // `ShuffleChallenges` change; it is purely verifier-internal, and the
        // Verificatum-interop path is unaffected. Soundness error <= 1/q per
        // failing equation (full-width t_i). This replaces 3N exponentiations
        // with two multi-exps (sizes 2N and N).
        let big_n = ciphertexts.len();
        let t_n: Vec<C::Scalar> = (0..big_n).map(|_| C::random_scalar()).collect();

        // LHS = ∏ B_i^{v·t_i} · ∏ B'_i^{t_i} -- one multi-exp of size 2N.
        let mut lhs2_bases: Vec<&C::Element> = commitments.big_b_n.iter().collect();
        lhs2_bases.extend(commitments.big_b_prime_n.iter());
        let mut lhs2_exps: Vec<C::Scalar> = t_n.iter().map(|t| v.mul(t)).collect();
        lhs2_exps.extend(t_n.iter().cloned());
        let lhs_2 = C::Element::vartime_multi_exp(&lhs2_bases, &lhs2_exps)?;

        // RHS = g^{Σ t_i·k_b_i} · ∏ B_{i-1}^{t_i·k_e_i}, with B_{i-1} running
        // over [h_1, B_1, ..., B_{N-1}].
        let sum_t_kb = t_n
            .iter()
            .zip(responses.k_b_n.iter())
            .map(|(t, k_b)| t.mul(k_b))
            .fold(C::Scalar::zero(), |acc, x| acc.add(&x));
        let mut rhs2_bases: Vec<&C::Element> = Vec::with_capacity(big_n);
        rhs2_bases.push(big_b_0);
        // cannot underflow, ciphertexts.len() > 0
        #[allow(clippy::arithmetic_side_effects)]
        rhs2_bases.extend(commitments.big_b_n[..big_n - 1].iter());
        let rhs2_exps: Vec<C::Scalar> = t_n
            .iter()
            .zip(responses.k_e_n.iter())
            .map(|(t, k_e)| t.mul(k_e))
            .collect();
        let rhs_2 = g
            .exp(&sum_t_kb)
            .mul(&C::Element::vartime_multi_exp(&rhs2_bases, &rhs2_exps)?);

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

        // Componentwise vartime multi-exp over the 2W output-ciphertext columns;
        // the responses `k_e_n` are public.
        let v5_u_bases: Vec<[C::Element; W]> =
            permuted_ciphertexts.iter().map(|w| w.u().clone()).collect();
        let v5_v_bases: Vec<[C::Element; W]> =
            permuted_ciphertexts.iter().map(|w| w.v().clone()).collect();
        let w_prime_n_k_e_n_fold: [[C::Element; W]; 2] = [
            <[C::Element; W]>::dist_vartime_multi_exp(&v5_u_bases, &responses.k_e_n)?,
            <[C::Element; W]>::dist_vartime_multi_exp(&v5_v_bases, &responses.k_e_n)?,
        ];

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
        #[cfg_attr(
            feature = "custom-warnings",
            crate::warning("The following code is not optimized. Parallelize with rayon")
        )]
        let u_n: Vec<C::Element> = r_h_permuted
            .into_par_iter()
            .map(|(r, h)| {
                let g = C::generator();
                let g_r = g.exp(r);
                g_r.mul(h)
            })
            .collect();

        let s_w_permuted = w_permuted.into_par_iter().zip(s_permuted.into_par_iter());

        #[cfg_attr(
            feature = "custom-warnings",
            crate::warning("The following code is not optimized. Parallelize with rayon")
        )]
        let w_prime_n: Vec<Ciphertext<C, W>> = s_w_permuted
            .into_par_iter()
            .map(|(c, s)| c.re_encrypt(s, &self.pk.y))
            .collect();

        let ret = PermutationData::new(r_n, s_n, u_n, w_prime_n);
        Ok(ret)
    }

    /// Domain separation tags for the e-challenge (batching seed) input:
    /// the full statement `(g, h, u, pk, w, w')` plus the context.
    const DS_TAGS_CHALLENGE_E: [&[u8]; 7] = [
        b"g",
        b"h_n",
        b"u_n",
        b"pk",
        b"w_n",
        b"w_prime_n",
        b"shuffle_proof_challenge_e_context",
    ];

    /// Domain separation tags for the v-challenge input: the batching seed
    /// (which transitively binds the statement and the batching vector `e`)
    /// plus the proof commitments and the context.
    const DS_TAGS_CHALLENGE_V: [&[u8]; 8] = [
        b"seed",
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

/// The output of [`bridging_commitments`]: the commitments and the two scalar
/// recurrences reused downstream.
#[allow(clippy::struct_field_names)] // the `_n` suffix is the protocol's vector notation
struct BridgingChain<C: Context> {
    /// The bridging commitments `B_1, ..., B_N`.
    big_b_n: Vec<C::Element>,
    /// The discrete-log recurrence `d_i` (`d_N` is the Step-4 response `d`).
    d_n: Vec<C::Scalar>,
    /// The prefix products `p_i = ∏_{k≤i} e'_k` (reused for `B'`).
    p_n: Vec<C::Scalar>,
}

/// The bridging commitments `B_i` of the shuffle proof (PROTOCOL.md §6.3), via
/// the closed form `B_i = g^{d_i}·h_1^{p_i}` instead of the sequential
/// recurrence `B_0 = h_1, B_i = g^{b_i}·B_{i-1}^{e'_i}`.
///
/// The recurrence is inherently serial (each `B_i` needs `B_{i-1}`) — the one
/// unparallelizable stretch in the prover. The closed form is two fixed-base
/// batches over `g` and `h_1`, fully parallel, with exponents
///
/// ```text
///   d_i = b_i + e'_i·d_{i-1}   (the discrete log of B_i base g; d_1 = b_1)
///   p_i = ∏_{k≤i} e'_k          (prefix product; p_1 = e'_1)
/// ```
///
/// built by two cheap sequential scalar scans. Returns [`BridgingChain`]: `d`
/// is also the Step-4 response `d = d_N`, so it is computed once here rather
/// than again later, and `p` feeds the matching `big_b_prime_n` closed form.
///
/// `d` and `p` are as secret as `b`/`e'`, and [`exp_many`](GroupElement::exp_many)
/// is constant-time, so nothing here leaks the permutation.
fn bridging_commitments<C: Context>(
    g: &C::Element,
    h_1: &C::Element,
    b_n: &[C::Scalar],
    e_prime_n: &[C::Scalar],
) -> BridgingChain<C> {
    let n = b_n.len();
    let mut d_n: Vec<C::Scalar> = Vec::with_capacity(n);
    let mut p_n: Vec<C::Scalar> = Vec::with_capacity(n);
    for i in 0..n {
        if i == 0 {
            d_n.push(b_n[0].clone());
            p_n.push(e_prime_n[0].clone());
        } else {
            // cannot underflow, i > 0
            #[allow(clippy::arithmetic_side_effects)]
            let prev = i - 1;
            d_n.push(b_n[i].add(&e_prime_n[i].mul(&d_n[prev])));
            p_n.push(p_n[prev].mul(&e_prime_n[i]));
        }
    }
    let g_d = g.exp_many(&d_n);
    let h1_p = h_1.exp_many(&p_n);
    let big_b_n: Vec<C::Element> = g_d.iter().zip(h1_p.iter()).map(|(a, b)| a.mul(b)).collect();
    BridgingChain { big_b_n, d_n, p_n }
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
#[derive(Debug, Canonical, PartialEq, Clone)]
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
#[derive(Debug, Canonical, PartialEq, Clone)]
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
#[derive(Debug, Canonical, PartialEq, Clone)]
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
    use crate::traits::groups::GroupElement;
    use crate::traits::groups::GroupScalar;
    use crate::utils::serialization::{Deserializable, Serializable};
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
            let actual = super::bounded_combine(&values, Scalar::zero, |acc, next| acc.add(next));

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
    fn test_shuffle_batched_v2_rejects_ristretto() {
        test_shuffle_batched_v2_rejects::<RCtx, 2>();
        test_shuffle_batched_v2_rejects::<RCtx, 3>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_shuffle_batched_v2_rejects_p256() {
        test_shuffle_batched_v2_rejects::<PCtx, 2>();
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

    /// Batched Verification 2 must reject a proof whose `k_b_n` response is
    /// tampered. `k_b` appears *only* in V2 and does not feed the challenge
    /// `v` (derived from the commitments alone), so corrupting it isolates the
    /// batched check: if the random-weighted batch were unsound, this would
    /// slip through.
    fn test_shuffle_batched_v2_rejects<C: Context, const W: usize>() {
        let count = 10;
        let keypair: KeyPair<C> = KeyPair::generate();

        let messages: Vec<[C::Element; W]> = (0..count)
            .map(|_| array::from_fn(|_| C::random_element()))
            .collect();
        let ciphertexts: Vec<Ciphertext<C, W>> =
            messages.iter().map(|m| keypair.encrypt(m)).collect();

        let generators = C::G::ind_generators(count, &vec![]).unwrap();
        let shuffler = Shuffler::<C, W>::new(generators, keypair.pkey);

        let (pciphertexts, mut proof) = shuffler.shuffle(&ciphertexts, &vec![]).unwrap();
        assert!(
            shuffler
                .verify(&ciphertexts, &pciphertexts, &proof, &vec![])
                .unwrap()
        );

        // Tamper one V2-only response; the challenge is unaffected, so only the
        // batched V2 equation can catch this.
        proof.responses.k_b_n[0] = proof.responses.k_b_n[0].add(&C::Scalar::one());
        assert!(
            !shuffler
                .verify(&ciphertexts, &pciphertexts, &proof, &vec![])
                .unwrap()
        );
    }

    /// The bridging-chain closed form `B_i = g^{d_i}·h_1^{p_i}` must reproduce
    /// the sequential recurrence `B_0 = h_1, B_i = g^{b_i}·B_{i-1}^{e'_i}`
    /// exactly, for several N including N = 1 (MSM.md §5.3). Bit-identity here
    /// is what keeps the proof — and Verificatum interop — unchanged.
    fn test_bridging_closed_form_matches_loop<C: Context>() {
        use crate::zkp::shuffle::bridging_commitments;
        for n in [1usize, 2, 5, 10, 65] {
            let g = C::generator();
            let mut rng = C::get_rng();
            let h_1 = C::Element::random(&mut rng);
            let b_n: Vec<C::Scalar> = (0..n).map(|_| C::random_scalar()).collect();
            let e_prime_n: Vec<C::Scalar> = (0..n).map(|_| C::random_scalar()).collect();

            // Sequential recurrence (the original loop), B_0 = h_1.
            let mut expected: Vec<C::Element> = Vec::with_capacity(n);
            let mut prev = h_1.clone();
            for i in 0..n {
                let b_i = g.exp(&b_n[i]).mul(&prev.exp(&e_prime_n[i]));
                expected.push(b_i.clone());
                prev = b_i;
            }

            let chain = bridging_commitments::<C>(&g, &h_1, &b_n, &e_prime_n);
            assert_eq!(chain.big_b_n, expected, "closed form disagrees at N = {n}");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_bridging_closed_form_ristretto() {
        test_bridging_closed_form_matches_loop::<RCtx>();
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    #[crate::warning("Miri test fails (Stacked Borrows)")]
    fn test_bridging_closed_form_p256() {
        test_bridging_closed_form_matches_loop::<PCtx>();
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
