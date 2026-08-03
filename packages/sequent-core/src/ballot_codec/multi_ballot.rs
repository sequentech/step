use std::collections::{HashMap, HashSet};
use std::num::TryFromIntError;

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::bigint;
use super::{vec, RawBallotContest};
use crate::ballot::{
    AreaPresentation, BallotStyle, Candidate, Contest, DeclineToVotePolicy,
    EUnderVotePolicy, MultiContestEncodingMode,
};
use crate::ballot_codec::{
    check_blank_vote_policy, check_invalid_vote_policy,
    check_max_min_votes_policy, check_min_vote_policy, check_over_vote_policy,
    check_under_vote_policy, validate_contest_configuration,
    ContestCodecContext,
};
use crate::error::BallotError;
use crate::mixed_radix;
use crate::plaintext::{
    map_decoded_ballot_choices_to_decoded_contests, DecodedVoteContest,
    InvalidPlaintextError, InvalidPlaintextErrorType,
};
use crate::types::ceremonies::CountingAlgType;
use num_bigint::BigUint;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::encrypt::encode_to_plaintext_decoded_multi_contest;
use crate::util::normalize_vote::normalize_election;
use num_bigint::ToBigUint;
use num_traits::{ToPrimitive, Zero};

fn is_candidate_selected(
    candidate: &Candidate,
    choices: &[ContestChoice],
) -> bool {
    choices.iter().any(|choice| {
        choice.candidate_id == candidate.id && choice.selected > -1
    })
}

fn is_marker_candidate_selected(
    candidate: Option<&Candidate>,
    choices: &[ContestChoice],
) -> bool {
    candidate
        .map(|candidate| is_candidate_selected(candidate, choices))
        .unwrap_or(false)
}

/// Precomputed constants for encoding and decoding multi-contest ballots.
///
/// Holds one [`ContestCodecContext`] per contest, sorted by contest id (the
/// encoding order), together with the mixed radix bases of the whole ballot.
/// Batch paths that process many ballots for the same contests should build
/// this context once and use the `*_with_context` methods of
/// [`BallotChoices`].
pub struct MultiBallotCodecContext<'a> {
    /// Per-contest codec contexts, sorted by contest id.
    pub contest_contexts: Vec<ContestCodecContext<'a>>,
    /// Choice slots per contest in the multi-contest encoding, aligned
    /// index-for-index with `contest_contexts`. Defaults to `max_votes`;
    /// widened to the candidate count for `EXPANDED_CAPACITY` styles.
    pub vote_slot_counts: Vec<usize>,
    /// Whether the ballot-level decline-to-vote flag is encoded.
    pub include_decline_to_vote: bool,
    /// The mixed radix bases for the whole ballot.
    pub bases: Vec<u64>,
}

impl<'a> MultiBallotCodecContext<'a> {
    /// Validates the contest configurations and precomputes the constants
    /// used to encode and decode multi-contest ballots.
    ///
    /// `mode` is the ballot style's persisted encoding mode, applied
    /// uniformly, not derived per-contest.
    pub fn new(
        contests: &'a [Contest],
        include_decline_to_vote: bool,
        mode: MultiContestEncodingMode,
    ) -> Result<Self, String> {
        // Validated in contest.id order, the same order `new_unchecked`
        // builds contexts in, so the first invalid contest reported here
        // matches the order used everywhere else.
        let mut sorted_contests: Vec<&Contest> = contests.iter().collect();
        sorted_contests.sort_by(|a, b| a.id.cmp(&b.id));
        for contest in sorted_contests {
            validate_contest_configuration(contest)?;
        }

        Self::new_unchecked(contests, include_decline_to_vote, mode)
    }

    /// Precomputes the constants used to encode and decode multi-contest
    /// ballots without validating the contest configurations.
    ///
    /// Used by decode paths that validate each contest configuration at
    /// the point it is decoded, to preserve the order in which errors are
    /// reported (see `BallotChoices::decode`).
    pub fn new_unchecked(
        contests: &'a [Contest],
        include_decline_to_vote: bool,
        mode: MultiContestEncodingMode,
    ) -> Result<Self, String> {
        // The order of the contests is computed sorting by id.
        // The selections must be encoded to and decoded from a ballot
        // following this order, given by contest.id.
        let mut sorted_contests: Vec<&Contest> = contests.iter().collect();
        sorted_contests.sort_by(|a, b| a.id.cmp(&b.id));

        // the base for explicit invalid ballot slot is 2:
        // 0: not invalid, 1: explicit invalid
        let mut bases: Vec<u64> = vec![];
        if include_decline_to_vote {
            bases.push(2);
        }

        let mut contest_contexts = Vec::with_capacity(sorted_contests.len());
        let mut vote_slot_counts = Vec::with_capacity(sorted_contests.len());
        for contest in sorted_contests {
            let context = ContestCodecContext::new_unchecked(contest);

            // Compact encoding only supports plurality
            if contest.get_counting_algorithm()
                != CountingAlgType::PluralityAtLarge
            {
                return Err(format!("get_bases: multi ballot encoding only supports plurality at large, received {}", contest.get_counting_algorithm()));
            }

            let num_valid_candidates: Result<u64, TryFromIntError> =
                context.sorted_normal_candidates.len().try_into();

            let num_valid_candidates =
                num_valid_candidates.map_err(|e| e.to_string())?;

            // Per-contest explicit invalid flag
            bases.push(2);
            if context.explicit_blank_candidate.is_some() {
                bases.push(2);
            }

            let vote_slot_count =
                if mode == MultiContestEncodingMode::EXPANDED_CAPACITY {
                    context.sorted_normal_candidates.len()
                } else {
                    usize::try_from(contest.max_votes).unwrap_or(0)
                };
            for _ in 0..vote_slot_count {
                // + 1: include the unset value.
                bases.push(num_valid_candidates + 1);
            }

            contest_contexts.push(context);
            vote_slot_counts.push(vote_slot_count);
        }

        Ok(MultiBallotCodecContext {
            contest_contexts,
            vote_slot_counts,
            include_decline_to_vote,
            bases,
        })
    }

