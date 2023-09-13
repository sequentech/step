// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::mixed_radix::{decode, encode};
use crate::plaintext::*;
use num_bigint::BigUint;
use num_traits::{Num, ToPrimitive};
use std::collections::HashMap;
use std::str;

#[derive(Debug, PartialEq, Eq)]
pub struct RawBallotQuestion {
    pub bases: Vec<u64>,
    pub choices: Vec<u64>,
}

pub trait BallotCodec {
    fn encode_plaintext_question(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<String, String>;
    fn decode_plaintext_question(
        &self,
        code: &str,
    ) -> Result<DecodedVoteQuestion, String>;

    fn encode_plaintext_question_to_bytes(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<Vec<u8>, String>;
    fn decode_plaintext_question_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<DecodedVoteQuestion, String>;

    // get bases (no write-ins)
    fn get_bases(&self) -> Vec<u64>;
    fn encode_to_raw_ballot(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<RawBallotQuestion, String>;
    fn decode_from_raw_ballot(
        &self,
        raw_ballot: &RawBallotQuestion,
    ) -> Result<DecodedVoteQuestion, String>;
}

fn encode_bigint_to_bytes(b: &BigUint) -> Result<Vec<u8>, String> {
    Ok(b.to_radix_le(256))
}
fn decode_bigint_from_bytes(b: &[u8]) -> Result<BigUint, String> {
    BigUint::from_radix_le(b, 256)
        .ok_or(format!("Conversion failed for bytes {:?}", b))
}

impl Question {
    pub(crate) fn get_char_map(&self) -> Box<dyn CharacterMap> {
        if self.base32_writeins() {
            Box::new(Base32Map)
        } else {
            Box::new(Utf8Map)
        }
    }

    fn encode_plaintext_question_bigint(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<BigUint, String> {
        let raw_ballot = self.encode_to_raw_ballot(plaintext)?;
        encode(&raw_ballot.choices, &raw_ballot.bases)
    }

    fn decode_plaintext_question_bigint(
        &self,
        bigint: &BigUint,
    ) -> Result<DecodedVoteQuestion, String> {
        let mut bases = self.get_bases();
        let last_base = self.get_char_map().base();
        let choices = decode(&bases, &bigint, last_base)?;

        while bases.len() < choices.len() {
            bases.push(last_base);
        }

        let raw_ballot = RawBallotQuestion { bases, choices };

        self.decode_from_raw_ballot(&raw_ballot)
    }
}

impl BallotCodec for Question {
    fn encode_plaintext_question(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<String, String> {
        let encoded_bigint = self.encode_plaintext_question_bigint(plaintext);

        encoded_bigint.map(|value| value.to_str_radix(10))
    }

    fn decode_plaintext_question(
        &self,
        code: &str,
    ) -> Result<DecodedVoteQuestion, String> {
        let encoded_bigint = BigUint::from_str_radix(code, 10)
            .map_err(|_| "Error parsing code".to_string())?;

        self.decode_plaintext_question_bigint(&encoded_bigint)
    }

    fn encode_plaintext_question_to_bytes(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<Vec<u8>, String> {
        let bigint = self.encode_plaintext_question_bigint(plaintext)?;
        encode_bigint_to_bytes(&bigint)
    }

    fn decode_plaintext_question_from_bytes(
        &self,
        bytes: &[u8],
    ) -> Result<DecodedVoteQuestion, String> {
        let bigint = decode_bigint_from_bytes(&bytes)?;
        self.decode_plaintext_question_bigint(&bigint)
    }

    fn encode_to_raw_ballot(
        &self,
        plaintext: &DecodedVoteQuestion,
    ) -> Result<RawBallotQuestion, String> {
        let mut bases = self.get_bases();
        let mut choices: Vec<u64> = vec![];

        let char_map = self.get_char_map();

        let answers_map = self
            .answers
            .iter()
            .map(|answer| (answer.id, answer))
            .collect::<HashMap<i64, &Answer>>();

        // sort answers by id
        let mut sorted_choices = plaintext.choices.clone();
        sorted_choices.sort_by_key(|q| q.id);

        // Separate the answers between:
        // - Invalid vote answer (if any)
        // - Write-ins (if any)
        // - Valid answers (normal answers + write-ins if any)
        let invalid_vote: u64 =
            if plaintext.is_explicit_invalid { 1 } else { 0 };
        choices.push(invalid_vote);

        for choice in sorted_choices.iter() {
            let answer = answers_map
                .get(&choice.id)
                .ok_or_else(|| "choice id is not a valid answer".to_string())?;
            if answer.is_explicit_invalid() {
                continue;
            }
            if self.tally_type == "plurality-at-large" {
                // We just flag if the candidate was selected or not with 1 for
                // selected and 0 otherwise
                choices.push(u64::from(choice.selected > -1));
            } else {
                // we add 1 because the counting starts with 1, as zero means
                // this answer was not voted / ranked
                let value =
                    (choice.selected + 1).to_u64().ok_or_else(|| {
                        "selected value must be positive or zero".to_string()
                    })?;
                choices.push(value);
            }
        }
        // Populate the bases and the raw_ballot values with the write-ins
        // if there's any. We will through each write-in (if any), and then
        // encode the write-in answer.text string with UTF-8 and use for
        // each byte a specific value with base 256 and end each write-in
        // with a \0 byte. Note that even write-ins.
        if self.allow_writeins() {
            for choice in sorted_choices.iter() {
                let answer = answers_map.get(&choice.id).ok_or_else(|| {
                    "choice id is not a valid answer".to_string()
                })?;
                let is_write_in = answer.is_write_in();
                if choice.write_in_text.is_none() && is_write_in {
                    // we don't do a bases.push_back(256) as this is done in
                    // getBases() to end it with a zero
                    choices.push(0);
                }
                if choice.write_in_text.is_some() && is_write_in {
                    let text = choice.write_in_text.clone().unwrap();
                    if text.is_empty() {
                        // we don't do a bases.push_back(256) as this is done in
                        // getBases() to end it with a zero
                        choices.push(0);
                    } else {
                        // MAPPER
                        let base = char_map.base();
                        let bytes = char_map.to_bytes(&text)?;
                        for byte in bytes {
                            choices.push(byte as u64);
                            bases.push(base);
                        }

                        // End it with a zero. we don't do a
                        // bases.push_back(256) as this is
                        // done in getBases()
                        choices.push(0);
                    }
                }
            }
        }

        Ok(RawBallotQuestion { bases, choices })
    }

    // Note: WIP
    fn decode_from_raw_ballot(
        &self,
        raw_ballot: &RawBallotQuestion,
    ) -> Result<DecodedVoteQuestion, String> {
        let choices = raw_ballot.choices.clone();
        let is_explicit_invalid: bool = !choices.is_empty() && (choices[0] > 0);
        let mut invalid_errors: Vec<InvalidPlaintextError> = vec![];
        let char_map = self.get_char_map();

        // 1. clone the question and reset the selections
        let mut sorted_answers = self.answers.clone();
        sorted_answers.sort_by_key(|q| q.id);

        // 1.2. Initialize selection
        let mut sorted_choices: Vec<DecodedVoteChoice> = vec![];

        // 2. sort & segment answers
        let valid_answers: Vec<&Answer> = self
            .answers
            .iter()
            .filter(|answer| !answer.is_explicit_invalid())
            .collect();
        let write_in_answers: Vec<&Answer> = self
            .answers
            .iter()
            .filter(|answer| answer.is_write_in())
            .collect();
        // 4. Do some verifications on the number of choices: Checking that the
        //    raw_ballot has as many choices as required
        if choices.len() < valid_answers.len() + 1 {
            // Invalid Ballot: Not enough choices to decode
            invalid_errors.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::EncodingError,
                answer_id: None,
                message: Some("errors.encoding.notEnoughChoices".to_string()),
                message_map: HashMap::new(),
            });
        }

        // 5. Populate the valid answers. We asume they are in the same order as
        // in    raw_ballot["choices"]
        // we add 1 to the index because raw_ballot.choice[0] is just the
        // invalidVoteFlag
        let mut index = 1usize;
        for answer in &valid_answers {
            if choices.len() <= index {
                break;
            }
            let choice_value = choices[index]
                .clone()
                .to_i64()
                .ok_or_else(|| "choice out of range".to_string())?;

            sorted_choices.push(DecodedVoteChoice {
                id: answer.id,
                selected: choice_value - 1,
                write_in_text: None,
            });

            index += 1;
        }
        // 6. Decode the write-in texts into UTF-8 and split by the \0
        // character,    finally the text for the write-ins.
        let mut write_in_index = index;
        for answer in &write_in_answers {
            if write_in_index >= choices.len() {
                break;
            }
            // collect the string bytes
            let mut write_in_bytes: Vec<u8> = vec![];

            while write_in_index < choices.len() && choices[write_in_index] != 0
            {
                let value_res = choices[write_in_index]
                    .to_u8()
                    .ok_or_else(|| "Write-in choice out of range".to_string());

                if let Ok(new_value) = value_res {
                    write_in_bytes.push(new_value);
                } else {
                    invalid_errors.push(InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::EncodingError,
                        answer_id: Some(answer.id),
                        message: Some(
                            "errors.encoding.writeInChoiceOutOfRange"
                                .to_string(),
                        ),
                        message_map: HashMap::from([(
                            "index".to_string(),
                            write_in_index.to_string(),
                        )]),
                    });
                }

                write_in_index += 1;
            }

            // check index is not out of bounds
            if write_in_index >= choices.len() {
                invalid_errors.push(InvalidPlaintextError {
                    error_type: InvalidPlaintextErrorType::EncodingError,
                    answer_id: Some(answer.id),
                    message: Some(
                        "errors.encoding.writeInNotEndInZero".to_string(),
                    ),
                    message_map: HashMap::new(),
                });
            }
            // skip the 0 character
            else if choices[write_in_index] == 0 {
                write_in_index += 1;
            }

            // MAPPER
            let write_in_str_res = char_map.to_string(&write_in_bytes);

            if write_in_str_res.is_err() {
                invalid_errors.push(InvalidPlaintextError {
                    error_type: InvalidPlaintextErrorType::EncodingError,
                    answer_id: Some(answer.id),
                    message: Some(
                        "errors.encoding.bytesToUtf8Conversion".to_string(),
                    ),
                    message_map: HashMap::from([(
                        "errorMessage".to_string(),
                        write_in_str_res.clone().unwrap_err(),
                    )]),
                });
            }

            let write_in_str = write_in_str_res.map(Some).unwrap_or(None);

            // add write_in to choice
            let n = sorted_choices
                .iter()
                .position(|choice| choice.id == answer.id)
                .unwrap();
            let mut choice = sorted_choices[n].clone();
            choice.write_in_text = write_in_str;
            sorted_choices[n] = choice;
        }

        if write_in_index < choices.len() {
            invalid_errors.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::EncodingError,
                answer_id: None,
                message: Some("errors.encoding.ballotTooLarge".to_string()),
                message_map: HashMap::new(),
            });
        }

        // explicit invalid error
        if is_explicit_invalid && !self.allow_explicit_invalid() {
            invalid_errors.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::Explicit,
                answer_id: None,
                message: Some("errors.explicit.notAllowed".to_string()),
                message_map: HashMap::new(),
            });
        }

