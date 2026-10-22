// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 Free & Fair
// See LICENSE.md for details

//! Proof of equality plaintexts.

use crate::context::Context;
use crate::traits::groups::CryptographicGroup;
use crate::traits::groups::DistGroupOps;
use crate::traits::groups::GroupElement;
use crate::traits::groups::GroupScalar;
use crate::traits::groups::ReplGroupOps;
use crate::traits::groups::ReplScalarOps;
use crate::utils::error::Error;
use crate::utils::serialization::Serializable;
use canonical_derive::Canonical;
use rayon::prelude::*;

/// The `(base, exponent)` pairs one instance contributes to the batched
/// check's multi-exponentiation.
type BatchTerms<'a, C> = Vec<(&'a <C as Context>::Element, <C as Context>::Scalar)>;

/// The statement and proof of one Naor-Yung ciphertext, borrowed for
/// [`PlEqProof::verify_batch`].
///
/// The keys `y`, `z` and the context are shared by the whole batch and are
/// passed alongside the list rather than repeated per instance.
#[derive(Debug, Clone, Copy)]
pub struct PlEqInstance<'a, C: Context, const W: usize> {
    /// The ciphertext component `u_b`, of width `W`
    pub u_b: &'a [C::Element; W],
    /// The ciphertext component `v_b`, of width `W`
    pub v_b: &'a [C::Element; W],
    /// The ciphertext component `u_a`, of width `W`
    pub u_a: &'a [C::Element; W],
    /// The proof of equality of plaintexts for `(u_b, v_b, u_a)`
    pub proof: &'a PlEqProof<C, W>,
}

/**
 * Proof of equality plaintexts.
 *
 * Given public values `y`, `g`, `z`, `u_b`, `v_b` `u_a` and
 * secrets `p` and `r` proves equality of plaintexts (`u_b`, `v_b`)
 * and (`u_a`, `v_b`). This proof is applied to ciphertexts
 * in the Naor-Yung encryption scheme.
 *
 * See `EVS`: Protocol 10.8
 *
 * See [`crate::cryptosystem::naoryung`]
 *
 * # Examples
 * ```
 * use cryptography::context::Context;
 * use cryptography::context::RistrettoCtx as RCtx;
 * use cryptography::groups::ristretto255::RistrettoElement;
 * use cryptography::groups::ristretto255::RistrettoScalar;
 * use cryptography::traits::groups::GroupElement;
 * use cryptography::traits::groups::DistGroupOps;
 * use cryptography::zkp::dlogeq::DlogEqProof;
 * use cryptography::cryptosystem::naoryung::KeyPair;
 * use cryptography::zkp::pleq::PlEqProof;
 * use cryptography::traits::groups::GroupScalar;
 *
 * // This context that will be used to derive
 * // the Naor-Yung second public key, `z`
 * let keypair_context = &[];
 * let ny: KeyPair<RCtx> = KeyPair::generate(keypair_context).unwrap();
 *
 * let msg = [RCtx::random_element(), RCtx::random_element()];
 * let mut rng = RCtx::get_rng();
 * let r = <[RistrettoScalar; 2]>::random(&mut rng);
 * // in a real application, the plaintext equality proof would be
 * // computed automatically by this function, here we compute
 * // the proof manually as well to demonstrate its usage
 * let ciphertext = ny.encrypt_with_r(&msg, &r, &[]).unwrap();
 * // Set to some relevant context value
 * let proof_context = &[];
 * let proof = PlEqProof::<RCtx, 2>::prove(
 *    &ny.pkey.pk_b,
 *    &ny.pkey.pk_a,
 *    &ciphertext.u_b,
 *    &ciphertext.v_b,
 *    &ciphertext.u_a,
 *    &r,
 *    proof_context).unwrap();
 *
 * let ok = proof.verify(&ny.pkey.pk_b,
 *    &ny.pkey.pk_a,
 *    &ciphertext.u_b,
 *    &ciphertext.v_b,
 *    &ciphertext.u_a,
 *    proof_context).unwrap();
 *
 * assert!(ok);
 * ```
 */
