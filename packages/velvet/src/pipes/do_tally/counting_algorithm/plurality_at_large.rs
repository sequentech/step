// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::{CountingAlgorithm, Error};
use crate::pipes::do_tally::{
    add_count, counting_algorithm::utils::*, multiply_count, tally::Tally, tally::TallyBallot,
    BlankVotes, CandidateResult, ContestResult, ExtendedMetricsContest, InvalidVotes,
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
    /// Counts the ballots. A ballot with multiplier `m` is counted exactly as
    /// `m` copies of it would be -- every ballot, blank, invalid and declined
    /// count included -- while its area weight scales only the votes it gives
    /// candidates. Every addition is checked: a count that wrapped would be
    /// published as if it were right.
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
        let mut total_ballots: u64 = 0;
        let mut total_weight: u64 = 0;

        let mut total_declined_to_vote: u64 = 0;
        let mut total_blank_ballots: u64 = 0;

        for TallyBallot {
            vote,
            weight,
            multiplier,
        } in votes
        {
            let multiplier = *multiplier;
            let weight = weight.unwrap_or_default();
            total_ballots = add_count(total_ballots, multiplier, "ballots")?;

            extended_metrics = update_extended_metrics(
                vote,
                &extended_metrics,
                &contest,
                &explicit_blank_candidate_ids,
                multiplier,
            )?;

            if vote.is_blank_ballot {
                total_blank_ballots = add_count(total_blank_ballots, multiplier, "blank ballots")?;
            }

            match classify_ballot(vote, &explicit_blank_candidate_ids) {
                BallotClass::ExplicitInvalid => {
                    count_invalid_votes.explicit =
                        add_count(count_invalid_votes.explicit, multiplier, "invalid votes")?;
                    count_invalid = add_count(count_invalid, multiplier, "invalid votes")?;
                }
                BallotClass::ImplicitInvalid => {
                    count_invalid_votes.implicit =
                        add_count(count_invalid_votes.implicit, multiplier, "invalid votes")?;
                    count_invalid = add_count(count_invalid, multiplier, "invalid votes")?;
                }
                BallotClass::Declined => {
                    total_declined_to_vote =
                        add_count(total_declined_to_vote, multiplier, "declined ballots")?;
                }
                BallotClass::ExplicitBlank => {
                    blank_votes.explicit =
                        add_count(blank_votes.explicit, multiplier, "blank votes")?;
                    count_valid = add_count(count_valid, multiplier, "valid votes")?;
                }
                BallotClass::ImplicitBlank => {
                    blank_votes.implicit =
                        add_count(blank_votes.implicit, multiplier, "blank votes")?;
                    count_valid = add_count(count_valid, multiplier, "valid votes")?;
                }
                BallotClass::Valid => {
                    let candidate_votes = multiply_count(weight, multiplier, "candidate votes")?;
                    for choice in &vote.choices {
                        if choice.selected >= 0 {
                            let count = vote_count.entry(choice.id.clone()).or_insert(0);
                            *count = add_count(*count, candidate_votes, "candidate votes")?;
                            total_weight =
                                add_count(total_weight, candidate_votes, "total weight")?;
                        }
                    }

                    count_valid = add_count(count_valid, multiplier, "valid votes")?;
                }
            }
        }

        extended_metrics.total_ballots = total_ballots;
        extended_metrics.total_weight = total_weight;
        extended_metrics.total_declined_to_vote = total_declined_to_vote;
        extended_metrics.total_blank_ballots = total_blank_ballots;
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
            .try_fold(contest_result, |acc, x| {
                acc.aggregate_checked_channels(x, false)
            })?;

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

    fn blank_ballot_vote() -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: true,
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
        plurality_at_large_counting(
            ballots
                .into_iter()
                .map(|ballot| TallyBallot::new(ballot, Weight::default()))
                .collect(),
        )
    }

    fn plurality_at_large_counting(ballots: Vec<TallyBallot>) -> PluralityAtLarge {
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

    #[test]
    fn blank_ballot_is_counted_without_changing_existing_blank_vote_figures() {
        let tally = plurality_at_large(vec![blank_ballot_vote()]);

        let result = tally
            .process_ballots(TallyOperation::ProcessBallotsAll)
            .expect("blank ballot should be processed");

        let metrics = result
            .extended_metrics
            .expect("extended metrics should be present");
        assert_eq!(metrics.total_blank_ballots, 1);
        assert_eq!(metrics.total_declined_to_vote, 0);
        // A blank ballot counts as a valid, implicitly blank ballot in this
        // contest, exactly like a regular blank vote would — the new
        // ballot-level counter is additive, not a replacement.
        assert_eq!(result.total_valid_votes, 1);
        assert_eq!(result.total_invalid_votes, 0);
        assert_eq!(result.blank_votes.explicit, 0);
        assert_eq!(result.blank_votes.implicit, 1);
    }

    #[test]
    fn declined_ballot_is_not_counted_as_blank_ballot() {
        let tally = plurality_at_large(vec![declined_vote()]);

        let result = tally
            .process_ballots(TallyOperation::ProcessBallotsAll)
            .expect("declined ballot should be processed");

        let metrics = result
            .extended_metrics
            .expect("extended metrics should be present");
        assert_eq!(metrics.total_declined_to_vote, 1);
        assert_eq!(metrics.total_blank_ballots, 0);
    }

    fn vote_selecting(selected: &[&str]) -> DecodedVoteContest {
        DecodedVoteContest {
            contest_id: "contest".to_string(),
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
            choices: ["normal", "blank"]
                .into_iter()
                .map(|id| DecodedVoteChoice {
                    id: id.to_string(),
                    selected: if selected.contains(&id) { 0 } else { -1 },
                    write_in_text: None,
                })
                .collect(),
        }
    }

    fn explicit_invalid_vote() -> DecodedVoteContest {
        DecodedVoteContest {
            is_explicit_invalid: true,
            ..vote_selecting(&[])
        }
    }

    /// One ballot of every kind the count distinguishes.
    fn ballots_of_every_kind() -> Vec<DecodedVoteContest> {
        vec![
            vote_selecting(&["normal"]),
            vote_selecting(&["blank"]),
            vote_selecting(&[]),
            mixed_explicit_blank_vote(),
            explicit_invalid_vote(),
            declined_vote(),
            blank_ballot_vote(),
        ]
    }

    fn result_json(tally: &PluralityAtLarge) -> serde_json::Value {
        serde_json::to_value(
            tally
                .process_ballots(TallyOperation::ProcessBallotsAll)
                .expect("ballots should be counted"),
        )
        .expect("result should serialize")
    }

    /// The equivalence the batch layout rests on: counting a ballot once with
    /// multiplier `m` publishes exactly what counting `m` copies of it did,
    /// in every figure, not just the candidate totals.
    #[test]
    fn a_multiplied_ballot_counts_exactly_like_its_copies() {
        for (offset, multiplier) in [1u64, 2, 4, 8, 64].into_iter().enumerate() {
            let mut multiplied = vec![];
            let mut copies = vec![];
            for (index, ballot) in ballots_of_every_kind().into_iter().enumerate() {
                // A different multiplier per kind, so a figure that ignores it
                // cannot match by accident.
                let multiplier = multiplier << ((index + offset) % 3);
                copies.extend(
                    std::iter::repeat_n(ballot.clone(), multiplier as usize)
                        .map(|copy| TallyBallot::new(copy, Weight::default())),
                );
                multiplied.push(TallyBallot {
                    vote: ballot,
                    weight: Weight::default(),
                    multiplier,
                });
            }
            assert_eq!(
                result_json(&plurality_at_large_counting(multiplied)),
                result_json(&plurality_at_large_counting(copies)),
                "multiplier {multiplier}"
            );
        }
    }

    #[test]
    fn a_large_multiplier_is_counted_without_copies() {
        let multiplier = 1u64 << 31;
        let result = plurality_at_large_counting(vec![
            TallyBallot {
                vote: vote_selecting(&["normal"]),
                weight: Weight::default(),
                multiplier,
            },
            TallyBallot::new(vote_selecting(&["normal"]), Weight::default()),
        ])
        .process_ballots(TallyOperation::ProcessBallotsAll)
        .expect("ballots should be counted");

        let normal = result
            .candidate_result
            .iter()
            .find(|candidate| candidate.candidate.id == "normal")
            .expect("candidate result");
        assert_eq!(normal.total_count, multiplier + 1);
        assert_eq!(result.total_valid_votes, multiplier + 1);
        let metrics = result.extended_metrics.expect("extended metrics");
        assert_eq!(metrics.total_weight, multiplier + 1);
        assert_eq!(metrics.total_ballots, multiplier + 1);
    }

    /// An area weight is not a multiplier: it scales the votes a ballot gives
    /// candidates and nothing else, as it always has.
    #[test]
    fn an_area_weight_scales_only_candidate_votes() {
        let weight: Weight = serde_json::from_value(serde_json::json!(5)).unwrap();
        let result = plurality_at_large_counting(vec![
            TallyBallot::new(vote_selecting(&["normal"]), weight),
            TallyBallot::new(vote_selecting(&["normal"]), weight),
            TallyBallot::new(vote_selecting(&["blank"]), weight),
        ])
        .process_ballots(TallyOperation::ProcessBallotsAll)
        .expect("ballots should be counted");

        let normal = result
            .candidate_result
            .iter()
            .find(|candidate| candidate.candidate.id == "normal")
            .expect("candidate result");
        assert_eq!(normal.total_count, 10);
        assert_eq!(result.total_valid_votes, 3);
        assert_eq!(result.blank_votes.explicit, 1);
        let metrics = result.extended_metrics.expect("extended metrics");
        assert_eq!(metrics.total_weight, 10);
        assert_eq!(metrics.total_ballots, 3);
    }

    #[test]
    fn a_candidate_total_that_would_wrap_is_an_error() {
        let weight: Weight = serde_json::from_value(serde_json::json!(u64::MAX / 2 + 1)).unwrap();
        let tally = plurality_at_large_counting(vec![
            TallyBallot::new(vote_selecting(&["normal"]), weight),
            TallyBallot::new(vote_selecting(&["normal"]), weight),
        ]);

        assert!(matches!(
            tally.process_ballots(TallyOperation::ProcessBallotsAll),
            Err(Error::CountOverflow(_))
        ));
    }

    #[test]
    fn a_weight_times_multiplier_that_would_wrap_is_an_error() {
        let weight: Weight = serde_json::from_value(serde_json::json!(2)).unwrap();
        let tally = plurality_at_large_counting(vec![TallyBallot {
            vote: vote_selecting(&["normal"]),
            weight,
            multiplier: u64::MAX,
        }]);

        assert!(matches!(
            tally.process_ballots(TallyOperation::ProcessBallotsAll),
            Err(Error::CountOverflow(_))
        ));
    }
}
