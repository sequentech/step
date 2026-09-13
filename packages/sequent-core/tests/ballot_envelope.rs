// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The fixed ballot envelope has 29 payload bytes and one length byte.
//! Check the public decoder boundary with values that cannot come from our
//! encoder: malformed or independently produced ballots still need a safe error.

#![cfg(feature = "default_features")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(private_interfaces, private_bounds, unnameable_types)]
#![deny(rustdoc::missing_crate_level_docs, rustdoc::broken_intra_doc_links)]
#![deny(
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::unwrap_used,
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::indexing_slicing,
    clippy::future_not_send,
    clippy::arithmetic_side_effects,
    clippy::suspicious,
    clippy::complexity,
    clippy::style,
    clippy::perf,
    clippy::pedantic
)]

use support::TestResult;

mod support;

use sequent_core::ballot::BallotStyle;
use sequent_core::ballot_codec::multi_ballot::BallotChoices;
use sequent_core::ballot_codec::{
    decode_array_to_vec, encode_vec_to_array, vec_to_30_array, PlaintextCodec,
};
use sequent_core::fixtures::ballot_codec::get_test_contest;
use serde_json::json;

#[test]
fn contest_decoders_reject_lengths_larger_than_the_payload() {
    let contest = get_test_contest();

    // Exercise every invalid byte, including the off-by-one value 30 and 255.
    // A panic is a test failure; the caller must receive an ordinary error.
    for length in 30..=u8::MAX {
        let mut envelope = [0; 30];
        envelope[0] = length;

        assert!(contest.decode_plaintext_contest(&envelope).is_err());
        assert!(contest
            .decode_plaintext_contest_to_biguint(&envelope)
            .is_err());
    }
}

#[test]
fn multi_contest_decoder_propagates_the_envelope_error() -> TestResult {
    // No contest processing should happen when the outer envelope is invalid.
    let style: BallotStyle = serde_json::from_value(json!({
        "id": "test-style",
        "tenant_id": "test-tenant",
        "election_event_id": "test-event",
        "election_id": "test-election",
        "area_id": "test-area",
        "contests": []
    }))?;
    let mut envelope = [0; 30];
    envelope[0] = u8::MAX;

    assert_eq!(
        BallotChoices::decode_from_30_bytes(&envelope, &style)
            .err()
            .ok_or("expected the invalid input to be rejected")?,
        "Invalid plaintext length 255, maximum is 29"
    );
    Ok(())
}

#[test]
fn envelope_encoding_matches_an_independent_byte_layout() -> TestResult {
    let payload = vec![0x01, 0x80, 0xff];
    let mut expected = [0; 30];
    expected[..4].copy_from_slice(&[3, 0x01, 0x80, 0xff]);

    assert_eq!(encode_vec_to_array(&payload)?, expected);
    assert_eq!(decode_array_to_vec(&expected)?, payload);
    Ok(())
}

#[test]
fn envelope_decoder_checks_every_possible_length_prefix() -> TestResult {
    for length in 0..=u8::MAX {
        let mut envelope = [0xab; 30];
        envelope[0] = length;
        let decoded = decode_array_to_vec(&envelope);

        if length <= 29 {
            assert_eq!(decoded?, vec![0xab; usize::from(length)]);
        } else {
            assert_eq!(
                decoded
                    .err()
                    .ok_or("expected the invalid input to be rejected")?,
                format!("Invalid plaintext length {length}, maximum is 29")
            );
        }
    }
    Ok(())
}

#[test]
fn envelope_encoder_accepts_its_boundary_and_rejects_one_more_byte(
) -> TestResult {
    assert_eq!(encode_vec_to_array(&vec![])?, [0; 30]);

    let mut expected = [0xab; 30];
    expected[0] = 29;
    assert_eq!(encode_vec_to_array(&vec![0xab; 29])?, expected);
    assert!(encode_vec_to_array(&vec![0xab; 30]).is_err());
    Ok(())
}

#[test]
fn unprefixed_array_keeps_all_thirty_payload_bytes() -> TestResult {
    // This helper has a different contract: no byte is reserved for a length.
    assert_eq!(vec_to_30_array(&vec![])?, [0; 30]);
    assert_eq!(vec_to_30_array(&vec![0xab; 30])?, [0xab; 30]);
    assert!(vec_to_30_array(&vec![0xab; 31]).is_err());
    Ok(())
}