#[derive(Debug, Clone, Canonical, PartialEq)]
pub struct PlEqProof<C: Context, const W: usize> {
    /// Prover commitment
    pub big_a: [[C::Element; W]; 2],
    /// Challenge response
    pub k: [C::Scalar; W],
}

impl<C: Context, const W: usize> PlEqProof<C, W> {
    /// Construct a proof of equality of plaintexts from the given values.
    pub fn new(big_a: [[C::Element; W]; 2], k: [C::Scalar; W]) -> Self {
        PlEqProof { big_a, k }
    }

    /// Prove equality of plaintexts for a Naor-Yung ciphertext.
    ///
    /// # Parameters
    ///
    /// - `y`: The Naor-Yung public key `y`
    /// - `z`: The Naor-Yung public key `z`
    /// - `u_b`: The Naor-Yung ciphertext component `u_b`, of width `W`
    /// - `v_b`: The Naor-Yung ciphertext component `v_b`, of width `W`
    /// - `u_a`: The Naor-Yung ciphertext component `u_a`, of width `W`
    /// - `r`: The random scalar used in the encryption, of width `W`
    /// - `proof_context`: proof context label (ZKP CONTEXT)
    ///
    /// # Errors
    ///
    /// - `HashToElementError` if challenge generation returns error
    ///
    /// Returns a [`PlEqProof`] instance.
    #[allow(clippy::many_single_char_names)]
    pub fn prove(
        y: &C::Element,
        z: &C::Element,
        u_b: &[C::Element; W],
        v_b: &[C::Element; W],
        u_a: &[C::Element; W],
        r: &[C::Scalar; W],
        proof_context: &[u8],
    ) -> Result<PlEqProof<C, W>, Error> {
        let g = C::generator();
        let mut rng = C::get_rng();
        let a_prime = <[C::Scalar; W]>::random(&mut rng);
        let a = a_prime.mul(r);
        let big_a_g = g.repl_exp(&a);
        let big_a_z = z.repl_exp(&a);

        let big_a = [big_a_g, big_a_z];

        let (input, dsts) = Self::challenge_input(&g, y, z, u_b, v_b, u_a, &big_a, proof_context);
        let input: Vec<&[u8]> = input.iter().map(Vec::as_slice).collect();
        let v = C::G::hash_to_scalar(&input, &dsts)?;

        let vr = v.repl_mul(r);
        let k = vr.add(&a);

        Ok(PlEqProof::new(big_a, k))
    }

    /// Verify this Schnorr proof of knowledge.
    ///
    /// - `y`: The Naor-Yung public key `y`
    /// - `z`: The Naor-Yung public key `z`
    /// - `u_b`: The Naor-Yung ciphertext component `u_b`, of width `W`
    /// - `v_b`: The Naor-Yung ciphertext component `v_b`, of width `W`
    /// - `u_a`: The Naor-Yung ciphertext component `u_a`, of width `W`
    /// - `proof_context`: proof context label (ZKP CONTEXT)
    ///
    /// # Errors
    ///
    /// - `HashToElementError` if challenge generation returns error
    ///
    /// Returns `true` if the proof is valid, `false` otherwise.
    pub fn verify(
        &self,
        y: &C::Element,
        z: &C::Element,
        u_b: &[C::Element; W],
        v_b: &[C::Element; W],
        u_a: &[C::Element; W],
        context: &[u8],
    ) -> Result<bool, Error> {
        let g = C::generator();
        let (input, dsts) = Self::challenge_input(&g, y, z, u_b, v_b, u_a, &self.big_a, context);
        let input: Vec<&[u8]> = input.iter().map(Vec::as_slice).collect();
        let v = C::G::hash_to_scalar(&input, &dsts)?;

        let f_k = [g, z.clone()].map(|e| e.repl_exp(&self.k));

        let u_v = [u_b, u_a].map(|e| e.dist_exp(&v));

        let u_v_big_a = u_v.mul(&self.big_a);

        Ok(u_v_big_a.equals(&f_k))
    }

