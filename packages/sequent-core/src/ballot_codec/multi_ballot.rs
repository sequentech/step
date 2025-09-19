use std::collections::{HashMap, HashSet};
use std::num::TryFromIntError;

// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::bigint;
use super::{vec, RawBallotContest};
use crate::ballot::{BallotStyle, Candidate, Contest, EUnderVotePolicy};
use crate::ballot_codec::check_blank_vote_policy;
use crate::mixed_radix;
use crate::plaintext::{
    DecodedVoteContest, InvalidPlaintextError, InvalidPlaintextErrorType,
};
use num_bigint::BigUint;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use num_bigint::ToBigUint;
use num_traits::{ToPrimitive, Zero};

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
/// This ballot only supports plurality counting
/// algorithms. It does not support write-ins.
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
}
impl ContestChoices {
    pub fn new(contest_id: String, choices: Vec<ContestChoice>) -> Self {
        ContestChoices {
            contest_id,
            // is_explicit_invalid,
            choices,
        }
    }

    /// Return contest choices from a DecodedVoteContest
    ///
    /// Used in testing when generating ballots with the non-sparse
    /// encoding (non multi-contest ballots)
    pub fn from_decoded_vote_contest(dcv: &DecodedVoteContest) -> Self {
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
}
#[derive(
    Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone, Hash,
)]
/// A decoded contest choice contains the candidate_id as a String.
pub struct DecodedContestChoice(pub String);

/// The choices for the set of contests returned when decoding a multi-content
/// ballot.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct DecodedBallotChoices {
    pub is_explicit_invalid: bool,
    pub choices: Vec<DecodedContestChoices>,
    pub serial_number: Option<String>,
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

            let contest_choices = Self::encode_contest(&contest, &plaintext)?;

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

        // We set all values as unset (0) by default
        let mut contest_choices = vec![0u64; max_votes];
        let mut marked = 0;
        for p in &plaintext.choices {
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

            contest_choices[marked] = mark;
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
        // A choice of a candidate is represented as that candidate's
        // position in the candidate list, sorted by id.
        let mut invalid_errors: Vec<InvalidPlaintextError> = vec![];
        let mut invalid_alerts: Vec<InvalidPlaintextError> = vec![];
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

            let choice = DecodedContestChoice(candidate.id.clone());

            next_choices.push(choice);
        }

        // Duplicate values will be ignored
        let unique: HashSet<DecodedContestChoice> =
            HashSet::from_iter(next_choices.iter().cloned());

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

        // handle blank vote policy
        let blankVoteErrors = check_blank_vote_policy(
            &presentation,
            num_selected_candidates,
            is_explicit_invalid,
        );
        invalid_errors.extend(blankVoteErrors.invalid_errors);
        invalid_alerts.extend(blankVoteErrors.invalid_alerts);

        let under_vote_policy =
            presentation.under_vote_policy.clone().unwrap_or_default();
        if under_vote_policy != EUnderVotePolicy::ALLOWED
            && num_selected_candidates < max_votes
            && num_selected_candidates >= min_votes
        {
            invalid_alerts.push(InvalidPlaintextError {
                error_type: InvalidPlaintextErrorType::Implicit,
                candidate_id: None,
                message: Some("errors.implicit.underVote".to_string()),
                message_map: HashMap::from([
                    ("type".to_string(), "alert".to_string()),
                    (
                        "numSelected".to_string(),
                        num_selected_candidates.to_string(),
                    ),
                    ("min".to_string(), min_votes.to_string()),
                    ("max".to_string(), max_votes.to_string()),
                ]),
            });
        }

        let c = DecodedContestChoices::new(
            contest.id.clone(),
            unique.into_iter().collect(),
            invalid_errors,
            invalid_alerts,
        );

        Ok(c)
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
            // Compact encoding only supports plurality
            if contest.get_counting_algorithm().as_str() != "plurality-at-large"
            {
                return Err(format!("get_bases: multi ballot encoding only supports plurality at large, received {}", contest.get_counting_algorithm()));
            }

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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::ballot::{BallotStyle, Candidate, Contest};
    use rand::{seq::SliceRandom, Rng};

    #[test]
    fn test_roundtrip() {
        let (ballot, style) = random_ballot(5);
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
            outc.sort_by_key(|c| c.clone().0);

            for (j, ic) in inc.iter().enumerate() {
                let oc = outc[j].clone();

                assert_eq!(ic.candidate_id, oc.0);
            }
        }
    }

    #[test]
    fn test_mixed_radix_encode() {
        let (ballot, style) = random_ballot(5);

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

    fn random_ballot(contests: usize) -> (BallotChoices, BallotStyle) {
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

                random_contest(contest_id, candidates, min_votes, max_votes)
            })
            .collect();

        let choices: Vec<ContestChoices> = contests
            .iter()
            .map(|c| random_contest_choices(&c))
            .collect();

        let ballot_style = random_ballot_style(contests);

        let ballot = BallotChoices::new(false, choices);

        (ballot, ballot_style)
    }

    fn random_choice(id: String) -> ContestChoice {
        let mut rng = rand::thread_rng();
        // we do not include -1 here as an unset choice will cause the test to
        // fail due to
        // 1) mismatched number of choices (an unset value does not produce a
        //    choice when decoding)
        // 2) number of choices below min_votes
        ContestChoice::new(id, rng.gen_range(0..10) as i64)
    }

    fn random_contest_choices(contest: &Contest) -> ContestChoices {
        let mut rng = rand::thread_rng();
        let count = rng.gen_range(contest.min_votes..=contest.max_votes);

        let mut cs = contest.candidates.clone();
        cs.shuffle(&mut rng);
        let choices = cs
            .iter()
            .take(count as usize)
            .map(|c| random_choice(c.id.clone()))
            .collect();

        ContestChoices::new(contest.id.clone(), choices)
    }

    fn random_contest(
        id: String,
        candidates: Vec<Candidate>,
        min_votes: i64,
        max_votes: i64,
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
            counting_algorithm: Some("plurality-at-large".to_string()),
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
            contests,
            election_event_presentation: None,
            election_presentation: None,
            election_dates: None,
            election_event_annotations: None,
            election_annotations: None,
        }
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
