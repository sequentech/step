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

/// The scalar `−1/α` for `k` parties, the exponent converting one of braid's
/// decryption factors into Verificatum's.
pub fn negated_inverse_alpha(k: usize) -> Result<P256Scalar> {
    let alpha = lagrange::alpha(k);
    let bytes = alpha.to_bytes_be();
    if bytes.len() > 32 {
        return Err(anyhow!("alpha does not fit in a scalar for k = {k}"));
    }
    let mut fixed = [0u8; 32];
    fixed[32 - bytes.len()..].copy_from_slice(&bytes);

    let scalar = P256Scalar::from_bytes_reduced(&fixed);
    let inverse = scalar
        .inv()
        .ok_or_else(|| anyhow!("alpha is not invertible for k = {k}"))?;
    Ok(inverse.neg())
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
/// The combination is by Lagrange coefficient, `y' = ∏_l (y'_l)^{c_l}` and
/// `k_x = Σ_l c_l k_{x,l}`, so each party contributing
///
/// ```text
/// y'_l = g^{r_l}      B'_l = A^{r_l}      k_{x,l} = r_l − v·x_l
/// ```
///
/// reconstructs exactly that, with `r = Σ c_l r_l` and `x = Σ c_l x_l`.
///
/// Note `α` does **not** appear here: it scales the *factors*
/// ([`to_vmn_factors`]) and is cancelled by the `α·c_l` combination exponents,
/// leaving the proof itself over the unscaled `x_l`.
///
/// `secret` is the party's share `x_l`, which braid reconstructs locally during
/// decryption and never publishes — so unlike the other conversions here, this
/// cannot be produced after the fact from board data.
pub fn prove_decryption<const W: usize>(
    secret: &P256Scalar,
    batched_u: &[P256Element; W],
    challenge: &P256Scalar,
    randomizer: &P256Scalar,
) -> BatchedDecryptionProof<W> {
    BatchedDecryptionProof {
        y_prime: P256Element::generator().exp(randomizer),
        b_prime: std::array::from_fn(|i| batched_u[i].exp(randomizer)),
        k_x: randomizer.sub(&challenge.mul(secret)),
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
