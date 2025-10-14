// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{
    counting_algorithm::common::*, tally::Tally, CandidateResult, ContestResult,
    ExtendedMetricsContest, InvalidVotes,
};
use sequent_core::ballot::{self, Candidate, Contest};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest, InvalidPlaintextErrorType};
use sequent_core::sqlite::candidate;
use std::cmp;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use tracing::{info, instrument};

use super::Result;

#[derive(PartialEq, Debug)]
enum ECandidateStatus {
    Active,
    Eliminated,
}

impl ECandidateStatus {
    fn is_active(&self) -> bool {
        self == &ECandidateStatus::Active
    }
}

#[derive(PartialEq, Debug)]
enum BallotStatus {
    Valid,
    Exhausted,
    Invalid,
    Blank,
}

#[derive(Debug)]
struct BallotsStatus<'a> {
    ballots: Vec<(BallotStatus, &'a DecodedVoteContest)>,
    count_valid: u64,
    count_invalid_votes: InvalidVotes,
    count_blank: u64,
    extended_metrics: ExtendedMetricsContest,
}

impl BallotsStatus<'_> {
    /// Set initial statuses for all the ballots depending on if they are valid, invalid or blank.
    /// Set the metrics and counts.
    #[instrument(skip_all)]
    fn initialize_statuses<'a>(
        votes: &'a Vec<DecodedVoteContest>,
        contest: &Contest,
    ) -> BallotsStatus<'a> {
        let mut count_invalid_votes = InvalidVotes {
            explicit: 0,
            implicit: 0,
        };
        let mut count_blank: u64 = 0;
        let mut extended_metrics = ExtendedMetricsContest::default();
        let mut ballots = Vec::with_capacity(votes.len());

        for vote in votes {
            let status = match (vote.is_invalid(), vote.is_blank()) {
                (true, _) => {
                    if vote.is_explicit_invalid {
                        count_invalid_votes.explicit += 1;
                    } else {
                        count_invalid_votes.implicit += 1;
                    }
                    BallotStatus::Invalid
                }
                (false, true) => {
                    count_blank += 1;
                    BallotStatus::Blank
                }
                (false, false) => BallotStatus::Valid,
            };
            extended_metrics = update_extended_metrics(vote, &extended_metrics, contest);
            ballots.push((status, vote));
        }
        let total_ballots = votes.len() as u64;
        extended_metrics.total_ballots = total_ballots;
        let count_valid = total_ballots
            - count_invalid_votes.explicit
            - count_invalid_votes.implicit
            - count_blank;
        BallotsStatus {
            ballots,
            count_valid,
            count_invalid_votes,
            extended_metrics,
            count_blank,
        }
    }

    /// We take into account the redristribution of votes here.
    /// The first choice is the one which candidate is not eliminated in order of preference.
    /// This avoids having to modify the ballots list in memory.
    #[instrument(skip_all)]
    fn is_first_not_eliminated_choice(
        &self,
        candidate_id: &str,
        choices: &Vec<DecodedVoteChoice>,
        candidates_status: &CandidatesStatus,
    ) -> bool {
        // First find the original preferece of the candidate
        let candidate_choice = choices.iter().find(|vote| vote.id == candidate_id);
        let preference = match candidate_choice {
            Some(candidate_choice) if candidate_choice.selected >= 0 => candidate_choice.selected,
            _ => return false,
        };
        // Check if all choices with more preference are eliminated candidates, only then it is a first choice in this round
        // if preference is 0 (highest) then it does not enter the loop and the fn returns true
        for p in 0..preference {
            // Find the candidate id for this preference
            let choice_candidate_id = choices
                .iter()
                .find(|choice| choice.selected == p)
                .map(|choice| choice.id.clone());
            match choice_candidate_id {
                Some(choice_candidate_id) => {
                    if candidates_status.is_active_candidate(&choice_candidate_id) {
                        return false; // A non eliminated candidate is found
                    }
                }
                None => return false,
            }
        }
        true
    }

    #[instrument(skip_all)]
    fn count_candidate_first_choices(
        &self,
        candidate_id: &str,
        candidates_status: &CandidatesStatus,
    ) -> u64 {
        let wins = self
            .ballots
            .iter()
            .filter_map(|(ballot_status, vote)| {
                if *ballot_status == BallotStatus::Valid {
                    Some(vote)
                } else {
                    None
                }
            })
            .fold(0, |acc, vote| {
                match self.is_first_not_eliminated_choice(
                    candidate_id,
                    &vote.choices,
                    candidates_status,
                ) {
                    true => acc + 1,
                    false => acc,
                }
            });
        wins
    }
}

