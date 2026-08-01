// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Turning braid's DKG and decryption output into Verificatum's form.
//!
//! Two conversions, neither of which needs any secret material — both work from
//! what braid already publishes on the board:
//!
//! - **The polynomial in the exponent.** Verificatum wants `Γ`, the joint Shamir
//!   polynomial in the exponent. braid's DKG publishes each dealer's commitments
//!   to its own polynomial's coefficients, and the joint polynomial is their
//!   sum, so `Γ_s = ∏_d C_{d,s}`.
//! - **The decryption factors.** braid publishes `u^{x_l}`; Verificatum expects
//!   `u^{−x_l/α}`. Since `−1/α` is a scalar, that is
//!   `(u^{x_l})^{−1/α}` — computable from the published factor alone, with no
//!   access to `x_l`.
//!
//! The batched *proof* is a different matter: `k_x = r − v·x_l/α` genuinely
//! needs the trustee's secret, so it cannot be produced after the fact from
//! board data.

use anyhow::{anyhow, Result};

use cryptography::context::P256Ctx;
use cryptography::groups::p256::element::P256Element;
use cryptography::groups::p256::scalar::P256Scalar;
use cryptography::traits::groups::{GroupElement, GroupScalar};

use crate::messages::artifact::Shares;

use vcompat::lagrange;

/// The joint polynomial in the exponent, `Γ_s = ∏_d C_{d,s}`.
///
/// `dealer_commitments` holds one vector per dealer, each the commitments to
/// that dealer's polynomial coefficients, in coefficient order. Every dealer
/// must have contributed the same number of coefficients — the threshold — since
/// they are shares of one joint polynomial.
///
/// `Γ_0` is the joint public key, which is what Algorithm 24 cross-checks, so
/// the caller can verify this against the DKG's own public key output.
pub fn polynomial_in_exponent(dealer_commitments: &[Vec<P256Element>]) -> Result<Vec<P256Element>> {
    let first = dealer_commitments
        .first()
        .ok_or_else(|| anyhow!("no dealers contributed commitments"))?;
    let threshold = first.len();
    if threshold == 0 {
        return Err(anyhow!("a dealer contributed no commitments"));
    }

    let mut gamma = Vec::with_capacity(threshold);
    for coefficient in 0..threshold {
        let mut product = P256Element::one();
        for (dealer, commitments) in dealer_commitments.iter().enumerate() {
            if commitments.len() != threshold {
                return Err(anyhow!(
                    "dealer {dealer} committed to {} coefficients, expected {threshold}",
                    commitments.len()
                ));
            }
            product = product.mul(&commitments[coefficient]);
        }
        gamma.push(product);
    }
    Ok(gamma)
}

/// The scalar `1/α` for `k` parties.
///
/// A party scales its share by this once and uses the result for both its
/// decryption factors and its proof reply.
pub fn inverse_alpha(k: usize) -> Result<P256Scalar> {
    let alpha = lagrange::alpha(k);
    let bytes = alpha.to_bytes_be();
    if bytes.len() > 32 {
        return Err(anyhow!("alpha does not fit in a scalar for k = {k}"));
    }
    let mut fixed = [0u8; 32];
    fixed[32 - bytes.len()..].copy_from_slice(&bytes);

    P256Scalar::from_bytes_reduced(&fixed)
        .inv()
        .ok_or_else(|| anyhow!("alpha is not invertible for k = {k}"))
}

/// The scalar `−1/α` for `k` parties, the exponent converting one of braid's
/// decryption factors into Verificatum's.
pub fn negated_inverse_alpha(k: usize) -> Result<P256Scalar> {
    Ok(inverse_alpha(k)?.neg())
}

/// Convert one of braid's decryption factors, `u^{x_l}`, into Verificatum's
/// `u^{−x_l/α}`.
///
/// Purely a re-exponentiation of the published value; no secret is involved.
pub fn to_vmn_factor<const W: usize>(
    braid_factor: &[P256Element; W],
    exponent: &P256Scalar,
) -> [P256Element; W] {
    std::array::from_fn(|i| braid_factor[i].exp(exponent))
}

/// Convert a party's whole array of decryption factors.
pub fn to_vmn_factors<const W: usize>(
    braid_factors: &[[P256Element; W]],
    k: usize,
) -> Result<Vec<[P256Element; W]>> {
    let exponent = negated_inverse_alpha(k)?;
    Ok(braid_factors
        .iter()
        .map(|f| to_vmn_factor(f, &exponent))
        .collect())
}

/// The all-identity array Verificatum writes for a party that did not take part
/// (`DistrElGamalSession`: *"Not active, setting to all-one array."*).
///
/// The file cannot be omitted — the verifier's `readArray` halts on a missing
/// one — so a non-participating trustee still needs an array of this shape, with
/// its flag false in `CorrectIndices.bt`.
pub fn inactive_factors<const W: usize>(ciphertexts: usize) -> Vec<[P256Element; W]> {
    vec![std::array::from_fn(|_| P256Element::one()); ciphertexts]
}