        // implicit invalid errors
        let num_selected_answers = sorted_choices
            .iter()
            .filter(|choice| choice.selected > -1)
            .count();

        if num_selected_answers > usize::try_from(self.max).unwrap() {
            invalid_errors.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::Implicit,
                answer_id: None,
                message: Some("errors.implicit.selectedMax".to_string()),
                message_map: HashMap::from([
                    (
                        "numSelected".to_string(),
                        num_selected_answers.to_string(),
                    ),
                    ("max".to_string(), self.max.to_string()),
                ]),
            });
        } else if num_selected_answers < usize::try_from(self.min).unwrap() {
            invalid_errors.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::Implicit,
                answer_id: None,
                message: Some("errors.implicit.selectedMin".to_string()),
                message_map: HashMap::from([
                    (
                        "numSelected".to_string(),
                        num_selected_answers.to_string(),
                    ),
                    ("min".to_string(), self.min.to_string()),
                ]),
            });
        }

        Ok(DecodedVoteQuestion {
            is_explicit_invalid,
            invalid_errors,
            choices: sorted_choices,
        })
    }

    fn get_bases(&self) -> Vec<u64> {
        // Calculate the base for answers. It depends on the
        // `question.tally_type`:
        // - plurality-at-large: base 2 (value can be either 0 o 1)
        // - preferential (*bordas*): question.max + 1
        // - cummulative: question.extra_options.cumulative_number_of_checkboxes
        //   + 1
        let answer_base: u64 = match self.tally_type.as_str() {
            "plurality-at-large" => 2,
            "cumulative" => self.cumulative_number_of_checkboxes() + 1u64,
            _ => (self.max + 1i64).try_into().unwrap(),
        };

        let num_valid_answers: usize = self
            .answers
            .iter()
            .filter(|answer| !answer.is_explicit_invalid())
            .count();

        // Set the initial bases and raw ballot, populate bases using the valid
        // answers list
        let mut bases: Vec<u64> = vec![2];
        for _i in 0..num_valid_answers {
            bases.push(answer_base);
        }

        // Add bases for null terminators.
        if self.allow_writeins() {
            let char_map = self.get_char_map();
            let write_in_base = char_map.base();
            for question in self.answers.iter() {
                if question.is_write_in() {
                    bases.push(write_in_base);
                }
            }
        }
        bases
    }
}