/// Number of first choices for each candidate id
type CandidatesWins = HashMap<String, u64>;

#[derive(Debug, Default)]
struct CandidatesStatus(HashMap<String, ECandidateStatus>);

impl Deref for CandidatesStatus {
    type Target = HashMap<String, ECandidateStatus>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CandidatesStatus {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl CandidatesStatus {
    #[instrument(skip_all)]
    fn is_active_candidate(&self, candidate_id: &str) -> bool {
        self.get(candidate_id)
            .map(|candidate_status| candidate_status.is_active())
            .unwrap_or(false)
    }

    #[instrument(skip_all)]
    fn get_active_candidates(&self) -> Vec<String> {
        let mut active_candidates: Vec<String> = Vec::new();
        for (candidate_id, candidate_status) in &self.0 {
            if candidate_status.is_active() {
                active_candidates.push(candidate_id.clone());
            }
        }
        active_candidates
    }

    #[instrument(skip_all)]
    fn set_candidate_to_eliminated(&mut self, candidate_id: &str) {
        self.insert(candidate_id.to_string(), ECandidateStatus::Eliminated);
    }
}

#[derive(Default, Debug)]
struct Round {
    winner: Option<String>,
    candidates_wins: CandidatesWins,
}

#[derive(Default, Debug)]
struct RunoffStatus {
    candidates_status: CandidatesStatus,
    round_count: u64,
    rounds: Vec<Round>,
}

impl RunoffStatus {
    #[instrument(skip_all)]
    fn initialize_statuses(candidates: &Vec<Candidate>) -> RunoffStatus {
        let mut status: RunoffStatus = RunoffStatus {
            candidates_status: CandidatesStatus(HashMap::new()),
            ..Default::default()
        };
        for candidate in candidates {
            status
                .candidates_status
                .insert(candidate.id.clone(), ECandidateStatus::Active);
        }
        status
    }

    #[instrument(skip_all)]
    fn get_last_round_winner(&self) -> Option<String> {
        self.rounds.last().and_then(|r| r.winner.clone())
    }

    #[instrument(skip_all)]
    fn run_round(&mut self, ballots_status: &mut BallotsStatus) {
        // TODO: Implement rounds
        let mut round = Round::default();
        let mut candidates_wins: CandidatesWins = HashMap::new();
        let mut min_wins = ballots_status.count_valid;

        let act_candidates = self.candidates_status.get_active_candidates();
        for candidate_id in act_candidates {
            let wins = ballots_status
                .count_candidate_first_choices(&candidate_id, &self.candidates_status);
            min_wins = min_wins.min(wins);
            candidates_wins.insert(candidate_id.clone(), wins);
            if wins > ballots_status.count_valid / 2 {
                round.winner = Some(candidate_id.clone());
            }
        }

        if round.winner.is_none()
        /* And there is not a tie */
        {
            // find the Active candidate(s) with the fewest votes. (filter by min number in candidates_wins)
            // if there s a tie (more than one is left after the filter) try to find only one by the loopback rule
            // Continue the loop back until the tie is broken

            // Simultaneous Elimination: If not found the clear looser, eliminate all the candidates with the min wins,
            // but if all active candidates have min wins (all to be eliminated) then there is a winner tie, so end the election and the winner will be decided by lot.
        }

        round.candidates_wins = candidates_wins;
        self.round_count += 1;
        self.rounds.push(round);
    }
}

pub struct InstantRunoff {
    pub tally: Tally,
}

impl InstantRunoff {
    #[instrument(skip_all)]
    pub fn new(tally: Tally) -> Self {
        Self { tally }
    }
}

impl CountingAlgorithm for InstantRunoff {
    #[instrument(err, skip_all)]
    fn tally(&self) -> Result<ContestResult> {
        let contest = &self.tally.contest;
        let votes: &Vec<DecodedVoteContest> = &self.tally.ballots;

        let mut ballots_status = BallotsStatus::initialize_statuses(votes, contest);
        let count_blank = ballots_status.count_blank;
        let count_valid = ballots_status.count_valid;
        let count_invalid_votes = ballots_status.count_invalid_votes;
        let count_invalid = count_invalid_votes.explicit + count_invalid_votes.implicit;
        let extended_metrics = ballots_status.extended_metrics;
        let mut candidates_status = RunoffStatus::initialize_statuses(&contest.candidates);

        //... TODO: Finnish to implement rounds
        while candidates_status.get_last_round_winner().is_none() {
            candidates_status.run_round(&mut ballots_status);
        }

        let mut vote_count: HashMap<String, u64> = HashMap::new(); // TODO: Remove left over from plurality

        // Set percentage votes for each candidate
        // TODO: recicle code from plurality to common
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
