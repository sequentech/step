// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::RawBallotQuestion;
use crate::ballot_codec::*;
use crate::mixed_radix::{decode, encode};
use crate::plaintext::*;
use num_bigint::BigUint;
use num_traits::{Num, Zero};
use std::cmp::max;

pub fn encode_bigint_to_bytes(b: &BigUint) -> Result<Vec<u8>, String> {
    Ok(b.to_radix_le(256))
}
pub fn decode_bigint_from_bytes(b: &[u8]) -> Result<BigUint, String> {
    BigUint::from_radix_le(b, 256)
        .ok_or(format!("Conversion failed for bytes {:?}", b))
}

const TWO_OVER_232: &str =
    "6901746346790563787434755862277025452451108972170386555162524223799296";

pub trait BigUIntCodec {
    fn encode_plaintext_question_bigint(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<BigUint, String>;
    fn decode_plaintext_question_bigint(
        &self,
        bigint: &BigUint,
    ) -> Result<DecodedVoteContest, String>;

    fn bigint_to_raw_ballot(
        &self,
        bigint: &BigUint,
    ) -> Result<RawBallotQuestion, String>;

    fn available_write_in_characters(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<i32, String>;
}

impl BigUIntCodec for Question {
    fn available_write_in_characters(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<i32, String> {
        let mut raw_ballot = self.encode_to_raw_ballot(plaintext)?;
        let mut bigint = encode(&raw_ballot.choices, &raw_ballot.bases)?;
        let mut bytes_vec = encode_bigint_to_bytes(&bigint)?;

        let char_map = self.get_char_map();
        let base = char_map.base();

        if bytes_vec.len() < 29 {
            let available_chars_estimate =
                self.available_write_in_characters_estimate(&plaintext)?;
            let mut count = max(available_chars_estimate - 4, 0);
            for n in 0..count {
                raw_ballot.bases.push(base);
                raw_ballot.choices.push(base - 1);
            }
            bigint = encode(&raw_ballot.choices, &raw_ballot.bases)?;
            bytes_vec = encode_bigint_to_bytes(&bigint)?;
            while bytes_vec.len() < 29 {
                count += 1;
                raw_ballot.bases.push(base);
                raw_ballot.choices.push(base - 1);
                bigint = encode(&raw_ballot.choices, &raw_ballot.bases)?;
                bytes_vec = encode_bigint_to_bytes(&bigint)?;
            }
            Ok(count)
        } else {
            let mut count = 0;
            while bytes_vec.len() > 29 {
                count += 1;
                raw_ballot.bases.pop();
                raw_ballot.choices.pop();
                bigint = encode(&raw_ballot.choices, &raw_ballot.bases)?;
                bytes_vec = encode_bigint_to_bytes(&bigint)?;
            }
            Ok(-count)
        }
    }
    /*
    fn available_write_in_characters(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<i32, String> {
        use std::ops::Mul;
        let max = BigUint::from_str_radix(TWO_OVER_232, 10).unwrap();
        let char_map = self.get_char_map();
        let base = BigUint::from(char_map.base());
        let zero: BigUint = Zero::zero();

        let encoded_int = self.encode_plaintext_question_bigint(&plaintext)?;

        let base2 = base.pow(3);
        if encoded_int > max {
            let mut count: u32 = 1;
            let mut excess = base.pow(count);
            while max.clone().mul(excess) < encoded_int {
                count += 1;
                excess = base.pow(count);
            }
            Ok(-(count as i32))
        } else {
            let mut count = 1;
            let mut excess = base.pow(count);
            while encoded_int.clone().mul(excess) < max {
                count += 1;
                excess = base.pow(count);
            }
            Ok((count - 1) as i32)
        }
    }
    */
    fn encode_plaintext_question_bigint(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<BigUint, String> {
        let raw_ballot = self.encode_to_raw_ballot(plaintext)?;
        encode(&raw_ballot.choices, &raw_ballot.bases)
    }

    fn bigint_to_raw_ballot(
        &self,
        bigint: &BigUint,
    ) -> Result<RawBallotQuestion, String> {
        let mut bases = self.get_bases();
        let last_base = self.get_char_map().base();
        let choices = decode(&bases, &bigint, last_base)?;

        while bases.len() < choices.len() {
            bases.push(last_base);
        }

        Ok(RawBallotQuestion { bases, choices })
    }

    fn decode_plaintext_question_bigint(
        &self,
        bigint: &BigUint,
    ) -> Result<DecodedVoteContest, String> {
        let raw_ballot = self.bigint_to_raw_ballot(&bigint)?;

        self.decode_from_raw_ballot(&raw_ballot)
    }
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::*;
    use crate::util::normalize_vote_question;
    use std::cmp;

    #[test]
    fn test_encoding_plaintext_bigint() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);
            let encoded_bigint = fixture
                .question
                .encode_plaintext_question_bigint(&fixture.plaintext);
            let decoded_plaintext = encoded_bigint.clone().map(|value| {
                fixture
                    .question
                    .decode_plaintext_question_bigint(&value)
                    .unwrap()
            });

            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("encoding_plaintext_bigint").cloned()
                });
            if let Some(error) = expected_error {
                if error != *"disabled" {
                    assert_eq!(
                        error,
                        encoded_bigint.expect_err("Expected error!")
                    );
                }
            } else {
                println!(
                    "bigint10: {}",
                    encoded_bigint.clone().unwrap().to_str_radix(10)
                );
                assert_eq!(
                    fixture.encoded_ballot_bigint,
                    encoded_bigint
                        .expect("Expected value but got error")
                        .to_str_radix(10)
                );
                assert_eq!(
                    normalize_vote_question(
                        &fixture.plaintext,
                        fixture.question.tally_type.as_str(),
                        false
                    )
                    .choices,
                    normalize_vote_question(
                        &decoded_plaintext
                            .expect("Expected value but got error"),
                        fixture.question.tally_type.as_str(),
                        false
                    )
                    .choices
                );
            }
        }
    }

    #[test]
    fn test_available_write_in_characters() {
        let ballot_style = get_writein_ballot_style();
        let contest = ballot_style.configuration.questions[0].clone();
        let plaintext = get_too_long_writein_plaintext();
        let available_chars =
            contest.available_write_in_characters(&plaintext).unwrap();
        assert_eq!(available_chars, 0);
    }
}
