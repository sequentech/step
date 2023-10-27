// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::{collections::HashMap, fs, path::Path};

use sequent_core::{ballot::Contest, plaintext::DecodedVoteContest};

use super::{CandidateResult, ContestResult, CountingAlgorithm};
use crate::pipes::do_tally::{invalid_vote::InvalidVote, tally::Tally};

use super::{Error, Result};

pub struct PluralityAtLarge {
    pub tally: Tally,
}

impl PluralityAtLarge {
    pub fn new(tally: Tally) -> Self {
        Self { tally }
    }
}

impl CountingAlgorithm for PluralityAtLarge {
    fn tally(&self) -> Result<ContestResult> {
        let contest = &self.tally.contest;
        let votes = &self.tally.ballots;

        let mut vote_count: HashMap<String, u64> = HashMap::new();
        let mut vote_count_invalid: HashMap<InvalidVote, u64> = HashMap::new();
        let mut count_valid: u64 = 0;

        for vote in votes {
            if vote.invalid_errors.len() > 0 {
                if vote.is_explicit_invalid {
                    *vote_count_invalid.entry(InvalidVote::Explicit).or_insert(0) += 1;
                } else {
                    *vote_count_invalid.entry(InvalidVote::Implicit).or_insert(0) += 1;
                }
            } else {
                for choice in &vote.choices {
                    if choice.selected >= 0 {
                        *vote_count.entry(choice.id.clone()).or_insert(0) += 1;
                        count_valid += 1;
                    }
                }
            }
        }

        let result: Vec<CandidateResult> = vote_count
            .into_iter()
            .map(|(choice_id, total_count)| CandidateResult {
                choice_id,
                total_count,
            })
            .collect();

        let result = contest
            .candidates
            .iter()
            .map(|c| {
                result
                    .iter()
                    .find(|r| r.choice_id == c.id)
                    .cloned()
                    .unwrap_or(CandidateResult {
                        choice_id: c.id.clone(),
                        total_count: 0,
                    })
            })
            .collect::<Vec<CandidateResult>>();

        let contest_result = ContestResult {
            contest_id: self.tally.contest.id.to_string(),
            total_valid_votes: count_valid,
            total_invalid_votes: vote_count_invalid,
            candidate_result: result,
        };

        Ok(contest_result)
    }
}
