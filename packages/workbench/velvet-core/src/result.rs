// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tally result types.
//!
//! These types describe the output of a counting-algorithm run for a single
//! contest and area, plus their aggregation. Moved out of `velvet` so the
//! types are available to pure-computation consumers (notably WASM builds for
//! the workbench).

use std::cmp;
use std::collections::HashMap;

use sequent_core::ballot::{Candidate, Contest, Weight};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
pub struct InvalidVotes {
    pub explicit: u64,
    pub implicit: u64,
}

impl InvalidVotes {
    #[instrument]
    pub fn aggregate(&self, other: &InvalidVotes) -> InvalidVotes {
        let mut sum = self.clone();

        sum.explicit += other.explicit;
        sum.implicit += other.implicit;
        sum
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, Copy)]
pub struct ExtendedMetricsContest {
    // Voted more candidates than the allowed amount per contest
    pub over_votes: u64,
    // Voted less than the number of votes allowed for each contest.
    pub under_votes: u64,
    // Total actual marks count of candidates in the contest. Only counted UV and fully votes.
    pub votes_actually: u64,
    // Total expected marks for candidates if all votes were normal
    // (no under-votes, no over-votes) (valid-ballots X number of
    // votes possible in the contest)
    pub expected_votes: u64,
    //Total counted ballots
    pub total_ballots: u64,
    pub weight: Weight, // Used to store the actual weight used to tally an specific area.
    pub total_weight: u64, // Used to calculate the right percentage_votes in aggregate
}

impl ExtendedMetricsContest {
    #[instrument(skip_all)]
    pub fn aggregate(&self, other: &ExtendedMetricsContest) -> ExtendedMetricsContest {
        let mut result = self.clone();
        result.over_votes += other.over_votes;
        result.under_votes += other.under_votes;
        result.votes_actually += other.votes_actually;
        result.expected_votes += other.expected_votes;
        result.total_ballots += other.total_ballots;
        result.total_weight += other.total_weight;
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedMetricsElection {
    // Number of valid ballots processed by the ACM without any
    // single mark on all contests.
    pub abstentions: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContestResult {
    pub contest: Contest,
    pub census: u64,
    pub percentage_census: f64,
    pub auditable_votes: u64,
    pub percentage_auditable_votes: f64,
    pub total_votes: u64,
    pub percentage_total_votes: f64,
    pub total_valid_votes: u64,
    pub percentage_total_valid_votes: f64,
    pub total_invalid_votes: u64,
    pub percentage_total_invalid_votes: f64,
    pub total_blank_votes: u64,
    pub percentage_total_blank_votes: f64,
    pub invalid_votes: InvalidVotes,
    pub percentage_invalid_votes_explicit: f64,
    pub percentage_invalid_votes_implicit: f64,
    pub candidate_result: Vec<CandidateResult>,
    pub extended_metrics: Option<ExtendedMetricsContest>,
    pub process_results: Option<Value>, // The results from the counting algorithm process
}

impl ContestResult {
    #[instrument(skip_all)]
    pub fn calculate_percentages(&self) -> ContestResult {
        let total_weight = self
            .extended_metrics
            .clone()
            .unwrap_or_default()
            .total_weight;
        let candidate_result: Vec<CandidateResult> = self
            .candidate_result
            .clone()
            .into_iter()
            .map(|candidate_result| {
                let percentage_votes = (candidate_result.total_count as f64
                    / cmp::max(1, total_weight) as f64)
                    * 100.0;
                let mut new_candidate_result = candidate_result.clone();
                new_candidate_result.percentage_votes = percentage_votes.clamp(0.0, 100.0);

                new_candidate_result
            })
            .collect();
        let total_votes = self.total_votes;
        let total_votes_base = cmp::max(1, total_votes) as f64;
        let count_valid = self.total_valid_votes;

        let census_base = cmp::max(1, self.census) as f64;

        // `percentage_auditable_votes` is calculated over `census_base`.
        // Otherwise we could end up with strange percentages. Imagine a test
        // election with 2 auditable votes and 1 valid vote. That's maybe 66%
        // auditable votes over the census, but 200% over total votes.
        let percentage_auditable_votes = (self.auditable_votes as f64) * 100.0 / census_base;
        let percentage_total_votes = (total_votes as f64) * 100.0 / census_base;
        let percentage_total_valid_votes = (count_valid as f64 * 100.0) / total_votes_base;
        let percentage_total_invalid_votes =
            (self.total_invalid_votes as f64 * 100.0) / total_votes_base;
        let percentage_total_blank_votes =
            (self.total_blank_votes as f64 * 100.0) / total_votes_base;
        let percentage_invalid_votes_explicit =
            (self.invalid_votes.explicit as f64 * 100.0) / total_votes_base;
        let percentage_invalid_votes_implicit =
            (self.invalid_votes.implicit as f64 * 100.0) / total_votes_base;

        let mut contest_result = self.clone();
        contest_result.percentage_census = 100.0;
        contest_result.percentage_auditable_votes = percentage_auditable_votes.clamp(0.0, 100.0);
        contest_result.percentage_total_votes = percentage_total_votes.clamp(0.0, 100.0);
        contest_result.percentage_total_valid_votes =
            percentage_total_valid_votes.clamp(0.0, 100.0);
        contest_result.percentage_total_invalid_votes =
            percentage_total_invalid_votes.clamp(0.0, 100.0);
        contest_result.percentage_total_blank_votes =
            percentage_total_blank_votes.clamp(0.0, 100.0);
        contest_result.percentage_invalid_votes_explicit =
            percentage_invalid_votes_explicit.clamp(0.0, 100.0);
        contest_result.percentage_invalid_votes_implicit =
            percentage_invalid_votes_implicit.clamp(0.0, 100.0);
        contest_result.candidate_result = candidate_result;
        contest_result
    }

    #[instrument(skip_all)]
    pub fn aggregate(&self, other: &ContestResult, add_census: bool) -> ContestResult {
        let mut aggregate = self.clone();
        if add_census {
            aggregate.census += other.census;
        }
        let aggregate_metrics = aggregate.extended_metrics.unwrap_or_default();
        aggregate.extended_metrics =
            Some(aggregate_metrics.aggregate(&other.extended_metrics.clone().unwrap_or_default()));
        aggregate.total_votes += other.total_votes;
        aggregate.total_valid_votes += other.total_valid_votes;
        aggregate.total_invalid_votes += other.total_invalid_votes;
        aggregate.total_blank_votes += other.total_blank_votes;
        aggregate.invalid_votes = aggregate.invalid_votes.aggregate(&other.invalid_votes);

        let mut candidate_map: HashMap<String, CandidateResult> = HashMap::new();

        for candidate_result in &self.candidate_result {
            candidate_map.insert(
                candidate_result.candidate.id.clone(),
                candidate_result.clone(),
            );
        }

        for candidate_result in &other.candidate_result {
            candidate_map
                .entry(candidate_result.candidate.id.clone())
                .and_modify(|entry| entry.total_count += candidate_result.total_count)
                .or_insert_with(|| candidate_result.clone());
        }

        aggregate.candidate_result = candidate_map.into_values().collect();

        aggregate.calculate_percentages()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateResult {
    pub candidate: Candidate,
    pub percentage_votes: f64,
    pub total_count: u64,
}
