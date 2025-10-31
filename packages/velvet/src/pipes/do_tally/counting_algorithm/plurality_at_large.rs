// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{
    counting_algorithm::utils::*, tally::Tally, CandidateResult, ContestResult,
    ExtendedMetricsContest, InvalidVotes,
};
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use std::cmp;
use std::collections::HashMap;
use tracing::{info, instrument};

use super::Result;

pub struct PluralityAtLarge {
    pub tally: Tally,
}

impl PluralityAtLarge {
    #[instrument(skip_all)]
    pub fn new(tally: Tally) -> Self {
        Self { tally }
    }
    #[instrument(err, skip_all)]
    pub fn process_ballots(&self, op: TallyOperation) -> Result<ContestResult> {
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

        let mut extended_metrics = ExtendedMetricsContest::default();
        let mut total_ballots = 0;
        let mut total_weight = 0;

        for (vote, weight_opt) in votes {
            let weight = weight_opt.clone().unwrap_or_default();
            total_ballots += 1;

            extended_metrics = update_extended_metrics(vote, &extended_metrics, &contest);
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
                        *vote_count.entry(choice.id.clone()).or_insert(0) += weight;
                        total_weight += weight;
                        if is_blank {
                            is_blank = false;
                        }
                    }
                }

                if is_blank {
                    count_blank += 1;
                }

                count_valid += 1;
            }
        }

        extended_metrics.total_ballots = total_ballots;
        extended_metrics.total_weight = total_weight;
        let percentage_votes_denominator = total_weight;

        let candidate_result = match op {
            TallyOperation::SkipCandidateResults => Vec::new(),
            _ => self.tally.create_candidate_results(
                vote_count,
                count_blank,
                count_invalid_votes.clone(),
                extended_metrics.clone(),
                count_valid,
                count_invalid,
                percentage_votes_denominator,
            )?,
        };

        self.tally.create_contest_result(
            None,
            candidate_result,
            count_blank,
            count_invalid_votes,
            extended_metrics,
            count_valid,
            count_invalid,
            percentage_votes_denominator,
        )
    }
}

impl CountingAlgorithm for PluralityAtLarge {
    #[instrument(err, skip_all)]
    fn tally(&self) -> Result<ContestResult> {
        let contest_result = match self.tally.scope_operation {
            ScopeOperation::Contest(op) if op == TallyOperation::AggregateResults => {
                self.tally.aggregate_results()?
            }
            ScopeOperation::Contest(op) => self.process_ballots(op)?,
            ScopeOperation::Area(op) => {
                if op == TallyOperation::AggregateResults {
                    return Err(Error::InvalidTallyOperation(format!(
                        "TallyOperation {op} is not supported for PluralityAtLarge at Area level"
                    )));
                }
                self.process_ballots(op)?
            }
        };

        let aggregate = self
            .tally
            .tally_sheet_results
            .iter()
            .fold(contest_result, |acc, x| acc.aggregate(x, false));

        Ok(aggregate)
    }
}
