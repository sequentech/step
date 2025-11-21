// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::Result;
use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{
    counting_algorithm::utils::*, tally::Tally, CandidateResult, ContestResult,
    ExtendedMetricsContest, InvalidVotes,
};
use rayon::vec;
use sequent_core::ballot::{Candidate, Contest, Weight};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use serde::{Deserialize, Serialize};
use serde_json;
use std::cmp;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use tracing::{info, instrument};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CandidateReference {
    pub id: String,
    pub name: String,
}

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
    pub fn initialize_ballots_status<'a>(
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

/// Outcome for each candidate in a round
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CandidateOutcome {
    pub name: String,
    pub wins: u64,
    pub transference: i64,
    pub percentage: f64,
}

type CandidatesOutcomes = HashMap<String, CandidateOutcome>;

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
    fn initialize_candidates_wins(&self) -> CandidatesOutcomes {
        let mut candidates_wins: CandidatesOutcomes = HashMap::new();
        for (candidate_id, status) in self.0.iter() {
            if status.is_active() {
                candidates_wins.insert(
                    candidate_id.clone(),
                    CandidateOutcome {
                        name: "".to_string(),
                        wins: 0,
                        transference: 0,
                        percentage: 0.0,
                    },
                );
            }
        }
        candidates_wins
    }

    #[instrument(skip_all)]
    fn get_active_candidate_ids(&self) -> Vec<String> {
        self.0
            .iter()
            .filter_map(|(candidate_id, status)| {
                if status.is_active() {
                    Some(candidate_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    #[instrument(skip_all)]
    fn set_candidate_to_eliminated(&mut self, candidate_id: &str) {
        self.insert(candidate_id.to_string(), ECandidateStatus::Eliminated);
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    pub winner: Option<CandidateReference>,
    pub candidates_wins: CandidatesOutcomes,
    pub eliminated_candidates: Option<Vec<CandidateReference>>,
    pub active_candidates_count: u64, // Number of active candidates when starting this round
    pub active_ballots_count: u64,    // Number of active ballots when starting this round
    pub exhausted_ballots_count: u64, // Number of exhausted ballots in this round
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct RunoffStatus {
    pub candidates_status: CandidatesStatus,
    pub name_references: Vec<CandidateReference>, // Maps candidate ID to name and serves as an ordered by results list in the end.
    pub round_count: u64,
    pub rounds: Vec<Round>,
    pub max_rounds: u64,
}

impl RunoffStatus {
    #[instrument(skip_all)]
    pub fn initialize_runoff(contest: &Contest) -> RunoffStatus {
        let max_rounds = contest.candidates.len() as u64 + 1; // At least 1 candidate is eliminated per round
        let mut candidates_status = CandidatesStatus(HashMap::new());
        let mut name_references = vec![];
        for candidate in &contest.candidates {
            candidates_status.insert(candidate.id.clone(), ECandidateStatus::Active);
            name_references.push(CandidateReference {
                id: candidate.id.clone(),
                name: candidate.name.clone().unwrap_or_default(),
            });
        }
        RunoffStatus {
            candidates_status,
            name_references,
            max_rounds,
            ..Default::default()
        }
    }

    #[instrument(skip_all)]
    pub fn get_candidate_name(&self, candidate_id: &str) -> Option<String> {
        self.name_references
            .iter()
            .find(|c| c.id == candidate_id)
            .map(|c| c.name.clone())
    }

    #[instrument(skip_all)]
    pub fn fill_candidate_wins_names(&self, round: &Round) -> Round {
        let candidates_wins = round
            .candidates_wins
            .iter()
            .map(|(candidate_id, outcome)| {
                (
                    candidate_id.clone(),
                    CandidateOutcome {
                        name: self.get_candidate_name(candidate_id).unwrap_or_default(),
                        ..outcome.clone()
                    },
                )
            })
            .collect();

        Round {
            candidates_wins,
            ..round.clone()
        }
    }

    #[instrument(skip_all)]
    pub fn get_last_round(&self) -> Option<Round> {
        self.rounds.last().cloned()
    }

    #[instrument(skip_all)]
    pub fn filter_candidates_by_number_of_wins(
        &self,
        candidates_wins: &CandidatesOutcomes,
        n: u64,
    ) -> Vec<String> {
        candidates_wins
            .iter()
            .filter_map(|(candidate_id, outcome)| {
                if outcome.wins == n {
                    Some(candidate_id.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Calculate vote transferences for each candidate by comparing with previous round
    #[instrument(skip_all)]
    pub fn calculate_transferences(&self, current_wins: &CandidatesOutcomes) -> CandidatesOutcomes {
        let previous_round = self.get_last_round();
        let mut new_current_wins = current_wins.clone();
        if let Some(prev_round) = previous_round {
            for (candidate_id, outcome) in new_current_wins.iter_mut() {
                let prev_wins = prev_round
                    .candidates_wins
                    .get(candidate_id)
                    .map(|o| o.wins)
                    .unwrap_or(0);
                outcome.transference = outcome.wins as i64 - prev_wins as i64;
            }
        }
        // If no previous round, transference stays at 0 (initial values)
        new_current_wins
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
            let candidates_to_untie: CandidatesOutcomes = round
                .candidates_wins
                .iter()
                .filter(|(candidate_id, _)| round_possible_losers.contains(candidate_id))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let min_wins = candidates_to_untie
                .values()
                .map(|o| o.wins)
                .min()
                .unwrap_or(0);
            let losers = self.filter_candidates_by_number_of_wins(&candidates_to_untie, min_wins);
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
        candidates_wins: &CandidatesOutcomes,
        candidates_to_eliminate: &Vec<String>,
    ) -> Option<Vec<CandidateReference>> {
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
        } else {
            // Simultaneous Elimination can create corner cases where a winner is decided unfairly.
            // So many electoral systems pick a random candidate from the reduced list instead.
            // Note: Some systems can do simultaneous elimination when it is mathematically safe,
            // this is if the distance to the next more voted candidate is big enough.
            let mut eliminated = vec![];
            for candidate_id in &reduced_list {
                self.candidates_status
                    .set_candidate_to_eliminated(candidate_id);
                eliminated.push(CandidateReference {
                    id: candidate_id.clone(),
                    name: self.get_candidate_name(candidate_id).unwrap_or_default(),
                });
            }
            return Some(eliminated);
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
        active_candidate_ids: &Vec<String>,
    ) -> Option<String> {
        let mut choices: Vec<DecodedVoteChoice> = choices
            .iter()
            .filter(|choice| choice.selected >= 0)
            .cloned()
            .collect();

        choices.sort_by(|a, b| a.selected.cmp(&b.selected));
        for choice in choices {
            if active_candidate_ids.contains(&choice.id) {
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
        let act_candidate_ids = self.candidates_status.get_active_candidate_ids();
        let act_candidates_count = act_candidate_ids.len() as u64;
        let mut act_ballots = 0;
        let mut exhausted_ballots = self
            .get_last_round()
            .unwrap_or_default()
            .exhausted_ballots_count;

        for (ballot_st, ballot, weight) in ballots_status.ballots.iter_mut() {
            if *ballot_st != BallotStatus::Valid {
                continue;
            }
            let candidate_id = self.find_first_active_choice(&ballot.choices, &act_candidate_ids);
            let w = weight.unwrap_or_default();
            if let Some(candidate_id) = candidate_id {
                if let Some(outcome) = candidates_wins.get_mut(&candidate_id) {
                    outcome.wins += w;
                }
                act_ballots += 1;
            } else {
                *ballot_st = BallotStatus::Exhausted;
                exhausted_ballots += 1;
            }
        }

        candidates_wins = self.calculate_transferences(&candidates_wins);

        // Calculate percentages using act_ballots as denominator
        let act_ballots_f64 = cmp::max(1, act_ballots) as f64;
        for outcome in candidates_wins.values_mut() {
            outcome.percentage = ((outcome.wins as f64) / act_ballots_f64).clamp(0.0, 1.0);
        }

        // Check if there is a winner
        let max_wins = candidates_wins.values().map(|o| o.wins).max().unwrap_or(0);
        if max_wins > act_ballots / 2 {
            let winner_id = self
                .filter_candidates_by_number_of_wins(&candidates_wins, max_wins)
                .first()
                .cloned();
            round.winner = winner_id.and_then(|id| {
                Some(CandidateReference {
                    id: id.clone(),
                    name: self.get_candidate_name(&id).unwrap_or_default(),
                })
            });
        }

        // Eliminate candidates for the next round
        let continue_next_round = match round.winner.is_some() {
            true => false,
            false => {
                // Find the Active candidate(s) with the fewest votes
                let least_wins = candidates_wins.values().map(|o| o.wins).min().unwrap_or(0);
                let candidates_to_eliminate: Vec<String> =
                    self.filter_candidates_by_number_of_wins(&candidates_wins, least_wins);
                let eliminated_candidates =
                    self.do_round_eliminations(&candidates_wins, &candidates_to_eliminate);
                let continue_next_round = eliminated_candidates.is_some();
                round.eliminated_candidates = eliminated_candidates;
                continue_next_round
            }
        };
        round.active_ballots_count = act_ballots;
        round.active_candidates_count = act_candidates_count;
        round.exhausted_ballots_count = exhausted_ballots;
        round.candidates_wins = candidates_wins;
        round = self.fill_candidate_wins_names(&round);
        self.rounds.push(round);
        self.round_count += 1;
        return continue_next_round;
    }

    /// Order name_references to have the best results at the beginning
    #[instrument(skip_all)]
    pub fn order_name_references_by_result(&self) -> Vec<CandidateReference> {
        let mut new_name_references: Vec<CandidateReference> = vec![];
        if let Some(winner) = self.get_last_round().and_then(|r| r.winner.clone()) {
            new_name_references.push(winner);
        }
        for round in self.rounds.iter().rev() {
            for (candidate_id, candidate_outcome) in &round.candidates_wins {
                if new_name_references
                    .iter()
                    .find(|c| &c.id == candidate_id)
                    .is_none()
                {
                    new_name_references.push(CandidateReference {
                        id: candidate_id.clone(),
                        name: candidate_outcome.name.clone(),
                    })
                }
            }
        }
        new_name_references
    }

    #[instrument(skip_all)]
    pub fn run(&mut self, ballots_status: &mut BallotsStatus) {
        let mut iterations = 0;
        while self.run_next_round(ballots_status) && iterations < self.max_rounds {
            iterations += 1;
        }
        self.name_references = self.order_name_references_by_result();
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
    pub fn process_ballots(&self, op: TallyOperation) -> Result<ContestResult> {
        let contest = &self.tally.contest;
        let votes: &Vec<(DecodedVoteContest, Weight)> = &self.tally.ballots;

        let mut ballots_status = BallotsStatus::initialize_ballots_status(votes, contest);
        let count_blank = ballots_status.count_blank;
        let count_valid = ballots_status.count_valid;
        let count_invalid_votes = ballots_status.count_invalid_votes;
        let count_invalid = count_invalid_votes.explicit + count_invalid_votes.implicit;
        let extended_metrics = ballots_status.extended_metrics;
        let percentage_votes_denominator = count_valid - count_blank;

        let (candidate_result, process_results) = match op {
            TallyOperation::SkipCandidateResults => (vec![], None),
            _ => {
                let mut runoff = RunoffStatus::initialize_runoff(&contest);
                runoff.run(&mut ballots_status);

                let mut vote_count: HashMap<String, u64> = HashMap::new(); // vote_count has only the last round results or it could be left empty because the full results are in runoff_value
                if let Some(results) = runoff.get_last_round() {
                    vote_count = results
                        .candidates_wins
                        .into_iter()
                        .map(|(candidate_id, outcome)| (candidate_id, outcome.wins))
                        .collect();
                }

                // Create a json value from runoff object.
                let runoff_value = serde_json::to_value(runoff)
                    .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                let candidate_result = self.tally.create_candidate_results(
                    vote_count,
                    count_blank,
                    count_invalid_votes.clone(),
                    extended_metrics.clone(),
                    count_valid,
                    count_invalid,
                    percentage_votes_denominator,
                )?;
                (candidate_result, Some(runoff_value))
            }
        };

        self.tally.create_contest_result(
            process_results,
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

impl CountingAlgorithm for InstantRunoff {
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
                        "TallyOperation {op} is not supported for InstantRunoff at Area level"
                    )));
                }
                self.process_ballots(op)?
            }
        };
        Ok(contest_result)
    }
}
