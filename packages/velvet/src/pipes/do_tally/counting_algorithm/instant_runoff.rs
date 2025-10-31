// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::Result;
use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{
    counting_algorithm::utils::*, tally::Tally, CandidateResult, ContestResult,
    ExtendedMetricsContest, InvalidVotes,
};
use sequent_core::ballot::{Candidate, Contest, Weight};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use serde::{Deserialize, Serialize};
use serde_json;
use std::cmp;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use tracing::{info, instrument};

#[derive(PartialEq, Debug, Copy, Clone, Deserialize, Serialize)]
pub enum ECandidateStatus {
    Active,
    Eliminated,
}

impl ECandidateStatus {
    fn is_active(&self) -> bool {
        self == &ECandidateStatus::Active
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
enum BallotStatus {
    Valid,
    Exhausted,
    Invalid,
    Blank,
}

#[derive(Debug)]
pub struct BallotsStatus<'a> {
    ballots: Vec<(BallotStatus, &'a DecodedVoteContest, Weight)>,
    count_valid: u64,
    count_invalid_votes: InvalidVotes,
    count_blank: u64,
    extended_metrics: ExtendedMetricsContest,
}

impl BallotsStatus<'_> {
    /// Set initial statuses for all the ballots depending on if they are valid, invalid or blank.
    /// Set the metrics and counts.
    #[instrument(skip_all)]
    pub fn initialize_statuses<'a>(
        votes: &'a Vec<(DecodedVoteContest, Weight)>,
        contest: &Contest,
    ) -> BallotsStatus<'a> {
        let mut count_invalid_votes = InvalidVotes {
            explicit: 0,
            implicit: 0,
        };
        let mut count_blank: u64 = 0;
        let mut extended_metrics = ExtendedMetricsContest::default();
        let mut ballots = Vec::with_capacity(votes.len());

