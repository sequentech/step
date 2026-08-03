// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Verificatum's modified Lagrange coefficients and the `α` factor.
//!
//! There is no golden vector for these — the reference corpus has `k = λ = 1`,
//! where `α = 1` and every coefficient is `1`, so it cannot exercise any of it
//! (see `testdata/verificatum/README.md`). These tests therefore check the
//! defining properties directly, and `vmnv` adjudicates the real thing once a
//! multi-party proof is emitted.

use num_bigint::{BigInt, BigUint};
use num_traits::{One, Signed, Zero};

use vsvmn::wire::lagrange::{
    alpha, correct_set, lcm_up_to, modified_lagrange_coefficient,
    modified_lagrange_coefficients,
};

/// P-256's group order, the modulus these coefficients live in.
fn q() -> BigUint {
    BigUint::parse_bytes(
        b"ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551",
        16,
    )
    .expect("valid P-256 order")
}

#[test]
fn lcm_matches_the_definition() {
    // lcm(1..k) for small k, computed independently.
    for (k, expected) in [
        (1usize, 1u32),
        (2, 2),
        (3, 6),
        (4, 12),
        (5, 60),
        (6, 60),
        (7, 420),
        (8, 840),
    ] {
        assert_eq!(lcm_up_to(k), BigUint::from(expected), "lcm(1..{k})");
    }
}

#[test]
fn alpha_is_the_square_of_that() {
    for k in 1..=8usize {
        let l = lcm_up_to(k);
        assert_eq!(alpha(k), &l * &l, "alpha({k})");
    }
    // The value braid's own sessions will use most often.
    assert_eq!(alpha(3), BigUint::from(36u32));
}

#[test]
fn delta_is_the_first_threshold_true_entries() {
    // Index 0 is ignored, matching CorrectIndices.bt.
    let correct = [false, true, false, true, true];
    assert_eq!(correct_set(&correct, 2), Some(vec![1, 3]));
    assert_eq!(correct_set(&correct, 3), Some(vec![1, 3, 4]));
    // Not a subset choice: it takes a prefix, so a caller marking too many
    // correct silently gets the first ones rather than an error.
    assert_eq!(correct_set(&[false, true, true, true], 2), Some(vec![1, 2]));
    // Too few true entries is a failure, matching "attempting to combine too
    // few decryption factors".
    assert_eq!(correct_set(&[false, true, false], 2), None);
}

#[test]
fn coefficients_are_small_signed_integers() {
    let q = q();
    // Delta = {1, 2} with k = 2: c_1 = 2/(2-1) = 2, c_2 = 1/(1-2) = -1,
    // and alpha = lcm(1,2)^2 = 4, so the modified values are 8 and -4.
    let delta = vec![1usize, 2];
    let coefficients = modified_lagrange_coefficients(&delta, 2, &q);
    assert_eq!(coefficients, vec![BigInt::from(8), BigInt::from(-4)]);

    // The point of the reduction: these stay tiny rather than being 256-bit
    // field elements, and negative values are expected.
    assert!(coefficients.iter().any(|c| c.is_negative()));
    for c in &coefficients {
        assert!(c.magnitude() < &BigUint::from(u64::MAX), "coefficient stayed small");
    }
}

/// The 3-of-3 case, pinned because it is what `braid`'s `vmnv -mix` interop test
/// runs on and what `VERIFICATUM.md` quotes.
///
/// With alpha = lcm(1,2,3)^2 = 36: c_1 = (2/1)(3/2) = 3, c_2 = (1/-1)(3/1) = -3,
/// c_3 = (1/-2)(2/-1) = 1.
#[test]
fn the_three_of_three_coefficients() {
    let coefficients = modified_lagrange_coefficients(&[1, 2, 3], 3, &q());
    assert_eq!(
        coefficients,
        vec![BigInt::from(108), BigInt::from(-108), BigInt::from(36)]
    );
}

#[test]
fn coefficients_reconstruct_a_shared_secret() {
    // The defining property: for a degree lambda-1 polynomial p, evaluating at
    // the indices in Delta and combining with the modified coefficients gives
    // alpha * p(0). Uses small integers so the arithmetic is checkable by hand.
    let q = q();
    let k = 3usize;
    let _threshold = 2usize;

    // p(z) = 7 + 5z, so the secret is 7.
    let p = |z: i64| BigInt::from(7 + 5 * z);

    for delta in [vec![1usize, 2], vec![1, 3], vec![2, 3]] {
        let coefficients = modified_lagrange_coefficients(&delta, k, &q);
        let mut total = BigInt::zero();
        for (position, &index) in delta.iter().enumerate() {
            total += &coefficients[position] * p(index as i64);
        }
        // Combination yields alpha * secret, which is why dividing each share
        // by alpha beforehand makes it cancel.
        let expected = BigInt::from(alpha(k)) * BigInt::from(7);
        let modulus = BigInt::from(q.clone());
        assert_eq!(
            ((total - &expected) % &modulus + &modulus) % &modulus,
            BigInt::zero(),
            "Delta = {delta:?} must reconstruct alpha * p(0)"
        );
    }
}

#[test]
fn a_single_party_is_the_degenerate_case() {
    // What the reference corpus exercises, and why it cannot test any of the
    // above: alpha is 1 and the only coefficient is 1.
    let q = q();
    assert_eq!(alpha(1), BigUint::one());
    assert_eq!(
        modified_lagrange_coefficient(&[1], 1, 1, &q),
        BigInt::one()
    );
}
