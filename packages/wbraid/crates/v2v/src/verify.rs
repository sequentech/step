// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verifying Verificatum's proofs with vsc's cryptography.
//!
//! The ingest direction: read a proof Verificatum produced and check it here,
//! rather than emitting one for `vmnv` to check. Between the two, each side's
//! output is checked by the other's verifier.
//!
//! This module is the first piece of that verifier and covers **decryption**
//! (VMNV Algorithm 22). The shuffle half already works through the
//! [`crate::challenges`] seam, which lets vsc's own `Shuffler::verify_with`
//! check a VMN proof directly — there is no shuffle code here because none is
//! needed.
//!
//! # Fail closed
//!
//! Whatever this grows into must refuse anything it does not implement, rather
//! than skipping it. That is not a stylistic preference: `vmnv` itself has a
//! defect of exactly that shape — it evaluates "too few proofs are valid" and
//! then routes the conclusion to a handler that only prints (see
//! `tests/they_verify_ours.rs`). A verifier that silently passes what it did not
//! check is worse than one that is honestly incomplete.

use anyhow::{anyhow, Result};

use cryptography::context::P256Ctx;
use cryptography::cryptosystem::elgamal::Ciphertext;
use cryptography::groups::p256::element::P256Element;
use cryptography::groups::p256::scalar::P256Scalar;
use cryptography::traits::groups::{DistGroupOps, GroupElement, GroupScalar};

use crate::decrypt::{batch, BatchedDecryptionProof};
use crate::encode;
use crate::wire::bytetree::ByteTree;
use crate::wire::crypto::{dec_challenge, dec_seed, Hashfunction, Prg};
use crate::wire::lagrange::{correct_set, p256_modified_lagrange_coefficients};

/// Session parameters a verifier needs, all of which come from the protocol
/// info file and the proof directory rather than being assumed.
pub struct SessionParams {
    /// The global prefix rho, derived from the protocol info file.
    pub rho: Vec<u8>,
    /// The hash function the random oracles are built from (`<rohash>`).
    pub hash: Hashfunction,
    /// Bit length of each batching exponent (`<ebitlenro>`).
    pub n_e: usize,
    /// Bit length of challenges (`<vbitlenro>`).
    pub n_v: usize,
    /// Number of parties `k` (`<nopart>`).
    pub parties: usize,
    /// Threshold lambda (`<thres>`).
    pub threshold: usize,
}

/// One party's published decryption contribution, as it appears in the proof
/// directory: `DecryptionFactors<l>.bt`, `DecrFactCommitment<l>.bt` and
/// `DecrFactReply<l>.bt`.
///
/// Present for **every** party in `1..=k`, including any that took no part —
/// their files hold the all-identity array and a zero reply, and are still
/// hashed into the transcript.
pub struct PartyContribution<'a, const W: usize> {
    /// This party's decryption factors, one per ciphertext.
    pub factors: &'a [[P256Element; W]],
    /// Its proof commitment `(y', B')` and reply `k_x`.
    pub proof: &'a BatchedDecryptionProof<W>,
}

