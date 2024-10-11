use std::collections::{HashMap, HashSet};
use std::num::TryFromIntError;

// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::bigint;
use super::{vec, RawBallotContest};
use crate::ballot::{BallotStyle, Candidate, Contest};
use crate::mixed_radix;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A multi contest ballot.
///
/// This ballot only supports plurality counting
/// algorithms. It does not support write-ins.
/// It does not support per-contest invalid flags.
///
/// A multi contest ballot can be encoded in to a
/// 30 byte representation suitable for encryption
/// the ballot into a single ciphertext, provided
/// there is sufficient space.
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

#[derive(Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct ContestChoices {
    pub contest_id: String,
    pub is_explicit_invalid: bool,
    pub choices: Vec<ContestChoice>,
}
impl ContestChoices {
    pub fn new(
        contest_id: String,
        is_explicit_invalid: bool,
        choices: Vec<ContestChoice>,
    ) -> Self {
        ContestChoices {
            contest_id,
            is_explicit_invalid,
            choices,
        }
    }
}
#[derive(
    Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone, Hash,
)]
pub struct ContestChoice {
    pub id: String,
    pub selected: i64,
    pub write_in_text: Option<String>,
}
impl ContestChoice {
    pub fn new(
        id: String,
        selected: i64,
        write_in_text: Option<String>,
    ) -> Self {
        ContestChoice {
            id,
            selected,
            write_in_text,
        }
    }
}

pub struct DecodedBallotChoices {
    pub is_explicit_invalid: bool,
    pub choices: Vec<ContestChoices>,
}
impl DecodedBallotChoices {
    pub fn get_contest_ids(&self) -> Vec<String> {
        self.choices.iter().map(|c| c.contest_id.clone()).collect()
    }
}

impl BallotChoices {
    /// Encode this ballot into a 30 byte representation
    ///
    /// The steps in this encoding are
    /// 1. Retrieve this ballot's contests from the supplied BallotStyle
    /// 2. Encode this ballot into a mixed radix representation
    /// 3. Convert the mixed radix representation into a radix-10 big integer
    /// 4. Convert the radix-10 big integer into a 30-byte representation
    ///
    /// Returns a fixed-size array of 30 bytes encoding this ballot.
    pub fn encode_to_30_bytes(
        &self,
        config: &BallotStyle,
    ) -> Result<[u8; 30], String> {
        let contests: Result<Vec<(Contest, ContestChoices)>, String> = self
            .choices
            .clone()
            .into_iter()
            .map(|choices| {
                let contest = config
                    .contests
                    .iter()
                    .find(|contest| contest.id == choices.contest_id)
                    .ok_or_else(|| {
                        format!(
                            "Can't find contest with id {} on ballot style",
                            choices.contest_id
                        )
                    })?;

                Ok((contest.clone(), choices))
            })
            .collect();

        let (contests, choices): (Vec<Contest>, Vec<ContestChoices>) =
            contests?.into_iter().unzip();

        let raw_ballot = Self::encode_to_raw_ballot(
            &contests,
            &choices,
            self.is_explicit_invalid,
        )?;

        let candidates: Vec<i64> =
            contests.iter().map(|c| c.max_votes).collect();

        let bigint =
            mixed_radix::encode(&raw_ballot.choices, &raw_ballot.bases)?;
        let bytes = bigint::encode_bigint_to_bytes(&bigint)?;

        vec::encode_vec_to_array(&bytes)
    }

