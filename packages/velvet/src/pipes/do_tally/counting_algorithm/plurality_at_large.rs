// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;
use tracing::instrument;

use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{tally::Tally, CandidateResult, ContestResult, InvalidVotes};

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
        let mut count_invalid_votes = InvalidVotes {
            explicit: 0,
            implicit: 0,
        };
        let mut count_valid: u64 = 0;
        let mut count_invalid: u64 = 0;
        let mut count_blank: u64 = 0;

        for vote in votes {
            if !vote.invalid_errors.is_empty() {
                if vote.is_explicit_invalid {
                    count_invalid_votes.explicit += 1;
                } else {
                    count_invalid_votes.implicit += 1;
                }
                count_invalid += 1;
            } else {
                let mut is_blank = true;

                for choice in &vote.choices {
                    if choice.selected >= 0 {
                        *vote_count.entry(choice.id.clone()).or_insert(0) += 1;
                        is_blank = false;
                    }
                }

                if is_blank {
                    count_blank += 1;
                }

                count_valid += 1;
            }
        }

        let result: Result<Vec<CandidateResult>> = vote_count
            .into_iter()
            .map(|(id, total_count)| {
                let candidate = self
                    .tally
                    .contest
                    .candidates
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .ok_or(Error::CandidateNotFound(id))?;

                let percentage_votes =
                    (total_count as f64 / (count_valid - count_blank) as f64) * 100.0;

                Ok(CandidateResult {
                    candidate,
                    percentage_votes,
                    total_count,
                })
            })
            .collect();
        let result = result?;

        let result: Result<Vec<CandidateResult>> = contest
            .candidates
            .iter()
            .map(|c| {
                let candidate_result = result.iter().find(|rc| rc.candidate.id == c.id).cloned();

                if let Some(candidate_result) = candidate_result {
                    Ok(candidate_result)
                } else {
                    let candidate = self
                        .tally
                        .contest
                        .candidates
                        .iter()
                        .find(|rc| rc.id == c.id)
                        .cloned();

                    if let Some(candidate) = candidate {
                        return Ok(CandidateResult {
                            candidate,
                            percentage_votes: 0.0,
                            total_count: 0,
                        });
                    }

                    Err(Error::CandidateNotFound(c.id.to_string()))
                }
            })
            .collect();
        let result = result?;

        let contest_result = ContestResult {
            contest: self.tally.contest.clone(),
            total_votes: count_valid + count_invalid,
            total_valid_votes: count_valid,
            total_invalid_votes: count_invalid,
            total_blank_votes: count_blank,
            invalid_votes: count_invalid_votes,
            census: self.tally.census,
            candidate_result: result,
        };

        Ok(contest_result)
    }
}