        for (vote, weight) in votes {
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
            ballots.push((status, vote, weight.clone()));
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
}

/// Number of first choices for each candidate id
type CandidatesWins = HashMap<String, u64>;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CandidatesStatus(pub HashMap<String, ECandidateStatus>);

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
    fn initialize_candidates_wins(&self) -> CandidatesWins {
        let mut candidates_wins: CandidatesWins = HashMap::new();
        for candidate_id in self.get_active_candidates() {
            candidates_wins.insert(candidate_id.to_string(), 0);
        }
        candidates_wins
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    pub winner: Option<String>,
    pub candidates_wins: CandidatesWins,
    pub eliminated_candidates: Option<Vec<String>>,
    pub active_candidates_count: u64, // Number of active candidates when starting this round
    pub active_ballots_count: u64,    // Number of active ballots when starting this round
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct RunoffStatus {
    pub candidates_status: CandidatesStatus,
    pub round_count: u64,
    pub rounds: Vec<Round>,
    pub max_rounds: u64,
}

impl RunoffStatus {
    #[instrument(skip_all)]
    pub fn initialize_statuses(candidates: &Vec<Candidate>) -> RunoffStatus {
        let max_rounds = candidates.len() as u64 + 1; // At least 1 candidate is eliminated per round
        let mut status: RunoffStatus = RunoffStatus {
            candidates_status: CandidatesStatus(HashMap::new()),
            max_rounds,
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
    pub fn get_last_round(&self) -> Option<Round> {
        self.rounds.last().cloned()
    }

    #[instrument(skip_all)]
    pub fn filter_candidates_by_number_of_wins(
        &self,
        candidates_wins: &CandidatesWins,
        n: u64,
    ) -> Vec<String> {
        candidates_wins
            .iter()
            .filter_map(|(candidate_id, wins)| {
                if *wins == n {
                    Some(candidate_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Tries to reduce the candidates to eliminate by the look back rule.
    /// Returns a list of candidates to eliminate.
    /// When the list is reduced to 1 candidate, returns only that candidate, but if there is a tie, returns the latest reduced list.
    #[instrument(skip_all)]
    pub fn find_single_candidate_to_eliminate(
        &self,
        candidates_to_eliminate: &Vec<String>,
    ) -> Vec<String> {
        let mut round_possible_losers = candidates_to_eliminate.clone();
        for round in self.rounds.iter().rev() {
            // Get the relevant results
            let candidates_to_untie: HashMap<String, u64> = round
                .candidates_wins
                .iter()
                .filter(|(id, _)| round_possible_losers.contains(id))
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            let min_wins = candidates_to_untie.values().min().unwrap_or(&0);
            let losers = self.filter_candidates_by_number_of_wins(&candidates_to_untie, *min_wins);
            if losers.len() == 1 {
                return losers;
            } else {
                // Continue the loop back until the tie is broken
                round_possible_losers = losers;
            }
        }
        round_possible_losers
    }

    /// Returns which candidates were eliminated.
    /// Returns None if cannot do the eliminations because a tie was found.
    #[instrument(skip_all)]
    pub fn do_round_eliminations(
        &mut self,
        candidates_wins: &CandidatesWins,
        candidates_to_eliminate: &Vec<String>,
    ) -> Option<Vec<String>> {
        let active_count = candidates_wins.len();
        let reduced_list = match candidates_to_eliminate.len() {
            0 => return None,
            1 => candidates_to_eliminate.clone(),
            _ => self.find_single_candidate_to_eliminate(candidates_to_eliminate), // Loop back case
                                                                                   // If there s a tie (more than one have least_wins) try to find the looser by the loopback rule.
        };

        if active_count == reduced_list.len() {
            // if all active candidates have the same wins (all to be eliminated) then there is a winner tie, so end the election and the winner will be decided by lot.
            return None;
        } else if reduced_list.len() == 1 {
            // If there is only one candidate left, eliminate it.
            self.candidates_status
                .set_candidate_to_eliminated(&reduced_list[0]);
            return Some(reduced_list);
        } else {
            // Simultaneous Elimination can create corner cases where a winner is decided unfairly.
            // So many electoral systems pick a random candidate from the reduced list instead.
            // Note: Some systems can do simultaneous elimination when it is mathematically safe,
            // this is if the distance to the next more voted candidate is big enough.
            for candidate_id in &reduced_list {
                self.candidates_status
                    .set_candidate_to_eliminated(candidate_id);
            }
            return Some(reduced_list);
        }
    }

    /// Returns None if the ballot is Exhausted.
    /// We take into account the redristribution of votes here...
    /// The first choice is the first not eliminated candidate_id in order of preference.
    /// This avoids having to modify the ballots list in memory.
    #[instrument(skip_all)]
    pub fn find_first_active_choice(
        &self,
        choices: &Vec<DecodedVoteChoice>,
        active_candidates: &Vec<String>,
    ) -> Option<String> {
        let mut choices: Vec<DecodedVoteChoice> = choices
            .iter()
            .filter(|choice| choice.selected >= 0)
            .cloned()
            .collect();

        choices.sort_by(|a, b| a.selected.cmp(&b.selected));
        for choice in choices {
            if active_candidates.contains(&choice.id) {
                return Some(choice.id.clone());
            }
        }
        None
    }

    /// Returns true if the process should continue for a next round.
    /// Returns false if there is a winner or a tie was concluded.
    #[instrument(skip_all)]
    pub fn run_next_round(&mut self, ballots_status: &mut BallotsStatus) -> bool {
        let mut round = Round::default();
        let mut candidates_wins = self.candidates_status.initialize_candidates_wins();
        let act_candidates = self.candidates_status.get_active_candidates();
        let act_candidates_count = act_candidates.len() as u64;
        let mut act_ballots = 0;
        for (ballot_st, ballot, weight) in ballots_status.ballots.iter_mut() {
            if *ballot_st != BallotStatus::Valid {
                continue;
            }
            act_ballots += 1;
            let candidate_id = self.find_first_active_choice(&ballot.choices, &act_candidates);
            let w = weight.unwrap_or_default();
            if let Some(candidate_id) = candidate_id {
                candidates_wins
                    .entry(candidate_id.clone())
                    .and_modify(|e| *e += w)
                    .or_insert(w);
            } else {
                *ballot_st = BallotStatus::Exhausted;
            }
        }

        info!("act_ballots in round {}: {act_ballots}", self.round_count);

        let max_wins = candidates_wins.values().max().unwrap_or(&0);
        if *max_wins > act_ballots / 2 {
            let candidate_id_opt = self
                .filter_candidates_by_number_of_wins(&candidates_wins, *max_wins)
                .first()
                .cloned();
            round.winner = candidate_id_opt;
        }

        let continue_next_round = match round.winner.is_some() {
            true => false,
            false => {
                // Find the Active candidate(s) with the fewest votes
                let least_wins = candidates_wins.values().min().unwrap_or(&0);
                let candidates_to_eliminate: Vec<String> =
                    self.filter_candidates_by_number_of_wins(&candidates_wins, *least_wins);
                let eliminated_candidates =
                    self.do_round_eliminations(&candidates_wins, &candidates_to_eliminate);
                let continue_next_round = eliminated_candidates.is_some();
                round.eliminated_candidates = eliminated_candidates;
                continue_next_round
            }
        };
        round.active_ballots_count = act_ballots;
        round.active_candidates_count = act_candidates_count;
        round.candidates_wins = candidates_wins;
        self.rounds.push(round);
        self.round_count += 1;
        return continue_next_round;
    }

    #[instrument(skip_all)]
    pub fn run(&mut self, ballots_status: &mut BallotsStatus) {
        let mut iterations = 0;
        while self.run_next_round(ballots_status) && iterations < self.max_rounds {
            iterations += 1;
        }
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

    #[instrument(err, skip_all)]
    pub fn process_ballots(&self) -> Result<ContestResult> {
        let contest = &self.tally.contest;
        let votes: &Vec<(DecodedVoteContest, Weight)> = &self.tally.ballots;

        let mut ballots_status = BallotsStatus::initialize_statuses(votes, contest);
        let count_blank = ballots_status.count_blank;
        let count_valid = ballots_status.count_valid;
        let count_invalid_votes = ballots_status.count_invalid_votes;
        let count_invalid = count_invalid_votes.explicit + count_invalid_votes.implicit;
        let extended_metrics = ballots_status.extended_metrics;
        let mut runoff = RunoffStatus::initialize_statuses(&contest.candidates);
        runoff.run(&mut ballots_status);

        let mut vote_count: HashMap<String, u64> = HashMap::new(); // TODO: Adapt the output results to have every round information.
        if let Some(results) = runoff.get_last_round() {
            vote_count = results.candidates_wins;
        }

        // Create a json value from runoff object.
        let runoff_value =
            serde_json::to_value(runoff).map_err(|e| Error::UnexpectedError(e.to_string()))?;
        let percentage_votes_denominator = count_valid - count_blank;

        self.tally.create_contest_result(
            Some(runoff_value),
            vote_count,
            count_blank,
            count_invalid_votes,
            extended_metrics,
            count_valid,
            count_invalid,
            percentage_votes_denominator,
        )
    }
}

impl CountingAlgorithm for InstantRunoff {
    #[instrument(err, skip_all)]
    fn tally(&self) -> Result<ContestResult> {
        let contest_result = match self.tally.scope_operation {
            ScopeOperation::Contest(op) => {
                if op != TallyOperation::ProcessBallots {
                    return Err(Error::InvalidTallyOperation(format!(
                        "TallyOperation {op} is not supported for InstantRunoff at Contest level"
                    )));
                }
                self.process_ballots()?
            }
            ScopeOperation::Area(op) => {
                if op != TallyOperation::ParticipationSummary {
                    return Err(Error::InvalidTallyOperation(format!(
                        "TallyOperation {op} is not supported for InstantRunoff at Area level"
                    )));
                }
                // TODO
                //self.participation_summary()?
                ContestResult::default()
            }
        };

        // Maybe IRV should not support tally sheets.
        let aggregate = self
            .tally
            .tally_sheet_results
            .iter()
            .fold(contest_result, |acc, x| acc.aggregate(x, false));

        Ok(aggregate)
    }
}