/// The commitment and reply Verificatum records for a party whose contribution
/// is absent or unparseable: `τ = node(1, 1)` and `σ = 0`.
///
/// `DistrElGamalSessionBasic.setCommitment` falls back to
/// `yp[l] = getONE(); Bp[l] = getONE()` and `setReply` to `k_x[l] = getZERO()`,
/// and `DistrElGamalSession` then writes those defaults out to
/// `DecrFactCommitment<l>.bt` and `DecrFactReply<l>.bt`.
///
/// These values are **not** free to choose even though the party is excluded
/// from Δ: every party's commitment is hashed into the decryption challenge
/// (`getCommitment()` builds the container over all `k`), so getting them wrong
/// moves `v` and breaks the participants' proofs. Pair with
/// [`inactive_factors`] and a `false` flag in `CorrectIndices.bt`.
pub fn inactive_proof<const W: usize>() -> BatchedDecryptionProof<W> {
    BatchedDecryptionProof {
        y_prime: P256Element::one(),
        b_prime: std::array::from_fn(|_| P256Element::one()),
        k_x: P256Scalar::zero(),
    }
}

/// Extract each dealer's coefficient commitments from braid's `Shares` bodies,
/// in dealer order.
pub fn dealer_commitments(shares: &[Shares<P256Ctx>]) -> Vec<Vec<P256Element>> {
    shares.iter().map(|s| s.commitments.clone()).collect()
}

/// One party's batched proof that its decryption factors are correct.
///
/// Replaces the `N` per-ciphertext DLEQ proofs braid produces with a single
/// proof covering the whole array, which is the Bellare-style batching
/// Verificatum uses.
///
/// Serializes as `τ^dec = node(y', B')` and `σ^dec = k_x` (VMNV §8.6).
pub struct BatchedDecryptionProof<const W: usize> {
    /// `y' = g^r`, the commitment against the party's public key.
    pub y_prime: P256Element,
    /// `B' = A^r`, the commitment against the batched first components.
    pub b_prime: [P256Element; W],
    /// `k_x = r − v·x_l`, the reply.
    pub k_x: P256Scalar,
}

/// Produce one party's batched decryption proof.
///
/// # The equations
///
/// From `DistrElGamalSessionBasic.verifyCombined`, the combined check is
///
/// ```text
/// y^{−v} · y' = g^{k_x}          B^v · B' = A^{k_x}
/// ```
///
/// where `y` is the joint public key, `A = ∏_i u_i^{e_i}` the batched first
/// components, and `B = ∏_i f_i^{e_i}` the batched *combined* factors, which
/// equal `A^{−x}`. Substituting gives `y' = g^r`, `B' = A^r` and `k_x = r − v·x`
/// for a random `r`.
///
/// # α is inside the reply, not outside it
///
/// `scaled_secret` is `x_l/α`, **not** the raw share — the same value the party's
/// decryption factors are computed from, without the negation.
///
/// The reason is that `DistrElGamalSessionBasic.combine` applies the *modified*
/// Lagrange coefficients `α·c_l` to `y'`, `B'` and `k_x`, exactly as
/// `combineDecryptionFactors` does to the factors. So a party contributing
///
/// ```text
/// y'_l = g^{r_l}    B'_l = A^{r_l}    k_{x,l} = r_l − v·(x_l/α)
/// ```
///
/// gives `k_x = Σ_l α c_l k_{x,l} = Σ_l α c_l r_l − v Σ_l c_l x_l = r − v·x`,
/// with `r = Σ_l α c_l r_l`, and the same `α c_l` combination reproduces
/// `y' = g^r` and `B' = A^r`. The α cancels inside the reply rather than being
/// absent from it.
///
/// **Verificatum's specification and Verificatum's implementation disagree
/// here.** VMNV §8.6 (Algorithm 22) combines the factors by `α c_l` but the
/// proof pieces by the plain `c_l`, which needs a reply over the unscaled `x_l`;
/// §2.4 matches, saying the party "proves that the secret key `x_l` it used is
/// given by `y_l = g^{x_l}`". `DistrElGamalSessionBasic` does the above instead
/// — and it is both what VMN's prover writes replies with and what `vmnv` checks
/// them with, so the document is the only place the other convention appears.
///
/// Both are internally consistent and they differ in the emitted bytes, so an
/// emitter has to pick one. They coincide only when `α = 1`, i.e. `k = 1`, which
/// is why the single-party reference corpus cannot tell them apart. We follow
/// the implementation, because the implementation is what `vmnv` runs.
///
/// The scaled share is derived from `x_l`, which braid reconstructs locally
/// during decryption and never publishes — so unlike the other conversions here,
/// this cannot be produced after the fact from board data.
pub fn prove_decryption<const W: usize>(
    scaled_secret: &P256Scalar,
    batched_u: &[P256Element; W],
    challenge: &P256Scalar,
    randomizer: &P256Scalar,
) -> BatchedDecryptionProof<W> {
    BatchedDecryptionProof {
        y_prime: P256Element::generator().exp(randomizer),
        b_prime: std::array::from_fn(|i| batched_u[i].exp(randomizer)),
        k_x: randomizer.sub(&challenge.mul(scaled_secret)),
    }
}

/// Batch an array of width-`W` elements by the exponents `e`: `∏_i x_i^{e_i}`.
///
/// Used for both `A` (over the ciphertexts' first components) and `B` (over the
/// combined decryption factors).
pub fn batch<const W: usize>(
    elements: &[[P256Element; W]],
    exponents: &[P256Scalar],
) -> Result<[P256Element; W]> {
    if elements.len() != exponents.len() {
        return Err(anyhow!(
            "{} elements but {} batching exponents",
            elements.len(),
            exponents.len()
        ));
    }
    let mut accumulator: [P256Element; W] = std::array::from_fn(|_| P256Element::one());
    for (element, exponent) in elements.iter().zip(exponents) {
        for i in 0..W {
            accumulator[i] = accumulator[i].mul(&element[i].exp(exponent));
        }
    }
    Ok(accumulator)
}
