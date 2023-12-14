// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use tracing::instrument;

use super::CountingAlgorithm;
use crate::pipes::do_tally::{
    invalid_vote::InvalidVote, tally::Tally, CandidateResult, ContestResult,
};

use super::Result;

pub struct PluralityAtLarge {
    pub tally: Tally,
}

impl PluralityAtLarge {
    #[instrument(skip_all)]
    pub fn new(tally: Tally) -> Self {
        Self { tally }
    }
}

impl CountingAlgorithm for PluralityAtLarge {
    #[instrument(skip_all)]
    fn tally(&self) -> Result<ContestResult> {
        let contest = &self.tally.contest;
        let votes = &self.tally.ballots;

        let mut vote_count: HashMap<String, u64> = HashMap::new();
        let mut vote_count_invalid: HashMap<InvalidVote, u64> = HashMap::new();
        let mut count_valid: u64 = 0;
        let mut count_invalid: u64 = 0;

        for vote in votes {
            if !vote.invalid_errors.is_empty() {
                if vote.is_explicit_invalid {
                    *vote_count_invalid.entry(InvalidVote::Explicit).or_insert(0) += 1;
                } else {
                    *vote_count_invalid.entry(InvalidVote::Implicit).or_insert(0) += 1;
                }
                count_invalid += 1;
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
            .map(|(id, total_count)| CandidateResult {
                candidate: self
                    .tally
                    .contest
                    .candidates
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .unwrap(),
                total_count,
            })
            .collect();

        let result = contest
            .candidates
            .iter()
            .map(|c| {
                result
                    .iter()
                    .find(|rc| rc.candidate.id == c.id)
                    .cloned()
                    .unwrap_or(CandidateResult {
                        candidate: self
                            .tally
                            .contest
                            .candidates
                            .iter()
                            .find(|rc| rc.id == c.id)
                            .unwrap()
                            .clone(),
                        total_count: 0,
                    })
            })
            .collect::<Vec<CandidateResult>>();

        let contest_result = ContestResult {
            contest: self.tally.contest.clone(),
            total_votes: count_valid + count_invalid,
            total_valid_votes: count_valid,
            total_invalid_votes: count_invalid,
            invalid_votes: vote_count_invalid,
            candidate_result: result,
        };

        Ok(contest_result)
    }
}