pub(crate) trait CharacterMap {
    fn to_bytes(&self, s: &str) -> Result<Vec<u8>, String>;
    fn to_string(&self, bytes: &[u8]) -> Result<String, String>;
    fn base(&self) -> u64;
}

struct Utf8Map;
struct Base32Map;

impl CharacterMap for Utf8Map {
    fn to_bytes(&self, s: &str) -> Result<Vec<u8>, String> {
        Ok(s.as_bytes().to_vec())
    }
    fn to_string(&self, bytes: &[u8]) -> Result<String, String> {
        str::from_utf8(&bytes)
            .map_err(|e| format!("{}", e))
            .map(|s| s.to_string())
    }
    fn base(&self) -> u64 {
        256u64
    }
}

impl CharacterMap for Base32Map {
    fn to_bytes(&self, s: &str) -> Result<Vec<u8>, String> {
        s.to_uppercase()
            .chars()
            .map(|c| {
                TO_BYTE
                    .get(&c)
                    .ok_or(format!(
                        "Character '{}' cannot be mapped to byte",
                        c
                    ))
                    .copied()
            })
            .collect()
    }
    fn to_string(&self, bytes: &[u8]) -> Result<String, String> {
        let chars: Result<Vec<char>, String> = bytes
            .iter()
            .map(|b| {
                TO_CHAR
                    .get(&b)
                    .ok_or(format!("Byte '{}' cannot be mapped to char", b))
                    .copied()
            })
            .collect();

        Ok(String::from_iter(chars?))
    }
    fn base(&self) -> u64 {
        32u64
    }
}

