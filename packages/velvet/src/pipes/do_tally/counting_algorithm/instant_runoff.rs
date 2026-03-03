// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::Result;
use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{
    counting_algorithm::utils::*, tally::Tally, CandidateResult, ContestResult,
    ExtendedMetricsContest, InvalidVotes,
};
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::vec;
use sequent_core::ballot::{Candidate, Contest, TieBreakingPolicy, Weight};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use serde::{Deserialize, Serialize};
use serde_json;
use std::cmp;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use tracing::{info, instrument, warn};

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RunoffStatus {
    pub candidates_status: CandidatesStatus,
    pub name_references: Vec<CandidateReference>, // Maps candidate ID to name and serves as an ordered by results list in the end.
    pub round_count: u64,
    pub rounds: Vec<Round>,
    pub max_rounds: u64,
    pub tie_resolutions: Vec<TieBreakingState>, // Tracks all tie resolutions (random and external)
}

/// Method used to break a tie
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TieBreakingMethod {
    Random,
    ExternalProcedure,
    Lookback,
}

/// State of a tie that requires external resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieBreakingState {
    pub round_number: u64,
    pub tied_candidate_ids: Vec<String>,
    pub vote_counts: Vec<u64>,
    pub method_used: TieBreakingMethod,
    pub resolved_by_candidate_id: Option<String>,
}

/// Result of running the IRV algorithm
#[derive(Debug)]
pub enum RunoffResult {
    /// Normal completion (winner found or max rounds reached)
    Completed(RunoffStatus),
    /// Tie detected with external procedure policy - needs admin input
    RequiresExternalInput {
        state: RunoffStatus,
        tie_info: TieBreakingState,
    },
}

/// Internal enum for round-by-round execution
enum RoundResult {
    /// Continue to next round
    Continue,
    /// Runoff is finished (winner found or max rounds)
    Finished,
    /// Tie requires external input
    RequiresInput {
        candidates_to_eliminate: Vec<String>,
        round_number: u64,
    },
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

    #[instrument]
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
    #[instrument]
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

    pub fn determine_winner_by_lot(
        &mut self,
        candidates_to_eliminate: &Vec<String>,
        candidates_wins: &CandidatesOutcomes,
        tie_breaking_policy: &TieBreakingPolicy,
        tie_resolution_candidate: Option<&str>,
    ) -> Option<(CandidateReference, Vec<CandidateReference>)> {
        // FULL TIE: All active candidates have the same (lowest) number of votes

        // Extract vote counts for tied candidates
        let vote_counts: Vec<u64> = candidates_to_eliminate
            .iter()
            .map(|id| candidates_wins.get(id).map(|o| o.wins).unwrap_or(0))
            .collect();

        // Check if we have a pre-configured tie resolution (resume scenario)
        if let Some(resolved_candidate_id) = tie_resolution_candidate {
            if candidates_to_eliminate.contains(&resolved_candidate_id.to_string()) {
                info!(
                    "IRV tie detected among {} candidates. Using pre-configured resolution: {}",
                    candidates_to_eliminate.len(),
                    resolved_candidate_id
                );

                // Record this resolution
                self.tie_resolutions.push(TieBreakingState {
                    round_number: self.round_count + 1,
                    tied_candidate_ids: candidates_to_eliminate.clone(),
                    vote_counts: vote_counts.clone(),
                    method_used: TieBreakingMethod::ExternalProcedure,
                    resolved_by_candidate_id: Some(resolved_candidate_id.to_string()),
                });

                let winner_name = self
                    .get_candidate_name(resolved_candidate_id)
                    .unwrap_or_default();
                let winner = CandidateReference {
                    id: resolved_candidate_id.to_string(),
                    name: winner_name.clone(),
                };

                // Mark all others as eliminated, keep only the resolved winner active
                let mut eliminated = Vec::new();
                for candidate_id in candidates_to_eliminate {
                    if candidate_id == resolved_candidate_id {
                        continue;
                    }
                    self.candidates_status
                        .set_candidate_to_eliminated(candidate_id);
                    eliminated.push(CandidateReference {
                        id: candidate_id.to_string(),
                        name: self.get_candidate_name(candidate_id).unwrap_or_default(),
                    });
                }

                return Some((winner, eliminated));
            } else {
                warn!(
                    "Configured tie resolution candidate {} is not among tied candidates: {:?}. Ignoring.",
                    resolved_candidate_id,
                    candidates_to_eliminate
                );
            }
        }

        // Check policy - if external procedure and no resolution, pause needed
        if *tie_breaking_policy == TieBreakingPolicy::EXTERNAL_PROCEDURE {
            info!(
                "IRV tie detected among {} candidates with EXTERNAL_PROCEDURE policy. Pausing for admin input.",
                candidates_to_eliminate.len()
            );
            return None; // Signal that we need to pause for external input
        }

        // No meaningful elimination possible → winner decided by lot (random)
        let mut rng = thread_rng();
        let Some(winner_id) = candidates_to_eliminate.choose(&mut rng) else {
            return None;
        };
        let winner_name = self.get_candidate_name(winner_id).unwrap_or_default();
        info!(
            "IRV full tie detected among {} candidates. Selecting winner by lot: {} ({})",
            candidates_to_eliminate.len(),
            winner_name,
            winner_id
        );

        // Record this random resolution
        self.tie_resolutions.push(TieBreakingState {
            round_number: self.round_count + 1,
            tied_candidate_ids: candidates_to_eliminate.clone(),
            vote_counts,
            method_used: TieBreakingMethod::Random,
            resolved_by_candidate_id: Some(winner_id.to_string()),
        });

        let winner = CandidateReference {
            id: winner_id.to_string(),
            name: winner_name.clone(),
        };
        // Mark all others as eliminated, keep only the random winner active
        let mut eliminated = Vec::new();
        for candidate_id in candidates_to_eliminate {
            if candidate_id == winner_id {
                continue;
            }
            self.candidates_status
                .set_candidate_to_eliminated(candidate_id);
            eliminated.push(CandidateReference {
                id: candidate_id.to_string(),
                name: self.get_candidate_name(candidate_id).unwrap_or_default(),
            });
        }
        // Winner remains active, will be detected in next round or immediately
        return Some((winner, eliminated));
    }

