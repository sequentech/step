// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::ballot::{Candidate, Contest};
use crate::ballot_codec::{
    check_blank_vote_policy, check_duplicated_rank_policy,
    check_invalid_vote_policy, check_max_min_votes_policy,
    check_min_vote_policy, check_over_vote_policy,
    check_preference_gaps_policy, check_under_vote_policy, BasesCodec,
    CharacterMap,
};
use crate::plaintext::{
    DecodedVoteChoice, DecodedVoteContest, InvalidPlaintextError,
    InvalidPlaintextErrorType, PreferencialOrderErrorType,
};
use crate::types::ceremonies::CountingAlgType;
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
    #[must_use]
    pub const fn new(bases: Vec<u64>, choices: Vec<u64>) -> Self {
        RawBallotContest { bases, choices }
    }
}

pub trait RawBallotCodec {
    /// Helper function to update all policy checks and error/alert fields for a
    /// decoded contest. This is used in the `decode_from_raw_ballot`.
    fn update_decoded_contest_policies(
        &self,
        decoded_contest: &mut DecodedVoteContest,
        is_explicit_invalid: bool,
    );

    /// Encodes the contest to a raw ballot.
    ///
    /// # Errors
    /// Returns an error if encoding fails.
    fn encode_to_raw_ballot(
        &self,
        plaintext: &DecodedVoteContest,
    ) -> Result<RawBallotContest, String>;

    /// Decodes a raw ballot to a `DecodedVoteContest`.
    ///
    /// # Errors
    /// Returns an error if decoding fails.
    fn decode_from_raw_ballot(
        &self,
        raw_ballot: &RawBallotContest,
    ) -> Result<DecodedVoteContest, String>;

    /// Estimates available write-in characters.
    ///
    /// # Errors
    /// Returns an error if estimation fails.
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
        let raw_ballot = self.encode_to_raw_ballot(plaintext)?;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let used_bits = raw_ballot
            .bases
            .iter()
            .map(|el| (*el as f64).log2().ceil() as u64)
            .sum::<u64>();
        let remaining_bits: i32 = 29_i32
            .checked_mul(8)
            .and_then(|v| {
                v.checked_sub(
                    i32::try_from(used_bits)
                        .map_err(|_| "used_bits too large for i32".to_string())
                        .ok()?,
                )
            })
            .ok_or_else(|| {
                "Overflow in remaining_bits calculation".to_string()
            })?;

        let char_map = self.get_char_map();
        #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
        let base_bits = (char_map.base() as f64).log2().ceil() as i32;

        if base_bits == 0 {
            return Err("Base bits cannot be zero".to_string());
        }