    /// Verify many proofs against the same keys and context as one random
    /// linear combination, returning the indices of the instances that fail.
    ///
    /// Each instance `i` states, per component `w`, `g^{k_{i,w}} = A_{g,i,w}
    /// u_{b,i,w}^{v_i}` and `z^{k_{i,w}} = A_{z,i,w} u_{a,i,w}^{v_i}`, with
    /// `v_i` its own challenge as in [`verify`](Self::verify). Drawing
    /// independent uniform weights `t_{i,w}` and `s_{i,w}` — the verifier's
    /// own randomness, not part of any transcript — all `2WN` equations are
    /// checked at once as
    ///
    /// `g^{Σ t k} z^{Σ s k} = ∏ A_g^{t} ∏ A_z^{s} ∏ u_b^{t v} ∏ u_a^{s v}`
    ///
    /// which holds identically when every instance is valid, and with
    /// probability exactly `1/q` over the weights when any instance is not
    /// (Bellare–Garay–Rabin small-exponent batching). The right-hand side is
    /// one variable-time multi-exponentiation of size `4WN`; the per-instance
    /// challenges are hashed in parallel. When the combination fails, every
    /// instance is verified individually so that the failures are attributed.
    ///
    /// All inputs are public data, so variable-time arithmetic is sound here.
    ///
    /// # Parameters
    ///
    /// - `y`: The Naor-Yung public key `y`
    /// - `z`: The Naor-Yung public key `z`
    /// - `instances`: The statements and proofs to verify
    /// - `context`: proof context label (ZKP CONTEXT), shared by all instances
    ///
    /// # Errors
    ///
    /// - `HashToElementError` if challenge generation returns error
    /// - `BatchVerificationInconsistent` if the combination fails but every
    ///   instance verifies individually, which cannot happen for correct
    ///   arithmetic; the verifier fails closed rather than accept
    ///
    /// Returns the ascending indices of the instances whose proof is invalid;
    /// an empty list means every instance verified.
    pub fn verify_batch(
        y: &C::Element,
        z: &C::Element,
        instances: &[PlEqInstance<'_, C, W>],
        context: &[u8],
    ) -> Result<Vec<usize>, Error> {
        if instances.is_empty() {
            return Ok(vec![]);
        }
        let g = C::generator();

        let challenges: Vec<C::Scalar> = instances
            .par_iter()
            .map(|inst| {
                let (input, dsts) = Self::challenge_input(
                    &g,
                    y,
                    z,
                    inst.u_b,
                    inst.v_b,
                    inst.u_a,
                    &inst.proof.big_a,
                    context,
                );
                let input: Vec<&[u8]> = input.iter().map(Vec::as_slice).collect();
                C::G::hash_to_scalar(&input, &dsts)
            })
            .collect::<Result<_, _>>()?;

        // Per instance: the two summed fixed-base exponents (Σ t k, Σ s k) and
        // the 4W (base, exponent) pairs of the right-hand side.
        let (sums, terms): (Vec<[C::Scalar; 2]>, Vec<BatchTerms<'_, C>>) = instances
            .par_iter()
            .zip(challenges.par_iter())
            .map(|(inst, challenge)| {
                let mut rng = C::get_rng();
                let mut sum_g = C::Scalar::zero();
                let mut sum_z = C::Scalar::zero();
                let mut terms = Vec::with_capacity(W.saturating_mul(4));
                let [big_a_g, big_a_z] = &inst.proof.big_a;
                let components = inst
                    .proof
                    .k
                    .iter()
                    .zip(big_a_g)
                    .zip(big_a_z)
                    .zip(inst.u_b)
                    .zip(inst.u_a);
                for ((((response, a_g), a_z), u_b), u_a) in components {
                    let weight_g = C::Scalar::random(&mut rng);
                    let weight_z = C::Scalar::random(&mut rng);
                    sum_g = sum_g.add(&weight_g.mul(response));
                    sum_z = sum_z.add(&weight_z.mul(response));
                    terms.push((u_b, weight_g.mul(challenge)));
                    terms.push((u_a, weight_z.mul(challenge)));
                    terms.push((a_g, weight_g));
                    terms.push((a_z, weight_z));
                }
                ([sum_g, sum_z], terms)
            })
            .unzip();

        let [sum_g, sum_z] = sums.into_par_iter().reduce(
            || [C::Scalar::zero(), C::Scalar::zero()],
            |acc, x| [acc[0].add(&x[0]), acc[1].add(&x[1])],
        );
        let (bases, exponents): (Vec<&C::Element>, Vec<C::Scalar>) =
            terms.into_par_iter().flatten_iter().unzip();

        let lhs = g.exp(&sum_g).mul(&z.exp(&sum_z));
        let rhs = C::Element::vartime_multi_exp(&bases, &exponents)?;
        if lhs.equals(&rhs) {
            return Ok(vec![]);
        }

        let mut failing: Vec<usize> = instances
            .par_iter()
            .enumerate()
            .filter_map(|(i, inst)| {
                match inst
                    .proof
                    .verify(y, z, inst.u_b, inst.v_b, inst.u_a, context)
                {
                    Ok(true) => None,
                    Ok(false) => Some(Ok(i)),
                    Err(e) => Some(Err(e)),
                }
            })
            .collect::<Result<_, _>>()?;
        failing.sort_unstable();
        if failing.is_empty() {
            return Err(Error::BatchVerificationInconsistent(
                "batched plaintext-equality check failed but every proof verifies individually"
                    .into(),
            ));
        }
        Ok(failing)
    }

    /// Domain separation tags for the challenge input
    #[crate::warning("Challenge inputs are incomplete.")]
    const DS_TAGS: [&[u8]; 8] = [
        b"g",
        b"y",
        b"z",
        b"u_b",
        b"v_b",
        b"u_a",
        b"big_a",
        b"pleq_context",
    ];

    /// Computes the challenge input for the plaintext equality proof.
    ///
    /// # Params
    ///
    /// - `g`: The generator element
    /// - `y`: The Naor-Yung public key `y`
    /// - `z`: The Naor-Yung public key `z`
    /// - `u_b`: The Naor-Yung ciphertext component `u_b`, of width `W`
    /// - `v_b`: The Naor-Yung ciphertext component `v_b`, of width `W`
    /// - `u_a`: The Naor-Yung ciphertext component `u_a`, of width `W`
    /// - `big_a`: The prover commitments, of width `W`
    /// - `proof_context`: proof context label (ZKP CONTEXT)
    ///
    /// Returns byte arrays for input values and domain separation tags.
    /// These values will be passed to the hash function to compute
    /// the challenge.
    #[allow(clippy::too_many_arguments)]
    fn challenge_input(
        g: &C::Element,
        y: &C::Element,
        z: &C::Element,
        u_b: &[C::Element; W],
        v_b: &[C::Element; W],
        u_a: &[C::Element; W],
        big_a: &[[C::Element; W]; 2],
        context: &[u8],
    ) -> ([Vec<u8>; 8], [&'static [u8]; 8]) {
        let a = [
            g.ser(),
            y.ser(),
            z.ser(),
            u_b.ser(),
            v_b.ser(),
            u_a.ser(),
            big_a.ser(),
            context.to_vec(),
        ];

        (a, Self::DS_TAGS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::context::P256Ctx as PCtx;
    use crate::context::RistrettoCtx as RCtx;
    use crate::cryptosystem::naoryung::KeyPair;
    use crate::traits::groups::DistScalarOps;
    use crate::utils::serialization::{Deserializable, Serializable};

    #[test]
    fn test_pleq_verify_batch_accepts_ristretto() {
        test_pleq_verify_batch_accepts::<RCtx>();
    }

    #[test]
    fn test_pleq_verify_batch_accepts_p256() {
        test_pleq_verify_batch_accepts::<PCtx>();
    }

    #[test]
    fn test_pleq_verify_batch_attributes_failures_ristretto() {
        test_pleq_verify_batch_attributes_failures::<RCtx>();
    }

    #[test]
    fn test_pleq_verify_batch_attributes_failures_p256() {
        test_pleq_verify_batch_attributes_failures::<PCtx>();
    }

    fn ballots<Ctx: Context, const W: usize>(
        ny: &KeyPair<Ctx>,
        n: usize,
    ) -> Vec<crate::cryptosystem::naoryung::Ciphertext<Ctx, W>> {
        (0..n)
            .map(|_| {
                let msg: [Ctx::Element; W] = std::array::from_fn(|_| Ctx::random_element());
                ny.encrypt(&msg, &[]).unwrap()
            })
            .collect()
    }

    fn instances<Ctx: Context, const W: usize>(
        cs: &[crate::cryptosystem::naoryung::Ciphertext<Ctx, W>],
    ) -> Vec<PlEqInstance<'_, Ctx, W>> {
        cs.iter()
            .map(|c| PlEqInstance {
                u_b: &c.u_b,
                v_b: &c.v_b,
                u_a: &c.u_a,
                proof: &c.proof,
            })
            .collect()
    }

    /// Valid lists of every awkward size, at two widths, batch-verify with no
    /// failing index.
    fn test_pleq_verify_batch_accepts<Ctx: Context>() {
        let ny: KeyPair<Ctx> = KeyPair::generate(&[]).unwrap();
        for n in [0usize, 1, 2, 33] {
            let cs = ballots::<Ctx, 2>(&ny, n);
            let failing = PlEqProof::<Ctx, 2>::verify_batch(
                &ny.pkey.pk_b,
                &ny.pkey.pk_a,
                &instances(&cs),
                &[],
            )
            .unwrap();
            assert!(
                failing.is_empty(),
                "n = {n}: unexpected failures {failing:?}"
            );
        }
        let cs = ballots::<Ctx, 3>(&ny, 5);
        let failing =
            PlEqProof::<Ctx, 3>::verify_batch(&ny.pkey.pk_b, &ny.pkey.pk_a, &instances(&cs), &[])
                .unwrap();
        assert!(failing.is_empty(), "W = 3: unexpected failures {failing:?}");
    }

    /// A tampered response and a pair of proofs swapped between ballots (each
    /// now proving the wrong statement) are attributed to exactly their indices,
    /// the batch agrees with per-item verification everywhere, and a different
    /// context rejects the whole list.
    fn test_pleq_verify_batch_attributes_failures<Ctx: Context>() {
        let ny: KeyPair<Ctx> = KeyPair::generate(&[]).unwrap();
        let mut cs = ballots::<Ctx, 2>(&ny, 9);
        let bad = cs.get_mut(3).unwrap();
        bad.proof = PlEqProof::new(
            bad.proof.big_a.clone(),
            bad.proof.k.dist_add(&Ctx::Scalar::one()),
        );
        let p6 = cs.get(6).unwrap().proof.clone();
        let p7 = cs.get(7).unwrap().proof.clone();
        cs.get_mut(6).unwrap().proof = p7;
        cs.get_mut(7).unwrap().proof = p6;

        let failing =
            PlEqProof::<Ctx, 2>::verify_batch(&ny.pkey.pk_b, &ny.pkey.pk_a, &instances(&cs), &[])
                .unwrap();
        assert_eq!(failing, vec![3, 6, 7]);

        for (i, c) in cs.iter().enumerate() {
            let ok = c
                .proof
                .verify(&ny.pkey.pk_b, &ny.pkey.pk_a, &c.u_b, &c.v_b, &c.u_a, &[])
                .unwrap();
            assert_eq!(
                ok,
                !failing.contains(&i),
                "index {i}: batch and per-item disagree"
            );
        }

        let all = PlEqProof::<Ctx, 2>::verify_batch(
            &ny.pkey.pk_b,
            &ny.pkey.pk_a,
            &instances(&cs),
            b"other",
        )
        .unwrap();
        assert_eq!(all.len(), 9, "a different context must reject every ballot");
    }

    #[test]
    fn test_pleq_proof_valid_ristretto() {
        test_pleq_proof_valid::<RCtx>();
    }

    #[test]
    fn test_pleq_proof_valid_p256() {
        test_pleq_proof_valid::<PCtx>();
    }

    #[test]
    fn test_pleq_proof_serialization_ristretto() {
        test_pleq_proof_serialization::<RCtx>();
    }

    #[test]
    fn test_pleq_proof_serialization_p256() {
        test_pleq_proof_serialization::<PCtx>();
    }

    fn test_pleq_proof_valid<Ctx: Context>() {
        let ny: KeyPair<Ctx> = KeyPair::generate(&vec![]).unwrap();

        let msg = [Ctx::random_element(), Ctx::random_element()];
        let mut rng = Ctx::get_rng();
        let r = <[Ctx::Scalar; 2]>::random(&mut rng);
        let ciphertext = ny.encrypt_with_r(&msg, &r, &vec![]).unwrap();

        let proof = PlEqProof::<Ctx, 2>::prove(
            &ny.pkey.pk_b,
            &ny.pkey.pk_a,
            &ciphertext.u_b,
            &ciphertext.v_b,
            &ciphertext.u_a,
            &r,
            &vec![],
        )
        .unwrap();

        let ok = proof
            .verify(
                &ny.pkey.pk_b,
                &ny.pkey.pk_a,
                &ciphertext.u_b,
                &ciphertext.v_b,
                &ciphertext.u_a,
                &vec![],
            )
            .unwrap();

        assert!(ok);

        let original_k = proof.k;
        let tampered_k = original_k.dist_add(&Ctx::Scalar::one());

        let tampered_proof = PlEqProof::<Ctx, 2>::new(proof.big_a, tampered_k);

        let not_ok = tampered_proof
            .verify(
                &ny.pkey.pk_b,
                &ny.pkey.pk_a,
                &ciphertext.u_b,
                &ciphertext.v_b,
                &ciphertext.u_a,
                &vec![],
            )
            .unwrap();

        assert!(!not_ok);
    }

    fn test_pleq_proof_serialization<Ctx: Context>() {
        let ny: KeyPair<Ctx> = KeyPair::generate(&vec![]).unwrap();

        let msg = [Ctx::random_element(), Ctx::random_element()];
        let mut rng = Ctx::get_rng();
        let r = <[Ctx::Scalar; 2]>::random(&mut rng);
        let ciphertext = ny.encrypt_with_r(&msg, &r, &vec![]).unwrap();

        let proof = PlEqProof::<Ctx, 2>::prove(
            &ny.pkey.pk_b,
            &ny.pkey.pk_a,
            &ciphertext.u_b,
            &ciphertext.v_b,
            &ciphertext.u_a,
            &r,
            &vec![],
        )
        .unwrap();
        let bytes = proof.ser();
        let proof_d = PlEqProof::<Ctx, 2>::deser(&bytes).unwrap();

        let ok = proof_d
            .verify(
                &ny.pkey.pk_b,
                &ny.pkey.pk_a,
                &ciphertext.u_b,
                &ciphertext.v_b,
                &ciphertext.u_a,
                &vec![],
            )
            .unwrap();

        assert!(ok);
    }
}