    /// Returns which candidates were eliminated.
    /// Returns None if cannot do the eliminations because a tie was found.
    #[instrument]
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
        if 2 * max_wins > act_ballots {
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
                if let Some(eliminated_candidates) = eliminated_candidates {
                    round.eliminated_candidates = Some(eliminated_candidates);
                } else {
                    // NOTE: This method is for backward compatibility and always uses RANDOM policy
                    // For proper tie-breaking policy support, callers should use run_with_policy() instead
                    if let Some((winner, eliminated_candidates)) = self.determine_winner_by_lot(
                        &candidates_to_eliminate,
                        &candidates_wins,
                        &TieBreakingPolicy::RANDOM,
                        None,
                    ) {
                        round.winner = Some(winner);
                        round.eliminated_candidates = Some(eliminated_candidates);
                    };
                };
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
    /// Apply an external tie-breaking decision
    /// Eliminates all candidates except the chosen winner
    /// Returns error if winner_id is invalid
    #[instrument]
    pub fn apply_external_tie_decision(&mut self, winner_id: &str) -> Result<(), String> {
        // Validate winner_id is in candidates_status
        if !self.candidates_status.contains_key(winner_id) {
            return Err(format!("Invalid candidate ID: {}", winner_id));
        }

        // Get all active candidates
        let active_candidates = self.candidates_status.get_active_candidate_ids();

        // Validate winner is actually active
        if !active_candidates.contains(&winner_id.to_string()) {
            return Err(format!(
                "Candidate {} is not active (may already be eliminated)",
                winner_id
            ));
        }

        // Eliminate all except the chosen winner
        for candidate_id in &active_candidates {
            if candidate_id != winner_id {
                self.candidates_status
                    .set_candidate_to_eliminated(&candidate_id);
            }
        }

        // Update last round with winner and eliminated list
        // Get candidate names before mutable borrow
        let winner = CandidateReference {
            id: winner_id.to_string(),
            name: self.get_candidate_name(winner_id).unwrap_or_default(),
        };

        let eliminated: Vec<CandidateReference> = active_candidates
            .into_iter()
            .filter(|id| id != winner_id)
            .map(|id| CandidateReference {
                id: id.clone(),
                name: self.get_candidate_name(&id).unwrap_or_default(),
            })
            .collect();

        if let Some(last_round) = self.rounds.last_mut() {
            last_round.winner = Some(winner);
            last_round.eliminated_candidates = Some(eliminated);
        }

        Ok(())
    }

    /// Run next round with tie-breaking policy support
    /// Returns RoundResult indicating what should happen next
    fn run_next_round_with_policy(
        &mut self,
        ballots_status: &mut BallotsStatus,
        tie_breaking_policy: &TieBreakingPolicy,
        tie_resolution_candidate: Option<&str>,
    ) -> RoundResult {
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
        if 2 * max_wins > act_ballots {
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
        let round_result = match round.winner.is_some() {
            true => RoundResult::Finished,
            false => {
                // Find the Active candidate(s) with the fewest votes
                let least_wins = candidates_wins.values().map(|o| o.wins).min().unwrap_or(0);
                let candidates_to_eliminate: Vec<String> =
                    self.filter_candidates_by_number_of_wins(&candidates_wins, least_wins);
                let eliminated_candidates =
                    self.do_round_eliminations(&candidates_wins, &candidates_to_eliminate);

                match eliminated_candidates {
                    Some(eliminated_candidates) => {
                        round.eliminated_candidates = Some(eliminated_candidates);
                        RoundResult::Continue
                    }
                    None => {
                        // Tie detected - check policy and resolution
                        if let Some((winner, eliminated_candidates)) = self.determine_winner_by_lot(
                            &candidates_to_eliminate,
                            &candidates_wins,
                            tie_breaking_policy,
                            tie_resolution_candidate,
                        ) {
                            // Random policy resolved the tie
                            round.winner = Some(winner);
                            round.eliminated_candidates = Some(eliminated_candidates);
                            RoundResult::Finished
                        } else {
                            // External procedure policy - pause needed
                            // Don't update round yet, will be updated when resumed
                            round.active_ballots_count = act_ballots;
                            round.active_candidates_count = act_candidates_count;
                            round.exhausted_ballots_count = exhausted_ballots;
                            round.candidates_wins = candidates_wins.clone();
                            round = self.fill_candidate_wins_names(&round);
                            self.rounds.push(round);
                            self.round_count += 1;

                            return RoundResult::RequiresInput {
                                candidates_to_eliminate,
                                round_number: self.round_count,
                            };
                        }
                    }
                }
            }
        };

        round.active_ballots_count = act_ballots;
        round.active_candidates_count = act_candidates_count;
        round.exhausted_ballots_count = exhausted_ballots;
        round.candidates_wins = candidates_wins;
        round = self.fill_candidate_wins_names(&round);
        self.rounds.push(round);
        self.round_count += 1;

        round_result
    }

    /// Run the IRV algorithm with tie-breaking policy support
    /// Returns RunoffResult indicating completion or need for external input
    #[instrument(skip_all)]
    pub fn run_with_policy(
        &mut self,
        ballots_status: &mut BallotsStatus,
        tie_breaking_policy: &TieBreakingPolicy,
        tie_resolution_candidate: Option<&str>,
    ) -> RunoffResult {
        let mut iterations = 0;
        while iterations < self.max_rounds {
            match self.run_next_round_with_policy(
                ballots_status,
                tie_breaking_policy,
                tie_resolution_candidate,
            ) {
                RoundResult::Continue => {
                    iterations += 1;
                }
                RoundResult::Finished => {
                    self.name_references = self.order_name_references_by_result();
                    return RunoffResult::Completed(self.clone());
                }
                RoundResult::RequiresInput {
                    candidates_to_eliminate,
                    round_number,
                } => {
                    // Build tie state
                    let last_round = self.get_last_round().unwrap();
                    let vote_counts: Vec<u64> = candidates_to_eliminate
                        .iter()
                        .map(|id| {
                            last_round
                                .candidates_wins
                                .get(id)
                                .map(|o| o.wins)
                                .unwrap_or(0)
                        })
                        .collect();

                    let tie_info = TieBreakingState {
                        round_number,
                        tied_candidate_ids: candidates_to_eliminate,
                        vote_counts,
                        method_used: TieBreakingMethod::ExternalProcedure,
                        resolved_by_candidate_id: None,
                    };

                    return RunoffResult::RequiresExternalInput {
                        state: self.clone(),
                        tie_info,
                    };
                }
            }
        }

        self.name_references = self.order_name_references_by_result();
        RunoffResult::Completed(self.clone())
    }

    /// Keep existing run() method for backward compatibility
    /// Calls run_with_policy with RANDOM policy
    pub fn run(&mut self, ballots_status: &mut BallotsStatus) {
        let result = self.run_with_policy(ballots_status, &TieBreakingPolicy::RANDOM, None);
        match result {
            RunoffResult::Completed(status) => {
                *self = status;
            }
            RunoffResult::RequiresExternalInput { .. } => {
                // Should never happen with RANDOM policy
                panic!("Unexpected RequiresExternalInput with RANDOM policy");
            }
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

                // Get tie-breaking policy from contest
                let tie_breaking_policy = contest.get_tie_breaking_policy();

                // Check for tie resolution in contest annotations (for resume scenario)
                let tie_resolution_candidate: Option<String> = contest
                    .annotations
                    .as_ref()
                    .and_then(|annotations| annotations.get("tie_resolution"))
                    .and_then(|json_str| {
                        // Parse the JSON string
                        let tie_res_value: serde_json::Value =
                            serde_json::from_str(json_str).ok()?;
                        // Extract the candidate ID
                        tie_res_value
                            .get("resolved_by_candidate_id")
                            .and_then(|id_value| id_value.as_str())
                            .map(|s| s.to_string())
                    });

                if let Some(ref candidate_id) = tie_resolution_candidate {
                    info!(
                        "Found tie resolution in contest annotations: will choose candidate {} if tie occurs",
                        candidate_id
                    );
                }

                // Run with policy support
                let runoff_result = runoff.run_with_policy(
                    &mut ballots_status,
                    &tie_breaking_policy,
                    tie_resolution_candidate.as_deref(),
                );

                // Handle the result
                let (runoff, process_results_value) = match runoff_result {
                    RunoffResult::Completed(status) => {
                        // Normal completion - serialize runoff state
                        let mut runoff_value = serde_json::to_value(&status)
                            .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                        // If there were any tie resolutions, embed them at the top level for easy detection
                        if !status.tie_resolutions.is_empty() {
                            if let Some(obj) = runoff_value.as_object_mut() {
                                obj.insert(
                                    "resolved_tie_resolutions".to_string(),
                                    serde_json::to_value(&status.tie_resolutions)
                                        .unwrap_or(serde_json::json!([])),
                                );
                            }
                        }

                        (status, runoff_value)
                    }
                    RunoffResult::RequiresExternalInput { state, tie_info } => {
                        info!(
                            "Tie detected requiring external input: round {}, {} candidates tied - creating partial results",
                            tie_info.round_number,
                            tie_info.tied_candidate_ids.len()
                        );

                        // Serialize the partial runoff state with embedded tie information
                        let mut partial_runoff_value = serde_json::to_value(&state)
                            .map_err(|e| Error::UnexpectedError(e.to_string()))?;

                        // Embed tie information in the process_results for windmill to detect
                        if let Some(obj) = partial_runoff_value.as_object_mut() {
                            obj.insert(
                                "pending_tie_resolution".to_string(),
                                serde_json::json!({
                                    "round_number": tie_info.round_number,
                                    "tied_candidate_ids": tie_info.tied_candidate_ids,
                                    "vote_counts": tie_info.vote_counts,
                                    "total_votes": count_valid + count_invalid,
                                    "method_used": format!("{:?}", tie_info.method_used),
                                }),
                            );
                        }

                        (state, partial_runoff_value)
                    }
                };

                let mut vote_count: HashMap<String, u64> = HashMap::new(); // vote_count has only the last round results or it could be left empty because the full results are in process_results_value
                if let Some(results) = runoff.get_last_round() {
                    vote_count = results
                        .candidates_wins
                        .into_iter()
                        .map(|(candidate_id, outcome)| (candidate_id, outcome.wins))
                        .collect();
                }

                let candidate_result = self.tally.create_candidate_results(
                    vote_count,
                    count_blank,
                    count_invalid_votes.clone(),
                    extended_metrics.clone(),
                    count_valid,
                    count_invalid,
                    percentage_votes_denominator,
                )?;
                (candidate_result, Some(process_results_value))
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