        if remaining_bits > 0 {
            // div_ceil for positive numbers
            let rem = u32::try_from(remaining_bits).map_err(|_| {
                "remaining_bits negative in div_ceil".to_string()
            })?;
            let base = u32::try_from(base_bits)
                .map_err(|_| "base_bits negative in div_ceil".to_string())?;
            #[allow(clippy::cast_possible_wrap)]
            {
                Ok(rem.div_ceil(base) as i32)
            }
        } else {
            #[allow(clippy::arithmetic_side_effects)]
            let div = remaining_bits / base_bits;
            #[allow(clippy::arithmetic_side_effects)]
            let needs_adjust = remaining_bits % base_bits != 0;
            #[allow(clippy::arithmetic_side_effects)]
            Ok(div - i32::from(needs_adjust))
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
        let invalid_vote: u64 = u64::from(plaintext.is_explicit_invalid);
        choices.push(invalid_vote);

        for choice in &sorted_choices {
            let candidate =
                candidates_map.get(&choice.id).ok_or_else(|| {
                    "choice id is not a valid candidate".to_string()
                })?;
            if candidate.is_explicit_invalid() {
                continue;
            }
            let alg = self.get_counting_algorithm();
            if alg == CountingAlgType::PluralityAtLarge {
                // We just flag if the candidate was selected or not with 1
                // for selected and 0 otherwise
                choices.push(u64::from(choice.selected > -1));
            } else {
                // we add 1 because the counting starts with 1, as zero
                // means this candidate was not voted /
                // ranked (selected was -1). This should work for IRV and
                // other preferencial counting algorithms
                let sel_plus_1 = choice
                    .selected
                    .checked_add(1)
                    .ok_or_else(|| "Overflow in selected+1".to_string())?;
                let value = sel_plus_1.to_u64().ok_or_else(|| {
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
            for choice in &sorted_choices {
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
                if let Some(text) = choice.write_in_text.clone() {
                    if is_write_in {
                        if text.is_empty() {
                            // we don't do a bases.push_back(256) as this is done in
                            // getBases() to end it with a zero
                            choices.push(0);
                        } else {
                            // MAPPER
                            let base = char_map.base();
                            let bytes = char_map.to_bytes(&text)?;
                            for byte in bytes {
                                choices.push(u64::from(byte));
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
        let is_explicit_invalid: bool =
            !choices.is_empty() && choices.first().is_some_and(|v| *v > 0);

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
        if choices.len() < valid_candidates.len().saturating_add(1) {
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
            let choice_value = choices
                .get(index)
                .ok_or_else(|| "choice index out of range".to_string())?
                .to_i64()
                .ok_or_else(|| "choice out of range".to_string())?;

            decoded_contest.choices.push(DecodedVoteChoice {
                id: candidate.id.clone(),
                selected: choice_value
                    .checked_sub(1)
                    .ok_or_else(|| "Overflow in selected-1".to_string())?,
                write_in_text: None,
            });

            index = index
                .checked_add(1)
                .ok_or_else(|| "Overflow in index+1".to_string())?;
        }

        // 6. Decode the write-in texts into UTF-8 and split by the \0 character
        decode_write_ins(
            &write_in_candidates,
            &mut decoded_contest,
            &choices,
            char_map.as_ref(),
            index,
        );

        self.update_decoded_contest_policies(
            &mut decoded_contest,
            is_explicit_invalid,
        );
        Ok(decoded_contest)
    }

    /// Updates all policy checks and error/alert fields for a decoded contest.
    fn update_decoded_contest_policies(
        &self,
        decoded_contest: &mut DecodedVoteContest,
        is_explicit_invalid: bool,
    ) {
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
            max_votes,
            min_votes,
        );
        decoded_contest.update(under_vote_check);

        // handle blank vote policy
        let blank_vote_check = check_blank_vote_policy(
            &presentation,
            num_selected_candidates,
            is_explicit_invalid,
        );
        decoded_contest.update(blank_vote_check);

        if self.get_counting_algorithm().is_preferential() {
            match decoded_contest.validate_preferencial_order() {
                Ok(()) => {}
                Err(errors) => {
                    for error in errors {
                        match error {
                            PreferencialOrderErrorType::PreferenceOrderWithGaps => {
                                let check =
                                    check_preference_gaps_policy(&presentation);
                                decoded_contest.update(check);
                            }
                            PreferencialOrderErrorType::DuplicatedPosition => {
                                let check =
                                    check_duplicated_rank_policy(&presentation);
                                decoded_contest.update(check);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Helper to decode write-in candidates for `decode_from_raw_ballot`
fn decode_write_ins(
    write_in_candidates: &[&Candidate],
    decoded_contest: &mut DecodedVoteContest,
    choices: &[u64],
    char_map: &dyn CharacterMap,
    mut write_in_index: usize,
) {
    for candidate in write_in_candidates {
        if write_in_index >= choices.len() {
            break;
        }
        // collect the string bytes
        let mut write_in_bytes: Vec<u8> = vec![];

        while write_in_index < choices.len()
            && choices.get(write_in_index).is_some_and(|v| *v != 0)
        {
            let value_res = choices
                .get(write_in_index)
                .ok_or_else(|| "write_in_index out of range".to_string())
                .and_then(|v| {
                    v.to_u8().ok_or_else(|| {
                        "Write-in choice out of range".to_string()
                    })
                });

            if let Ok(new_value) = value_res {
                write_in_bytes.push(new_value);
            } else {
                decoded_contest.invalid_errors.push(InvalidPlaintextError {
                    error_type: InvalidPlaintextErrorType::EncodingError,
                    candidate_id: Some(candidate.id.clone()),
                    message: Some(
                        "errors.encoding.writeInChoiceOutOfRange".to_string(),
                    ),
                    message_map: HashMap::from([(
                        "index".to_string(),
                        write_in_index.to_string(),
                    )]),
                });
            }

            write_in_index =
                write_in_index.checked_add(1).unwrap_or(choices.len());
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
        else if choices.get(write_in_index).is_some_and(|v| *v == 0) {
            write_in_index =
                write_in_index.checked_add(1).unwrap_or(choices.len());
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
                    write_in_str_res
                        .clone()
                        .expect_err("Expected error in write_in_str_res"),
                )]),
            });
        }

        let write_in_str = write_in_str_res.ok();

        // add write_in to choice
        let n = decoded_contest
            .choices
            .iter()
            .position(|choice| choice.id == candidate.id)
            .expect("Choice for write-in candidate not found");
        if let Some(choice_mut) = decoded_contest.choices.get_mut(n) {
            choice_mut.write_in_text = write_in_str;
        }
    }
}
#[cfg(test)]
mod tests {

    use crate::ballot;
    use crate::ballot::EUnderVotePolicy;
    use crate::ballot_codec::*;
    use crate::fixtures::ballot_codec::*;
    use crate::mixed_radix::encode;
    use crate::types::ceremonies::CountingAlgType;
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
                        == CountingAlgType::PluralityAtLarge
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

    #[test]
    fn test_irv_invalid_ballot() {
        let fixture = get_irv_fixture_invalid_ballot();

        // Encode the plaintext to raw ballot
        let encoded_raw_ballot = fixture
            .contest
            .encode_to_raw_ballot(&fixture.plaintext)
            .expect("Failed to encode plaintext to raw ballot");

        // Decode the raw ballot back to plaintext
        let decoded_plaintext = fixture
            .contest
            .decode_from_raw_ballot(&encoded_raw_ballot)
            .expect("Failed to decode raw ballot to plaintext");

        // Compare the selections of the choices
        assert_eq!(
            decoded_plaintext.is_invalid(),
            true,
            "Ballot should be invalid"
        );
    }

    #[test]
    fn test_irv_encode_decode() {
        let fixture = get_irv_fixture_valid_ballot();

        // Encode the plaintext to raw ballot
        let encoded_raw_ballot = fixture
            .contest
            .encode_to_raw_ballot(&fixture.plaintext)
            .expect("Failed to encode plaintext to raw ballot");

        // Decode the raw ballot back to plaintext
        let decoded_plaintext = fixture
            .contest
            .decode_from_raw_ballot(&encoded_raw_ballot)
            .expect("Failed to decode raw ballot to plaintext");

        // Compare the selections of the choices
        assert_eq!(
            fixture.plaintext.choices.len(),
            decoded_plaintext.choices.len(),
            "Number of choices should match"
        );

        for idx in 0..fixture.plaintext.choices.len() {
            assert_eq!(
                fixture.plaintext.choices[idx].id,
                decoded_plaintext.choices[idx].id,
                "Choice ID should match at index {}",
                idx
            );
            assert_eq!(
                fixture.plaintext.choices[idx].selected,
                decoded_plaintext.choices[idx].selected,
                "Choice selection should match at index {}",
                idx
            );
        }
    }
}
