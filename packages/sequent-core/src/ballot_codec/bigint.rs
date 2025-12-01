// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::RawBallotContest;
use crate::ballot_codec::*;
use crate::mixed_radix::{decode, encode};
use crate::plaintext::*;
use crate::services::error_checker::check_contest;
use num_bigint::BigUint;

pub fn encode_bigint_to_bytes(b: &BigUint) -> Result<Vec<u8>, String> {
    Ok(b.to_radix_le(256))
}
pub fn decode_bigint_from_bytes(b: &[u8]) -> Result<BigUint, String> {
    BigUint::from_radix_le(b, 256)
        .ok_or(format!("Conversion failed for bytes {:?}", b))
}

pub trait BigUIntCodec {
    fn encode_plaintext_contest_bigint(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<BigUint, String>;

    fn decode_plaintext_contest_bigint(
        &self,
        bigint: &BigUint,
    ) -> Result<DecodedVoteContest, String>;

    fn bigint_to_raw_ballot(
        &self,
        bigint: &BigUint,
    ) -> Result<RawBallotContest, String>;

    fn available_write_in_characters(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<i32, String>;
}

fn remove_character(raw_ballot: &RawBallotContest) -> RawBallotContest {
    let mut bases = raw_ballot.bases.clone();
    let mut choices = raw_ballot.choices.clone();
    let mut i = choices.len() - 1;
    while 0 == choices[i] {
        i -= 1;
    }
    choices.remove(i);
    bases.remove(i);
    RawBallotContest {
        bases: bases,
        choices: choices,
    }
}

fn add_character(raw_ballot: &RawBallotContest) -> RawBallotContest {
    let mut bases = raw_ballot.bases.clone();
    let mut choices = raw_ballot.choices.clone();
    let mut i = choices.len() - 1;
    while 0 == choices[i] && i > 0 {
        i -= 1;
    }
    choices.insert(i, bases[i] - 1);
    bases.insert(i, bases[i]);
    RawBallotContest {
        bases: bases,
        choices: choices,
    }
}

impl BigUIntCodec for Contest {
    fn available_write_in_characters(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<i32, String> {
        let available_chars_estimate =
            self.available_write_in_characters_estimate(&plaintext)?;
        let mut raw_ballot = self.encode_to_raw_ballot(plaintext)?;
        let mut bigint = encode(&raw_ballot.choices, &raw_ballot.bases)?;
        let mut bytes_vec = encode_bigint_to_bytes(&bigint)?;

        if bytes_vec.len() <= 29 {
            if available_chars_estimate > 10 {
                Ok(available_chars_estimate)
            } else {
                let mut count = 0;
                while bytes_vec.len() <= 29 {
                    count += 1;
                    raw_ballot = add_character(&raw_ballot);
                    bigint = encode(&raw_ballot.choices, &raw_ballot.bases)?;
                    bytes_vec = encode_bigint_to_bytes(&bigint)?;
                }
                Ok(count - 1)
            }
        } else {
            if available_chars_estimate < -10 {
                Ok(available_chars_estimate)
            } else {
                let mut count = 0;
                while bytes_vec.len() > 29 {
                    count += 1;
                    raw_ballot = remove_character(&raw_ballot);
                    bigint = encode(&raw_ballot.choices, &raw_ballot.bases)?;
                    bytes_vec = encode_bigint_to_bytes(&bigint)?;
                }
                Ok(-count)
            }
        }
    }

    fn encode_plaintext_contest_bigint(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<BigUint, String> {
        let raw_ballot = self.encode_to_raw_ballot(plaintext)?;
        encode(&raw_ballot.choices, &raw_ballot.bases)
    }

    fn bigint_to_raw_ballot(
        &self,
        bigint: &BigUint,
    ) -> Result<RawBallotContest, String> {
        let mut bases = self.get_bases().map_err(|e| e.to_string())?;
        let last_base = self.get_char_map().base();
        let choices = decode(&bases, &bigint, last_base)?;

        while bases.len() < choices.len() {
            bases.push(last_base);
        }

        Ok(RawBallotContest { bases, choices })
    }

    fn decode_plaintext_contest_bigint(
        &self,
        bigint: &BigUint,
    ) -> Result<DecodedVoteContest, String> {
        let raw_ballot = self.bigint_to_raw_ballot(&bigint)?;

        let decoded_base = self.decode_from_raw_ballot(&raw_ballot)?;
        let with_more_errors = check_contest(&self, &decoded_base);
        Ok(with_more_errors)
    }
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::*;
    use crate::util::normalize_vote::normalize_vote_contest;
    use std::cmp;

    #[test]
    fn test_encoding_plaintext_bigint() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);
            let encoded_bigint = fixture
                .contest
                .encode_plaintext_contest_bigint(&fixture.plaintext);
            let decoded_plaintext = encoded_bigint.clone().map(|value| {
                fixture
                    .contest
                    .decode_plaintext_contest_bigint(&value)
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
                    normalize_vote_contest(
                        &fixture.plaintext,
                        fixture.contest.get_counting_algorithm(),
                        false,
                        &vec![]
                    )
                    .choices,
                    normalize_vote_contest(
                        &decoded_plaintext
                            .expect("Expected value but got error"),
                        fixture.contest.get_counting_algorithm(),
                        false,
                        &vec![]
                    )
                    .choices
                );
            }
        }
    }

    #[test]
    fn test_available_write_in_characters() {
        let ballot_style = get_writein_ballot_style();
        let contest = ballot_style.contests[0].clone();
        for n in -8..8 {
            let plaintext = get_too_long_writein_plaintext(n);
            let available_chars =
                contest.available_write_in_characters(&plaintext).unwrap();
            assert_eq!(available_chars as i64, -n);
        }
    }
}