    /// Encode this ballot into a mixed radix representation
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
        contests: &Vec<Contest>,
        plaintexts: &Vec<ContestChoices>,
        explicit_invalid: bool,
    ) -> Result<RawBallotContest, String> {
        let bases = Self::get_bases(contests).map_err(|e| e.to_string())?;
        let mut choices: Vec<u64> = vec![];

        // Construct a map of plaintexts, this will allow us to
        // handle calls in which passed in contests and plaintexts
        // may not be in the same [parallel] order. We will
        // obtain plaintexts from this map using the contest_id.
        let plaintexts_map = plaintexts
            .iter()
            .map(|plaintext| (plaintext.contest_id.clone(), plaintext))
            .collect::<HashMap<String, &ContestChoices>>();

        // The order of the contests is computed sorting by id.
        // The selections must be encoded to and decoded from a ballot
        // following this order, given by contest.id.
        let mut sorted_contests = contests.clone();
        sorted_contests.sort_by_key(|c| c.id.clone());

        let invalid_vote: u64 = if explicit_invalid { 1 } else { 0 };
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
        // choices when decoding
        let mut sorted_candidates = contest.candidates.clone();
        sorted_candidates.sort_by_key(|c| c.id.clone());

        // Note how the position for the candidate is mapped to the first
        // element in the tuple. This position will be used below when
        // marking choices.
        let candidates_map = sorted_candidates
            .iter()
            .filter(|candidate| !candidate.is_explicit_invalid())
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
                "Plaintext vector contained fewer than min_votes elements"
            ));
        }
        if plaintext.choices.len() > max_votes {
            return Err(format!(
                "Plaintext vector contained more than max_votes elements"
            ));
        }

        // We set all values as unset (0) by default
        let mut contest_choices = vec![0u64; max_votes];
        let mut marked = 0;
        for p in &plaintext.choices {
            let (position, _candidate) =
                candidates_map.get(&p.id).ok_or_else(|| {
                    "choice id is not a valid candidate".to_string()
                })?;

            if p.selected > -1 {
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
                let mark = (position + 1).try_into().map_err(|_| {
                    format!("u64 conversion on candidate position")
                })?;
                contest_choices[marked] = mark;
                marked += 1;

                if marked == max_votes {
                    break;
                }
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

    /// Decode a mixed radix representation of the ballot
    ///
    /// The following conditions will return an error.
    ///
    /// FIXME
    /// In the current implementation these errors short
    /// circuit the operation.
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
    pub fn decode(
        contests: &Vec<Contest>,
        raw_ballot: &RawBallotContest,
    ) -> Result<DecodedBallotChoices, String> {
        let mut contest_choices: Vec<ContestChoices> = vec![];
        let choices = raw_ballot.choices.clone();

        // Each contest contributes max_votes slots
        let expected_choices = contests.iter().fold(0, |a, b| a + b.max_votes);
        let expected_choices: usize = expected_choices
            .try_into()
            .map_err(|_| format!("u64 conversion on contest max_votes"))?;

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

        // This explicit invalid flat is at the ballot level
        let is_explicit_invalid: bool = !choices.is_empty() && (choices[0] > 0);
        // Skip past the explicit invalid slot
        let mut choice_index = 1;

        for contest in sorted_contests {
            let max_votes: usize = contest
                .max_votes
                .try_into()
                .map_err(|_| format!("u64 conversion on contest max_votes"))?;
            let next =
                Self::decode_contest(&contest, &choices[choice_index..])?;
            choice_index += max_votes;
            contest_choices.push(next);
        }

        let ret = DecodedBallotChoices {
            is_explicit_invalid,
            choices: contest_choices,
        };

        Ok(ret)
    }

    /// Decodes one contest in the ballot
    ///
    /// Returns a ContestChoice for the choices slice argument,
    /// which will be read up to index contest.max_votes. This
    /// ContestChoice will be added to the overall DecodedBallotChoices.
    /// It is the responsibility of the caller to advance the choice slice
    /// as choices are decoded.
    fn decode_contest(
        contest: &Contest,
        choices: &[u64],
    ) -> Result<ContestChoices, String> {
        // A choice of a candidate is represented as that candidate's
        // position in the candidate list, sorted by id.
        let mut sorted_candidates: Vec<Candidate> = contest
            .candidates
            .clone()
            .into_iter()
            .filter(|candidate| !candidate.is_explicit_invalid())
            .collect();

        sorted_candidates.sort_by_key(|c| c.id.clone());

        let max_votes: usize = contest
            .max_votes
            .try_into()
            .map_err(|_| format!("u64 conversion on contest max_votes"))?;
        let min_votes: usize = contest
            .min_votes
            .try_into()
            .map_err(|_| format!("u64 conversion on contest min_votes"))?;

        let mut next_choices = vec![];
        for i in 0..max_votes {
            let next = choices[i];
            let next = usize::try_from(next)
                .map_err(|_| format!("u64 conversion on plaintext choice"))?;
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

            let choice = ContestChoice {
                id: candidate.id.clone(),
                selected: 1,
                write_in_text: None,
            };

            next_choices.push(choice);
        }

        // Duplicate values will be ignored
        let unique: HashSet<ContestChoice> =
            HashSet::from_iter(next_choices.iter().cloned());

        if unique.len() != next_choices.len() {
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

        let c = ContestChoices {
            contest_id: contest.id.clone(),
            // FIXME we are currently not using this field
            is_explicit_invalid: false,
            choices: unique.into_iter().collect(),
        };

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
    // representation of this ballot.
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

        for contest in contests {
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
                // include a per-ballot invalid flag
                bases.push(u64::from(num_valid_candidates + 1));
            }
        }

        Ok(bases)
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
}

#[cfg(test)]
mod tests {

    use rand::{seq::SliceRandom, Rng, RngCore};

    use super::*;
    use crate::ballot::{BallotStyle, Candidate, Contest};

    #[test]
    fn test_roundtrip() {
        let (ballot, style) = random_data(5);
        ballot.encode_to_30_bytes(&style).unwrap();
    }

    fn random_data(count: usize) -> (BallotChoices, BallotStyle) {
        let mut rng = rand::thread_rng();
        let contests: Vec<Contest> = (0..count)
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

        println!(
            "max size is {}",
            BallotChoices::maximum_size_bytes(&contests).unwrap()
        );

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
        ContestChoice::new(id, (rng.next_u32() % 10) as i64, None)
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

        ContestChoices::new(contest.id.clone(), false, choices)
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
        }
    }

    fn s() -> String {
        "foo".to_string()
    }
}
