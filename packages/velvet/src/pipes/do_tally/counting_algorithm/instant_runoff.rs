// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{
    tally::Tally, CandidateResult, ContestResult, ExtendedMetricsContest, InvalidVotes,
};
use sequent_core::ballot::Contest;
use sequent_core::plaintext::{DecodedVoteContest, InvalidPlaintextErrorType};
use std::cmp;
use std::collections::HashMap;
use tracing::{event, instrument, Level};

use super::Result;

enum CandidateStatus {
    Active,
    Eliminated,
}

enum BallotStatus {
    Valid,
    Exhausted,
}

/// With this type we will need to:
/// - Create a list with all the candidate ids and statuses set to CandidateStatus::Active
/// - Check if an specific candidate id is Active
type CandidatesStatus = HashMap<String, CandidateStatus>;

type CandidateFirstChoices = HashMap<String, u64>;

/// With this type we will need to:
/// - Create an array with all the ballots status to BallotStatus::Valid
/// - For an specific candidate id, count all the ballots which rank is 0 (first choice)
type BallotsStatus = Vec<BallotStatus, &DecodedVoteContest>;

pub struct InstantRunoff {
    pub tally: Tally,
}

impl InstantRunoff {
    #[instrument(skip_all)]
    pub fn new(tally: Tally) -> Self {
        Self { tally }
    }

    fn initialize_candidates_status(&self) -> CandidatesStatus {
        let candidates = self.tally.contest.candidates;
        let mut candidates_status: CandidatesStatus = HashMap::new();
        for candidate in candidates {
            candidates_status.insert(candidate.id.clone(), CandidateStatus::Active);
        }
        candidates_status
    }

    fn initialize_ballots_status(&self) -> BallotsStatus {
        let votes = &self.tally.ballots;
        // Create an array like Vec<BallotStatus, &DecodedVoteContest>
        // with all the statues to BallotStatus::Valid
        let mut statuses: BallotsStatus = Vec::with_capacity(votes.len());
        for vote in votes {
            statuses.push(BallotStatus::Valid, vote);
        }
        statuses
    }
}

impl CountingAlgorithm for InstantRunoff {
    #[instrument(err, skip_all)]
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

        let mut extended_metrics = ExtendedMetricsContest::default();

        for vote in votes {
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
                        *vote_count.entry(choice.id.clone()).or_insert(0) += 1;
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

        extended_metrics.total_ballots = votes.len() as u64;

        let candidate_results_map: HashMap<String, CandidateResult> = vote_count
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
                        / cmp::max(1, count_valid - count_blank) as f64)
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

        let result: Vec<CandidateResult> = contest
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

        let total_votes = count_valid + count_invalid;
        let total_votes_base = cmp::max(1, total_votes) as f64;

        let census_base = cmp::max(1, self.tally.census) as f64;
        let percentage_auditable_votes = (self.tally.auditable_votes as f64) * 100.0 / census_base;
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
            auditable_votes: self.tally.auditable_votes,
            percentage_auditable_votes: percentage_auditable_votes.clamp(0.0, 100.0),
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
            extended_metrics: Some(extended_metrics),
        };

        let aggregate = self
            .tally
            .tally_sheet_results
            .iter()
            .fold(contest_result, |acc, x| acc.aggregate(x, false));

        Ok(aggregate)
    }
}
