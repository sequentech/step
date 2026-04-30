use std::collections::{HashMap, HashSet};
use std::num::TryFromIntError;

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::bigint;
use super::{vec, RawBallotContest};
use crate::ballot::{
    AreaPresentation, BallotStyle, Candidate, Contest, EUnderVotePolicy,
};
use crate::ballot_codec::{
    check_blank_vote_policy, check_duplicated_rank_policy,
    check_invalid_vote_policy, check_max_min_votes_policy,
    check_min_vote_policy, check_over_vote_policy,
    check_preference_gaps_policy, check_under_vote_policy,
    validate_contest_preferencial_order, CheckerResult,
};
use crate::error::BallotError;
use crate::mixed_radix;
use crate::plaintext::{
    map_decoded_ballot_choices_to_decoded_contests, DecodedVoteContest,
    InvalidPlaintextError, InvalidPlaintextErrorType,
    PreferencialOrderErrorType,
};
use crate::types::ceremonies::CountingAlgType;
use num_bigint::BigUint;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::encrypt::encode_to_plaintext_decoded_multi_contest;
use crate::util::normalize_vote::normalize_election;
use num_bigint::ToBigUint;
use num_traits::{ToPrimitive, Zero};
use std::hash::{Hash, Hasher};
/// A multi contest ballot.
///
/// A multi contest ballot can be encoded in to a
/// 30 byte representation allowing encrypting
/// choices from multiple contests into a single ciphertext,
/// provided there is sufficient space.
///
/// An upper bound on the bytes needed to encode a multi contest ballot
/// can be computed with BallotChoices::maximum_size_bytes, given a list
/// of contests.
///
/// This ballot does not support write-ins.
/// It does not support per-contest invalid flags.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct BallotChoices {
    pub is_explicit_invalid: bool,
    pub choices: Vec<ContestChoices>,
}
impl BallotChoices {
    pub fn new(
        is_explicit_invalid: bool,
        choices: Vec<ContestChoices>,
    ) -> Self {
        BallotChoices {
            is_explicit_invalid,
            choices,
        }
    }
}

/// The choices for a contest.
///
/// Does not support write-ins.
/// Does not support invalid flags.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct ContestChoices {
    pub contest_id: String,
    pub choices: Vec<ContestChoice>,
    pub counting_algorithm: CountingAlgType,
}
impl ContestChoices {
    pub fn new(
        contest_id: String,
        choices: Vec<ContestChoice>,
        counting_algorithm: CountingAlgType,
    ) -> Self {
        ContestChoices {
            contest_id,
            // is_explicit_invalid,
            choices,
            counting_algorithm,
        }
    }

    /// Return contest choices from a DecodedVoteContest
    ///
    /// Used in testing when generating ballots with the non-sparse
    /// encoding (non multi-contest ballots)
    pub fn from_decoded_vote_contest(
        dcv: &DecodedVoteContest,
        counting_algorithm: &CountingAlgType,
    ) -> Self {
        let choices: Vec<ContestChoice> = dcv
            .choices
            .iter()
            // Only values > -1 are interpreted as set values
            // Values not present will be automatically interpreted as unset
            .filter(|dc| dc.selected > -1)
            .map(|dc| ContestChoice {
                candidate_id: dc.id.clone(),
                selected: dc.selected,
            })
            .collect();

        ContestChoices {
            contest_id: dcv.contest_id.clone(),
            choices,
            counting_algorithm: counting_algorithm.clone(),
        }
    }
}
#[derive(
    Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone, Hash,
)]

/// A single choice within a Contest.
///
/// Does not support write-ins.
pub struct ContestChoice {
    pub candidate_id: String,
    // This is could be eliminated until we are using some sort of score voting
    // Currently, a value of > -1 is interpreted as a selection, -1 is
    // interpreted as Unset.
    pub selected: i64,
}
impl ContestChoice {
    pub fn new(candidate_id: String, selected: i64) -> Self {
        ContestChoice {
            candidate_id,
            selected,
        }
    }
}

/// The choices for a contest returned when decoding.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedContestChoices {
    pub contest_id: String,
    pub choices: Vec<DecodedContestChoice>,
    pub invalid_errors: Vec<InvalidPlaintextError>,
    pub invalid_alerts: Vec<InvalidPlaintextError>,
}
impl DecodedContestChoices {
    pub fn new(
        contest_id: String,
        choices: Vec<DecodedContestChoice>,
        invalid_errors: Vec<InvalidPlaintextError>,
        invalid_alerts: Vec<InvalidPlaintextError>,
    ) -> Self {
        DecodedContestChoices {
            contest_id,
            choices,
            invalid_errors,
            invalid_alerts,
        }
    }

    pub fn validate_preferencial_order(
        &self,
    ) -> Result<(), Vec<PreferencialOrderErrorType>> {
        // Discard the unselected choices and sort the selected ones by their preference order
        let choices: Vec<i64> = self
            .choices
            .iter()
            .filter(|choice| choice.selected >= 0)
            .map(|choice| choice.selected)
            .collect();

        validate_contest_preferencial_order(choices)
    }
}
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone, Hash)]
/// A decoded contest choice contains the candidate_id as a String.
pub struct DecodedContestChoice {
    pub id: String,
    pub selected: i64,
}

/// The choices for the set of contests returned when decoding a multi-content
/// ballot.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedBallotChoices {
    pub is_explicit_invalid: bool,
    pub choices: Vec<DecodedContestChoices>,
    pub serial_number: Option<String>,
}

impl BallotStyle {
    /// Returns a map containint the
    pub fn get_counting_algorithms(
        &self,
    ) -> Result<HashMap<String, CountingAlgType>, BallotError> {
        self.contests
            .iter()
            .map(|contest| {
                Ok((contest.id.clone(), contest.get_counting_algorithm()))
            })
            .collect()
    }
}

impl BallotChoices {
    /// Encode this ballot into a 30 byte representation
    ///
    /// The following steps take place:
    ///
    /// 1) BallotChoices -> RawBallotContest (this is a mixed-radix structure)
    /// 2) RawBallotContest -> BigUint
    /// 3) BigUint -> Vec<u8>
    /// 4) Vec<u8> -> [u8; 30]
    ///
    /// Returns a fixed-size array of 30 bytes encoding this ballot.
    pub fn encode_to_30_bytes(
        &self,
        config: &BallotStyle,
    ) -> Result<[u8; 30], String> {
        let raw_ballot = self.encode_to_raw_ballot(&config)?;

        let bigint =
            mixed_radix::encode(&raw_ballot.choices, &raw_ballot.bases)?;

        let bytes = bigint::encode_bigint_to_bytes(&bigint)?;

        vec::encode_vec_to_array(&bytes)
    }

