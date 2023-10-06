// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::*;
use crate::plaintext::*;

pub trait PlaintextCodec {
    fn encode_plaintext_question(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<[u8; 30], String>;
    fn decode_plaintext_question(
        &self,
        code: &[u8; 30],
    ) -> Result<DecodedVoteContest, String>;

    fn encode_plaintext_question_to_bytes(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<Vec<u8>, String>;
    fn decode_plaintext_question_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<DecodedVoteContest, String>;
}

impl PlaintextCodec for Question {
    fn encode_plaintext_question(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<[u8; 30], String> {
        let plaintext_bytes_vec =
            self.encode_plaintext_question_to_bytes(plaintext)?;
        encode_vec_to_array(&plaintext_bytes_vec)
    }

    fn decode_plaintext_question(
        &self,
        code: &[u8; 30],
    ) -> Result<DecodedVoteContest, String> {
        let plaintext_bytes = decode_array_to_vec(code);

        self.decode_plaintext_question_from_bytes(&plaintext_bytes)
    }

    fn encode_plaintext_question_to_bytes(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<Vec<u8>, String> {
        let bigint = self.encode_plaintext_question_bigint(plaintext)?;
        encode_bigint_to_bytes(&bigint)
    }

    fn decode_plaintext_question_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<DecodedVoteContest, String> {
        let bigint = decode_bigint_from_bytes(&bytes)?;
        self.decode_plaintext_question_bigint(&bigint)
    }
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::*;
    use std::cmp;

    #[test]
    fn test_encoding_plaintext() {
        let decoded_question = get_test_decoded_vote_question();
        let question = get_test_question();
        let encoded_bigint = question
            .encode_plaintext_question_bigint(&decoded_question)
            .unwrap(); // test
        let encoded_plaintext = question
            .encode_plaintext_question(&decoded_question)
            .unwrap();

        let plaintext_bytes = decode_array_to_vec(&encoded_plaintext); // test
        let decoded_bigint =
            decode_bigint_from_bytes(&plaintext_bytes).unwrap(); // test

        let decoded_plaintext = question
            .decode_plaintext_question(&encoded_plaintext)
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
            normalize_vote_question(
                &decoded_question,
                question.tally_type.as_str()
            ),
            normalize_vote_question(
                &decoded_plaintext,
                question.tally_type.as_str()
            )
        );
    }

    #[test]
    fn test_question_encode_plaintext() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);

            let encoded_ballot = fixture
                .question
                .encode_plaintext_question(&fixture.plaintext);
            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("question_encode_plaintext").cloned()
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
    fn test_question_decode_plaintext() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);

            let decoded_ballot = &fixture
                .question
                .decode_plaintext_question(&fixture.encoded_ballot)
                .unwrap();
            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("question_decode_plaintext").cloned()
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
                    &fixture.plaintext.invalid_errors
                );
            }
            if expected_error.is_none()
                || !expected_error
                    .unwrap()
                    .contains(&"decode_choices".to_string())
            {
                assert_eq!(
                    normalize_vote_question(
                        &decoded_ballot,
                        fixture.question.tally_type.as_str()
                    ),
                    normalize_vote_question(
                        &fixture.plaintext,
                        fixture.question.tally_type.as_str()
                    )
                );
            }
        }
    }
}
