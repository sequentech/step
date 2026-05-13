// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Pure-data tally context shared across counting algorithms.
//!
//! Holds already-decoded ballots, the contest definition, scope/operation
//! metadata, and helpers that turn raw vote counts into `ContestResult` /
//! `CandidateResult` values. All file I/O lives in the velvet pipeline
//! layer — this module only operates on in-memory data.

use std::cmp;
use std::collections::HashMap;

use sequent_core::ballot::{Contest, Weight};
use sequent_core::plaintext::DecodedVoteContest;
use sequent_core::types::ceremonies::{CountingAlgType, ScopeOperation};
use sequent_core::types::hasura::core::TallySheet;
use serde_json::Value;
use tracing::instrument;

use crate::counting::error::{Error, Result};
use crate::result::{CandidateResult, ContestResult, ExtendedMetricsContest, InvalidVotes};

pub struct Tally {
    pub id: CountingAlgType,
    pub scope_operation: ScopeOperation,
    pub contest: Contest,
    pub ballots: Vec<(DecodedVoteContest, Weight)>,
    pub census: u64,
    pub auditable_votes: u64,
    pub tally_sheet_results: Vec<ContestResult>,
    pub tally_results: Vec<ContestResult>,
}

impl Tally {
    /// Construct a `Tally` from already-decoded ballots. The previous
    /// `Tally::new(... ballots_files: Vec<(PathBuf, Weight)> ...)`
    /// constructor lives in `velvet` (it does file I/O); call this
    /// in-memory variant from pure-computation contexts (workbench, tests).
    #[instrument(skip(contest, ballots, tally_sheet_results, tally_results), name = "Tally::from_ballots")]
    pub fn from_ballots(
        contest: &Contest,
        scope_operation: ScopeOperation,
        ballots: Vec<(DecodedVoteContest, Weight)>,
        census: u64,
        auditable_votes: u64,
        tally_sheet_results: Vec<ContestResult>,
        tally_results: Vec<ContestResult>,
    ) -> Result<Self> {
        let contest = contest.clone();
        let id = contest
            .counting_algorithm
            .ok_or_else(|| Error::UnexpectedError("contest is missing counting_algorithm".into()))?;

        Ok(Self {
            id,
            scope_operation,
            contest,
            ballots,
            census,
            auditable_votes,
            tally_sheet_results,
            tally_results,
        })
    }

    #[instrument(skip_all)]
    pub fn aggregate_results(&self) -> Result<ContestResult> {
        if self.tally_results.is_empty() {
            return Err(Error::EmptyTallyResults);
        }
        let mut contest_result = ContestResult::default();
        contest_result.contest = self.contest.clone();
        let aggregated = self
            .tally_results
            .iter()
            .fold(contest_result, |acc, x| acc.aggregate(x, true));
        Ok(aggregated)
    }

    #[instrument(err, skip_all)]
    pub fn create_candidate_results(
        &self,
        vote_count: HashMap<String, u64>,
        count_blank: u64,
        count_invalid_votes: InvalidVotes,
        extended_metrics: ExtendedMetricsContest,
        _count_valid: u64,
        _count_invalid: u64,
        percentage_votes_denominator: u64,
    ) -> Result<Vec<CandidateResult>> {
        let contest = &self.contest;

        // Create candidate results map from vote_count
        let candidate_results_map: HashMap<String, CandidateResult> = vote_count
            .into_iter()
            .map(|(id, total_count)| {
                let candidate = self
                    .contest
                    .candidates
                    .iter()
                    .find(|c| c.id == id)
                    .cloned()
                    .ok_or(Error::CandidateNotFound(id))?;

                let is_explicit_blank = candidate.is_explicit_blank();
                let is_explicit_invalid = candidate.is_explicit_invalid();

                if is_explicit_blank {
                    let percentage_votes = (count_blank as f64
                        / cmp::max(1, extended_metrics.total_ballots) as f64)
                        * 100.0;

                    Ok(CandidateResult {
                        candidate,
                        percentage_votes: percentage_votes.clamp(0.0, 100.0),
                        total_count: count_blank,
                    })
                } else if is_explicit_invalid {
                    let percentage_votes = (count_invalid_votes.explicit as f64
                        / cmp::max(1, extended_metrics.total_ballots) as f64)
                        * 100.0;

                    Ok(CandidateResult {
                        candidate,
                        percentage_votes: percentage_votes.clamp(0.0, 100.0),
                        total_count: count_invalid_votes.explicit,
                    })
                } else {
                    let percentage_votes = (total_count as f64
                        / cmp::max(1, percentage_votes_denominator) as f64)
                        * 100.0;

                    Ok(CandidateResult {
                        candidate,
                        percentage_votes: percentage_votes.clamp(0.0, 100.0),
                        total_count,
                    })
                }
            })
            .collect::<Result<Vec<CandidateResult>>>()?
            .into_iter()
            .map(|cand| (cand.candidate.id.clone(), cand))
            .collect();

        // Create result vector from all candidates in contest
        let candidate_result: Vec<CandidateResult> = contest
            .candidates
            .iter()
            .map(|candidate| {
                let candidate_result = candidate_results_map.get(&candidate.id).cloned();

                if let Some(candidate_result) = candidate_result {
                    Ok(candidate_result)
                } else {
                    let is_explicit_blank = candidate.is_explicit_blank();
                    let is_explicit_invalid = candidate.is_explicit_invalid();

                    if is_explicit_blank {
                        let percentage_votes = (count_blank as f64
                            / cmp::max(1, extended_metrics.total_ballots) as f64)
                            * 100.0;

                        Ok(CandidateResult {
                            candidate: candidate.clone(),
                            percentage_votes: percentage_votes.clamp(0.0, 100.0),
                            total_count: count_blank,
                        })
                    } else if is_explicit_invalid {
                        let percentage_votes = (count_invalid_votes.explicit as f64
                            / cmp::max(1, extended_metrics.total_ballots) as f64)
                            * 100.0;

                        Ok(CandidateResult {
                            candidate: candidate.clone(),
                            percentage_votes: percentage_votes.clamp(0.0, 100.0),
                            total_count: count_invalid_votes.explicit,
                        })
                    } else {
                        Ok(CandidateResult {
                            candidate: candidate.clone(),
                            percentage_votes: 0.0,
                            total_count: 0,
                        })
                    }
                }
            })
            .collect::<Result<Vec<CandidateResult>>>()?;
        Ok(candidate_result)
    }