use phf::phf_map;
static TO_BYTE: phf::Map<char, u8> = phf_map! {
    // 0 is reserved for null terminator
    'A' => 1u8,
    'B' => 2u8,
    'C' => 3u8,
    'D' => 4u8,
    'E' => 5u8,
    'F' => 6u8,
    'G' => 7u8,
    'H' => 8u8,
    'I' => 9u8,
    'J' => 10u8,
    'K' => 11u8,
    'L' => 12u8,
    'M' => 13u8,
    'N' => 14u8,
    'O' => 15u8,
    'P' => 16u8,
    'Q' => 17u8,
    'R' => 18u8,
    'S' => 19u8,
    'T' => 20u8,
    'U' => 21u8,
    'V' => 22u8,
    'W' => 23u8,
    'X' => 24u8,
    'Y' => 25u8,
    'Z' => 26u8,
    ' ' => 27u8,
    '(' => 28u8,
    ')' => 29u8,
    '.' => 30u8,
    ',' => 31u8,
};
static TO_CHAR: phf::Map<u8, char> = phf_map! {
    // 0 is reserved for null terminator
    1u8 => 'A',
    2u8 => 'B',
    3u8 => 'C',
    4u8 => 'D',
    5u8 => 'E',
    6u8 => 'F',
    7u8 => 'G',
    8u8 => 'H',
    9u8 => 'I',
    10u8 => 'J',
    11u8 => 'K',
    12u8 => 'L',
    13u8 => 'M',
    14u8 => 'N',
    15u8 => 'O',
    16u8 => 'P',
    17u8 => 'Q',
    18u8 => 'R',
    19u8 => 'S',
    20u8 => 'T',
    21u8 => 'U',
    22u8 => 'V',
    23u8 => 'W',
    24u8 => 'X',
    25u8 => 'Y',
    26u8 => 'Z',
    27u8 => ' ',
    28u8 => '(',
    29u8 => ')',
    30u8 => '.',
    31u8 => ',',
};

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::bases_fixture;
    use crate::fixtures::ballot_codec::get_fixtures;
    use rand::Rng;
    use std::cmp;

    #[test]
    fn test_question_bases() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);

            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("question_bases").cloned()
                });

            if expected_error.is_some() {
                assert_ne!(
                    &fixture.question.get_bases(),
                    &fixture.raw_ballot.bases
                );
            } else {
                assert_eq!(
                    &fixture.question.get_bases(),
                    &fixture.raw_ballot.bases
                );
            }
        }
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
    fn test_question_encode_to_raw_ballot() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);
            let raw_ballot =
                fixture.question.encode_to_raw_ballot(&fixture.plaintext);
            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("question_encode_to_raw_ballot").cloned()
                });

            if let Some(error) = expected_error {
                if error != *"disabled" {
                    assert_eq!(error, raw_ballot.expect_err("Expected error!"));
                }
            } else {
                assert_eq!(
                    fixture.raw_ballot,
                    raw_ballot.expect("Expected value but got error")
                );
            }
        }
    }

    #[test]
    fn test_question_encode_raw_ballot() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);
            let encoded_bigint =
                encode(&fixture.raw_ballot.choices, &fixture.raw_ballot.bases);
            let encoded_ballot =
                encoded_bigint.map(|value| value.to_str_radix(10));

            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("question_encode_raw_ballot").cloned()
                });

            if expected_error.is_some() {
                assert_eq!(
                    expected_error.unwrap(),
                    encoded_ballot.expect_err("Expected error!")
                );
            } else {
                assert_eq!(
                    &fixture.encoded_ballot,
                    &encoded_ballot.expect("Expected value")
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
                    decoded_ballot.choices.len(),
                    fixture.plaintext.choices.len()
                );
                for idx in 0..decoded_ballot.choices.len() {
                    let mut res_choice = decoded_ballot.choices[idx].clone();
                    let mut choice = fixture.plaintext.choices[idx].clone();
                    if choice.write_in_text.is_some() {
                        res_choice.write_in_text = res_choice
                            .write_in_text
                            .or_else(|| Some("".to_string()));
                    }
                    if fixture.question.tally_type == "plurality-at-large" {
                        choice.selected = cmp::min(choice.selected, 0);
                    }
                    assert_eq!(choice, res_choice);
                }
            }
        }
    }

    #[test]
    fn test_question_decode_raw_ballot() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);
            let decoded_ballot_res =
                fixture.question.decode_from_raw_ballot(&fixture.raw_ballot);
            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("question_decode_raw_ballot").cloned()
                });
            if expected_error.is_some() {
                decoded_ballot_res.expect_err("Expected error");
            } else {
                let decoded_ballot =
                    decoded_ballot_res.expect("Expected ballot but got error");
                for idx in 0..decoded_ballot.choices.len() {
                    assert_eq!(
                        decoded_ballot.choices[idx].id,
                        fixture.plaintext.choices[idx].id
                    );
                    assert_eq!(
                        decoded_ballot.choices[idx].write_in_text,
                        fixture.plaintext.choices[idx].write_in_text
                    );
                    if fixture.question.tally_type == "plurality-at-large" {
                        assert_eq!(
                            decoded_ballot.choices[idx].selected,
                            cmp::min(
                                fixture.plaintext.choices[idx].selected,
                                0
                            )
                        );
                    } else {
                        assert_eq!(
                            decoded_ballot.choices[idx].selected,
                            fixture.plaintext.choices[idx].selected
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_bases() {
        let fixtures = bases_fixture();
        for fixture in fixtures.iter() {
            let bases = fixture.question.get_bases();
            assert_eq!(bases, fixture.bases);
        }
    }

    #[test]
    fn test_character_maps() {
        let map = Base32Map;
        for key in TO_BYTE.keys() {
            let s = key.to_string();
            let forward = map.to_bytes(&s).unwrap();
            let backward = map.to_string(&forward).unwrap();

            assert_eq!(s, backward);
        }
        let map = Utf8Map;
        // arbitrary range
        for i in 0u32..1024u32 {
            let char = char::from_u32(i);
            if char.is_some() {
                let char = char.unwrap().to_string();
                let forward = map.to_bytes(&char).unwrap();
                let backward = map.to_string(&forward).unwrap();

                assert_eq!(char, backward);
            }
        }
    }

    #[test]
    fn test_write_in_base32() {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ )(.,";
        const MAX_LEN: usize = 40;
        let mut rng = rand::thread_rng();

        let writein: String = (0..MAX_LEN)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect();

        let ballot = DecodedVoteQuestion {
            is_explicit_invalid: false,
            invalid_errors: vec![],
            choices: vec![
                DecodedVoteChoice {
                    id: 0,
                    selected: 1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 1,
                    selected: 1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 2,
                    selected: 5,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 3,
                    selected: 3,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: 4,
                    selected: 1,
                    //                   123456789012345679012345678901234567890
                    // write_in_text:
                    // Some("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ".
                    // to_string()),
                    write_in_text: Some(writein.clone()),
                },
            ],
        };

        let mut question =
            crate::fixtures::ballot_codec::get_configurable_question(
                1,
                5,
                "plurality-at-large".to_string(),
                true,
                Some(vec![4]),
            );
        let result = question
            .encode_plaintext_question_to_bytes(&ballot)
            .unwrap();
        let bytes_large = result.len();

        let mut extra_options =
            question.extra_options.as_ref().unwrap().clone();
        extra_options.base32_writeins = Some(true);
        question.extra_options = Some(extra_options);

        let result = question
            .encode_plaintext_question_to_bytes(&ballot)
            .unwrap();
        let bytes_small = result.len();

        let result = question
            .decode_plaintext_question_from_bytes(&result)
            .unwrap();
        println!("************* {:?} ************", result);
        let back = result.choices[4].write_in_text.as_ref().unwrap();
        assert_eq!(*back, writein);
        assert!(bytes_small < 27);
        assert!(bytes_small < bytes_large);
    }
}