    /// Encode this multi-ballot into a mixed radix representation
    ///
    /// The following conditions will return an error:
    ///
    /// * The plaintexts for a given contest were not found.
    /// * The length of a contest choice vector was greater than
    ///   contest.max_votes.
    /// * The length of a contest choice vector was smaller than
    ///   contest.min_votes.
    /// * The set choices (!=0) for a contest had duplicates.
    /// * The number of set choices (!= 0) for a given contest choice vector was
    ///   smaller than contest.min_votes.
    /// * A choice id in a given contest choice vector was invalid.
    ///
    /// The resulting encoded choice vector is a
    /// contiguous list of contest choices groups, each of
    /// size contest.max_votes. An alternative implementation
    /// could add explicit separators between contest choice
    /// groups.
    ///
    /// Returns the encoded ballot, with n sets of contest choices
    /// each of size contest.max_votes, plus one invalid flag.
    /// The total number of choices is given by the following:
    /// contests.iter().fold(0, |a, b| a + b.max_votes) + 1
    fn encode_to_raw_ballot(
        &self,
        config: &BallotStyle,
    ) -> Result<RawBallotContest, String> {
        let contests = self.get_contests(config)?;

        let bases = Self::get_bases(&contests).map_err(|e| e.to_string())?;
        let mut choices: Vec<u64> = vec![];

        // Construct a map of plaintexts, this will allow us to
        // handle calls in which passed in contests and plaintexts
        // may not be in the same [parallel] order. We will
        // obtain plaintexts from this map using the contest_id.
        let plaintexts_map = self
            .choices
            .iter()
            .map(|plaintext| (plaintext.contest_id.clone(), plaintext))
            .collect::<HashMap<String, &ContestChoices>>();

        // The order of the contests is computed sorting by id.
        // The selections must be encoded to and decoded from a ballot
        // following this order, given by contest.id.
        let mut sorted_contests = contests.clone();
        sorted_contests.sort_by_key(|c| c.id.clone());

        let invalid_vote: u64 = if self.is_explicit_invalid { 1 } else { 0 };
        choices.push(invalid_vote);

        // Iterate in contest order
        for contest in sorted_contests {
            let plaintext = plaintexts_map.get(&contest.id).ok_or(format!(
                "Could not find plaintexts for contest {:?}",
                contest
            ))?;

            let contest_choices = self.encode_contest(&contest, &plaintext)?;

            // Accumulate the choices for each contest
            choices.extend(contest_choices);
        }

        Ok(RawBallotContest { bases, choices })
    }

    /// Encodes one contest in the ballot
    ///
    /// Returns a choice vector of length contest.max_votes,
    /// which the caller will append to the overall ballot choice vector.
    fn encode_contest(
        &self,
        contest: &Contest,
        plaintext: &ContestChoices,
    ) -> Result<Vec<u64>, String> {
        // A choice of a candidate is represented as that candidate's
        // position in the candidate list, sorted by id. The
        // same sorting order must be used to interpret
        // choices when decoding.
        let mut sorted_candidates: Vec<Candidate> = contest
            .candidates
            .clone()
            .into_iter()
            .filter(|candidate| !candidate.is_explicit_invalid())
            .collect();
        sorted_candidates.sort_by_key(|c| c.id.clone());

        // Note how the position for the candidate is mapped to the first
        // element in the tuple. This position will be used below when
        // marking choices.
        let candidates_map = sorted_candidates
            .iter()
            .enumerate()
            .map(|c| (c.1.id.clone(), (c.0, c.1)))
            .collect::<HashMap<String, (usize, &Candidate)>>();

        let max_votes: usize = contest
            .max_votes
            .try_into()
            .map_err(|_| format!("u64 conversion on contest max_votes"))?;
        let min_votes: usize = contest
            .min_votes
            .try_into()
            .map_err(|_| format!("u64 conversion on contest min_votes"))?;

        if plaintext.choices.len() < min_votes {
            return Err(format!(
                "Plaintext vector contained fewer than min_votes elements ({} > {})", plaintext.choices.len(), min_votes
            ));
        }
        if plaintext.choices.len() > max_votes {
            return Err(format!(
                "Plaintext vector contained more than max_votes elements ({} > {})", plaintext.choices.len(), max_votes
            ));
        }

        let choices_order = match plaintext.counting_algorithm.is_preferential()
        {
            true => {
                // Setting the choices in order of preference to support
                // preferencial multiballot. When decoding, we
                // will take the order of the
                // vector to determine the order of preference of each choice.
                // The invalid ones with seected = -1 will be at the beginning
                // but will be ignored when decoding anyway
                // because are marked to 0.
                let mut pref_choices: Vec<ContestChoice> =
                    plaintext.choices.clone();
                pref_choices.sort_by_key(|c| c.selected);
                pref_choices
            }
            false => plaintext.choices.clone(),
        };

        // We set all values as unset (0) by default
        let mut contest_choices = vec![0u64; max_votes];
        let mut marked = 0;

        for p in &choices_order {
            let (position, _candidate) =
                candidates_map.get(&p.candidate_id).ok_or_else(|| {
                    "choice id is not a valid candidate".to_string()
                })?;

            // The slot's base is
            //
            // number of candidates + 1, such that
            //
            // 0    = unset
            // >0   = the chosen candidate, with an offset of +1.
            //
            // A choice of a candidate is represented as that
            // candidate's position in the candidate
            // list, sorted by id. The same sorting order must be used
            // to interpret choices when decoding.
            let mark = if p.selected > -1 {
                (position + 1).try_into().map_err(|_| {
                    format!("u64 conversion on candidate position")
                })?
            } else {
                // unset
                0
            };

            match plaintext.counting_algorithm.is_preferential() {
                true => {
                    if p.selected > -1 {
                        //position the choice in the array in the selected order
                        //for the decode know restore the selected value
                        let index =
                            usize::try_from(p.selected).map_err(|_| {
                                format!(
                                    "uzise conversion on choice selected value"
                                )
                            })?;
                            if index >= max_votes {
                                return Err(format!(
                                    "choice selected value {} is out of range [0, {})",
                                    p.selected, max_votes
                                ));
                            }
                        contest_choices[index] = mark;
                    }
                }
                false => {
                    contest_choices[marked] = mark;
                }
            }

            marked += 1;

            if marked == max_votes {
                break;
            }
        }

        // There can be no duplicates among the set values (!= 0)
        let set_values: Vec<u64> = contest_choices
            .iter()
            .cloned()
            .filter(|v| *v != 0)
            .collect();
        let unique: HashSet<u64> =
            HashSet::from_iter(set_values.iter().cloned());
        if unique.len() != set_values.len() {
            return Err(format!("Plaintext vector contained duplicate values"));
        }

        if marked < min_votes {
            return Err(format!(
                "Plaintext vector contained fewer than min_votes marks"
            ));
        }

        Ok(contest_choices)
    }

