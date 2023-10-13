// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::*;
use crate::hasura_types::Uuid;
use crate::plaintext::*;
use num_traits::ToPrimitive;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct RawBallotQuestion {
    pub bases: Vec<u64>,
    pub choices: Vec<u64>,
}

pub trait RawBallotCodec {
    fn encode_to_raw_ballot(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<RawBallotQuestion, String>;
    fn decode_from_raw_ballot(
        &self,
        raw_ballot: &RawBallotQuestion,
    ) -> Result<DecodedVoteContest, String>;

    fn available_write_in_characters_estimate(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<i32, String>;
}

impl RawBallotCodec for Question {
    fn available_write_in_characters_estimate(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<i32, String> {
        let raw_ballot = self.encode_to_raw_ballot(&plaintext)?;
        let used_bits = raw_ballot
            .bases
            .iter()
            .map(|el| (*el as f64).log2().ceil() as u64)
            .sum::<u64>() as i32;
        // we have a maximum of 29 bytes and each character takes 5 bits
        let remaining_bits: i32 = 29 * 8 - used_bits;

        let char_map = self.get_char_map();
        let base_bits = (char_map.base() as f64).log2().ceil() as i32;

        if remaining_bits > 0 {
            Ok(remaining_bits.div_ceil(base_bits))
        } else {
            Ok(remaining_bits.div_floor(base_bits))
        }
    }

    fn encode_to_raw_ballot(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<RawBallotQuestion, String> {
        let mut bases = self.get_bases();
        let mut choices: Vec<u64> = vec![];

        let char_map = self.get_char_map();

        let answers_map = self
            .answers
            .iter()
            .map(|answer| (answer.id.clone(), answer))
            .collect::<HashMap<Uuid, &Answer>>();

        // sort answers by id
        let mut sorted_choices = plaintext.choices.clone();
        sorted_choices.sort_by_key(|q| q.id.clone());

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

    fn decode_from_raw_ballot(
        &self,
        raw_ballot: &RawBallotQuestion,
    ) -> Result<DecodedVoteContest, String> {
        let choices = raw_ballot.choices.clone();
        let is_explicit_invalid: bool = !choices.is_empty() && (choices[0] > 0);
        let mut invalid_errors: Vec<InvalidPlaintextError> = vec![];
        let char_map = self.get_char_map();

        // 1. clone the question and reset the selections
        let mut sorted_answers = self.answers.clone();
        sorted_answers.sort_by_key(|q| q.id.clone());

        // 1.2. Initialize selection
        let mut sorted_choices: Vec<DecodedVoteChoice> = vec![];

        // 2. sort & segment answers
        let valid_answers: Vec<&Answer> = sorted_answers
            .iter()
            .filter(|answer| !answer.is_explicit_invalid())
            .collect();
        let write_in_answers: Vec<&Answer> = sorted_answers
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
                id: answer.id.clone(),
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
                        answer_id: Some(answer.id.clone()),
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
                    answer_id: Some(answer.id.clone()),
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
                    answer_id: Some(answer.id.clone()),
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

        Ok(DecodedVoteContest {
            contest_id: self.id.clone(),
            is_explicit_invalid,
            invalid_errors,
            choices: sorted_choices,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::*;
    use crate::mixed_radix::encode;
    use std::cmp;

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

            let encoded_ballot = encoded_bigint
                .map(|value| encode_bigint_to_bytes(&value).unwrap());
            let encoded_byte_array = encoded_ballot
                .clone()
                .map(|value| encode_vec_to_array(&value).unwrap());

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
                    &encoded_byte_array.expect("Expected value")
                );
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

    /*
    #[test]
    fn test_available_write_in_characters_estimate() {
        let ballot_style = get_writein_ballot_style();
        let contest = ballot_style.configuration.questions[0].clone();
        let plaintext = get_too_long_writein_plaintext();
        let available_chars = contest
            .available_write_in_characters_estimate(&plaintext)
            .unwrap();
        assert_eq!(available_chars, -1);
        let raw_ballot = contest.encode_to_raw_ballot(&plaintext).unwrap();
        assert_eq!(
            raw_ballot.bases,
            vec![
                2, 2, 2, 2, 2, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32, 32,
                32, 32
            ]
        );
        let bigint = contest
            .encode_plaintext_question_bigint(&plaintext)
            .unwrap();
        let bytes_vec = encode_bigint_to_bytes(&bigint).unwrap();
        assert_eq!(bigint.to_str_radix(10), "32534883079239123674464000999010439768324383932309717385996999631494");
        assert_eq!(bytes_vec.len(), 29);
    }
    */
}