    #[instrument(err, skip_all)]
    pub fn create_contest_result(
        &self,
        process_results: Option<Value>,
        candidate_result: Vec<CandidateResult>,
        count_blank: u64,
        count_invalid_votes: InvalidVotes,
        extended_metrics: ExtendedMetricsContest,
        count_valid: u64,
        count_invalid: u64,
        _percentage_votes_denominator: u64,
    ) -> Result<ContestResult> {
        // Calculate percentages
        let total_votes = count_valid + count_invalid;
        let total_votes_base = cmp::max(1, total_votes) as f64;

        let census_base = cmp::max(1, self.census) as f64;
        let percentage_auditable_votes = (self.auditable_votes as f64) * 100.0 / census_base;
        let percentage_total_votes = (total_votes as f64) * 100.0 / census_base;
        let percentage_total_valid_votes = (count_valid as f64 * 100.0) / total_votes_base;
        let percentage_total_invalid_votes = (count_invalid as f64 * 100.0) / total_votes_base;
        let percentage_total_blank_votes = (count_blank as f64 * 100.0) / total_votes_base;
        let percentage_invalid_votes_explicit =
            (count_invalid_votes.explicit as f64 * 100.0) / total_votes_base;
        let percentage_invalid_votes_implicit =
            (count_invalid_votes.implicit as f64 * 100.0) / total_votes_base;

        // Create ContestResult
        let contest_result = ContestResult {
            contest: self.contest.clone(),
            census: self.census,
            percentage_census: 100.0,
            auditable_votes: self.auditable_votes,
            percentage_auditable_votes: percentage_auditable_votes.clamp(0.0, 100.0),
            total_votes,
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
            candidate_result,
            extended_metrics: Some(extended_metrics),
            process_results,
        };
        Ok(contest_result)
    }
}

/// Convert a manually-entered tally sheet into a `ContestResult`. Pure
/// function over Hasura-derived data — no I/O.
#[instrument(err, skip_all)]
pub fn process_tally_sheet(tally_sheet: &TallySheet, contest: &Contest) -> Result<ContestResult> {
    let Some(content) = tally_sheet.content.clone() else {
        return Err(Error::UnexpectedError("missing tally sheet content".into()));
    };
    let invalid_votes = content.invalid_votes.unwrap_or(Default::default());

    let count_invalid_votes = InvalidVotes {
        explicit: invalid_votes.explicit_invalid.unwrap_or(0),
        implicit: invalid_votes.implicit_invalid.unwrap_or(0),
    };
    let count_invalid: u64 = count_invalid_votes.explicit + count_invalid_votes.implicit;
    let count_blank: u64 = content.total_blank_votes.unwrap_or(0);

    let candidate_results = content
        .candidate_results
        .values()
        .map(|candidate| -> Result<CandidateResult> {
            let Some(found_candidate) = contest
                .candidates
                .iter()
                .find(|c| candidate.candidate_id == c.id)
            else {
                return Err(Error::CandidateNotFound(candidate.candidate_id.clone()));
            };

            Ok(CandidateResult {
                candidate: found_candidate.clone(),
                percentage_votes: 0.0,
                total_count: candidate.total_votes.unwrap_or(0),
            })
        })
        .collect::<Result<Vec<CandidateResult>>>()?;

    let count_valid: u64 = candidate_results
        .iter()
        .map(|candidate_result| candidate_result.total_count)
        .sum();

    let total_votes = count_valid + count_invalid;

    let contest_result = ContestResult {
        contest: contest.clone(),
        census: content.census.unwrap_or(0),
        percentage_census: 100.0,
        auditable_votes: 0,
        percentage_auditable_votes: 0.0,
        total_votes,
        percentage_total_votes: 0.0,
        total_valid_votes: count_valid,
        percentage_total_valid_votes: 0.0,
        total_invalid_votes: count_invalid,
        percentage_total_invalid_votes: 0.0,
        total_blank_votes: count_blank,
        percentage_total_blank_votes: 0.0,
        percentage_invalid_votes_explicit: 0.0,
        percentage_invalid_votes_implicit: 0.0,
        invalid_votes: count_invalid_votes,
        candidate_result: candidate_results,
        extended_metrics: None,
        process_results: None,
    };
    Ok(contest_result.calculate_percentages())
}