    /// Decodes a multi-ballot from 30 bytes.
    ///
    /// The following steps take place:
    ///
    /// 1) [u8; 30] -> Vec<u8>
    /// 2) Vec<u8> -> BigUint
    /// 3) BigUint -> RawBallotContest (this is a mixed-radix structure)
    /// 4) RawBallotContest -> DecodedBallotChoices
    ///
    /// The following conditions will return an error.
    ///
    /// =================================
    /// FIXME
    /// In the current implementation these errors short
    /// circuit the operation.
    ///
    /// * choices.len() != expected_choices + 1
    /// * let Some(candidate) = candidate else {
    /// return Err(format!(
    ///    "Candidate selection out of range {} (length: {})",
    ///    next,
    ///    sorted_candidates.len()
    /// ));};
    /// * let next = usize::try_from(next).map_err(|_| { format!("u64 -> usize
    ///   conversion on plaintext choice") })?;
    /// * is_explicit_invalid && !self.allow_explicit_invalid() {
    /// * max_votes: Option<usize> = match usize::try_from(self.max_votes)
    /// * min_votes: Option<usize> = match usize::try_from(self.min_votes)
    /// * decoded_contest = handle_over_vote_policy(
    /// * num_selected_candidates < min_votes
    /// * under_vote_policy != EUnderVotePolicy::ALLOWED &&
    ///   num_selected_candidates < max_votes && num_selected_candidates >=
    ///   min_votes
    /// * if let Some(blank_vote_policy) = presentation.blank_vote_policy { if
    ///   num_selected_candidates == 0
    /// =================================
    ///
    /// * The number of overall choices does not match the expected value
    /// * A contest choice is out of range (larger than the number of
    ///   candidates)
    /// * There are fewer contest choices than contest.min_votes
    /// * There is an i64 -> u64 conversion error on
    /// * contest.min_votes
    /// * contest.max_votes
    /// * There is a u64 -> usize conversion error on a choice
    ///
    /// The decoding processes the choices vector as a
    /// contiguous list of contest choices groups, each of
    /// size contest.max_votes. An alternative implementation
    /// could add explicit separators between contest choice
    /// groups.
    ///
    /// Returns the decoded ballot. Because this is a multi
    /// contest ballot, it will have n ContestChoices and
    /// an overall invalid flag.
    pub fn decode_from_30_bytes(
        bytes: &[u8; 30],
        style: &BallotStyle,
    ) -> Result<DecodedBallotChoices, String> {
        let bytes = vec::decode_array_to_vec(&bytes);
        let bigint = bigint::decode_bigint_from_bytes(&bytes)?;

        Self::decode_from_bigint(&bigint, &style.contests, None)
    }

    /// Returns a decoded ballot from a BigUint
    ///
    /// Convenience method.
    pub fn decode_from_bigint(
        bigint: &BigUint,
        contests: &Vec<Contest>,
        serial_number_counter: Option<&mut u32>,
    ) -> Result<DecodedBallotChoices, String> {
        let raw_ballot = Self::bigint_to_raw_ballot(&bigint, contests)?;

        Self::decode(&raw_ballot, contests, serial_number_counter)
    }

    /// Decode a mixed radix representation of the ballot.
    pub fn decode(
        raw_ballot: &RawBallotContest,
        contests: &Vec<Contest>,
        serial_number_counter: Option<&mut u32>,
    ) -> Result<DecodedBallotChoices, String> {
        let mut contest_choices: Vec<DecodedContestChoices> = vec![];
        let choices = raw_ballot.choices.clone();

        // Each contest contributes max_votes slots
        let expected_choices = contests.iter().fold(0, |a, b| a + b.max_votes);
        let expected_choices: usize =
            expected_choices.try_into().map_err(|_| {
                format!("i64 -> usize conversion on contest max_votes")
            })?;

        // The first slot is used for explicit invalid ballot, so + 1
        if choices.len() != expected_choices + 1 {
            return Err(format!(
                "Unexpected number of choices {} != {}",
                choices.len(),
                expected_choices
            ));
        }

        // The order of the contests is computed sorting by id.
        // The selections must be encoded to and decoded from a ballot
        // following this order, given by contest.id.
        let mut sorted_contests = contests.clone();
        sorted_contests.sort_by_key(|c| c.id.clone());

        // This explicit invalid flag is at the ballot level
        let is_explicit_invalid: bool = !choices.is_empty() && (choices[0] > 0);
        // Skip past the explicit invalid slot
        let mut choice_index = 1;

        for contest in sorted_contests {
            let max_votes: usize =
                contest.max_votes.try_into().map_err(|_| {
                    format!("i64 -> usize conversion on contest max_votes")
                })?;
            let next = Self::decode_contest(
                &contest,
                &choices[choice_index..],
                is_explicit_invalid,
            )?;
            choice_index += max_votes;
            contest_choices.push(next);
        }

        let serial_number = match serial_number_counter {
            Some(serial_number) => {
                let sn = Some(format!("{:09}", *serial_number));
                *serial_number += 1;
                sn
            }
            None => None,
        };

        let ret = DecodedBallotChoices {
            is_explicit_invalid,
            choices: contest_choices,
            serial_number,
        };

        Ok(ret)
    }