    /// Decode a mixed radix representation of the ballot given this
    /// context's per-contest codec contexts, sorted by contest id.
    ///
    /// When `validate_contest_configurations` is true, each contest
    /// configuration is validated right before the contest is decoded,
    /// preserving the order in which errors are reported.
    fn decode_sorted_contexts(
        &self,
        validate_contest_configurations: bool,
        raw_ballot: &RawBallotContest,
        serial_number_counter: Option<&mut u32>,
    ) -> Result<DecodedBallotChoices, String> {
        let mut contest_choices: Vec<DecodedContestChoices> = vec![];
        let choices = &raw_ballot.choices;

        // Each contest contributes vote_slot_count slots plus one invalid
        // flag and, when configured, one explicit blank flag.
        let expected_vote_slots: usize = self.vote_slot_counts.iter().sum();

        let expected_blank_slots = self
            .contest_contexts
            .iter()
            .filter(|contest_context| {
                contest_context.explicit_blank_candidate.is_some()
            })
            .count();

        // One per-contest invalid flag, optional per-contest blank flags, and
        // vote_slot_count slots per contest, plus a ballot-level invalid flag
        // when decline-to-vote is enabled.
        let expected_choices = expected_vote_slots
            + self.contest_contexts.len()
            + expected_blank_slots
            + usize::from(self.include_decline_to_vote);
        if choices.len() != expected_choices {
            return Err(format!(
                "Unexpected number of choices {} != {}",
                choices.len(),
                expected_choices
            ));
        }

        let is_explicit_invalid = if self.include_decline_to_vote {
            !choices.is_empty() && (choices[0] > 0)
        } else {
            false
        };
        let mut choice_index = usize::from(self.include_decline_to_vote);

        // Contest contexts are sorted by contest id, the order in which
        // selections are encoded and decoded.
        for (contest_context, &vote_slot_count) in
            self.contest_contexts.iter().zip(&self.vote_slot_counts)
        {
            let contest_is_explicit_invalid: bool = choices
                .get(choice_index)
                .map(|value| *value > 0)
                .unwrap_or(false);
            choice_index += 1;

            let contest_is_explicit_blank =
                if contest_context.explicit_blank_candidate.is_some() {
                    let value = choices
                        .get(choice_index)
                        .map(|value| *value > 0)
                        .unwrap_or(false);
                    choice_index += 1;
                    value
                } else {
                    false
                };

            if validate_contest_configurations {
                validate_contest_configuration(contest_context.contest)?;
            }
            let next = BallotChoices::decode_contest(
                contest_context,
                vote_slot_count,
                &choices[choice_index..],
                contest_is_explicit_invalid,
                contest_is_explicit_blank,
                is_explicit_invalid,
            )?;
            choice_index += vote_slot_count;
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

        Ok(DecodedBallotChoices {
            is_explicit_invalid,
            choices: contest_choices,
            serial_number,
        })
    }
}

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
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct BallotChoices {
    pub is_explicit_invalid: bool,
    pub choices: Vec<ContestChoices>,
    pub counting_algorithm: CountingAlgType,
}
impl BallotChoices {
    pub fn new(
        is_explicit_invalid: bool,
        choices: Vec<ContestChoices>,
        counting_algorithm: CountingAlgType,
    ) -> Self {
        BallotChoices {
            is_explicit_invalid,
            choices,
            counting_algorithm,
        }
    }
}

/// The choices for a contest.
///
/// Does not support write-ins.
#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct ContestChoices {
    pub contest_id: String,
    pub choices: Vec<ContestChoice>,
    pub is_explicit_invalid: bool,
}
impl ContestChoices {
    pub fn new(
        contest_id: String,
        choices: Vec<ContestChoice>,
        is_explicit_invalid: bool,
    ) -> Self {
        ContestChoices {
            contest_id,
            choices,
            is_explicit_invalid,
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
            is_explicit_invalid: dcv.is_explicit_invalid,
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
    pub is_explicit_invalid: bool,
    pub choices: Vec<DecodedContestChoice>,
    pub invalid_errors: Vec<InvalidPlaintextError>,
    pub invalid_alerts: Vec<InvalidPlaintextError>,
}
impl DecodedContestChoices {
    pub fn new(
        contest_id: String,
        choices: Vec<DecodedContestChoice>,
        is_explicit_invalid: bool,
        invalid_errors: Vec<InvalidPlaintextError>,
        invalid_alerts: Vec<InvalidPlaintextError>,
    ) -> Self {
        DecodedContestChoices {
            contest_id,
            choices,
            is_explicit_invalid,
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

impl BallotStyle {
    /// Returns whether decline-to-vote is enabled for this election.
    ///
    /// When disabled (the default), multi-contest ballots omit the ballot-level
    /// decline-to-vote bit from the mixed-radix encoding for backwards
    /// compatibility with existing elections.
    pub fn decline_to_vote_enabled(&self) -> bool {
        self.election_presentation
            .as_ref()
            .and_then(|presentation| {
                presentation.decline_to_vote_policy.clone()
            })
            == Some(DeclineToVotePolicy::ENABLED)
    }

    /// Returns Error if all counting algorithms are not the same.
    pub fn get_counting_algorithm(
        &self,
    ) -> Result<CountingAlgType, BallotError> {
        let first_counting_algorithm: CountingAlgType = self
            .contests
            .first()
            .map(|c| c.get_counting_algorithm())
            .unwrap_or_default();
        match self
            .contests
            .iter()
            .all(|c| c.get_counting_algorithm() == first_counting_algorithm)
        {
            true => Ok(first_counting_algorithm),
            false => Err(BallotError::ConsistencyCheck(
                "Mixing different counting algorithms".to_string(),
            )),
        }
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

    /// Encode this multi-ballot into a mixed radix representation.
    ///
    /// The resulting choice vector is a contiguous list of per-contest
    /// groups, each of size `vote_slot_count` (= `max_votes` in `LEGACY`
    /// mode, or the candidate count in `EXPANDED_CAPACITY` mode), plus one
    /// invalid flag and, when configured, one blank flag per contest, plus
    /// a ballot-level invalid flag when decline-to-vote is enabled.
    fn encode_to_raw_ballot(
        &self,
        config: &BallotStyle,
    ) -> Result<RawBallotContest, String> {
        let contests = self.get_contests(config)?;
        let include_decline_to_vote = config.decline_to_vote_enabled();

        if self.is_explicit_invalid && !include_decline_to_vote {
            return Err(
                "Decline to vote is not enabled for this election".to_string()
            );
        }

        let context = MultiBallotCodecContext::new(
            &contests,
            include_decline_to_vote,
            config.multi_contest_encoding_mode.unwrap_or_default(),
        )?;
        let bases = context.bases.clone();
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

        if include_decline_to_vote {
            let invalid_vote: u64 =
                if self.is_explicit_invalid { 1 } else { 0 };
            choices.push(invalid_vote);
        }

        // Iterate in contest order (contest contexts are sorted by id, the
        // order in which selections must be encoded and decoded)
        for (contest_context, &vote_slot_count) in context
            .contest_contexts
            .iter()
            .zip(&context.vote_slot_counts)
        {
            let contest = contest_context.contest;
            let plaintext = plaintexts_map.get(&contest.id).ok_or(format!(
                "Could not find plaintexts for contest {:?}",
                contest
            ))?;

            let contest_invalid_vote = plaintext.is_explicit_invalid
                || is_marker_candidate_selected(
                    contest_context.explicit_invalid_candidate,
                    &plaintext.choices,
                );
            choices.push(u64::from(contest_invalid_vote));

            if contest_context.explicit_blank_candidate.is_some() {
                choices.push(u64::from(is_marker_candidate_selected(
                    contest_context.explicit_blank_candidate,
                    &plaintext.choices,
                )));
            }

            let contest_choices = self.encode_contest(
                contest_context,
                vote_slot_count,
                plaintext,
            )?;

            // Accumulate the choices for each contest
            choices.extend(contest_choices);
        }

        Ok(RawBallotContest { bases, choices })
    }

    /// Encodes one contest into a choice vector of length
    /// `vote_slot_count`.
    fn encode_contest(
        &self,
        context: &ContestCodecContext,
        vote_slot_count: usize,
        plaintext: &ContestChoices,
    ) -> Result<Vec<u64>, String> {
        let choices_order = match self.counting_algorithm.is_preferential() {
            true => {
                // Setting the choices in order of preference to support
                // preferencial multiballot. When decoding, we will take the
                // vector order to determine the order of preference.
                let mut pref_choices: Vec<ContestChoice> =
                    plaintext.choices.clone();
                pref_choices.sort_by_key(|c| c.selected);
                pref_choices
            }
            false => plaintext.choices.clone(),
        };

        let mut normal_choices = vec![];
        for choice in choices_order {
            let candidate = context
                .candidates_by_id
                .get(choice.candidate_id.as_str())
                .ok_or_else(|| {
                    "choice id is not a valid candidate".to_string()
                })?;
            if candidate.is_explicit_invalid() || candidate.is_explicit_blank()
            {
                continue;
            }
            normal_choices.push(choice);
        }

        if normal_choices.len() > vote_slot_count {
            return Err(format!(
                "Plaintext vector contained more elements than the contest's vote slots ({} > {})", normal_choices.len(), vote_slot_count
            ));
        }

        // We set all values as unset (0) by default
        let mut contest_choices = vec![0u64; vote_slot_count];
        let mut marked = 0;
        for p in &normal_choices {
            let position = context
                .normal_candidate_positions
                .get(p.candidate_id.as_str())
                .ok_or_else(|| {
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

            if marked == vote_slot_count {
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
    /// Structural codec errors still short-circuit decoding:
    ///
    /// * The number of overall choices does not match the expected layout.
    /// * A contest choice is out of range for the contest's candidate set.
    /// * There is an integer conversion error in a layout-defining value.
    ///
    /// Ballot policy checks, including min/max/under/blank/invalid vote
    /// policies, are reported in the decoded contest's invalid errors or
    /// alerts.
    ///
    /// The decoding processes the choices vector as a contiguous list of
    /// contest choices groups, each of size vote_slot_count.
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

        Self::decode_from_bigint(
            &bigint,
            &style.contests,
            style.decline_to_vote_enabled(),
            style.multi_contest_encoding_mode.unwrap_or_default(),
            None,
        )
    }

    /// Returns a decoded ballot from a BigUint
    ///
    /// Convenience method.
    pub fn decode_from_bigint(
        bigint: &BigUint,
        contests: &Vec<Contest>,
        include_decline_to_vote: bool,
        mode: MultiContestEncodingMode,
        serial_number_counter: Option<&mut u32>,
    ) -> Result<DecodedBallotChoices, String> {
        let context = MultiBallotCodecContext::new(
            contests,
            include_decline_to_vote,
            mode,
        )?;

        Self::decode_from_bigint_with_context(
            &context,
            bigint,
            serial_number_counter,
        )
    }

    /// Returns a decoded ballot from a BigUint, using a precomputed codec
    /// context.
    ///
    /// Batch decode paths should build the [`MultiBallotCodecContext`] once
    /// per contest set and call this method for every ballot.
    pub fn decode_from_bigint_with_context(
        context: &MultiBallotCodecContext,
        bigint: &BigUint,
        serial_number_counter: Option<&mut u32>,
    ) -> Result<DecodedBallotChoices, String> {
        let raw_ballot =
            Self::bigint_to_raw_ballot_with_context(context, bigint)?;

        Self::decode_with_context(context, &raw_ballot, serial_number_counter)
    }

    /// Decode a mixed radix representation of the ballot.
    pub fn decode(
        raw_ballot: &RawBallotContest,
        contests: &Vec<Contest>,
        include_decline_to_vote: bool,
        mode: MultiContestEncodingMode,
        serial_number_counter: Option<&mut u32>,
    ) -> Result<DecodedBallotChoices, String> {
        // The contest configurations are validated inside decode_sorted_contexts
        // (its `validate_contest_configurations: true` below) so that errors
        // are reported in the same order in which decoding progresses.
        let context = MultiBallotCodecContext::new_unchecked(
            contests,
            include_decline_to_vote,
            mode,
        )?;

        context.decode_sorted_contexts(true, raw_ballot, serial_number_counter)
    }

    /// Decode a mixed radix representation of the ballot using a
    /// precomputed codec context.
    pub fn decode_with_context(
        context: &MultiBallotCodecContext,
        raw_ballot: &RawBallotContest,
        serial_number_counter: Option<&mut u32>,
    ) -> Result<DecodedBallotChoices, String> {
        // The contest configurations were already validated when the
        // context was built.
        context.decode_sorted_contexts(false, raw_ballot, serial_number_counter)
    }

    /// Decodes one contest in the ballot
    ///
    /// Returns a ContestChoice for the choices slice argument,
    /// which will be read up to position `vote_slot_count`. This
    /// ContestChoice will be added to the overall DecodedBallotChoices.
    /// Values set to 0 (unset) will not return a ContestChoice.
    /// It is the responsibility of the caller to advance the choice slice
    /// as choices are decoded.
    ///
    /// `is_ballot_declined` is the ballot-level decline-to-vote flag. A
    /// declined ballot is intentionally empty in every contest, so the
    /// per-contest selection policy checks (over/min/under/blank vote) are
    /// skipped for it; structural checks and the explicit-invalid policy
    /// check still apply.
    fn decode_contest(
        context: &ContestCodecContext,
        vote_slot_count: usize,
        choices: &[u64],
        is_explicit_invalid: bool,
        is_explicit_blank: bool,
        is_ballot_declined: bool,
    ) -> Result<DecodedContestChoices, String> {
        let contest = context.contest;

        let mut decoded_contest = DecodedContestChoices::new(
            contest.id.clone(),
            vec![],
            is_explicit_invalid,
            vec![],
            vec![],
        );
        // A choice of a candidate is represented as that candidate's
        // position in the non-marker candidate list, sorted by id.
        let sorted_candidates = &context.sorted_normal_candidates;

        let mut next_choices = vec![];
        for i in 0..vote_slot_count {
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
        decoded_contest.choices = unique.clone().into_iter().collect();
        if is_explicit_invalid {
            if let Some(candidate) = context.explicit_invalid_candidate {
                decoded_contest
                    .choices
                    .push(DecodedContestChoice(candidate.id.clone()));
            }
        }
        if is_explicit_blank {
            if let Some(candidate) = context.explicit_blank_candidate {
                decoded_contest
                    .choices
                    .push(DecodedContestChoice(candidate.id.clone()));
            }
        }

        let num_selected_candidates = next_choices.len();
        // Explicit invalid and explicit blank flags count as selections
        // for the min_votes, max_votes, undervote and blank-vote rules.
        let num_selected_with_markers = num_selected_candidates
            + usize::from(is_explicit_invalid)
            + usize::from(is_explicit_blank);

        if unique.len() != num_selected_candidates {
            // FIXME decide if we do something here
            // currently duplicates will be silently ignored, unless
            // they lead to fewer than min_votes values
        }

        let presentation = contest.presentation.clone().unwrap_or_default();

        let invalid_vote_policy_check =
            check_invalid_vote_policy(&presentation, is_explicit_invalid);
        decoded_contest.update(invalid_vote_policy_check);

        let (max_votes_opt, min_votes_opt, maxmin_errors) =
            check_max_min_votes_policy(contest.max_votes, contest.min_votes);
        decoded_contest.update(maxmin_errors);

        // A declined ballot is intentionally empty in every contest, so the
        // per-contest selection policies (over/min/under/blank vote) do not
        // apply to it. Without this, a declined ballot in a contest with
        // min_votes >= 1 or a NOT_ALLOWED blank vote policy would collect
        // implicit invalid errors and be tallied as invalid instead of
        // declined.
        if !is_ballot_declined {
            if let Some(max_votes_val) = max_votes_opt.clone() {
                let overvote_check = check_over_vote_policy(
                    &presentation,
                    num_selected_with_markers,
                    max_votes_val,
                );
                decoded_contest.update(overvote_check);
            }
            if let Some(min_votes_val) = min_votes_opt.clone() {
                let min_check = check_min_vote_policy(
                    num_selected_with_markers,
                    min_votes_val,
                );
                decoded_contest.update(min_check);
            }

            let under_vote_check = check_under_vote_policy(
                &presentation,
                num_selected_with_markers,
                max_votes_opt.clone(),
                min_votes_opt.clone(),
            );
            decoded_contest.update(under_vote_check);

            // handle blank vote policy. A selected explicit blank or explicit
            // invalid marker counts as a selection, so it is not a blank vote.
            let blank_vote_check = check_blank_vote_policy(
                &presentation,
                num_selected_with_markers,
                is_explicit_invalid,
            );
            decoded_contest.update(blank_vote_check);
        }

        Ok(decoded_contest)
    }

    // We are using a "sparse" mixed radix encoding of
    // selections, as opposed to the "dense" encoding
    // used in the single-contest implementation (which has
    // one boolean slot per candidate).
    //
    // In this sparse encoding the number of bases and choices per contest
    // is vote_slot_count. Each slot optionally holds a selected candidate;
    // the slot's base is
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
    // slots has no meaning.
    //
    // Returns the vector of bases for the mixed radix
    // representation of this ballot (one per-contest explicit invalid base = 2,
    // optionally one per-contest explicit blank base = 2, and optionally a
    // ballot-level explicit invalid base = 2 when decline-to-vote is enabled).
    pub fn get_bases(
        contests: &Vec<Contest>,
        include_decline_to_vote: bool,
        mode: MultiContestEncodingMode,
    ) -> Result<Vec<u64>, String> {
        let context = MultiBallotCodecContext::new(
            contests,
            include_decline_to_vote,
            mode,
        )?;

        Ok(context.bases)
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
        include_decline_to_vote: bool,
        mode: MultiContestEncodingMode,
    ) -> Result<RawBallotContest, String> {
        let context = MultiBallotCodecContext::new(
            contests,
            include_decline_to_vote,
            mode,
        )?;

        Self::bigint_to_raw_ballot_with_context(&context, bigint)
    }

    /// Decodes a bigint into a raw ballot (mixed radix representation),
    /// using a precomputed codec context.
    fn bigint_to_raw_ballot_with_context(
        context: &MultiBallotCodecContext,
        bigint: &BigUint,
    ) -> Result<RawBallotContest, String> {
        let bases = context.bases.clone();

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

    /// Payload limit enforced by `vec::encode_vec_to_array`.
    pub const MAX_SIZE_BYTES: usize = 29;

    /// Compute an upper bound on the number of bytes needed
    /// to encode a multi contest ballot with given contests.
    ///
    /// Returns a conservative upper bound, choosing the maximum
    /// value possible for each base. This value will be greater
    /// than any valid ballot
    pub fn maximum_size_bytes(
        contests: &Vec<Contest>,
        include_decline_to_vote: bool,
        mode: MultiContestEncodingMode,
    ) -> Result<usize, String> {
        let bases = Self::get_bases(contests, include_decline_to_vote, mode)?;

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
        normalize_election(decoded_multi_contests, ballot_style, true)
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
    use crate::ballot::{
        BallotStyle, Candidate, Contest, DeclineToVotePolicy,
        ElectionPresentation,
    };
    use crate::plaintext::DecodedVoteChoice;
    use crate::serialization::deserialize_with_path::deserialize_value;
    use rand::{seq::SliceRandom, Rng};
    use serde_json::json;

    #[test]
    fn test_multi_contest_reencoding_with_explicit_invalid() {
        // Create test data matching the scenario with explicit invalid
        // candidates
        let ballot_selection_json = json!([{
            "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
            "is_explicit_invalid": true,
            "is_decline_to_vote": false,
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
    fn test_multi_contest_explicit_blank_uses_dedicated_flag() {
        let mut contest = test_contest("1", 3, 2);
        mark_explicit_blank(&mut contest.candidates[2]);
        let blank_id = contest.candidates[2].id.clone();
        let style = test_ballot_style(vec![contest.clone()]);
        let ballot = BallotChoices::new(
            false,
            vec![ContestChoices::new(
                contest.id.clone(),
                vec![ContestChoice::new(blank_id.clone(), 0)],
                false,
            )],
            CountingAlgType::PluralityAtLarge,
        );

        let raw = ballot
            .encode_to_raw_ballot(&style)
            .expect("encoding should succeed");

        assert_eq!(raw.bases, vec![2, 2, 3, 3]);
        assert_eq!(raw.choices, vec![0, 1, 0, 0]);

        let decoded = BallotChoices::decode(
            &raw,
            &style.contests,
            false,
            style.multi_contest_encoding_mode.unwrap_or_default(),
            None,
        )
        .expect("decoding should succeed");
        assert_eq!(decoded.choices.len(), 1);
        assert_eq!(
            decoded.choices[0].choices,
            vec![DecodedContestChoice(blank_id)]
        );
    }

    #[test]
    fn test_multi_contest_explicit_invalid_candidate_uses_dedicated_flag() {
        let mut contest = test_contest("1", 2, 1);
        mark_explicit_invalid(&mut contest.candidates[0]);
        let invalid_id = contest.candidates[0].id.clone();
        let style = test_ballot_style(vec![contest.clone()]);
        let ballot = BallotChoices::new(
            false,
            vec![ContestChoices::new(
                contest.id.clone(),
                vec![ContestChoice::new(invalid_id.clone(), 0)],
                false,
            )],
            CountingAlgType::PluralityAtLarge,
        );

        let raw = ballot
            .encode_to_raw_ballot(&style)
            .expect("encoding should succeed");

        assert_eq!(raw.bases, vec![2, 2]);
        assert_eq!(raw.choices, vec![1, 0]);

        let decoded = BallotChoices::decode(
            &raw,
            &style.contests,
            false,
            style.multi_contest_encoding_mode.unwrap_or_default(),
            None,
        )
        .expect("decoding should succeed");
        assert_eq!(decoded.choices.len(), 1);
        assert!(decoded.choices[0].is_explicit_invalid);
        assert_eq!(
            decoded.choices[0].choices,
            vec![DecodedContestChoice(invalid_id)]
        );
    }

    #[test]
    fn test_multi_contest_under_min_reports_error_instead_of_throwing() {
        let contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("a".to_string(), "1".to_string()),
                random_candidate("b".to_string(), "1".to_string()),
            ],
            1,
            1,
        );
        let style = test_ballot_style(vec![contest.clone()]);
        let decoded = decoded_vote_contest(&contest, false, &[]);

        let result = test_multi_contest_reencoding(&vec![decoded], &style)
            .expect("under-min ballots should be encoded and reported");

        assert!(has_invalid_error(&result[0], "errors.implicit.selectedMin"));
    }

    #[test]
    fn test_multi_contest_explicit_blank_satisfies_min_vote_policy() {
        let mut contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("a".to_string(), "1".to_string()),
                random_candidate("blank".to_string(), "1".to_string()),
            ],
            1,
            1,
        );
        mark_explicit_blank(&mut contest.candidates[1]);
        let blank_id = contest.candidates[1].id.clone();
        let style = test_ballot_style(vec![contest.clone()]);
        let decoded =
            decoded_vote_contest(&contest, false, &[blank_id.clone()]);

        let result = test_multi_contest_reencoding(&vec![decoded], &style)
            .expect("explicit blank should satisfy min vote policy");

        assert!(!has_invalid_error(
            &result[0],
            "errors.implicit.selectedMin"
        ));
        let blank_choice = result[0]
            .choices
            .iter()
            .find(|choice| choice.id == blank_id)
            .expect("explicit blank candidate should be present");
        assert_eq!(blank_choice.selected, 0);
    }

    #[test]
    fn test_multi_contest_explicit_invalid_candidate_satisfies_min() {
        let mut contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("invalid".to_string(), "1".to_string()),
                random_candidate("a".to_string(), "1".to_string()),
            ],
            1,
            1,
        );
        mark_explicit_invalid(&mut contest.candidates[0]);
        let invalid_id = contest.candidates[0].id.clone();
        let style = test_ballot_style(vec![contest.clone()]);
        let decoded =
            decoded_vote_contest(&contest, false, &[invalid_id.clone()]);

        let decoded_contests = vec![decoded];
        let (plaintext, _ballot_choices) =
            encode_to_plaintext_decoded_multi_contest(
                &decoded_contests,
                &style,
            )
            .expect("explicit invalid candidate should encode");
        let decoded_ballot_choices =
            BallotChoices::decode_from_30_bytes(&plaintext, &style)
                .expect("explicit invalid candidate should decode");
        let result = map_decoded_ballot_choices_to_decoded_contests(
            decoded_ballot_choices,
            &style.contests,
        )
        .expect("explicit invalid candidate should map");

        assert!(result[0].is_explicit_invalid);
        assert!(!has_invalid_error(
            &result[0],
            "errors.implicit.selectedMin"
        ));
        let invalid_choice = result[0]
            .choices
            .iter()
            .find(|choice| choice.id == invalid_id)
            .expect("explicit invalid candidate should be present");
        assert_eq!(invalid_choice.selected, 0);
    }

    #[test]
    fn test_multi_contest_explicit_invalid_satisfies_min_vote_policy() {
        let contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("a".to_string(), "1".to_string()),
                random_candidate("b".to_string(), "1".to_string()),
            ],
            1,
            1,
        );
        let style = test_ballot_style(vec![contest.clone()]);
        let decoded = decoded_vote_contest(&contest, true, &[]);

        let result = test_multi_contest_reencoding(&vec![decoded], &style)
            .expect("explicit invalid should satisfy min vote policy");

        assert!(result[0].is_explicit_invalid);
        assert!(!has_invalid_error(
            &result[0],
            "errors.implicit.selectedMin"
        ));
    }

    #[test]
    fn test_multi_contest_explicit_blank_counts_towards_under_vote_alert() {
        let mut contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("a".to_string(), "1".to_string()),
                random_candidate("b".to_string(), "1".to_string()),
                random_candidate("blank".to_string(), "1".to_string()),
            ],
            1,
            2,
        );
        mark_explicit_blank(&mut contest.candidates[2]);
        contest
            .presentation
            .get_or_insert_with(Default::default)
            .under_vote_policy = Some(EUnderVotePolicy::WARN);
        contest
            .presentation
            .get_or_insert_with(Default::default)
            .blank_vote_policy = Some(crate::ballot::EBlankVotePolicy::WARN);
        let blank_id = contest.candidates[2].id.clone();
        let style = test_ballot_style(vec![contest.clone()]);
        let decoded =
            decoded_vote_contest(&contest, false, &[blank_id.clone()]);

        let result = test_multi_contest_reencoding(&vec![decoded], &style)
            .expect("explicit blank should encode and decode");

        assert!(has_invalid_alert(&result[0], "errors.implicit.underVote"));
        assert!(!has_invalid_error(
            &result[0],
            "errors.implicit.selectedMin"
        ));
        assert!(
            !has_invalid_alert(&result[0], "errors.implicit.blankVote")
                && !has_invalid_error(&result[0], "errors.implicit.blankVote"),
            "Explicit blank should not be reported as a blank vote"
        );
    }

    #[test]
    fn test_multi_contest_explicit_blank_counts_towards_max_votes() {
        let mut contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("a".to_string(), "1".to_string()),
                random_candidate("b".to_string(), "1".to_string()),
                random_candidate("blank".to_string(), "1".to_string()),
            ],
            0,
            1,
        );
        mark_explicit_blank(&mut contest.candidates[2]);
        let blank_id = contest.candidates[2].id.clone();
        let normal_id = contest.candidates[0].id.clone();
        let style = test_ballot_style(vec![contest.clone()]);
        // A normal candidate plus the explicit blank marker exceed
        // max_votes = 1.
        let decoded = decoded_vote_contest(
            &contest,
            false,
            &[normal_id.clone(), blank_id.clone()],
        );

        let result = test_multi_contest_reencoding(&vec![decoded], &style)
            .expect("ballot should encode and decode");

        assert!(
            has_invalid_error(&result[0], "errors.implicit.selectedMax"),
            "Explicit blank should count towards max_votes"
        );
    }

    #[test]
    fn test_multi_contest_expanded_capacity_allows_over_vote() {
        let contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("a".to_string(), "1".to_string()),
                random_candidate("b".to_string(), "1".to_string()),
                random_candidate("c".to_string(), "1".to_string()),
                random_candidate("d".to_string(), "1".to_string()),
                random_candidate("e".to_string(), "1".to_string()),
            ],
            0,
            2,
        );
        // over_vote_policy defaults to ALLOWED (ContestPresentation::default()).
        let selected_ids: Vec<String> = contest.candidates[0..4]
            .iter()
            .map(|c| c.id.clone())
            .collect();
        let mut style = test_ballot_style(vec![contest.clone()]);
        style.multi_contest_encoding_mode =
            Some(MultiContestEncodingMode::EXPANDED_CAPACITY);
        let decoded = decoded_vote_contest(&contest, false, &selected_ids);

        let result = test_multi_contest_reencoding(&vec![decoded], &style)
            .expect(
                "expanded capacity should encode a selection past max_votes",
            );

        let selected: Vec<&str> = result[0]
            .choices
            .iter()
            .filter(|choice| choice.selected > -1)
            .map(|choice| choice.id.as_str())
            .collect();
        assert_eq!(selected.len(), 4);
        assert!(selected_ids
            .iter()
            .all(|id| selected.contains(&id.as_str())));
        assert!(has_invalid_error(&result[0], "errors.implicit.selectedMax"));
        assert!(!has_invalid_alert(
            &result[0],
            "errors.implicit.selectedMax"
        ));
    }

    #[test]
    fn test_multi_contest_legacy_rejects_same_over_vote() {
        let contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("a".to_string(), "1".to_string()),
                random_candidate("b".to_string(), "1".to_string()),
                random_candidate("c".to_string(), "1".to_string()),
                random_candidate("d".to_string(), "1".to_string()),
                random_candidate("e".to_string(), "1".to_string()),
            ],
            0,
            2,
        );
        let selected_ids: Vec<String> = contest.candidates[0..4]
            .iter()
            .map(|c| c.id.clone())
            .collect();
        // multi_contest_encoding_mode left unset (LEGACY), unlike the test above.
        let style = test_ballot_style(vec![contest.clone()]);
        let decoded = decoded_vote_contest(&contest, false, &selected_ids);

        let err = test_multi_contest_reencoding(&vec![decoded], &style)
            .expect_err("legacy mode should reject a selection past max_votes");

        assert!(err.contains("vote slots"));
    }

    #[test]
    fn test_multi_contest_explicit_invalid_counts_towards_under_vote_alert() {
        let mut contest = random_contest(
            "1".to_string(),
            vec![
                random_candidate("invalid".to_string(), "1".to_string()),
                random_candidate("a".to_string(), "1".to_string()),
                random_candidate("b".to_string(), "1".to_string()),
            ],
            1,
            2,
        );
        mark_explicit_invalid(&mut contest.candidates[0]);
        contest
            .presentation
            .get_or_insert_with(Default::default)
            .under_vote_policy = Some(EUnderVotePolicy::WARN);
        let invalid_id = contest.candidates[0].id.clone();
        let style = test_ballot_style(vec![contest.clone()]);
        let decoded =
            decoded_vote_contest(&contest, false, &[invalid_id.clone()]);

        let decoded_contests = vec![decoded];
        let (plaintext, _ballot_choices) =
            encode_to_plaintext_decoded_multi_contest(
                &decoded_contests,
                &style,
            )
            .expect("explicit invalid candidate should encode");
        let decoded_ballot_choices =
            BallotChoices::decode_from_30_bytes(&plaintext, &style)
                .expect("explicit invalid candidate should decode");
        let result = map_decoded_ballot_choices_to_decoded_contests(
            decoded_ballot_choices,
            &style.contests,
        )
        .expect("explicit invalid candidate should map");

        assert!(result[0].is_explicit_invalid);
        assert!(has_invalid_alert(&result[0], "errors.implicit.underVote"));
        assert!(!has_invalid_error(
            &result[0],
            "errors.implicit.selectedMin"
        ));
    }

    #[test]
    fn test_multi_contest_reencoding_with_invalid_vote_multi_contests() {
        // Create test data matching the scenario with explicit invalid
        // candidates
        let ballot_selection_json = json!([{
            "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
            "is_explicit_invalid": true,
            "is_decline_to_vote": false,
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
                },{
                    "id": "3d3c78cc-df19-447d-a5d1-391268970d67",
                    "selected": -1
                }
            ]
        },
        {
            "contest_id": "ba08a9eb-49c9-44d7-a25e-b2e142e17b0b",
            "is_explicit_invalid": false,
            "is_decline_to_vote": false,
            "invalid_errors": [],
            "invalid_alerts": [],
            "choices": [
                {
                    "id": "05e14f41-720a-4fd5-842f-58355c0bbdc0",
                    "selected": -1
                },
                {
                    "id": "dfc5243d-2276-4859-8f76-b0f18f859e59",
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
            "contests": [
                {
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
                        "name": "B"
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
            },{
                "id": "ba08a9eb-49c9-44d7-a25e-b2e142e17b0b",
                "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                "name": "Contest2",
                "max_votes": 1,
                "min_votes": 0,
                "winning_candidates_num": 1,
                "voting_type": "non-preferential",
                "counting_algorithm": CountingAlgType::PluralityAtLarge,
                "is_encrypted": true,
                "candidates": [
                    {
                        "id": "05e14f41-720a-4fd5-842f-58355c0bbdc0",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "B"
                    },
                    {
                        "id": "dfc5243d-2276-4859-8f76-b0f18f859e59",
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

        // This test should pass now for explicit invalid per-contest candidates
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
        assert_eq!(output_contests.len(), 2);
        assert_eq!(output_contests[0].is_explicit_invalid, true);
        assert_eq!(output_contests[1].is_explicit_invalid, false);
    }

    #[test]
    fn test_multi_contest_reencoding_with_decline_to_vote() {
        // Create test data matching the scenario with decline to vote
        let ballot_selection_json = json!([{
            "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
            "is_explicit_invalid": false,
            "is_decline_to_vote": true,
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
                },{
                    "id": "3d3c78cc-df19-447d-a5d1-391268970d67",
                    "selected": -1
                }
            ]
        },
        {
            "contest_id": "ba08a9eb-49c9-44d7-a25e-b2e142e17b0b",
            "is_explicit_invalid": false,
            "is_decline_to_vote": true,
            "invalid_errors": [],
            "invalid_alerts": [],
            "choices": [
                {
                    "id": "05e14f41-720a-4fd5-842f-58355c0bbdc0",
                    "selected": -1
                },
                {
                    "id": "dfc5243d-2276-4859-8f76-b0f18f859e59",
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
            "contests": [
                {
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
                        "name": "B"
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
            },{
                "id": "ba08a9eb-49c9-44d7-a25e-b2e142e17b0b",
                "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                "name": "Contest2",
                "max_votes": 1,
                "min_votes": 0,
                "winning_candidates_num": 1,
                "voting_type": "non-preferential",
                "counting_algorithm": CountingAlgType::PluralityAtLarge,
                "is_encrypted": true,
                "candidates": [
                    {
                        "id": "05e14f41-720a-4fd5-842f-58355c0bbdc0",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "B"
                    },
                    {
                        "id": "dfc5243d-2276-4859-8f76-b0f18f859e59",
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
            },
            "election_presentation": {
                "decline_to_vote_policy": "enabled"
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
            "Multi-contest reencoding with decline to vote failed: {:?}",
            result.err()
        );

        let output_contests = result.unwrap();
        assert_eq!(output_contests.len(), 2);
        assert_eq!(output_contests[0].is_decline_to_vote, true);
        assert_eq!(output_contests[1].is_decline_to_vote, true);
    }

    #[test]
    fn test_multi_contest_reencoding_with_invalid_decline_to_vote() {
        // Create test data matching the scenario with decline to vote but invalid (only one contest is decline to vote)
        let ballot_selection_json = json!([
            {
            "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
            "is_explicit_invalid": false,
            "is_decline_to_vote": true,
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
                },{
                    "id": "3d3c78cc-df19-447d-a5d1-391268970d67",
                    "selected": -1
                }
            ]
        },
        {
            "contest_id": "ba08a9eb-49c9-44d7-a25e-b2e142e17b0b",
            "is_explicit_invalid": false,
            "is_decline_to_vote": false,
            "invalid_errors": [],
            "invalid_alerts": [],
            "choices": [
                {
                    "id": "05e14f41-720a-4fd5-842f-58355c0bbdc0",
                    "selected": -1
                },
                {
                    "id": "dfc5243d-2276-4859-8f76-b0f18f859e59",
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
            "contests": [
                {
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
                        "name": "B"
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
            },{
                "id": "ba08a9eb-49c9-44d7-a25e-b2e142e17b0b",
                "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                "name": "Contest2",
                "max_votes": 1,
                "min_votes": 0,
                "winning_candidates_num": 1,
                "voting_type": "non-preferential",
                "counting_algorithm": CountingAlgType::PluralityAtLarge,
                "is_encrypted": true,
                "candidates": [
                    {
                        "id": "05e14f41-720a-4fd5-842f-58355c0bbdc0",
                        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                        "name": "B"
                    },
                    {
                        "id": "dfc5243d-2276-4859-8f76-b0f18f859e59",
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
            },
            "election_presentation": {
                "decline_to_vote_policy": "enabled"
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
            result.is_err(),
            "Expected invalid mixed decline-to-vote ballot to fail, got: {:?}",
            result.ok()
        );
        let err = result.unwrap_err();
        assert!(
            err.contains(
                "Invalid number of contests with decline to vote 1 != 2"
            ),
            "Unexpected error: {err}"
        );
    }

    #[test]
    fn test_map_decoded_ballot_choices_is_decline_to_vote_mapping() {
        let contests = vec![
            test_contest("contest-a", 2, 1),
            test_contest("contest-b", 2, 1),
        ];

        for (ballot_decline_to_vote, expected_decline_to_vote) in
            [(false, false), (true, true)]
        {
            let decoded_ballot_choices = DecodedBallotChoices {
                is_explicit_invalid: ballot_decline_to_vote,
                choices: vec![
                    DecodedContestChoices::new(
                        "contest-a".to_string(),
                        vec![],
                        false,
                        vec![],
                        vec![],
                    ),
                    DecodedContestChoices::new(
                        "contest-b".to_string(),
                        vec![],
                        false,
                        vec![],
                        vec![],
                    ),
                ],
                serial_number: None,
            };

            let mapped = map_decoded_ballot_choices_to_decoded_contests(
                decoded_ballot_choices,
                &contests,
            )
            .expect("mapping decoded ballot choices should succeed");

            assert_eq!(mapped.len(), 2);
            for contest in &mapped {
                assert_eq!(
                    contest.is_decline_to_vote, expected_decline_to_vote,
                    "ballot is_explicit_invalid={ballot_decline_to_vote} should map to is_decline_to_vote={expected_decline_to_vote} on every contest"
                );
            }

            assert_eq!(mapped[0].is_explicit_invalid, false);
            assert_eq!(mapped[1].is_explicit_invalid, false);
        }
    }

    #[test]
    fn test_decline_to_vote_rejected_when_policy_disabled() {
        let style = test_ballot_style(vec![test_contest("1", 2, 1)]);
        let ballot = BallotChoices::new(
            true,
            vec![ContestChoices::new("1".to_string(), vec![], false)],
            CountingAlgType::PluralityAtLarge,
        );

        let err = ballot.encode_to_30_bytes(&style).expect_err(
            "decline to vote should be rejected when policy is disabled",
        );
        assert!(
            err.contains("Decline to vote is not enabled for this election"),
            "Unexpected error: {err}"
        );
    }

    #[test]
    fn test_declined_ballot_skips_selection_policy_checks() {
        // A contest with min_votes = 1: an empty non-declined ballot
        // collects errors.implicit.selectedMin, but a declined ballot is
        // intentionally empty and must not collect selection policy errors,
        // otherwise the tally classifies it as implicit invalid instead of
        // declined.
        let candidates: Vec<Candidate> = (0..3)
            .map(|i| random_candidate(i.to_string(), "1".to_string()))
            .collect();
        let contest = random_contest("1".to_string(), candidates, 1, 2);
        let mut style = test_ballot_style(vec![contest]);
        style.election_presentation = Some(ElectionPresentation {
            decline_to_vote_policy: Some(DeclineToVotePolicy::ENABLED),
            ..Default::default()
        });

        // Control: an empty ballot without the decline flag collects the
        // min-votes policy error.
        let empty_ballot = BallotChoices::new(
            false,
            vec![ContestChoices::new("1".to_string(), vec![], false)],
            CountingAlgType::PluralityAtLarge,
        );
        let plaintext = empty_ballot
            .encode_to_30_bytes(&style)
            .expect("empty ballot should encode");
        let decoded = BallotChoices::decode_from_30_bytes(&plaintext, &style)
            .expect("empty ballot should decode");
        assert!(!decoded.is_explicit_invalid);
        assert!(
            decoded.choices[0].invalid_errors.iter().any(|error| {
                error.message.as_deref() == Some("errors.implicit.selectedMin")
            }),
            "empty non-declined ballot should collect the min-votes error"
        );

        // Declined ballot: no selection policy errors must be collected.
        let declined_ballot = BallotChoices::new(
            true,
            vec![ContestChoices::new("1".to_string(), vec![], false)],
            CountingAlgType::PluralityAtLarge,
        );
        let plaintext = declined_ballot
            .encode_to_30_bytes(&style)
            .expect("declined ballot should encode");
        let decoded = BallotChoices::decode_from_30_bytes(&plaintext, &style)
            .expect("declined ballot should decode");
        assert!(decoded.is_explicit_invalid);
        assert!(
            decoded.choices[0].invalid_errors.is_empty(),
            "declined ballot must not collect selection policy errors: {:?}",
            decoded.choices[0].invalid_errors
        );
        assert!(
            decoded.choices[0].invalid_alerts.is_empty(),
            "declined ballot must not collect selection policy alerts: {:?}",
            decoded.choices[0].invalid_alerts
        );

        // The mapped contests classify as declined and blank, not invalid.
        let mapped = map_decoded_ballot_choices_to_decoded_contests(
            decoded,
            &style.contests,
        )
        .expect("mapping decoded ballot choices should succeed");
        assert!(mapped[0].is_decline_to_vote);
        assert!(!mapped[0].is_invalid());
        assert!(mapped[0].is_blank());
    }

    #[test]
    fn test_get_bases_output_shape() {
        let contest_a = test_contest("a", 3, 2);
        let contest_b = test_contest("b", 5, 1);
        let contests = vec![contest_b, contest_a];

        let bases = BallotChoices::get_bases(
            &contests,
            false,
            MultiContestEncodingMode::LEGACY,
        )
        .expect("get_bases should succeed");

        // per-contest flag + max_votes slots for each contest, sorted by id
        assert_eq!(bases.len(), 5);
        assert_eq!(bases[0], 2);
        assert_eq!(bases[1], 4);
        assert_eq!(bases[2], 4);
        assert_eq!(bases[3], 2);
        assert_eq!(bases[4], 6);

        let bases = BallotChoices::get_bases(
            &contests,
            true,
            MultiContestEncodingMode::LEGACY,
        )
        .expect("get_bases should succeed");

        assert_eq!(bases.len(), 6);
        assert_eq!(bases[0], 2);
        assert_eq!(bases[1], 2);
        assert_eq!(bases[2], 4);
        assert_eq!(bases[3], 4);
        assert_eq!(bases[4], 2);
        assert_eq!(bases[5], 6);
    }

    #[test]
    fn test_encode_to_raw_ballot_choices_layout() {
        let style = test_ballot_style(vec![
            test_contest("1", 2, 2),
            test_contest("2", 3, 1),
        ]);

        let candidate_id = style.contests[1].candidates[0].id.clone();
        let ballot = BallotChoices::new(
            false,
            vec![
                ContestChoices::new("1".to_string(), vec![], true),
                ContestChoices::new(
                    "2".to_string(),
                    vec![ContestChoice::new(candidate_id.clone(), 0)],
                    false,
                ),
            ],
            CountingAlgType::PluralityAtLarge,
        );

        let raw = ballot
            .encode_to_raw_ballot(&style)
            .expect("encoding should succeed");

        assert_eq!(raw.choices.len(), raw.bases.len());

        let mut sorted_contests = style.contests.clone();
        sorted_contests.sort_by_key(|c| c.id.clone());

        let mut index = 0;
        for contest in &sorted_contests {
            let contest_choices = ballot
                .choices
                .iter()
                .find(|c| c.contest_id == contest.id)
                .expect("contest choices should exist");

            assert_eq!(
                raw.choices[index],
                u64::from(contest_choices.is_explicit_invalid),
                "per-contest invalid flag at index {index} for contest {}",
                contest.id
            );
            index += 1;

            let max_votes: usize = contest
                .max_votes
                .try_into()
                .expect("max_votes should fit in usize");
            index += max_votes;
        }

        assert_eq!(
            index,
            raw.choices.len(),
            "choices vector should contain exactly one flag and max_votes slots per contest"
        );
    }

    #[test]
    fn test_roundtrip() {
        let (ballot, style) = random_ballot(5);
        println!("{:?}", ballot);

        let max_bytes = BallotChoices::maximum_size_bytes(
            &style.contests,
            style.decline_to_vote_enabled(),
            style.multi_contest_encoding_mode.unwrap_or_default(),
        )
        .unwrap();
        // `encode_vec_to_array` reserves byte 0 for the length prefix, so the
        // payload limit is 29, not the 30-byte array size. Asserting against
        // 30 would let this test pass for a style that cannot encode.
        assert!(max_bytes <= BallotChoices::MAX_SIZE_BYTES);

        println!("max bytes: {:?}", max_bytes);

        let bytes = ballot.encode_to_30_bytes(&style).unwrap();
        println!("bytes {:?}", bytes);

        let back = BallotChoices::decode_from_30_bytes(&bytes, &style).unwrap();

        let mut in_choices = ballot.choices.clone();
        in_choices.sort_by_key(|c| c.contest_id.clone());

        let mut out_choices = back.choices.clone();
        out_choices.sort_by_key(|c| c.contest_id.clone());

        assert_eq!(ballot.is_explicit_invalid, back.is_explicit_invalid);
        assert_eq!(in_choices.len(), out_choices.len());

        for (i, inc) in in_choices.iter().enumerate() {
            let outc = out_choices[i].clone();

            assert_eq!(inc.contest_id, outc.contest_id);
            assert_eq!(inc.is_explicit_invalid, outc.is_explicit_invalid);
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

    // Quarantined: flaky. `random_ballot()` uses an unseeded `thread_rng()`, and the
    // verification loop below tracks slot positions with a fragile "skip past zero
    // slots" heuristic that desyncs on some random draws (~6.5% failure rate),
    // misreading a contest's `is_explicit_invalid` slot. Needs deterministic seeding
    // plus reconstructing expected slot positions from the style layout instead of the
    // heuristic. Tracking issue: https://github.com/sequentech/meta/issues/12418
    #[test]
    #[ignore = "flaky: unseeded RNG + fragile index tracking; see sequentech/meta tracking issue"]
    fn test_mixed_radix_encode() {
        let (ballot, style) = random_ballot(5);

        let mixed_radix = ballot.encode_to_raw_ballot(&style).unwrap();

        let include_decline_to_vote = style.decline_to_vote_enabled();
        let mut index = if include_decline_to_vote {
            assert_eq!(
                mixed_radix.choices[0],
                u64::from(ballot.is_explicit_invalid),
                "ballot-level decline-to-vote flag should be at index 0"
            );
            1
        } else {
            0
        };

        let mut sorted_choices = ballot.choices.clone();
        sorted_choices.sort_by_key(|c| c.contest_id.clone());

        for choices in sorted_choices.iter() {
            let contest = style
                .contests
                .iter()
                .find(|c| c.id == choices.contest_id)
                .unwrap();

            assert_eq!(
                mixed_radix.choices[index],
                u64::from(choices.is_explicit_invalid)
            );
            index += 1;

            let mut candidate_ids: Vec<String> =
                contest.candidates.iter().map(|c| c.id.clone()).collect();
            candidate_ids.sort();

            // Each contest occupies exactly max_votes slots after its
            // flag(s); remember where they start so we can skip any
            // trailing unset slots once all choices are verified.
            let contest_slots_start = index;
            let contest_max_votes = usize::try_from(contest.max_votes).unwrap();

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

            // Skip past any remaining unset slots of this contest so the
            // next contest's flags are read from the correct position.
            index = contest_slots_start + contest_max_votes;
        }
    }

    fn random_ballot(contests: usize) -> (BallotChoices, BallotStyle) {
        let mut rng = rand::thread_rng();
        let contests: Vec<Contest> = (0..contests)
            .map(|i| {
                let contest_id = i.to_string();

                // allow for 0 min_votes to test decline to vote
                let min_votes = rng.gen_range(0..5);
                let max_votes = if min_votes == 0 {
                    rng.gen_range(0..5)
                } else {
                    rng.gen_range(min_votes..(min_votes + 5))
                };

                let candidates =
                    rng.gen_range((max_votes.max(1))..max_votes + 20);

                let candidates: Vec<Candidate> = (0..candidates)
                    .map(|j| {
                        random_candidate(j.to_string(), contest_id.clone())
                    })
                    .collect();

                random_contest(contest_id, candidates, min_votes, max_votes)
            })
            .collect();

        let all_allow_decline_to_vote =
            contests.iter().all(|c| c.min_votes == 0);
        let use_decline_to_vote =
            all_allow_decline_to_vote && rng.gen_bool(0.15);

        let choices: Vec<ContestChoices> = if use_decline_to_vote {
            contests
                .iter()
                .map(|c| ContestChoices::new(c.id.clone(), vec![], false))
                .collect()
        } else {
            contests.iter().map(|c| random_contest_choices(c)).collect()
        };

        let mut ballot_style = random_ballot_style(contests);
        if use_decline_to_vote {
            ballot_style.election_presentation = Some(ElectionPresentation {
                decline_to_vote_policy: Some(DeclineToVotePolicy::ENABLED),
                ..Default::default()
            });
        }

        let ballot = BallotChoices::new(
            use_decline_to_vote,
            choices,
            CountingAlgType::PluralityAtLarge,
        );

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

        if contest.min_votes == 0 && rng.gen_bool(0.2) {
            return ContestChoices::new(contest.id.clone(), vec![], true);
        }

        let count = rng.gen_range(contest.min_votes..=contest.max_votes);

        let mut cs = contest.candidates.clone();
        cs.shuffle(&mut rng);
        let choices = cs
            .iter()
            .take(count as usize)
            .map(|c| random_choice(c.id.clone()))
            .collect();

        ContestChoices::new(contest.id.clone(), choices, false)
    }

    fn test_contest(
        id: &str,
        num_candidates: usize,
        max_votes: i64,
    ) -> Contest {
        let candidates: Vec<Candidate> = (0..num_candidates)
            .map(|i| random_candidate(i.to_string(), id.to_string()))
            .collect();

        random_contest(id.to_string(), candidates, 0, max_votes)
    }

    fn test_ballot_style(contests: Vec<Contest>) -> BallotStyle {
        random_ballot_style(contests)
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
            counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
            is_encrypted: true,
            candidates,
            presentation: None,
            tie_breaking_policy: None,
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

    fn mark_explicit_blank(candidate: &mut Candidate) {
        candidate
            .presentation
            .get_or_insert_with(Default::default)
            .is_explicit_blank = Some(true);
    }

    fn mark_explicit_invalid(candidate: &mut Candidate) {
        candidate
            .presentation
            .get_or_insert_with(Default::default)
            .is_explicit_invalid = Some(true);
    }

    fn decoded_vote_contest(
        contest: &Contest,
        is_explicit_invalid: bool,
        selected_ids: &[String],
    ) -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: contest.id.clone(),
            is_explicit_invalid,
            is_decline_to_vote: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: contest
                .candidates
                .iter()
                .map(|candidate| DecodedVoteChoice {
                    id: candidate.id.clone(),
                    selected: if selected_ids.contains(&candidate.id) {
                        0
                    } else {
                        -1
                    },
                    write_in_text: None,
                })
                .collect(),
        }
    }

    fn has_invalid_error(contest: &DecodedVoteContest, message: &str) -> bool {
        contest
            .invalid_errors
            .iter()
            .any(|error| error.message.as_deref() == Some(message))
    }

    fn has_invalid_alert(contest: &DecodedVoteContest, message: &str) -> bool {
        contest
            .invalid_alerts
            .iter()
            .any(|alert| alert.message.as_deref() == Some(message))
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
            multi_contest_encoding_mode: None,
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