/// Verify a proof of correct decryption and return the plaintexts it implies.
///
/// Returning the plaintexts rather than a boolean keeps the two questions
/// separate, as Algorithm 28 does: this establishes that the factors are
/// correct, and the caller then compares the result against the `Plaintexts.bt`
/// the proof claims. `vmnv` splits them the same way
/// (`verifyCombined` then `matchComputedPlaintexts`).
///
/// # Parameters
///
/// - `gamma`: the polynomial in the exponent, `lambda` entries, from
///   `PolynomialInExponent.bt`. `gamma[0]` is the joint public key.
/// - `ciphertexts`: the list decryption was applied to — for a `mixing` proof
///   the final mixer's output, not the session input.
/// - `contributions`: every party in `1..=k`, in index order.
/// - `correct`: `CorrectIndices.bt` as a boolean array of length `k+1`; entry 0
///   is ignored.
///
/// # Errors
///
/// Rejects — rather than returning `false` — on a malformed statement, so that
/// no caller can mistake "could not check" for "checked and passed". A proof
/// that is well formed but wrong is the one case that returns `Ok(None)`.
pub fn verify_decryption<const W: usize>(
    params: &SessionParams,
    gamma: &[P256Element],
    ciphertexts: &[Ciphertext<P256Ctx, W>],
    contributions: &[PartyContribution<W>],
    correct: &[bool],
) -> Result<Option<Vec<[P256Element; W]>>> {
    let n = ciphertexts.len();
    let k = params.parties;

    if contributions.len() != k {
        return Err(anyhow!(
            "{} contributions for {k} parties",
            contributions.len()
        ));
    }
    if gamma.len() != params.threshold {
        return Err(anyhow!(
            "the polynomial in the exponent has {} entries, expected {}",
            gamma.len(),
            params.threshold
        ));
    }
    if correct.len() != k + 1 {
        return Err(anyhow!(
            "CorrectIndices has {} entries, expected k+1 = {}",
            correct.len(),
            k + 1
        ));
    }
    for (index, contribution) in contributions.iter().enumerate() {
        if contribution.factors.len() != n {
            return Err(anyhow!(
                "party {} published {} factors for {n} ciphertexts",
                index + 1,
                contribution.factors.len()
            ));
        }
    }

    // Delta is the first lambda true flags, not an arbitrary subset -- VMN's
    // loops stop at the threshold, so a directory marking more than lambda
    // correct silently selects a prefix.
    let delta = correct_set(correct, params.threshold).ok_or_else(|| {
        anyhow!(
            "fewer than the threshold {} of parties are marked correct",
            params.threshold
        )
    })?;

    // --- combine the factors ----------------------------------------------
    //
    // alpha * c_l, small signed integers. The alpha cancels against the
    // `u^{-x_l/alpha}` the factors were computed with, leaving `u^{-x}`.
    let coefficients = signed_scalars(&p256_modified_lagrange_coefficients(&delta, k));
    let mut combined: Vec<[P256Element; W]> = vec![<[P256Element; W]>::one(); n];
    for (position, &party) in delta.iter().enumerate() {
        let factors = contributions[party - 1].factors;
        for (slot, factor) in combined.iter_mut().zip(factors) {
            *slot = slot.mul(&factor.dist_exp(&coefficients[position]));
        }
    }

    // --- rebuild the transcript -------------------------------------------
    //
    // The seed commits to gamma and to *every* party's factors, not just
    // Delta's, so a non-participant's placeholder array is load-bearing.
    let g_tree = encode::element_to_tree(&P256Element::generator())?;
    let ciphertexts_tree = encode::ciphertexts_to_tree(ciphertexts)?;
    let gamma_tree = encode::elements_to_tree(gamma)?;
    let factor_trees: Vec<ByteTree> = contributions
        .iter()
        .map(|c| encode::component_array_to_tree(c.factors))
        .collect::<Result<Vec<_>>>()?;

    let seed = dec_seed(
        params.hash,
        &params.rho,
        &g_tree,
        &ciphertexts_tree,
        &gamma_tree,
        &factor_trees,
    );
    let exponents = batching_exponents(params, &seed, n)?;

    let commitment_trees: Vec<ByteTree> = contributions
        .iter()
        .map(|c| {
            Ok(ByteTree::node(vec![
                encode::element_to_tree(&c.proof.y_prime)?,
                encode::elements_to_tree(&c.proof.b_prime)?,
            ]))
        })
        .collect::<Result<Vec<_>>>()?;
    let v = scalar_from_bytes(&dec_challenge(
        params.hash,
        params.n_v,
        &params.rho,
        &seed,
        &commitment_trees,
    ))?;

    // --- batch, and combine the proof pieces -------------------------------
    let bases: Vec<[P256Element; W]> = ciphertexts.iter().map(|c| c.0[0]).collect();
    let a = batch(&bases, &exponents)?;
    let b = batch(&combined, &exponents)?;

    // The same `alpha * c_l` used for the factors, which is what requires each
    // party to have replied over its *scaled* share. See `decrypt::prove_decryption`.
    let mut y_prime = P256Element::one();
    let mut b_prime = <[P256Element; W]>::one();
    let mut k_x = P256Scalar::zero();
    for (position, &party) in delta.iter().enumerate() {
        let proof = contributions[party - 1].proof;
        let coefficient = &coefficients[position];
        y_prime = y_prime.mul(&proof.y_prime.exp(coefficient));
        b_prime = b_prime.mul(&proof.b_prime.dist_exp(coefficient));
        k_x = k_x.add(&coefficient.mul(&proof.k_x));
    }

    // --- the two equations --------------------------------------------------
    let g = P256Element::generator();
    let first = gamma[0].exp(&v.neg()).mul(&y_prime).equals(&g.exp(&k_x));
    let second = b.dist_exp(&v).mul(&b_prime) == a.dist_exp(&k_x);
    if !(first && second) {
        return Ok(None);
    }

    // --- the plaintexts the proof implies -----------------------------------
    Ok(Some(
        ciphertexts
            .iter()
            .zip(&combined)
            .map(|(c, f)| {
                let v_component = c.0[1];
                std::array::from_fn(|w| v_component[w].mul(&f[w]))
            })
            .collect(),
    ))
}

/// Expand the seed into one `n_e`-bit batching exponent per ciphertext.
fn batching_exponents(params: &SessionParams, seed: &[u8], n: usize) -> Result<Vec<P256Scalar>> {
    let component = params.n_e.div_ceil(8);
    let stream = Prg::new(params.hash, seed).generate(component * n);
    stream
        .chunks(component)
        .map(|chunk| {
            // `e_i = t_i mod 2^n_e`, so mask the top byte when n_e is not a
            // whole number of bytes.
            let mut value = chunk.to_vec();
            let excess = params.n_e % 8;
            if excess != 0 && !value.is_empty() {
                value[0] &= 0xFFu8 >> (8 - excess);
            }
            scalar_from_bytes(&value)
        })
        .collect()
}

/// Interpret a big-endian byte string as a scalar, reduced into the field.
///
/// Verificatum reads these as unbounded non-negative integers and exponentiates
/// by them, which is the same thing in a group of prime order.
fn scalar_from_bytes(bytes: &[u8]) -> Result<P256Scalar> {
    if bytes.len() > 32 {
        return Err(anyhow!(
            "value of {} bytes is wider than a P-256 scalar",
            bytes.len()
        ));
    }
    let mut fixed = [0u8; 32];
    fixed[32 - bytes.len()..].copy_from_slice(bytes);
    Ok(P256Scalar::from_bytes_reduced(&fixed))
}

/// Turn the signed modified Lagrange coefficients into field elements.
fn signed_scalars(coefficients: &[(bool, [u8; 32])]) -> Vec<P256Scalar> {
    coefficients
        .iter()
        .map(|(negative, magnitude)| {
            let scalar = P256Scalar::from_bytes_reduced(magnitude);
            if *negative {
                scalar.neg()
            } else {
                scalar
            }
        })
        .collect()
}
