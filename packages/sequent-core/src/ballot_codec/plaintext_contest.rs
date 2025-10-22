// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::*;
use crate::plaintext::*;
use num_bigint::BigUint;

pub trait PlaintextCodec {
    fn encode_plaintext_contest(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<[u8; 30], String>;

    fn decode_plaintext_contest(
        &self,
        code: &[u8; 30],
    ) -> Result<DecodedVoteContest, String>;
    fn decode_plaintext_contest_to_biguint(
        &self,
        code: &[u8; 30],
    ) -> Result<BigUint, String>;

    fn encode_plaintext_contest_to_bytes(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<Vec<u8>, String>;

    fn decode_plaintext_contest_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<DecodedVoteContest, String>;
}

impl PlaintextCodec for Contest {
    fn encode_plaintext_contest(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<[u8; 30], String> {
        let plaintext_bytes_vec =
            self.encode_plaintext_contest_to_bytes(plaintext)?;
        encode_vec_to_array(&plaintext_bytes_vec)
    }

    fn decode_plaintext_contest(
        &self,
        code: &[u8; 30],
    ) -> Result<DecodedVoteContest, String> {
        let plaintext_bytes = decode_array_to_vec(code);

        self.decode_plaintext_contest_from_bytes(&plaintext_bytes)
    }

    fn decode_plaintext_contest_to_biguint(
        &self,
        code: &[u8; 30],
    ) -> Result<BigUint, String> {
        let plaintext_bytes = decode_array_to_vec(code);
        decode_bigint_from_bytes(&plaintext_bytes)
    }

    fn encode_plaintext_contest_to_bytes(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<Vec<u8>, String> {
        let bigint = self.encode_plaintext_contest_bigint(plaintext)?;
        encode_bigint_to_bytes(&bigint)
    }

    fn decode_plaintext_contest_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<DecodedVoteContest, String> {
        let bigint = decode_bigint_from_bytes(&bytes)?;
        self.decode_plaintext_contest_bigint(&bigint)
    }
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::*;
    use crate::util::normalize_vote::normalize_vote_contest;
    use std::cmp;

    #[test]
    fn test_encoding_plaintext() {
        let decoded_contest = get_test_decoded_vote_contest();
        let contest = get_test_contest();
        let invalid_candidate_ids = contest.get_invalid_candidate_ids();
        let encoded_bigint = contest
            .encode_plaintext_contest_bigint(&decoded_contest)
            .unwrap(); // test
        let encoded_plaintext =
            contest.encode_plaintext_contest(&decoded_contest).unwrap();

        let plaintext_bytes = decode_array_to_vec(&encoded_plaintext); // test
        let decoded_bigint =
            decode_bigint_from_bytes(&plaintext_bytes).unwrap(); // test

        let decoded_plaintext = contest
            .decode_plaintext_contest(&encoded_plaintext)
            .unwrap();

        println!(
            "encoded_plaintext {:?} encoded_bigint {}",
            encoded_plaintext,
            encoded_bigint.to_str_radix(10)
        );
        assert_eq!(
            encoded_bigint.to_str_radix(10),
            decoded_bigint.to_str_radix(10)
        );
        assert_eq!(
            normalize_vote_contest(
                &decoded_contest,
                contest.get_counting_algorithm(),
                false,
                &invalid_candidate_ids
            ),
            normalize_vote_contest(
                &decoded_plaintext,
                contest.get_counting_algorithm(),
                false,
                &invalid_candidate_ids
            )
        );
    }

    #[test]
    fn test_contest_encode_plaintext() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);

            let encoded_ballot =
                fixture.contest.encode_plaintext_contest(&fixture.plaintext);
            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("contest_encode_plaintext").cloned()
                });

            if let Some(error) = expected_error {
                if error != *"disabled" {
                    assert!(&encoded_ballot.is_err());
                    assert_eq!(
                        error,
                        encoded_ballot.expect_err("Expected error")
                    );
                }
            } else {
                assert_eq!(
                    fixture.encoded_ballot,
                    encoded_ballot.expect("Expected value")
                );
            }
        }
    }

    #[test]
    fn test_contest_decode_plaintext() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("\nfixture: {}", &fixture.title);

            let decoded_ballot = &fixture
                .contest
                .decode_plaintext_contest(&fixture.encoded_ballot)
                .unwrap();
            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("contest_decode_plaintext").cloned()
                });
            assert_eq!(
                &decoded_ballot.is_explicit_invalid,
                &fixture.plaintext.is_explicit_invalid
            );
            if expected_error.is_none()
                || !expected_error
                    .clone()
                    .unwrap()
                    .contains(&"invalid_errors".to_string())
            {
                assert_eq!(
                    &decoded_ballot.invalid_errors,
                    &fixture.plaintext.invalid_errors,
                    "&decoded_ballot.invalid_errors != &fixture.plaintext.invalid_errors"
                );
            }
            if expected_error.is_none()
                || !expected_error
                    .clone()
                    .unwrap()
                    .contains(&"invalid_alerts".to_string())
            {
                assert_eq!(
                    &decoded_ballot.invalid_alerts,
                    &fixture.plaintext.invalid_alerts,
                    "&decoded_ballot.invalid_alerts != &fixture.plaintext.invalid_alerts"
                );
            }
            if expected_error.is_none()
                || !expected_error
                    .unwrap()
                    .contains(&"decode_choices".to_string())
            {
                let invalid_candidate_ids =
                    fixture.contest.get_invalid_candidate_ids();
                assert_eq!(
                    normalize_vote_contest(
                        &decoded_ballot,
                        fixture.contest.get_counting_algorithm(),
                        false,
                        &invalid_candidate_ids
                    ),
                    normalize_vote_contest(
                        &fixture.plaintext,
                        fixture.contest.get_counting_algorithm(),
                        false,
                        &invalid_candidate_ids
                    )
                );
            }
        }
    }
}
