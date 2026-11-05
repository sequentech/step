// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The fixed ballot envelope has 29 payload bytes and one length byte.
//! Check the public decoder boundary with values that cannot come from our
//! encoder: malformed or independently produced ballots still need a safe error.

#![cfg(feature = "default_features")]

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
fn multi_contest_decoder_propagates_the_envelope_error() {
    // No contest processing should happen when the outer envelope is invalid.
    let style: BallotStyle = serde_json::from_value(json!({
        "id": "test-style",
        "tenant_id": "test-tenant",
        "election_event_id": "test-event",
        "election_id": "test-election",
        "area_id": "test-area",
        "contests": []
    }))
    .unwrap();
    let mut envelope = [0; 30];
    envelope[0] = u8::MAX;

    assert_eq!(
        BallotChoices::decode_from_30_bytes(&envelope, &style).unwrap_err(),
        "Invalid plaintext length 255, maximum is 29"
    );
}

#[test]
fn envelope_encoding_matches_an_independent_byte_layout() {
    let payload = vec![0x01, 0x80, 0xff];
    let mut expected = [0; 30];
    expected[..4].copy_from_slice(&[3, 0x01, 0x80, 0xff]);

    assert_eq!(encode_vec_to_array(&payload).unwrap(), expected);
    assert_eq!(decode_array_to_vec(&expected).unwrap(), payload);
}

#[test]
fn envelope_decoder_checks_every_possible_length_prefix() {
    for length in 0..=u8::MAX {
        let mut envelope = [0xab; 30];
        envelope[0] = length;
        let decoded = decode_array_to_vec(&envelope);

        if length <= 29 {
            assert_eq!(decoded.unwrap(), vec![0xab; usize::from(length)]);
        } else {
            assert_eq!(
                decoded.unwrap_err(),
                format!("Invalid plaintext length {}, maximum is 29", length)
            );
        }
    }
}

#[test]
fn envelope_encoder_accepts_its_boundary_and_rejects_one_more_byte() {
    assert_eq!(encode_vec_to_array(&vec![]).unwrap(), [0; 30]);

    let mut expected = [0xab; 30];
    expected[0] = 29;
    assert_eq!(encode_vec_to_array(&vec![0xab; 29]).unwrap(), expected);
    assert!(encode_vec_to_array(&vec![0xab; 30]).is_err());
}

#[test]
fn unprefixed_array_keeps_all_thirty_payload_bytes() {
    // This helper has a different contract: no byte is reserved for a length.
    assert_eq!(vec_to_30_array(&vec![]).unwrap(), [0; 30]);
    assert_eq!(vec_to_30_array(&vec![0xab; 30]).unwrap(), [0xab; 30]);
    assert!(vec_to_30_array(&vec![0xab; 31]).is_err());
}
