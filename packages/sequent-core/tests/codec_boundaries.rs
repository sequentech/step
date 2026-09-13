// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Decode independently written mixed-radix vectors, including values that the
//! encoder refuses to produce. A round trip alone cannot test malformed ballots.

#![cfg(feature = "default_features")]

use num_bigint::BigUint;
use sequent_core::ballot::*;
use sequent_core::ballot_codec::{
    multi_ballot::*, RawBallotCodec, RawBallotContest,
};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::CountingAlgType;
use serde_json::json;

fn contest() -> Contest {
    Contest {
        id: "council".into(),
        max_votes: 2,
        min_votes: 0,
        candidates: ["a", "b", "c"]
            .into_iter()
            .map(|id| Candidate {
                id: id.into(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn style(contests: &[Contest]) -> BallotStyle {
    serde_json::from_value(json!({"id": "style", "tenant_id": "north", "election_event_id": "mayor",
        "election_id": "city", "area_id": "district", "contests": contests})).unwrap()
}

fn choices(ids: &[&str]) -> BallotChoices {
    BallotChoices::new(
        false,
        false,
        vec![ContestChoices::new(
            "council".into(),
            ids.iter()
                .map(|id| ContestChoice {
                    candidate_id: (*id).into(),
                    selected: 0,
                })
                .collect(),
            false,
        )],
        CountingAlgType::PluralityAtLarge,
    )
}

#[test]
fn single_contest_base_and_integer_decoding_reject_duplicate_markers() {
    use sequent_core::ballot_codec::{BasesCodec, BigUIntCodec};
    let mut config = contest();
    assert_eq!(config.get_bases().unwrap(), vec![2, 2, 2, 2]);
    assert_eq!(
        config
            .bigint_to_raw_ballot(&BigUint::from(0_u8))
            .unwrap()
            .choices,
        vec![0, 0, 0, 0]
    );
    for candidate in &mut config.candidates[..2] {
        candidate.presentation = Some(CandidatePresentation {
            is_explicit_invalid: Some(true),
            ..Default::default()
        });
    }
    let expected = "errors.configuration.multipleExplicitInvalidCandidates";
    assert_eq!(config.get_bases().unwrap_err().to_string(), expected);
    assert_eq!(
        config
            .bigint_to_raw_ballot(&BigUint::from(0_u8))
            .unwrap_err(),
        expected
    );
}

#[test]
fn expanding_decoded_choices_requires_an_entry_for_each_votable_contest() {
    use sequent_core::plaintext::map_decoded_ballot_choices_to_decoded_contests;
    let mut contests = vec![contest()];
    let mut decoded = DecodedBallotChoices {
        is_explicit_invalid: false,
        is_blank_ballot: false,
        serial_number: None,
        choices: vec![DecodedContestChoices::new(
            "council".into(),
            vec![],
            false,
            vec![],
            vec![],
        )],
    };
    let valid = map_decoded_ballot_choices_to_decoded_contests(
        decoded.clone(),
        &contests,
    )
    .unwrap();
    assert_eq!(
        valid[0]
            .choices
            .iter()
            .map(|choice| (choice.id.as_str(), choice.selected))
            .collect::<Vec<_>>(),
        [("a", -1), ("b", -1), ("c", -1)]
    );
    decoded.choices.clear();
    assert_eq!(
        map_decoded_ballot_choices_to_decoded_contests(
            decoded.clone(),
            &contests
        )
        .unwrap_err(),
        "Can't find contest with id council on ballot style"
    );
    contests[0].is_acclaimed = Some(true);
    assert!(
        map_decoded_ballot_choices_to_decoded_contests(decoded, &contests)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn independent_mixed_radix_vector_preserves_selected_candidates_and_serial_numbers(
) {
    let contests = vec![contest()];
    let config = style(&contests);
    // [invalid=0, candidate a=1, candidate c=3] in bases [2,4,4]:
    // 0 + 1*2 + 3*(2*4) = 26. This expected value does not use our encoder.
    let encoded = choices(&["a", "c"]).encode_to_bigint(&config).unwrap();
    assert_eq!(encoded, BigUint::from(26u32));
    let raw = BallotChoices::bigint_to_raw_ballot(
        &encoded,
        &contests,
        false,
        false,
        MultiContestEncodingMode::LEGACY,
    )
    .unwrap();
    assert_eq!(raw.bases, vec![2, 4, 4]);
    assert_eq!(raw.choices, vec![0, 1, 3]);
    let mut serial = 7;
    let decoded = BallotChoices::decode_from_bigint(
        &encoded,
        &contests,
        false,
        false,
        MultiContestEncodingMode::LEGACY,
        Some(&mut serial),
    )
    .unwrap();
    assert_eq!(decoded.serial_number.as_deref(), Some("000000007"));
    assert_eq!(serial, 8);
    let mut ids: Vec<_> = decoded.choices[0]
        .choices
        .iter()
        .map(|choice| choice.0.as_str())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["a", "c"]);
}

#[test]
fn malformed_raw_choice_counts_and_candidate_positions_are_rejected() {
    let contests = vec![contest()];
    for raw in [
        RawBallotContest::new(vec![2, 4, 4], vec![]),
        RawBallotContest::new(vec![2, 4, 4], vec![0, 1]),
        RawBallotContest::new(vec![2, 4, 4], vec![0, 1, 2, 3]),
        RawBallotContest::new(vec![2, 4, 4], vec![0, 4, 0]),
    ] {
        assert!(BallotChoices::decode(
            &raw,
            &contests,
            false,
            false,
            MultiContestEncodingMode::LEGACY,
            None
        )
        .is_err());
    }
    let config = style(&contests);
    assert!(choices(&["unknown"]).encode_to_bigint(&config).is_err());
    assert!(choices(&["a", "a"]).encode_to_bigint(&config).is_err());
}

#[test]
fn duplicate_raw_selections_cannot_count_as_two_different_candidates() {
    let contests = vec![contest()];
    let duplicate = RawBallotContest::new(vec![2, 4, 4], vec![0, 1, 1]);
    assert!(
        BallotChoices::decode(
            &duplicate,
            &contests,
            false,
            false,
            MultiContestEncodingMode::LEGACY,
            None
        )
        .is_err(),
        "the decoder must reject duplicates that the encoder already forbids"
    );
}

#[test]
fn exhausted_serial_numbers_fail_without_wrapping_back_to_zero() {
    let mut serial = u32::MAX;
    let result = BallotChoices::decode_from_bigint(
        &BigUint::from(26u32),
        &vec![contest()],
        false,
        false,
        MultiContestEncodingMode::LEGACY,
        Some(&mut serial),
    );
    assert!(result.is_err());
    assert_eq!(serial, u32::MAX);
}

#[test]
fn mixed_radix_values_cannot_overrun_the_declared_ballot_slots() {
    let bases = vec![2, 4, 4];
    // Three slots admit exactly 2*4*4 = 32 values. Check the boundary
    // directly, without using the encoder to manufacture the input.
    assert_eq!(
        BallotChoices::decode_mixed_radix(&bases, &BigUint::from(31u32))
            .unwrap(),
        vec![1, 3, 3]
    );
    for value in [32u32, 33, 255] {
        assert!(BallotChoices::decode_mixed_radix(
            &bases,
            &BigUint::from(value)
        )
        .is_err());
    }
    assert_eq!(
        BallotChoices::decode_mixed_radix(&vec![], &BigUint::from(0u32))
            .unwrap(),
        Vec::<u64>::new()
    );
    assert!(
        BallotChoices::decode_mixed_radix(&vec![], &BigUint::from(1u32))
            .is_err()
    );
}

#[test]
fn oversized_ballot_payloads_fail_without_consuming_a_serial_number() {
    let contests = vec![contest()];
    let config = style(&contests);
    // This has a valid one-byte envelope but exceeds the [2,4,4] layout.
    // The envelope parser alone cannot distinguish it from a valid ballot.
    let mut envelope = [0u8; 30];
    envelope[0] = 1;
    envelope[1] = 26;
    let valid =
        BallotChoices::decode_from_30_bytes(&envelope, &config).unwrap();
    assert_eq!(valid.choices[0].choices.len(), 2);
    envelope[1] = 32;
    assert!(BallotChoices::decode_from_30_bytes(&envelope, &config).is_err());
    let mut serial = 42;
    assert!(BallotChoices::decode_from_bigint(
        &BigUint::from(32u32),
        &contests,
        false,
        false,
        MultiContestEncodingMode::LEGACY,
        Some(&mut serial),
    )
    .is_err());
    assert_eq!(serial, 42, "rejected input must not consume a ballot id");
}

#[test]
fn zero_radices_are_rejected_even_when_the_value_needs_only_padding() {
    for bases in [vec![0], vec![2, 0]] {
        for value in [0u32, 1, 2] {
            assert!(BallotChoices::decode_mixed_radix(
                &bases,
                &BigUint::from(value)
            )
            .is_err());
        }
    }
    // A unit radix is legitimate: its only digit is zero, so a later
    // non-unit slot must still receive the remaining value.
    assert_eq!(
        BallotChoices::decode_mixed_radix(&vec![1, 2], &BigUint::from(1u32))
            .unwrap(),
        vec![0, 1]
    );
}

#[test]
fn unsupported_or_mixed_counting_algorithms_are_not_encoded_as_plurality() {
    let mut ranked = contest();
    ranked.id = "ranked".into();
    ranked.counting_algorithm = Some(CountingAlgType::InstantRunoff);
    assert!(MultiBallotCodecContext::new(
        &[ranked.clone()],
        false,
        false,
        MultiContestEncodingMode::LEGACY
    )
    .is_err());
    assert!(style(&[contest(), ranked])
        .get_counting_algorithm()
        .is_err());
}

#[test]
fn out_of_range_signed_ranks_fail_before_incrementing_to_an_unsigned_choice() {
    let mut ranked = contest();
    ranked.counting_algorithm = Some(CountingAlgType::InstantRunoff);
    let plaintext = DecodedVoteContest {
        contest_id: "council".into(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: vec![DecodedVoteChoice {
            id: "a".into(),
            selected: i64::MAX,
            write_in_text: None,
        }],
    };
    assert!(ranked.encode_to_raw_ballot(&plaintext).is_err());
}

#[test]
fn malformed_write_in_bytes_report_the_candidate_without_losing_other_choices()
{
    let mut contest = contest();
    contest.candidates.truncate(2);
    contest.candidates[1].presentation = Some(CandidatePresentation {
        is_write_in: Some(true),
        ..Default::default()
    });
    contest.presentation = Some(ContestPresentation {
        allow_writeins: Some(true),
        base32_writeins: Some(false),
        ..Default::default()
    });
    // [invalid, candidate a, write-in b, text..., terminator]. Raw input is
    // deliberately independent of the encoder, which cannot create bad UTF-8.
    for (text, expected_error) in [
        (vec![65, 0], None),
        (
            vec![300, 0],
            Some("errors.encoding.writeInChoiceOutOfRange"),
        ),
        (vec![255, 0], Some("errors.encoding.bytesToUtf8Conversion")),
        (vec![65], Some("errors.encoding.writeInNotEndInZero")),
    ] {
        let mut choices = vec![0, 1, 1];
        choices.extend(text);
        let raw = RawBallotContest::new(vec![2, 2, 2], choices);
        let decoded = contest.decode_from_raw_ballot(&raw).unwrap();
        assert_eq!(decoded.choices[0].id, "a");
        assert_eq!(decoded.choices[0].selected, 0);
        match expected_error {
            Some(message) => assert!(decoded
                .invalid_errors
                .iter()
                .any(|error| error.message.as_deref() == Some(message)
                    && error.candidate_id.as_deref() == Some("b"))),
            None => {
                assert!(decoded.invalid_errors.is_empty());
                assert_eq!(
                    decoded.choices[1].write_in_text.as_deref(),
                    Some("A")
                );
            }
        }
    }
}
