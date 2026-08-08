// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{
    counting_algorithm::utils::*, tally::Tally, BlankVotes, CandidateResult, ContestResult,
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
        let explicit_blank_candidate_ids = get_explicit_blank_candidate_ids(contest);

        let mut vote_count: HashMap<String, u64> = HashMap::new();
        let mut count_invalid_votes = InvalidVotes::default();
        let mut count_valid: u64 = 0;
        let mut count_invalid: u64 = 0;
        let mut blank_votes = BlankVotes::default();

        let mut extended_metrics = ExtendedMetricsContest::default();
        let mut total_ballots = 0;
        let mut total_weight = 0;

        let mut total_declined_to_vote: u64 = 0;

        for (vote, weight_opt) in votes {
            let weight = weight_opt.clone().unwrap_or_default();
            total_ballots += 1;

            extended_metrics = update_extended_metrics(
                vote,
                &extended_metrics,
                &contest,
                &explicit_blank_candidate_ids,
            );

            match classify_ballot(vote, &explicit_blank_candidate_ids) {
                BallotClass::ExplicitInvalid => {
                    count_invalid_votes.explicit += 1;
                    count_invalid += 1;
                }
                BallotClass::ImplicitInvalid => {
                    count_invalid_votes.implicit += 1;
                    count_invalid += 1;
                }
                BallotClass::Declined => {
                    total_declined_to_vote = total_declined_to_vote.saturating_add(1);
                }
                BallotClass::ExplicitBlank => {
                    blank_votes.explicit += 1;
                    count_valid += 1;
                }
                BallotClass::ImplicitBlank => {
                    blank_votes.implicit += 1;
                    count_valid += 1;
                }
                BallotClass::Valid => {
                    for choice in &vote.choices {
                        if choice.selected >= 0 {
                            *vote_count.entry(choice.id.clone()).or_insert(0) += weight;
                            total_weight += weight;
                        }
                    }

                    count_valid += 1;
                }
            }
        }

        extended_metrics.total_ballots = total_ballots;
        extended_metrics.total_weight = total_weight;
        extended_metrics.total_declined_to_vote = total_declined_to_vote;
        let percentage_votes_denominator = total_weight;

        let candidate_result = match op {
            TallyOperation::SkipCandidateResults => Vec::new(),
            _ => self.tally.create_candidate_results(
                vote_count,
                blank_votes,
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
            blank_votes,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::{Candidate, CandidatePresentation, Contest, Weight};
    use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
    use sequent_core::types::ceremonies::CountingAlgType;

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

    fn mixed_explicit_blank_vote() -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
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

    fn declined_vote() -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: true,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: vec![
                DecodedVoteChoice {
                    id: "normal".to_string(),
                    selected: -1,
                    write_in_text: None,
                },
                DecodedVoteChoice {
                    id: "blank".to_string(),
                    selected: -1,
                    write_in_text: None,
                },
            ],
        }
    }

    fn plurality_at_large(ballots: Vec<DecodedVoteContest>) -> PluralityAtLarge {
        let contest = Contest {
            id: "contest".to_string(),
            max_votes: 1,
            // A declined ballot must count as declined even when the contest
            // requires selections.
            min_votes: 1,
            counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
            candidates: vec![candidate("normal", false), candidate("blank", true)],
            ..Contest::default()
        };
        let ballots = ballots
            .into_iter()
            .map(|ballot| (ballot, Weight::default()))
            .collect();

        PluralityAtLarge {
            tally: Tally {
                id: CountingAlgType::PluralityAtLarge,
                scope_operation: ScopeOperation::Contest(TallyOperation::ProcessBallotsAll),
                contest,
                ballots,
                census: 1,
                auditable_votes: 1,
                tally_sheet_results: vec![],
                tally_results: vec![],
            },
        }
    }

    #[test]
    fn mixed_explicit_blank_vote_is_implicit_invalid() {
        let tally = plurality_at_large(vec![mixed_explicit_blank_vote()]);

        let result = tally
            .process_ballots(TallyOperation::ProcessBallotsAll)
            .expect("mixed explicit blank vote should be processed");

        assert_eq!(result.total_valid_votes, 0);
        assert_eq!(result.total_invalid_votes, 1);
        assert_eq!(result.invalid_votes.explicit, 0);
        assert_eq!(result.invalid_votes.implicit, 1);
        assert_eq!(result.blank_votes.explicit, 0);
        assert_eq!(result.blank_votes.implicit, 0);
        assert!(result
            .candidate_result
            .iter()
            .all(|candidate| candidate.total_count == 0));
    }

    #[test]
    fn declined_ballot_is_counted_as_declined_only() {
        let tally = plurality_at_large(vec![declined_vote()]);

        let result = tally
            .process_ballots(TallyOperation::ProcessBallotsAll)
            .expect("declined ballot should be processed");

        let metrics = result
            .extended_metrics
            .expect("extended metrics should be present");
        assert_eq!(metrics.total_declined_to_vote, 1);
        assert_eq!(result.total_valid_votes, 0);
        assert_eq!(result.total_invalid_votes, 0);
        assert_eq!(result.invalid_votes.explicit, 0);
        assert_eq!(result.invalid_votes.implicit, 0);
        assert_eq!(result.blank_votes.explicit, 0);
        assert_eq!(result.blank_votes.implicit, 0);
        assert!(result
            .candidate_result
            .iter()
            .all(|candidate| candidate.total_count == 0));
    }
}
