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
