// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{tally::Tally, CandidateResult, ContestResult, InvalidVotes};
use std::cmp;
use std::collections::HashMap;
use tracing::instrument;

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
            if vote.is_invalid() {
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
                    (total_count as f64 / cmp::max(1, count_valid - count_blank) as f64) * 100.0;

                Ok(CandidateResult {
                    candidate,
                    percentage_votes: percentage_votes.clamp(0.0, 100.0),
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

        let total_votes = count_valid + count_invalid;
        let total_votes_base = cmp::max(1, total_votes) as f64;

        let census_base = cmp::max(1, self.tally.census) as f64;
        let percentage_total_votes = (total_votes as f64) * 100.0 / census_base;
        let percentage_total_valid_votes = (count_valid as f64 * 100.0) / total_votes_base;
        let percentage_total_invalid_votes = (count_invalid as f64 * 100.0) / total_votes_base;
        let percentage_total_blank_votes = (count_blank as f64 * 100.0) / total_votes_base;
        let percentage_invalid_votes_explicit =
            (count_invalid_votes.explicit as f64 * 100.0) / total_votes_base;
        let percentage_invalid_votes_implicit =
            (count_invalid_votes.implicit as f64 * 100.0) / total_votes_base;

        let contest_result = ContestResult {
            contest: self.tally.contest.clone(),
            census: self.tally.census,
            percentage_census: 100.0,
            total_votes: total_votes,
            percentage_total_votes: percentage_total_votes.clamp(0.0, 100.0),
            total_valid_votes: count_valid,
            percentage_total_valid_votes: percentage_total_valid_votes.clamp(0.0, 100.0),
            total_invalid_votes: count_invalid,
            percentage_total_invalid_votes: percentage_total_invalid_votes.clamp(0.0, 100.0),
            total_blank_votes: count_blank,
            percentage_total_blank_votes: percentage_total_blank_votes.clamp(0.0, 100.0),
            percentage_invalid_votes_explicit: percentage_invalid_votes_explicit.clamp(0.0, 100.0),
            percentage_invalid_votes_implicit: percentage_invalid_votes_implicit.clamp(0.0, 100.0),
            invalid_votes: count_invalid_votes,
            candidate_result: result,
        };

        let aggregate = self
            .tally
            .tally_sheet_results
            .iter()
            .fold(contest_result, |acc, x| acc.aggregate(x, false));

        Ok(aggregate)
    }
}
