// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::*;
use crate::ballot_codec::*;
use crate::plaintext::*;
use num_traits::ToPrimitive;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct RawBallotContest {
    pub bases: Vec<u64>,
    pub choices: Vec<u64>,
}
impl RawBallotContest {
    // FIXME add validation (eg all values within range)
    // FIXME ensure this struct is always created with via RawBallotContest::new
    pub fn new(bases: Vec<u64>, choices: Vec<u64>) -> Self {
        RawBallotContest { bases, choices }
    }
}

pub trait RawBallotCodec {
    fn encode_to_raw_ballot(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<RawBallotContest, String>;

    fn decode_from_raw_ballot(
        &self,
        raw_ballot: &RawBallotContest,
    ) -> Result<DecodedVoteContest, String>;

    fn available_write_in_characters_estimate(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<i32, String>;
}

impl RawBallotCodec for Contest {
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
    ) -> Result<RawBallotContest, String> {
        let mut bases = self.get_bases().map_err(|e| e.to_string())?;
        let mut choices: Vec<u64> = vec![];

        let char_map = self.get_char_map();

        let candidates_map = self
            .candidates
            .iter()
            .map(|candidate| (candidate.id.clone(), candidate))
            .collect::<HashMap<String, &Candidate>>();

        // sort candidates by id
        let mut sorted_choices = plaintext.choices.clone();
        sorted_choices.sort_by_key(|q| q.id.clone());

        // Separate the candidates between:
        // - Invalid vote candidate (if any)
        // - Write-ins (if any)
        // - Valid candidates (normal candidates + write-ins if any)
        let invalid_vote: u64 =
            if plaintext.is_explicit_invalid { 1 } else { 0 };
        choices.push(invalid_vote);

        for choice in sorted_choices.iter() {
            let candidate =
                candidates_map.get(&choice.id).ok_or_else(|| {
                    "choice id is not a valid candidate".to_string()
                })?;
            if candidate.is_explicit_invalid() {
                continue;
            }
            if self.get_counting_algorithm() == "plurality-at-large" {
                // We just flag if the candidate was selected or not with 1 for
                // selected and 0 otherwise
                choices.push(u64::from(choice.selected > -1));
            } else {
                // we add 1 because the counting starts with 1, as zero means
                // this candidate was not voted / ranked
                let value =
                    (choice.selected + 1).to_u64().ok_or_else(|| {
                        "selected value must be positive or zero".to_string()
                    })?;
                choices.push(value);
            }
        }
        // Populate the bases and the raw_ballot values with the write-ins
        // if there's any. We will through each write-in (if any), and then
        // encode the write-in candidate.text string with UTF-8 and use for
        // each byte a specific value with base 256 and end each write-in
        // with a \0 byte. Note that even write-ins.
        if self.allow_writeins() {
            for choice in sorted_choices.iter() {
                let candidate =
                    candidates_map.get(&choice.id).ok_or_else(|| {
                        "choice id is not a valid candidate".to_string()
                    })?;
                let is_write_in = candidate.is_write_in();
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

        Ok(RawBallotContest { bases, choices })
    }

    /**
     * Decodes a raw ballot
     */
    fn decode_from_raw_ballot(
        &self,
        raw_ballot: &RawBallotContest,
    ) -> Result<DecodedVoteContest, String> {
        // IMPORTANT: Do not return in the middle of the function if there's an
        // error. We want to collect ALL errors first, then return with as much
        // valid information (and a comprehensive error list) as possible at
        // the end of the function

        let choices = raw_ballot.choices.clone();
        let is_explicit_invalid: bool = !choices.is_empty() && (choices[0] > 0);

        // Prepare the return value to pass it around, its values can still be
        // modified.
        let mut decoded_contest = DecodedVoteContest {
            contest_id: self.id.clone(),
            is_explicit_invalid,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: vec![],
        };
        let char_map = self.get_char_map();

        // 1. clone the contest and reset the selections
        let mut sorted_candidates = self.candidates.clone();
        sorted_candidates.sort_by_key(|q| q.id.clone());

        // 2. sort & segment candidates
        let valid_candidates: Vec<&Candidate> = sorted_candidates
            .iter()
            .filter(|candidate| !candidate.is_explicit_invalid())
            .collect();
        let write_in_candidates: Vec<&Candidate> = sorted_candidates
            .iter()
            .filter(|candidate| candidate.is_write_in())
            .collect();
        // 4. Do some verifications on the number of choices: Checking that the
        //    raw_ballot has as many choices as required
        if choices.len() < valid_candidates.len() + 1 {
            // Invalid Ballot: Not enough choices to decode
            decoded_contest.invalid_errors.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::EncodingError,
                candidate_id: None,
                message: Some("errors.encoding.notEnoughChoices".to_string()),
                message_map: HashMap::new(),
            });
        }

        // 5. Populate the valid candidates. We asume they are in the same order
        //    as
        // in    raw_ballot["choices"]
        // we add 1 to the index because raw_ballot.choice[0] is just the
        // invalidVoteFlag
        let mut index = 1usize;
        for candidate in &valid_candidates {
            if choices.len() <= index {
                break;
            }
            // TODO: here we do return an error, because it's difficult to
            // recover from this one
            let choice_value = choices[index]
                .clone()
                .to_i64()
                .ok_or_else(|| "choice out of range".to_string())?;

            decoded_contest.choices.push(DecodedVoteChoice {
                id: candidate.id.clone(),
                selected: choice_value - 1,
                write_in_text: None,
            });

            index += 1;
        }
        // 6. Decode the write-in texts into UTF-8 and split by the \0
        // character,    finally the text for the write-ins.
        let mut write_in_index = index;
        for candidate in &write_in_candidates {
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
                    decoded_contest.invalid_errors.push(
                        InvalidPlaintextError {
                            error_type:
                                InvalidPlaintextErrorType::EncodingError,
                            candidate_id: Some(candidate.id.clone()),
                            message: Some(
                                "errors.encoding.writeInChoiceOutOfRange"
                                    .to_string(),
                            ),
                            message_map: HashMap::from([(
                                "index".to_string(),
                                write_in_index.to_string(),
                            )]),
                        },
                    );
                }

                write_in_index += 1;
            }

            // check index is not out of bounds
            if write_in_index >= choices.len() {
                decoded_contest.invalid_errors.push(InvalidPlaintextError {
                    error_type: InvalidPlaintextErrorType::EncodingError,
                    candidate_id: Some(candidate.id.clone()),
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
                decoded_contest.invalid_errors.push(InvalidPlaintextError {
                    error_type: InvalidPlaintextErrorType::EncodingError,
                    candidate_id: Some(candidate.id.clone()),
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
            let n = decoded_contest
                .choices
                .iter()
                .position(|choice| choice.id == candidate.id)
                .unwrap();
            let mut choice = decoded_contest.choices[n].clone();
            choice.write_in_text = write_in_str;
            decoded_contest.choices[n] = choice;
        }

        if write_in_index < choices.len() {
            decoded_contest.invalid_errors.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::EncodingError,
                candidate_id: None,
                message: Some("errors.encoding.ballotTooLarge".to_string()),
                message_map: HashMap::new(),
            });
        }

        let presentation = self.presentation.clone().unwrap_or_default();

        let invalid_vote_policy_errors =
            check_invalid_vote_policy(&presentation, is_explicit_invalid);
        decoded_contest.update(invalid_vote_policy_errors);

        // implicit invalid errors
        let num_selected_candidates = decoded_contest
            .choices
            .iter()
            .filter(|choice| choice.selected > -1)
            .count();

        let (max_votes, min_votes, maxmin_errors) =
            check_max_min_votes_policy(self.max_votes, self.min_votes);
        decoded_contest.update(maxmin_errors);

        if let Some(max_votes) = max_votes {
            let overvote_check = check_over_vote_policy(
                &presentation,
                num_selected_candidates,
                max_votes,
            );
            decoded_contest.update(overvote_check);
        }
        if let Some(min_votes) = min_votes {
            let min_check =
                check_min_vote_policy(num_selected_candidates, min_votes);
            decoded_contest.update(min_check);
        }

        let under_vote_check = check_under_vote_policy(
            &presentation,
            num_selected_candidates,
            max_votes.clone(),
            max_votes.clone(),
        );
        decoded_contest.update(under_vote_check);

        // handle blank vote policy
        let blank_vote_check = check_blank_vote_policy(
            &presentation,
            num_selected_candidates,
            is_explicit_invalid,
        );
        decoded_contest.update(blank_vote_check);

        Ok(decoded_contest)
    }
}

#[cfg(test)]
mod tests {
    use raw_ballot::EUnderVotePolicy;

    use crate::ballot;
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::*;
    use crate::mixed_radix::encode;
    use std::cmp;

    #[test]
    fn test_contest_encode_to_raw_ballot() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);
            let raw_ballot =
                fixture.contest.encode_to_raw_ballot(&fixture.plaintext);
            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("contest_encode_to_raw_ballot").cloned()
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
    fn test_contest_encode_raw_ballot() {
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
                    expected_map.get("contest_encode_raw_ballot").cloned()
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
    fn test_contest_decode_raw_ballot() {
        let fixtures = get_fixtures();
        for fixture in fixtures {
            println!("fixture: {}", &fixture.title);
            let decoded_ballot_res =
                fixture.contest.decode_from_raw_ballot(&fixture.raw_ballot);
            let expected_error =
                fixture.expected_errors.and_then(|expected_map| {
                    expected_map.get("contest_decode_raw_ballot").cloned()
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
                    if fixture.contest.get_counting_algorithm()
                        == "plurality-at-large"
                    {
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

                let num_selected_candidates = decoded_ballot
                    .choices
                    .iter()
                    .filter(|choice| choice.selected > -1)
                    .count();
                let max_votes = match usize::try_from(fixture.contest.max_votes)
                {
                    Ok(val) => Some(val),
                    Err(_) => None,
                };
                let min_votes = match usize::try_from(fixture.contest.min_votes)
                {
                    Ok(val) => Some(val),
                    Err(_) => None,
                };

                if let (Some(max_votes), Some(min_votes)) =
                    (max_votes, min_votes)
                {
                    // Test for undervote
                    if let Some(ballot::ContestPresentation {
                        under_vote_policy: Some(under_vote_policy),
                        ..
                    }) = fixture.contest.presentation
                    {
                        if num_selected_candidates < max_votes
                            && num_selected_candidates >= min_votes
                            && under_vote_policy != EUnderVotePolicy::ALLOWED
                        {
                            let has_under_vote_policy = decoded_ballot
                                .invalid_alerts
                                .iter()
                                .any(|alert| {
                                    alert.message
                                        == Some(
                                            "errors.implicit.underVote"
                                                .to_string(),
                                        )
                                });
                            assert!(
                                has_under_vote_policy,
                                "Expected undervote policy not found in invalid_alerts"
                            );
                        }
                    }
                    // Test for overvote
                    if num_selected_candidates > max_votes {
                        let has_max_vote_error =
                            decoded_ballot.invalid_errors.iter().any(|err| {
                                err.message
                                    == Some(
                                        "errors.implicit.selectedMax"
                                            .to_string(),
                                    )
                            });
                        assert!(has_max_vote_error, "Expected selected max overvote error not found in invalid_errors");
                    }
                }
            }
        }
    }
}