    /// Decodes one contest in the ballot
    ///
    /// Returns a ContestChoice for the choices slice argument,
    /// which will be read up to position contest.max_votes. This
    /// ContestChoice will be added to the overall DecodedBallotChoices.
    /// Values set to 0 (unset) will not return a ContestChoice.
    /// It is the responsibility of the caller to advance the choice slice
    /// as choices are decoded.
    fn decode_contest(
        contest: &Contest,
        choices: &[u64],
        is_explicit_invalid: bool,
    ) -> Result<DecodedContestChoices, String> {
        let mut decoded_contest = DecodedContestChoices::new(
            contest.id.clone(),
            vec![],
            vec![],
            vec![],
        );
        // A choice of a candidate is represented as that candidate's
        // position in the candidate list, sorted by id.
        let mut sorted_candidates: Vec<Candidate> = contest
            .candidates
            .clone()
            .into_iter()
            .filter(|candidate| !candidate.is_explicit_invalid())
            .collect();

        sorted_candidates.sort_by_key(|c| c.id.clone());

        let max_votes: usize = contest.max_votes.try_into().map_err(|_| {
            format!("i64 -> usize conversion on contest max_votes")
        })?;
        let min_votes: usize = contest.min_votes.try_into().map_err(|_| {
            format!("i64 -> usize conversion on contest min_votes")
        })?;

        let is_preferencial = contest
            .counting_algorithm
            .as_ref()
            .map_or(false, |a| a.is_preferential());

        let mut next_choices = vec![];
        for i in 0..max_votes {
            let next = choices[i];
            let next = usize::try_from(next).map_err(|_| {
                format!("u64 -> usize conversion on plaintext choice")
            })?;
            // Unset
            if next == 0 {
                continue;
            }
            // choices are offset by 1 to allow for the unset value at 0
            let next = next - 1;

            // A choice of a candidate is represented as that
            // candidate's position in the candidate
            // list, sorted by id. The same sorting order must be used
            // to interpret choices when encoding.
            let candidate = sorted_candidates.get(next);
            let Some(candidate) = candidate else {
                return Err(format!(
                    "Candidate selection out of range {} (length: {})",
                    next,
                    sorted_candidates.len()
                ));
            };

            let selected: usize = if is_preferencial { i } else { 0 };

            let choice = DecodedContestChoice {
                id: candidate.id.clone(),
                selected: selected as i64,
            };

            next_choices.push(choice);
        }

        // Duplicate values will be ignored
        let unique: HashSet<DecodedContestChoice> =
            HashSet::from_iter(next_choices.iter().cloned());
        decoded_contest.choices = unique.clone().into_iter().collect();

        let num_selected_candidates = next_choices.len();

        if unique.len() != num_selected_candidates {
            // FIXME decide if we do something here
            // currently duplicates will be silently ignored, unless
            // they lead to fewer than min_votes values
        }

        // This can happen with unset (= 0) values
        // The opposite is impossible due to the above
        // loop's range 0..max_votes
        if unique.len() < min_votes {
            return Err(format!(
                "Raw ballot vector contained fewer than min_votes choices"
            ));
        }

        let presentation = contest.presentation.clone().unwrap_or_default();

        let invalid_vote_policy_check =
            check_invalid_vote_policy(&presentation, is_explicit_invalid);
        decoded_contest.update(invalid_vote_policy_check);

        let (max_votes_opt, min_votes_opt, maxmin_errors) =
            check_max_min_votes_policy(contest.max_votes, contest.min_votes);
        decoded_contest.update(maxmin_errors);

        if let Some(max_votes_val) = max_votes_opt.clone() {
            let overvote_check = check_over_vote_policy(
                &presentation,
                num_selected_candidates,
                max_votes_val,
            );
            decoded_contest.update(overvote_check);
        }
        if let Some(min_votes_val) = min_votes_opt.clone() {
            let min_check =
                check_min_vote_policy(num_selected_candidates, min_votes_val);
            decoded_contest.update(min_check);
        }

        let under_vote_check = check_under_vote_policy(
            &presentation,
            num_selected_candidates,
            max_votes_opt.clone(),
            min_votes_opt.clone(),
        );
        decoded_contest.update(under_vote_check);

        // handle blank vote policy
        let blank_vote_check = check_blank_vote_policy(
            &presentation,
            num_selected_candidates,
            is_explicit_invalid,
        );
        decoded_contest.update(blank_vote_check);

        if is_preferencial {
            match decoded_contest.validate_preferencial_order() {
                Ok(()) => {}
                Err(errors) => {
                    for error in errors {
                        match error {
                            PreferencialOrderErrorType::DuplicatedPosition => {
                                let check =
                                    check_duplicated_rank_policy(&presentation);
                                decoded_contest.update(check);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(decoded_contest)
    }

    // We are using a "sparse" mixed radix encoding of
    // selections, as opposed to the "dense" encoding
    // used in the single-contest implementation (which has
    // one boolean slot per candidate).
    //
    // In this sparse encoding the number of bases and
    // choices is equal to the maximum number of votes, contest.max_votes.
    // Each of these will optionally contain a selected
    // candidate. The slot's base is
    //
    // number of candidates + 1, such that
    //
    // 0    = unset
    // >0   = the chosen candidate, with an offset of 1.
    //
    // A choice of a candidate is represented as that candidate's
    // position in the candidate list, sorted by id.
    //
    // In addition to sparsity, this implementation supports
    // multi contest ballots. The bases and choices for
    // each contest will be laid out contiguously,
    // in order per contest.id.
    //
    // This encoding only supports plurality, so the
    // order in which selections will be put in the
    // slots has no meaning. This implementation does not
    // support contest level invalid flags.
    //
    // Returns the vector of bases for the mixed radix
    // representation of this ballot (including a explicit invalid base = 2).
    pub fn get_bases(contests: &Vec<Contest>) -> Result<Vec<u64>, String> {
        // the base for explicit invalid ballot slot is 2:
        // 0: not invalid, 1: explicit invalid
        let mut bases: Vec<u64> = vec![2];

        // The set of bases for each contest
        // will be placed in order, for example
        //
        //   contest 0    contest 1     contest 2
        // [a, b, c, d,   e, f, g,     h, i, j, k]
        //
        // The order of the contests is computed
        // sorting by id.
        // The selections must be encoded to and decoded from a ballot
        // following this order, given by contest.id.
        let mut sorted_contests = contests.clone();
        sorted_contests.sort_by_key(|c| c.id.clone());

        for contest in sorted_contests {
            let num_valid_candidates: Result<u64, TryFromIntError> = contest
                .candidates
                .iter()
                .filter(|candidate| !candidate.is_explicit_invalid())
                .count()
                .try_into();

            let num_valid_candidates =
                num_valid_candidates.map_err(|e| e.to_string())?;

            let max_selections = contest.max_votes;
            for _ in 1..=max_selections {
                // + 1: include a per-ballot invalid flag
                bases.push(u64::from(num_valid_candidates + 1));
            }
        }

        Ok(bases)
    }

    /// Returns the contests corresponding to the choices in this ballot
    /// from the given ballot style.
    pub(crate) fn get_contests(
        &self,
        style: &BallotStyle,
    ) -> Result<Vec<Contest>, String> {
        self.choices
            .clone()
            .into_iter()
            .map(|choices| {
                let contest = style
                    .contests
                    .iter()
                    .find(|contest| contest.id == choices.contest_id)
                    .ok_or_else(|| {
                        format!(
                            "Can't find contest with id {} on ballot style",
                            choices.contest_id
                        )
                    })?;

                Ok(contest.clone())
            })
            .collect()
    }

    /// Decodes a bigint into a raw ballot (mixed radix representation).
    pub fn bigint_to_raw_ballot(
        bigint: &BigUint,
        contests: &Vec<Contest>,
    ) -> Result<RawBallotContest, String> {
        let bases = Self::get_bases(contests).map_err(|e| e.to_string())?;

        let choices = Self::decode_mixed_radix(&bases, &bigint)?;

        Ok(RawBallotContest { bases, choices })
    }

    /// Decode the choices in the given mixed radix bigint
    ///
    /// This function is adapted from mixed_radix::decode
    /// to remove its write-in functionality.
    pub fn decode_mixed_radix(
        bases: &Vec<u64>,
        encoded_value: &BigUint,
    ) -> Result<Vec<u64>, String> {
        let mut values: Vec<u64> = vec![];
        let mut accumulator: BigUint = encoded_value.clone();
        let mut index = 0usize;

        while accumulator > Zero::zero() {
            let base: BigUint = bases[index].to_biguint().ok_or_else(|| {
                format!(
                    "Error converting to biguint: bases[index={index:?}]={val}",
                    val = bases[index]
                )
            })?;

            let remainder = &accumulator % &base;
            values.push(remainder.to_u64().ok_or_else(|| {
                format!("Error converting to u64 remainder={remainder}")
            })?);

            accumulator = (&accumulator - &remainder) / &base;
            index += 1;
        }

        // If we didn't run all the bases, fill the rest with zeros
        while index < bases.len() {
            values.push(0);
            index += 1;
        }

        Ok(values)
    }

    /// Compute an upper bound on the number of bytes needed
    /// to encode a multi contest ballot with given contests.
    ///
    /// Returns a conservative upper bound, choosing the maximum
    /// value possible for each base. This value will be greater
    /// than any valid ballot
    pub fn maximum_size_bytes(
        contests: &Vec<Contest>,
    ) -> Result<usize, String> {
        let bases = Self::get_bases(contests)?;

        let choices: Vec<u64> = bases.iter().map(|b| b - 1).collect();

        let max = RawBallotContest::new(bases, choices);

        let bigint = mixed_radix::encode(&max.choices, &max.bases)?;
        let bytes = bigint::encode_bigint_to_bytes(&bigint)?;

        Ok(bytes.len())
    }

    /// Returns a vector of contest ids for this ballot
    ///
    /// Convenience method.
    pub fn get_contest_ids(&self) -> Vec<String> {
        self.choices.iter().map(|c| c.contest_id.clone()).collect()
    }

    /// Returns a bigint representation of this ballot
    ///
    /// Convenience method used in velvet test.
    pub fn encode_to_bigint(
        &self,
        config: &BallotStyle,
    ) -> Result<BigUint, String> {
        let raw_ballot = self.encode_to_raw_ballot(&config)?;

        mixed_radix::encode(&raw_ballot.choices, &raw_ballot.bases)
    }
}

// Multi-contest encoding does not support duplicate ranks for preferential voting.
// This function checks for duplicate ranks when the contest counting algorithm is preferential
// and adds an error to `invalid_errors` in the corresponding contest vote.
// Call this function before the encoding/decoding process with the original ballot to avoid runtime errors.
pub fn check_multi_contest_irv_duplicate_rank(
    ballot_style: &BallotStyle,
    decoded_multi_contests: &Vec<DecodedVoteContest>,
) -> Result<(bool, Vec<DecodedVoteContest>), String> {
    let counting_algorithms =
        ballot_style.get_counting_algorithms().map_err(|err| {
            format!("Error get contests counting algorithm {:?}", err)
        })?;

    let mut found_duplicate_rank_irv = false;
    let mut decoded_multi_contests = decoded_multi_contests.clone();

    for dvc in decoded_multi_contests.iter_mut() {
        let contest_counting_algorithm = counting_algorithms
            .get(&dvc.contest_id)
            .map_or(CountingAlgType::default(), |v| *v);

        if contest_counting_algorithm.is_preferential() {
            if let Err(errors) = dvc.validate_preferencial_order() {
                if errors.iter().any(|e| {
                    matches!(e, PreferencialOrderErrorType::DuplicatedPosition)
                }) {
                    let mut checker_result: CheckerResult = Default::default();
                    checker_result.invalid_errors.push(InvalidPlaintextError {
                        error_type: InvalidPlaintextErrorType::Implicit,
                        candidate_id: None,
                        message: Some(
                            "errors.implicit.duplicatedPosition".to_string(),
                        ),
                        message_map: HashMap::new(),
                    });

                    dvc.update(checker_result);
                    found_duplicate_rank_irv = true;
                }
            }
        }
    }
    Ok((found_duplicate_rank_irv, decoded_multi_contests))
}
/// Test multi-contest reencoding functionality
pub fn test_multi_contest_reencoding(
    decoded_multi_contests: &Vec<DecodedVoteContest>,
    ballot_style: &BallotStyle,
) -> Result<Vec<DecodedVoteContest>, String> {
    // encode ballot
    let (plaintext, _ballot_choices) =
        encode_to_plaintext_decoded_multi_contest(
            decoded_multi_contests,
            ballot_style,
        )
        .map_err(|err| format!("Error encoded decoded contests {:?}", err))?;

    let (found_duplicate_rank_irv, decoded_multi_contests_checked) =
        check_multi_contest_irv_duplicate_rank(
            &ballot_style,
            decoded_multi_contests,
        )
        .map_err(|err| format!("Error check duplicated rank {:?}", err))?;
    if found_duplicate_rank_irv {
        return Ok(decoded_multi_contests_checked.clone());
    }

    let decoded_ballot_choices =
        BallotChoices::decode_from_30_bytes(&plaintext, ballot_style).map_err(
            |err| format!("Error decoding ballot choices {:?}", err),
        )?;

    let output_decoded_contests =
        map_decoded_ballot_choices_to_decoded_contests(
            decoded_ballot_choices,
            &ballot_style.contests,
        )
        .map_err(|err| format!("Error mapping decoded contests {:?}", err))?;

    let input_compare =
        normalize_election(&decoded_multi_contests, ballot_style, true)
            .map_err(|err| format!("Error normalizing input {:?}", err))?;

    let output_compare =
        normalize_election(&output_decoded_contests, ballot_style, true)
            .map_err(|err| format!("Error normalizing output {:?}", err))?;

    if input_compare != output_compare {
        return Err(format!(
            "Consistency check failed. Input != Output, {:?} != {:?}",
            input_compare, output_compare
        ));
    }

    Ok(output_decoded_contests)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::ballot::{BallotStyle, Candidate, Contest};
    use crate::serialization::deserialize_with_path::deserialize_value;
    use rand::{seq::SliceRandom, Rng};
    use serde_json::{json, Value};
    use std::collections::HashSet;
    use uuid::Uuid;

    #[derive(Debug, Clone)]
    pub struct ContestInput {
        pub name: String,
        pub max_votes: usize,
        pub min_votes: usize,
        pub counting_algorithm: CountingAlgType,
        pub candidates_num: usize,
    }

    #[test]
    fn test_multi_contest_reencoding_with_explicit_invalid() {
        // Create test data matching the scenario with explicit invalid
        // candidates
        let ballot_selection_json = json!([{
            "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
            "is_explicit_invalid": true,
            "invalid_errors": [],
            "invalid_alerts": [],
            "choices": [
                {
                    "id": "05614f41-720a-4fd5-842f-58355c0bbdc0",
                    "selected": -1
                },
                {
                    "id": "dfc5a43d-2276-4859-8f76-b0f18f859e59",
                    "selected": -1
                }
            ]
        }]);

        // Create a minimal ballot style for testing
        let election_json = json!({
            "id": "b48da6fd-f7e5-4868-9abb-e23452f373ad",
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
            "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
            "public_key": {
                "public_key": "xEH1M/iIdDkZg1ENaP7yPZWtaOcnYLTmK+sFYmuDJVk",
                "is_demo": false
            },
            "area_id": "dcaf94aa-e2f8-460b-8da6-2a7907c04664",
            "contests": [{
                "id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                "name": "Contest",
                "max_votes": 1,
                "min_votes": 0,
                "winning_candidates_num": 1,
                "voting_type": "non-preferential",
                "counting_algorithm": CountingAlgType::PluralityAtLarge,
                "is_encrypted": true,
                "candidates": [
                    {
                        "id": "05614f41-720a-4fd5-842f-58355c0bbdc0",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "Null",
                        "presentation": {
                            "is_explicit_invalid": true
                        }
                    },
                    {
                        "id": "dfc5a43d-2276-4859-8f76-b0f18f859e59",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "A"
                    }
                ]
            }],
            "election_event_presentation": {
                "contest_encryption_policy": "multiple-contests"
            }
        });

        let decoded_multi_contests: Vec<DecodedVoteContest> =
            deserialize_value(ballot_selection_json)
                .expect("Failed to parse ballot selection");
        let ballot_style: BallotStyle =
            deserialize_value(election_json).expect("Failed to parse election");

        // This test should pass now with the fix for explicit invalid
        // candidates
        let result = test_multi_contest_reencoding(
            &decoded_multi_contests,
            &ballot_style,
        );

        assert!(
            result.is_ok(),
            "Multi-contest reencoding with explicit invalid candidate failed: {:?}",
            result.err()
        );

        // Verify the output maintains the explicit invalid flag
        let output_contests = result.unwrap();
        assert_eq!(output_contests.len(), 1);
        assert_eq!(output_contests[0].is_explicit_invalid, true);
    }

    #[test]
    fn test_multi_contest_reencoding_irv_with_gap() {
        // Create test data matching the scenario with explicit invalid
        // candidates
        let ballot_selection_json = json!([{
            "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
            "is_explicit_invalid": false,
            "invalid_errors": [],
            "invalid_alerts": [],
            "choices": [
                {
                    "id": "05614f41-720a-4fd5-842f-58355c0bbdc0",
                    "selected": 0
                },
                {
                    "id": "dfc5a43d-2276-4859-8f76-b0f18f859e59",
                    "selected": 2
                },
                {
                    "id": "3d3c78cc-df19-447d-a5d1-391268970d67",
                    "selected": -1
                },
            ]
        }]);

        // Create a minimal ballot style for testing
        let election_json = json!({
            "id": "b48da6fd-f7e5-4868-9abb-e23452f373ad",
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
            "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
            "public_key": {
                "public_key": "xEH1M/iIdDkZg1ENaP7yPZWtaOcnYLTmK+sFYmuDJVk",
                "is_demo": false
            },
            "area_id": "dcaf94aa-e2f8-460b-8da6-2a7907c04664",
            "contests": [{
                "id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                "name": "Contest",
                "max_votes": 3,
                "min_votes": 0,
                "winning_candidates_num": 1,
                "voting_type": "preferential",
                "counting_algorithm": CountingAlgType::InstantRunoff,
                "is_encrypted": true,
                "candidates": [
                    {
                        "id": "05614f41-720a-4fd5-842f-58355c0bbdc0",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "B",
                    },
                    {
                        "id": "dfc5a43d-2276-4859-8f76-b0f18f859e59",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "A"
                    },{
                        "id": "3d3c78cc-df19-447d-a5d1-391268970d67",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "C"
                    }
                ]
            }],
            "election_event_presentation": {
                "contest_encryption_policy": "multiple-contests"
            }
        });

        let decoded_multi_contests: Vec<DecodedVoteContest> =
            deserialize_value(ballot_selection_json)
                .expect("Failed to parse ballot selection");
        let ballot_style: BallotStyle =
            deserialize_value(election_json).expect("Failed to parse election");

        let result = test_multi_contest_reencoding(
            &decoded_multi_contests,
            &ballot_style,
        );

        assert!(
            result.is_ok(),
            "Multi-contest reencoding irv with gap failed: {:?}",
            result.err()
        );

        // Verify the output maintains the explicit invalid flag
        let output_contests = result.unwrap();

        let mut in_choices = decoded_multi_contests[0].choices.clone();
        in_choices.sort_by_key(|c| c.id.clone());

        let mut out_choices = output_contests[0].choices.clone();
        out_choices.sort_by_key(|c| c.id.clone());

        assert_eq!(in_choices.len(), out_choices.len());

        for (j, ic) in in_choices.iter().enumerate() {
            let oc = out_choices[j].clone();
            assert_eq!(ic.selected, oc.selected);
        }
    }

    #[test]
    fn test_multi_contest_reencoding_irv_with_duplicated_rank() {
        // Create test data matching the scenario with explicit invalid
        // candidates
        let ballot_selection_json = json!([{
            "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
            "is_explicit_invalid": false,
            "invalid_errors": [],
            "invalid_alerts": [],
            "choices": [
                {
                    "id": "05614f41-720a-4fd5-842f-58355c0bbdc0",
                    "selected": 0
                },
                {
                    "id": "dfc5a43d-2276-4859-8f76-b0f18f859e59",
                    "selected": 1
                },
                {
                    "id": "3d3c78cc-df19-447d-a5d1-391268970d67",
                    "selected": 0
                },
            ]
        }]);

        // Create a minimal ballot style for testing
        let election_json = json!({
            "id": "b48da6fd-f7e5-4868-9abb-e23452f373ad",
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
            "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
            "public_key": {
                "public_key": "xEH1M/iIdDkZg1ENaP7yPZWtaOcnYLTmK+sFYmuDJVk",
                "is_demo": false
            },
            "area_id": "dcaf94aa-e2f8-460b-8da6-2a7907c04664",
            "contests": [{
                "id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                "name": "Contest",
                "max_votes": 3,
                "min_votes": 0,
                "winning_candidates_num": 1,
                "voting_type": "preferential",
                "counting_algorithm": CountingAlgType::InstantRunoff,
                "is_encrypted": true,
                "candidates": [
                    {
                        "id": "05614f41-720a-4fd5-842f-58355c0bbdc0",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "B",
                    },
                    {
                        "id": "dfc5a43d-2276-4859-8f76-b0f18f859e59",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "A"
                    },{
                        "id": "3d3c78cc-df19-447d-a5d1-391268970d67",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "C"
                    }
                ]
            }],
            "election_event_presentation": {
                "contest_encryption_policy": "multiple-contests"
            }
        });

        let decoded_multi_contests: Vec<DecodedVoteContest> =
            deserialize_value(ballot_selection_json)
                .expect("Failed to parse ballot selection");
        let ballot_style: BallotStyle =
            deserialize_value(election_json).expect("Failed to parse election");

        let result = test_multi_contest_reencoding(
            &decoded_multi_contests,
            &ballot_style,
        );

        assert_eq!(result.is_err(), true, "Duplicate rank should cause error");
    }

    #[test]
    fn test_multi_contest_with_different_counting_algorithms() {
        let (election_json, ballot_selection_json) =
            create_random_ballot_election_json(vec![
                ContestInput {
                    name: "Contest 1".to_string(),
                    max_votes: 3,
                    min_votes: 0,
                    counting_algorithm: CountingAlgType::InstantRunoff,
                    candidates_num: 4,
                },
                ContestInput {
                    name: "Contest 2".to_string(),
                    max_votes: 2,
                    min_votes: 0,
                    counting_algorithm: CountingAlgType::PluralityAtLarge,
                    candidates_num: 3,
                },
                ContestInput {
                    name: "Contest 3".to_string(),
                    max_votes: 6,
                    min_votes: 0,
                    counting_algorithm: CountingAlgType::InstantRunoff,
                    candidates_num: 6,
                },
                ContestInput {
                    name: "Contest 4".to_string(),
                    max_votes: 4,
                    min_votes: 0,
                    counting_algorithm: CountingAlgType::PluralityAtLarge,
                    candidates_num: 8,
                },
            ]);

        let decoded_multi_contests: Vec<DecodedVoteContest> =
            deserialize_value(ballot_selection_json)
                .expect("Failed to parse ballot selection");
        let ballot_style: BallotStyle =
            deserialize_value(election_json).expect("Failed to parse election");

        let result = test_multi_contest_reencoding(
            &decoded_multi_contests,
            &ballot_style,
        );

        // Verify the output maintains the explicit invalid flag
        let output_contests = result.unwrap();

        let mut in_choices = decoded_multi_contests.clone();
        in_choices.sort_by_key(|c| c.contest_id.clone());

        let mut out_choices = output_contests.clone();
        out_choices.sort_by_key(|c| c.contest_id.clone());

        assert_eq!(in_choices.len(), out_choices.len());

        for (i, inc) in in_choices.iter().enumerate() {
            let outc = out_choices[i].clone();

            assert_eq!(inc.contest_id, outc.contest_id);
            assert_eq!(inc.choices.len(), outc.choices.len());

            let mut inc = inc.choices.clone();
            inc.sort_by_key(|c| c.id.clone());

            let mut outc = outc.choices.clone();
            outc.sort_by_key(|c| c.clone().id);

            for (j, ic) in inc.iter().enumerate() {
                let oc = outc[j].clone();

                assert_eq!(ic.id, oc.id);
            }
        }
    }

    #[test]
    fn test_roundtrip() {
        test_roundtrip_by_type(CountingAlgType::PluralityAtLarge)
    }

    #[test]
    fn test_roundtrip_irv() {
        test_roundtrip_by_type(CountingAlgType::InstantRunoff)
    }

    fn test_roundtrip_by_type(counting_algorithm: CountingAlgType) {
        let (ballot, style) = random_ballot(5, counting_algorithm);
        println!("{:?}", ballot);

        let max_bytes =
            BallotChoices::maximum_size_bytes(&style.contests).unwrap();
        assert!(max_bytes <= 30);

        println!("max bytes: {:?}", max_bytes);

        let bytes = ballot.encode_to_30_bytes(&style).unwrap();
        println!("bytes {:?}", bytes);

        let back = BallotChoices::decode_from_30_bytes(&bytes, &style).unwrap();

        let mut in_choices = ballot.choices.clone();
        in_choices.sort_by_key(|c| c.contest_id.clone());

        let mut out_choices = back.choices.clone();
        out_choices.sort_by_key(|c| c.contest_id.clone());

        assert_eq!(in_choices.len(), out_choices.len());

        for (i, inc) in in_choices.iter().enumerate() {
            let outc = out_choices[i].clone();

            assert_eq!(inc.contest_id, outc.contest_id);
            assert_eq!(inc.choices.len(), outc.choices.len());

            let mut inc = inc.choices.clone();
            inc.sort_by_key(|c| c.candidate_id.clone());

            let mut outc = outc.choices.clone();
            outc.sort_by_key(|c| c.clone().id);

            for (j, ic) in inc.iter().enumerate() {
                let oc = outc[j].clone();

                assert_eq!(ic.candidate_id, oc.id);
            }
        }
    }

    #[test]
    fn test_mixed_radix_encode() {
        let (ballot, style) =
            random_ballot(5, CountingAlgType::PluralityAtLarge);

        let mixed_radix = ballot.encode_to_raw_ballot(&style).unwrap();

        let mut sorted_choices = ballot.choices.clone();
        sorted_choices.sort_by_key(|c| c.contest_id.clone());

        let mut index: usize = 1;

        for choices in sorted_choices.iter() {
            let contest = style
                .contests
                .iter()
                .find(|c| c.id == choices.contest_id)
                .unwrap();
            let mut candidate_ids: Vec<String> =
                contest.candidates.iter().map(|c| c.id.clone()).collect();
            candidate_ids.sort();

            for choice in choices.choices.iter() {
                if choice.selected < -1 {
                    assert_eq!(mixed_radix.choices[index], 0);
                    index += 1;
                    continue;
                }

                let mut value;
                // skip past unset values
                loop {
                    value = mixed_radix.choices[index] as usize;
                    if value == 0 {
                        index += 1;
                    } else {
                        break;
                    }
                }

                assert_eq!(choice.candidate_id, candidate_ids[value - 1]);

                index += 1;
            }
        }
    }

    fn random_ballot(
        contests: usize,
        counting_algorithm: CountingAlgType,
    ) -> (BallotChoices, BallotStyle) {
        let mut rng = rand::thread_rng();
        let contests: Vec<Contest> = (0..contests)
            .map(|i| {
                let contest_id = i.to_string();

                let min_votes = rng.gen_range(1..5);
                let max_votes = rng.gen_range(min_votes..(min_votes + 5));

                let candidates = rng.gen_range(max_votes..max_votes + 20);

                let candidates: Vec<Candidate> = (0..candidates)
                    .map(|j| {
                        random_candidate(j.to_string(), contest_id.clone())
                    })
                    .collect();

                random_contest(
                    contest_id,
                    candidates,
                    min_votes,
                    max_votes,
                    counting_algorithm.clone(),
                )
            })
            .collect();

        let choices: Vec<ContestChoices> = contests
            .iter()
            .map(|c| random_contest_choices(&c, &counting_algorithm))
            .collect();

        let ballot_style = random_ballot_style(contests);

        let ballot = BallotChoices::new(false, choices);

        (ballot, ballot_style)
    }

    fn random_choice(id: String, max_votes: i64) -> ContestChoice {
        let mut rng = rand::thread_rng();
        // we do not include -1 here as an unset choice will cause the test to
        // fail due to
        // 1) mismatched number of choices (an unset value does not produce a
        //    choice when decoding)
        // 2) number of choices below min_votes
        ContestChoice::new(id, rng.gen_range(0..max_votes) as i64)
    }

    fn random_contest_choices(
        contest: &Contest,
        counting_algorithm: &CountingAlgType,
    ) -> ContestChoices {
        let allow_duplicate_choice_rank = !counting_algorithm.is_preferential();
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(contest.min_votes..=contest.max_votes);

        let mut cs = contest.candidates.clone();
        cs.shuffle(&mut rng);

        let mut used = HashSet::new();

        let choices = cs
            .iter()
            .take(count as usize)
            .map(|c| {
                let choice = if allow_duplicate_choice_rank {
                    random_choice(c.id.clone(), contest.max_votes)
                } else {
                    loop {
                        let choice =
                            random_choice(c.id.clone(), contest.max_votes);
                        if used.insert(choice.selected) {
                            break choice;
                        }
                    }
                };
                choice
            })
            .collect();

        ContestChoices::new(
            contest.id.clone(),
            choices,
            counting_algorithm.clone(),
        )
    }
    fn random_contest(
        id: String,
        candidates: Vec<Candidate>,
        min_votes: i64,
        max_votes: i64,
        counting_algorithm: CountingAlgType,
    ) -> Contest {
        Contest {
            id,
            tenant_id: s(),
            election_event_id: s(),
            election_id: s(),
            name: None,
            name_i18n: None,
            description: None,
            description_i18n: None,
            alias: None,
            alias_i18n: None,
            // set
            max_votes,
            // set
            min_votes,
            winning_candidates_num: 0,
            voting_type: None,
            counting_algorithm: Some(counting_algorithm),
            is_encrypted: true,
            candidates,
            presentation: None,
            created_at: None,
            annotations: None,
        }
    }

    fn random_candidate(id: String, contest_id: String) -> Candidate {
        Candidate {
            id,
            tenant_id: s(),
            election_event_id: s(),
            election_id: s(),
            contest_id: contest_id,
            name: None,
            name_i18n: None,
            description: None,
            description_i18n: None,
            alias: None,
            alias_i18n: None,
            candidate_type: None,
            presentation: None,
            annotations: None,
        }
    }

    fn random_ballot_style(contests: Vec<Contest>) -> BallotStyle {
        BallotStyle {
            id: s(),
            tenant_id: s(),
            election_event_id: s(),
            election_id: s(),
            num_allowed_revotes: None,
            description: None,
            // Set this
            public_key: None,
            area_id: s(),
            area_presentation: Some(AreaPresentation::default()),
            contests,
            election_event_presentation: None,
            election_presentation: None,
            election_dates: None,
            election_event_annotations: None,
            election_annotations: None,
            area_annotations: None,
        }
    }

    /// Creates both:
    /// - election_json
    /// - ballot_selection_json
    ///
    /// Rules:
    /// - Preferential contest:
    ///   selected is either -1 or a unique rank in 0..max_votes-1
    /// - Non-preferential contest:
    ///   selected is either -1 or 0
    ///   and at most max_votes
    pub fn create_random_ballot_election_json(
        contests_input: Vec<ContestInput>,
    ) -> (Value, Value) {
        let tenant_id = Uuid::new_v4().to_string();
        let election_event_id = Uuid::new_v4().to_string();
        let election_id = Uuid::new_v4().to_string();
        let area_id = Uuid::new_v4().to_string();

        let mut contests_json = Vec::new();
        let mut ballot_selection_json = Vec::new();

        let mut rng = rand::thread_rng();

        for contest_input in contests_input {
            let contest_id = Uuid::new_v4().to_string();

            let mut candidates_json = Vec::new();
            let mut candidate_ids = Vec::new();

            for i in 0..contest_input.candidates_num {
                let candidate_id = Uuid::new_v4().to_string();
                candidate_ids.push(candidate_id.clone());

                candidates_json.push(json!({
                    "id": candidate_id,
                    "tenant_id": tenant_id,
                    "election_event_id": election_event_id,
                    "election_id": election_id,
                    "contest_id": contest_id,
                    "name": format!("Candidate {}", i + 1),
                }));
            }

            // Build choices for ballot_selection_json
            let choices = if contest_input.counting_algorithm.is_preferential()
            {
                build_preferential_choices(
                    &candidate_ids,
                    contest_input.max_votes,
                    &mut rng,
                )
            } else {
                build_non_preferential_choices(
                    &candidate_ids,
                    contest_input.max_votes,
                    &mut rng,
                )
            };

            ballot_selection_json.push(json!({
                "contest_id": contest_id,
                "is_explicit_invalid": false,
                "invalid_errors": [],
                "invalid_alerts": [],
                "choices": choices
            }));

            contests_json.push(json!({
            "id": contest_id,
            "tenant_id": tenant_id,
            "election_event_id": election_event_id,
            "election_id": election_id,
            "name": contest_input.name,
            "max_votes": contest_input.max_votes,
            "min_votes": contest_input.min_votes,
            "winning_candidates_num": 1,
            "voting_type": if contest_input.counting_algorithm.is_preferential() {
                "preferential"
            } else {
                "non-preferential"
            },
            "counting_algorithm": contest_input.counting_algorithm,
            "is_encrypted": true,
            "candidates": candidates_json
        }));
        }

        let election_json = json!({
            "id": Uuid::new_v4().to_string(),
            "tenant_id": tenant_id,
            "election_event_id": election_event_id,
            "election_id": election_id,
            "public_key": {
                "public_key": "dummy-public-key",
                "is_demo": false
            },
            "area_id": area_id,
            "contests": contests_json,
            "election_event_presentation": {
                "contest_encryption_policy": "multiple-contests"
            }
        });

        (election_json, Value::Array(ballot_selection_json))
    }

    fn build_preferential_choices(
        candidate_ids: &[String],
        max_votes: usize,
        rng: &mut impl Rng,
    ) -> Vec<Value> {
        let candidates_len = candidate_ids.len();

        // how many candidates will get a rank
        let num_ranked = rng.gen_range(0..=max_votes.min(candidates_len));

        // pick random candidate positions
        let mut shuffled_candidate_indexes: Vec<usize> =
            (0..candidates_len).collect();
        shuffled_candidate_indexes.shuffle(rng);
        let ranked_candidate_indexes =
            &shuffled_candidate_indexes[..num_ranked];

        let mut possible_ranks: Vec<i64> = (0..max_votes as i64).collect();
        possible_ranks.shuffle(rng);
        let chosen_ranks = &possible_ranks[..num_ranked];

        // default all candidates to -1
        let mut selected_values = vec![-1_i64; candidates_len];

        // assign unique random ranks, gaps allowed
        for (&candidate_idx, &rank_value) in
            ranked_candidate_indexes.iter().zip(chosen_ranks.iter())
        {
            selected_values[candidate_idx] = rank_value;
        }

        candidate_ids
            .iter()
            .enumerate()
            .map(|(idx, candidate_id)| {
                json!({
                    "id": candidate_id,
                    "selected": selected_values[idx]
                })
            })
            .collect()
    }

    fn build_non_preferential_choices(
        candidate_ids: &[String],
        max_votes: usize,
        rng: &mut impl Rng,
    ) -> Vec<Value> {
        let candidates_len = candidate_ids.len();
        let num_selected = rng.gen_range(0..=max_votes.min(candidates_len));

        let mut shuffled_indexes: Vec<usize> = (0..candidates_len).collect();
        shuffled_indexes.shuffle(rng);

        let selected_indexes = &shuffled_indexes[..num_selected];

        let mut selected_values = vec![-1_i64; candidates_len];
        for &candidate_idx in selected_indexes {
            selected_values[candidate_idx] = 0;
        }

        candidate_ids
            .iter()
            .enumerate()
            .map(|(idx, candidate_id)| {
                json!({
                    "id": candidate_id,
                    "selected": selected_values[idx]
                })
            })
            .collect()
    }

    use ptree::item::TreeItem;
    use ptree::style::Style;
    use ptree::write_tree;
    use std::borrow::Cow;
    use std::fmt::Display;
    use std::fmt::Formatter;
    use std::io;

    impl TreeItem for ContestChoice {
        type Child = ContestChoice;
        fn write_self<W: io::Write>(
            &self,
            f: &mut W,
            style: &Style,
        ) -> io::Result<()> {
            write!(
                f,
                "{}",
                style.paint(format!(
                    "candidate-{} (selected = {})",
                    self.candidate_id, self.selected
                ))
            )
        }
        fn children(&self) -> Cow<[Self::Child]> {
            Cow::from(vec![])
        }
    }

    impl TreeItem for BallotChoices {
        type Child = ContestChoices;
        fn write_self<W: io::Write>(
            &self,
            f: &mut W,
            style: &Style,
        ) -> io::Result<()> {
            write!(f, "{}", style.paint(format!("ballot")))
        }
        fn children(&self) -> Cow<[Self::Child]> {
            Cow::from(self.choices.clone())
        }
    }

    impl TreeItem for ContestChoices {
        type Child = ContestChoice;
        fn write_self<W: io::Write>(
            &self,
            f: &mut W,
            style: &Style,
        ) -> io::Result<()> {
            write!(f, "{}", style.paint(format!("{}/choices", self.contest_id)))
        }
        fn children(&self) -> Cow<[Self::Child]> {
            Cow::from(self.choices.clone())
        }
    }

    impl TreeItem for Contest {
        type Child = Candidate;
        fn write_self<W: io::Write>(
            &self,
            f: &mut W,
            style: &Style,
        ) -> io::Result<()> {
            write!(f, "{}", style.paint(format!("contest-{}", self.id)))
        }
        fn children(&self) -> Cow<[Self::Child]> {
            Cow::from(self.candidates.clone())
        }
    }

    impl TreeItem for BallotStyle {
        type Child = Contest;
        fn write_self<W: io::Write>(
            &self,
            f: &mut W,
            style: &Style,
        ) -> io::Result<()> {
            write!(f, "{}", style.paint("ballot-style"))
        }
        fn children(&self) -> Cow<[Self::Child]> {
            Cow::from(self.contests.clone())
        }
    }

    impl TreeItem for Candidate {
        type Child = Self;

        fn write_self<W: io::Write>(
            &self,
            f: &mut W,
            style: &Style,
        ) -> io::Result<()> {
            write!(
                f,
                "{}",
                style.paint(format!(
                    "contest-{}/candidate-{}",
                    self.contest_id, self.id
                ))
            )
        }

        fn children(&self) -> Cow<[Self::Child]> {
            Cow::from(vec![])
        }
    }

    fn s() -> String {
        "foo".to_string()
    }

    impl Display for BallotChoices {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            let mut buffer = vec![];
            write_tree(self, &mut buffer).unwrap();
            let s = String::from_utf8(buffer).expect("Invalid UTF-8 sequence");

            write!(f, "{}", s)
        }
    }
}
