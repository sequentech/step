// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verificatum's modified Lagrange coefficients, and the `α` that makes them
//! integers.
//!
//! Threshold decryption reconstructs the secret as `x = Σ_{l∈Δ} c_l x_l` with
//! the usual Lagrange coefficients `c_l = ∏_{i∈Δ\{l}} i/(i−l)`. Those are field
//! elements, so combining decryption factors by them would mean a full-width
//! exponentiation per party.
//!
//! Verificatum avoids that (`DistrElGamalSessionBasic.prodFactor`,
//! `modifiedLagrangeCoefficient`):
//!
//! 1. `α = lcm(1,…,k)²` clears every denominator, so `α·c_l` is an **integer**;
//! 2. each party scales its secret down, publishing `f_l = u^{−x_l/α}`;
//! 3. combination uses the integer exponent `α·c_l`, giving
//!    `∏_l f_l^{α c_l} = u^{−Σ c_l x_l} = u^{−x}` — the α cancels;
//! 4. that integer is reduced to the representative of **smallest absolute
//!    value**, so it may be **negative** and stays small.
//!
//! Step 4 is the point: exponents are small signed integers rather than
//! 256-bit scalars. It also changes the bytes, so an emitter has to reproduce it
//! exactly rather than just getting the arithmetic right modulo `q`.
//!
//! # Δ is a prefix
//!
//! `modifiedLagrangeCoefficients` walks `correct` and stops after `threshold`
//! entries, so Δ is the **first λ** true flags — not an arbitrary subset. A
//! caller marking more than λ correct silently gets a prefix rather than an
//! error.

use num_bigint::{BigInt, BigUint};

/// `lcm(1, …, k)`, as `∏_{p ≤ k prime} p^{⌊log_p k⌋}`.
///
/// Mirrors `primeLog` accumulated over the prime table: for each prime at most
/// `k`, the largest power of that prime not exceeding `k`.
pub fn lcm_up_to(k: usize) -> BigUint {
    let mut result = BigUint::from(1u32);
    for prime in primes_up_to(k) {
        let mut power = prime;
        while power.saturating_mul(prime) <= k {
            power *= prime;
        }
        result *= BigUint::from(power);
    }
    result
}

/// `α = lcm(1, …, k)²` (`prodFactor`).
///
/// This is the factor each party divides its secret by before computing
/// decryption factors, and that the combination exponents multiply back in.
pub fn alpha(k: usize) -> BigUint {
    let l = lcm_up_to(k);
    &l * &l
}

/// Primes up to and including `k`, by trial division — `k` is a party count, so
/// tiny.
fn primes_up_to(k: usize) -> Vec<usize> {
    (2..=k)
        .filter(|n| (2..*n).take_while(|d| d * d <= *n).all(|d| n % d != 0))
        .collect()
}

/// The set Δ: the indices of the first `threshold` true entries of `correct`.
///
/// `correct` is indexed from 1 (entry 0 is ignored, matching
/// `CorrectIndices.bt`). Returns `None` if fewer than `threshold` are true,
/// which is Verificatum's "attempting to combine too few decryption factors".
pub fn correct_set(correct: &[bool], threshold: usize) -> Option<Vec<usize>> {
    let selected: Vec<usize> = correct
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, ok)| **ok)
        .map(|(index, _)| index)
        .take(threshold)
        .collect();
    (selected.len() == threshold).then_some(selected)
}

/// The modified Lagrange coefficient `α·c_i` for index `i` over the set `delta`,
/// as a signed integer of smallest absolute value modulo `q`.
///
/// `α·∏_{l∈Δ\{i}} l/(l−i)` is computed modulo `q` — the division is a modular
/// inverse — and the result is then mapped into `(−q/2, q/2]` by taking
/// `res − q` when that is smaller in absolute value. The α guarantees the value
/// is a genuine integer, so this reduction recovers it exactly rather than
/// merely finding a small congruent representative.
pub fn modified_lagrange_coefficient(
    delta: &[usize],
    index: usize,
    k: usize,
    q: &BigUint,
) -> BigInt {
    let mut result = alpha(k) % q;
    for &l in delta {
        if l == index {
            continue;
        }
        result = (result * BigUint::from(l)) % q;
        // l - index may be negative; reduce it into the field before inverting.
        let difference = if l > index {
            BigUint::from(l - index) % q
        } else {
            q - (BigUint::from(index - l) % q)
        };
        result = (result * mod_inverse(&difference, q)) % q;
    }

    // Smallest absolute value: res or res - q.
    let positive = BigInt::from(result);
    let negative = &positive - BigInt::from(q.clone());
    if negative.magnitude() < positive.magnitude() {
        negative
    } else {
        positive
    }
}

/// All modified Lagrange coefficients for Δ, in Δ's order.
pub fn modified_lagrange_coefficients(
    delta: &[usize],
    k: usize,
    q: &BigUint,
) -> Vec<BigInt> {
    delta
        .iter()
        .map(|&index| modified_lagrange_coefficient(delta, index, k, q))
        .collect()
}

/// `a^{-1} mod q` for prime `q`, by Fermat's little theorem.
fn mod_inverse(a: &BigUint, q: &BigUint) -> BigUint {
    a.modpow(&(q - BigUint::from(2u32)), q)
}
