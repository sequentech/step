// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Instant-runoff (IRV) counting algorithm.
//!
//! Lifted from velvet unchanged in behaviour; only import paths differ
//! (uses velvet-core's `CountingAlgorithm`, `Error`, `Tally`, result
//! types, and `update_extended_metrics` instead of the velvet-internal
//! crate paths).

use rand_core::RngCore;
use sequent_core::ballot::{Contest, TieBreakingPolicy, Weight};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::{
    ScopeOperation, TallyOperation, TallySessionResolutionData, TieBreakingMethod,
};
use serde::{Deserialize, Serialize};
use serde_json;
use std::cmp;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use tracing::{info, instrument};

use crate::counting::algorithm::CountingAlgorithm;
use crate::counting::error::{Error, Result};
use crate::counting::extended_metrics::*;
use crate::counting::tally::Tally;
use crate::result::{BlankVotes, ContestResult, ExtendedMetricsContest, InvalidVotes};

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
    blank_votes: BlankVotes,
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
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(contest);
        let mut count_invalid_votes = InvalidVotes::default();
        let mut blank_votes = BlankVotes::default();
        let mut count_declined_to_vote: u64 = 0;
        let mut extended_metrics = ExtendedMetricsContest::default();
        let mut ballots = Vec::with_capacity(votes.len());

        for (vote, weight) in votes {
            let status = match classify_ballot(vote, &explicit_blank_candidate_ids) {
                BallotClass::ExplicitInvalid => {
                    count_invalid_votes.explicit += 1;
                    BallotStatus::Invalid
                }
                BallotClass::ImplicitInvalid => {
                    count_invalid_votes.implicit += 1;
                    BallotStatus::Invalid
                }
                BallotClass::Declined => {
                    count_declined_to_vote = count_declined_to_vote.saturating_add(1);
                    BallotStatus::Blank
                }
                BallotClass::ExplicitBlank => {
                    blank_votes.explicit += 1;
                    BallotStatus::Blank
                }
                BallotClass::ImplicitBlank => {
                    blank_votes.implicit += 1;
                    BallotStatus::Blank
                }
                BallotClass::Valid => BallotStatus::Valid,
            };
            extended_metrics = update_extended_metrics(
                vote,
                &extended_metrics,
                contest,
                &explicit_blank_candidate_ids,
            );
            ballots.push((status, vote, weight.clone()));
        }
        let total_ballots = votes.len() as u64;
        extended_metrics.total_ballots = total_ballots;
        extended_metrics.total_declined_to_vote = count_declined_to_vote;
        let count_valid = total_ballots
            - count_invalid_votes.explicit
            - count_invalid_votes.implicit
            - count_declined_to_vote;
        BallotsStatus {
            ballots,
            count_valid,
            count_invalid_votes,
            extended_metrics,
            blank_votes,
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
    pub tie_breaking_policy: TieBreakingPolicy,
    pub tie_resolutions: Vec<TallySessionResolutionData>,
    pub pending_tie_resolution: Option<TallySessionResolutionData>,
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
            tie_breaking_policy: contest.get_tie_breaking_policy(),
            tie_resolutions: contest.get_tie_resolutions(),
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
        rng: &mut dyn RngCore,
        candidates_to_eliminate: &Vec<String>,
        candidates_wins: &CandidatesOutcomes,
    ) -> Option<(CandidateReference, Vec<CandidateReference>)> {
        // FULL TIE: All active candidates have the same (lowest) number of votes
        // No meaningful elimination possible → winner decided by tiebreak policy
        if candidates_to_eliminate.is_empty() {
            return None;
        }
        // Uniform index pick over `candidates_to_eliminate`. Done inline
        // here (instead of via `rand::seq::IndexedRandom::choose`) so
        // velvet-core only depends on `rand_core` — keeps the wasm
        // `getrandom` version footprint minimal.
        let index = (rng.next_u64() % candidates_to_eliminate.len() as u64) as usize;
        let winner_id = &candidates_to_eliminate[index];
        let winner_name = self.get_candidate_name(winner_id).unwrap_or_default();
        info!(
            "IRV full tie detected among {} candidates. Selecting winner by lot: {} ({})",
            candidates_to_eliminate.len(),
            winner_name,
            winner_id
        );

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

        // Fetch the single vote count
        let winner_votes = candidates_wins.get(winner_id).map_or(0, |o| o.wins);

        let resolution_data = TallySessionResolutionData {
            round_number: Some(self.round_count + 1),
            tied_candidate_ids: candidates_to_eliminate.clone(),
            vote_count: winner_votes,
            method_used: TieBreakingMethod::Random,
            resolved_by_candidate_id: Some(winner_id.clone()),
        };

        self.tie_resolutions.push(resolution_data);

        return Some((winner, eliminated));
    }

    pub fn determine_winner_by_external_procedure(
        &mut self,
        candidates_to_eliminate: &Vec<String>,
        candidates_wins: &CandidatesOutcomes,
    ) -> Option<(CandidateReference, Vec<CandidateReference>)> {
        let current_round = self.round_count + 1;

        // Check if there's a resolution that matches the tie.
        let existing_resolution = self.tie_resolutions.iter().find(|data| {
            data.round_number == Some(current_round)
                && data.tied_candidate_ids.len() == candidates_to_eliminate.len()
                && data
                    .tied_candidate_ids
                    .iter()
                    .all(|id| candidates_to_eliminate.contains(id))
                && data.resolved_by_candidate_id.is_some()
        });

        // If there is an existing resolution
        if let Some(data) = existing_resolution {
            if let Some(winner_id) = &data.resolved_by_candidate_id {
                let winner_name = self.get_candidate_name(winner_id).unwrap_or_default();

                let winner = CandidateReference {
                    id: winner_id.to_string(),
                    name: winner_name,
                };

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

                // Return since the resolution matched the tie.
                return Some((winner, eliminated));
            }
        }

        // Since they are all tied, just grab the vote count of the first candidate in the tie.
        let tied_votes = candidates_to_eliminate
            .first()
            .and_then(|id| candidates_wins.get(id))
            .map_or(0, |o| o.wins);

        let pending_data = TallySessionResolutionData {
            round_number: Some(current_round),
            tied_candidate_ids: candidates_to_eliminate.clone(),
            vote_count: tied_votes,
            method_used: TieBreakingMethod::ExternalProcedure,
            resolved_by_candidate_id: None,
        };

        self.pending_tie_resolution = Some(pending_data);

        None
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
            // if all active candidates have the same wins (all to be eliminated) then there is a winner tie, so end the election and the winner will be decided by tie breaking policy.
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
    pub fn run_next_round(
        &mut self,
        rng: &mut dyn RngCore,
        ballots_status: &mut BallotsStatus,
    ) -> bool {
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
                    let tie_resolution = match self.tie_breaking_policy {
                        TieBreakingPolicy::RANDOM => {
                            self.determine_winner_by_lot(rng, &candidates_to_eliminate, &candidates_wins)
                        }
                        TieBreakingPolicy::EXTERNAL_PROCEDURE => self
                            .determine_winner_by_external_procedure(
                                &candidates_to_eliminate,
                                &candidates_wins,
                            ),
                    };

                    if let Some((winner, eliminated_candidates)) = tie_resolution {
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
    pub fn run(&mut self, rng: &mut dyn RngCore, ballots_status: &mut BallotsStatus) {
        self.pending_tie_resolution = None;

        let mut iterations = 0;
        while self.run_next_round(rng, ballots_status) && iterations < self.max_rounds {
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
    pub fn process_ballots(
        &self,
        rng: &mut dyn RngCore,
        op: TallyOperation,
    ) -> Result<ContestResult> {
        let contest = &self.tally.contest;
        let votes: &Vec<(DecodedVoteContest, Weight)> = &self.tally.ballots;

        let mut ballots_status = BallotsStatus::initialize_ballots_status(votes, contest);
        let blank_votes = ballots_status.blank_votes;
        let count_blank = blank_votes.total();
        let count_valid = ballots_status.count_valid;
        let count_invalid_votes = ballots_status.count_invalid_votes;
        let count_invalid = count_invalid_votes.explicit + count_invalid_votes.implicit;
        // Cloned rather than moved: `ExtendedMetricsContest` is no longer
        // `Copy` (it carries a `VotesByChannel` map), and `ballots_status`
        // is still borrowed mutably below by `runoff.run`.
        let extended_metrics = ballots_status.extended_metrics.clone();
        let percentage_votes_denominator = count_valid - count_blank;

        let (candidate_result, process_results) = match op {
            TallyOperation::SkipCandidateResults => (vec![], None),
            _ => {
                let mut runoff = RunoffStatus::initialize_runoff(&contest);
                runoff.run(rng, &mut ballots_status);

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
                    blank_votes,
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
            blank_votes,
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
    fn tally(&self, rng: &mut dyn RngCore) -> Result<ContestResult> {
        let contest_result = match self.tally.scope_operation {
            ScopeOperation::Contest(op) if op == TallyOperation::AggregateResults => {
                self.tally.aggregate_results()?
            }
            ScopeOperation::Contest(op) => self.process_ballots(rng, op)?,
            ScopeOperation::Area(op) => {
                if op == TallyOperation::AggregateResults {
                    return Err(Error::InvalidTallyOperation(format!(
                        "TallyOperation {op} is not supported for InstantRunoff at Area level"
                    )));
                }
                self.process_ballots(rng, op)?
            }
        };

        // Paper / other-channel results arrive as pre-computed
        // ContestResults and are folded into the electronic tally.
        Ok(self
            .tally
            .tally_sheet_results
            .iter()
            .fold(contest_result, |result, tally_sheet_result| {
                result.aggregate(tally_sheet_result, false)
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::CandidateResult;
    use sequent_core::ballot::{Candidate, CandidatePresentation};

    /// velvet-core depends on `rand_core` alone (no `rand`), so tests
    /// supply their own generator. Deterministic on purpose: these cases
    /// must not reach a tie-break, and a fixed sequence makes that
    /// reproducible rather than luck.
    struct TestRng(u64);
    impl RngCore for TestRng {
        fn next_u32(&mut self) -> u32 {
            self.next_u64() as u32
        }
        fn next_u64(&mut self) -> u64 {
            // xorshift64*
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for chunk in dest.chunks_mut(8) {
                let bytes = self.next_u64().to_le_bytes();
                chunk.copy_from_slice(&bytes[..chunk.len()]);
            }
        }
    }
    use sequent_core::types::{participation::VotesByChannel, tally_sheets::VotingChannel};

    fn candidate(id: &str, is_explicit_blank: bool) -> Candidate {
        Candidate {
            id: id.to_string(),
            presentation: Some(CandidatePresentation {
                is_explicit_blank: Some(is_explicit_blank),
                ..CandidatePresentation::default()
            }),
            ..Candidate::default()
        }
    }

    fn contest() -> Contest {
        Contest {
            id: "contest".to_string(),
            max_votes: 1,
            candidates: vec![candidate("normal", false), candidate("blank", true)],
            ..Contest::default()
        }
    }

    fn contest_with_regular_candidates_and_blank() -> Contest {
        Contest {
            id: "contest".to_string(),
            max_votes: 1,
            counting_algorithm: Some(
                sequent_core::types::ceremonies::CountingAlgType::InstantRunoff,
            ),
            candidates: vec![
                candidate("candidate_a", false),
                candidate("candidate_b", false),
                candidate("blank", true),
            ],
            ..Contest::default()
        }
    }

    fn vote_with_selected_ids(selected_ids: &[&str]) -> DecodedVoteContest {
        let selected = |candidate_id: &str| {
            if selected_ids.contains(&candidate_id) {
                0
            } else {
                -1
            }
        };

        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: vec![
                DecodedVoteChoice {
                    id: "candidate_a".to_string(),
                    selected: selected("candidate_a"),
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "candidate_b".to_string(),
                    selected: selected("candidate_b"),
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "blank".to_string(),
                    selected: selected("blank"),
                    write_in_text: None,
                },
            ],
        }
    }

    fn instant_runoff(ballots: Vec<DecodedVoteContest>) -> InstantRunoff {
        let ballots = ballots
            .into_iter()
            .map(|ballot| (ballot, Weight::default()))
            .collect();

        InstantRunoff {
            tally: Tally {
                id: sequent_core::types::ceremonies::CountingAlgType::InstantRunoff,
                scope_operation: ScopeOperation::Contest(TallyOperation::ProcessBallotsAll),
                contest: contest_with_regular_candidates_and_blank(),
                ballots,
                census: 10,
                auditable_votes: 10,
                tally_sheet_results: vec![],
                tally_results: vec![],
            },
        }
    }

    fn mixed_explicit_blank_vote() -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: vec![
                DecodedVoteChoice {
                    id: "normal".to_string(),
                    selected: 0,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "blank".to_string(),
                    selected: 0,
                    write_in_text: None,
                },
            ],
        }
    }

    #[test]
    fn mixed_explicit_blank_vote_initializes_as_implicit_invalid() {
        let contest = contest();
        let votes = vec![(mixed_explicit_blank_vote(), Weight::default())];

        let status = BallotsStatus::initialize_ballots_status(&votes, &contest);

        assert_eq!(status.count_valid, 0);
        assert_eq!(status.count_invalid_votes.explicit, 0);
        assert_eq!(status.count_invalid_votes.implicit, 1);
        assert_eq!(status.blank_votes.explicit, 0);
        assert_eq!(status.blank_votes.implicit, 0);
        assert_eq!(status.ballots[0].0, BallotStatus::Invalid);
    }

    #[test]
    fn blank_heavy_contest_reports_blanks_as_valid_and_candidate_percentages_exclude_blanks() {
        let mut ballots = Vec::new();
        for _ in 0..3 {
            ballots.push(vote_with_selected_ids(&["blank"]));
            ballots.push(vote_with_selected_ids(&[]));
        }
        for _ in 0..4 {
            ballots.push(vote_with_selected_ids(&["candidate_a"]));
        }

        let result = instant_runoff(ballots)
            .process_ballots(&mut TestRng(0x5EED), TallyOperation::ProcessBallotsAll)
            .expect("blank-heavy contest should tally without underflow");

        assert_eq!(result.total_valid_votes, 10);
        assert_eq!(result.total_blank_votes, 6);
        assert_eq!(result.blank_votes.explicit, 3);
        assert_eq!(result.blank_votes.implicit, 3);
        assert_eq!(result.total_invalid_votes, 0);

        let candidate_a = result
            .candidate_result
            .iter()
            .find(|candidate| candidate.candidate.id == "candidate_a")
            .expect("candidate A result should exist");
        assert_eq!(candidate_a.total_count, 4);
        assert_eq!(candidate_a.percentage_votes, 100.0);

        let candidate_b = result
            .candidate_result
            .iter()
            .find(|candidate| candidate.candidate.id == "candidate_b")
            .expect("candidate B result should exist");
        assert_eq!(candidate_b.total_count, 0);
        assert_eq!(candidate_b.percentage_votes, 0.0);
    }

    #[test]
    fn contest_tally_includes_tally_sheet_results() {
        let mut tally = instant_runoff(vec![vote_with_selected_ids(&["candidate_a"])]);
        tally.tally.auditable_votes = 0;
        let candidate_a = tally
            .tally
            .contest
            .candidates
            .iter()
            .find(|candidate| candidate.id == "candidate_a")
            .unwrap()
            .clone();
        tally.tally.tally_sheet_results = vec![ContestResult {
            contest: tally.tally.contest.clone(),
            total_votes: 2,
            total_valid_votes: 2,
            candidate_result: vec![CandidateResult {
                candidate: candidate_a,
                percentage_votes: 100.0,
                total_count: 2,
            }],
            extended_metrics: Some(ExtendedMetricsContest {
                votes_by_channel: VotesByChannel::from([(VotingChannel::PAPER.into(), 2)]),
                ..Default::default()
            }),
            ..Default::default()
        }];

        let result = tally
            .tally(&mut TestRng(0x5EED))
            .expect("IRV contest tally should include tally sheets");

        assert_eq!(result.total_votes, 3);
        assert_eq!(result.total_valid_votes, 3);
        assert_eq!(
            result
                .candidate_result
                .iter()
                .find(|candidate| candidate.candidate.id == "candidate_a")
                .map(|candidate| candidate.total_count),
            Some(3)
        );
        assert_eq!(
            result
                .extended_metrics
                .as_ref()
                .and_then(|metrics| { metrics.votes_by_channel.get(&VotingChannel::PAPER.into()) }),
            Some(&2)
        );
    }
}
