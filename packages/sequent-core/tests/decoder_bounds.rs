// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "default_features")]

use num_bigint::BigUint;
use sequent_core::ballot_codec::{multi_ballot::BallotChoices, vec};
use sequent_core::mixed_radix;

#[test]
fn malformed_plaintext_length_does_not_panic() {
    for length in [30, 31, 255] {
        let mut input = [0; 30];
        input[0] = length;
        let result =
            std::panic::catch_unwind(|| vec::decode_array_to_vec(&input));
        assert!(result.is_ok(), "malformed length {length} must not panic");
        assert!(result.unwrap().is_err());
    }
}

#[test]
fn fixed_contest_capacity_overflow_does_not_panic() {
    let result = std::panic::catch_unwind(|| {
        BallotChoices::decode_mixed_radix(&[2], &BigUint::from(4u8))
    });
    assert!(result.is_ok(), "an oversized encoded value must not panic");
    assert!(result.unwrap().is_err());
}

#[test]
fn zero_radix_does_not_panic() {
    let result = std::panic::catch_unwind(|| {
        BallotChoices::decode_mixed_radix(&[0], &BigUint::from(1u8))
    });
    assert!(result.is_ok(), "a zero radix must not panic");
    assert!(result.unwrap().is_err());
    let result = std::panic::catch_unwind(|| {
        mixed_radix::decode(&[0], &BigUint::from(1u8), 256)
    });
    assert!(result.is_ok(), "a zero write-in radix must not panic");
    assert!(result.unwrap().is_err());
}

#[test]
fn zero_fallback_radix_does_not_panic() {
    let result = std::panic::catch_unwind(|| {
        mixed_radix::decode(&[2], &BigUint::from(2u8), 0)
    });
    assert!(result.is_ok(), "a zero fallback radix must not panic");
    assert!(result.unwrap().is_err());
}

#[test]
fn oversized_plaintext_errors_do_not_include_its_contents() {
    let input = vec![193; 31];
    let result = vec::encode_vec_to_array(&input).unwrap_err();
    assert!(!result.contains(&format!("{input:?}")));
}

#[test]
fn valid_length_prefixes_roundtrip_and_unframed_arrays_keep_all_thirty_bytes() {
    for length in 0..=29u8 {
        let input: Vec<u8> = (0..length).collect();
        let encoded = vec::encode_vec_to_array(&input).unwrap();
        assert_eq!(vec::decode_array_to_vec(&encoded).unwrap(), input);
    }
    let full = [211; 30];
    assert_eq!(vec::vec_to_30_array(&full).unwrap(), full);
}

#[test]
fn finite_radix_one_positions_remain_valid() {
    assert_eq!(
        BallotChoices::decode_mixed_radix(&[1, 2], &BigUint::from(1u8))
            .unwrap(),
        [0, 1]
    );
    assert_eq!(
        mixed_radix::decode(&[1, 2], &BigUint::from(1u8), 256).unwrap(),
        [0, 1]
    );
    // The fallback is irrelevant while the value fits the provided positions.
    assert_eq!(
        mixed_radix::decode(&[2], &BigUint::from(0u8), 1).unwrap(),
        [0]
    );
}

#[test]
fn radix_one_fallback_is_rejected_when_more_digits_are_needed() {
    assert!(mixed_radix::decode(&[2], &BigUint::from(2u8), 1).is_err());
    assert!(
        BallotChoices::decode_mixed_radix(&[], &BigUint::from(1u8)).is_err()
    );
}
