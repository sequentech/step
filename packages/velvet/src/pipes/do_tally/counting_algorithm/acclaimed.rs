// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use rand_core::RngCore;
use sequent_core::ballot::{Candidate, Contest};

use super::{CountingAlgorithm, Result};
use crate::pipes::do_tally::{CandidateResult, ContestResult, ExtendedMetricsContest};

/// Produces the canonical result for a contest decided without a vote.
///
/// This deliberately does not contain a [`Tally`](super::super::tally::Tally):
/// constructing one reads ballots and makes the result depend on a counting
/// algorithm, scope operation and aggregate inputs. None of those concepts
/// apply to an acclaimed contest.
pub(crate) struct Acclaimed {
    contest: Contest,
}

impl Acclaimed {
    pub(crate) fn new(contest: &Contest) -> Self {
        Self {
            contest: contest.clone(),
        }
    }
}

impl CountingAlgorithm for Acclaimed {
    // The synthetic result involves no counting, so the tie-breaking rng is
    // unused.
    fn tally(&self, _rng: &mut dyn RngCore) -> Result<ContestResult> {
        let candidate_result = self
            .contest
            .candidates
            .iter()
            .filter(|candidate| candidate.is_acclamation_eligible())
            .cloned()
            .map(|candidate| CandidateResult {
                candidate,
                percentage_votes: 0.0,
                total_count: 0,
            })
            .collect();

        Ok(ContestResult {
            contest: self.contest.clone(),
            candidate_result,
            extended_metrics: Some(ExtendedMetricsContest::default()),
            ..ContestResult::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use sequent_core::ballot::CandidatePresentation;
    use sequent_core::types::ceremonies::CountingAlgType;

    use super::*;

    fn candidate(id: &str, configure: impl FnOnce(&mut CandidatePresentation)) -> Candidate {
        let mut presentation = CandidatePresentation::new();
        configure(&mut presentation);
        Candidate {
            id: id.to_string(),
            presentation: Some(presentation),
            ..Candidate::default()
        }
    }

    #[test]
    fn synthesizes_a_canonical_zero_result_for_a_preferential_contest() {
        let contest = Contest {
            id: "acclaimed".to_string(),
            is_acclaimed: Some(true),
            counting_algorithm: Some(CountingAlgType::InstantRunoff),
            candidates: vec![
                candidate("first", |_| {}),
                candidate("explicit-blank", |p| p.is_explicit_blank = Some(true)),
                candidate("explicit-invalid", |p| p.is_explicit_invalid = Some(true)),
                candidate("disabled", |p| p.is_disabled = Some(true)),
                candidate("write-in", |p| p.is_write_in = Some(true)),
                candidate("second", |_| {}),
            ],
            ..Contest::default()
        };

        let result = Acclaimed::new(&contest)
            .tally(&mut rand::rng())
            .expect("result");

        assert_eq!(result.contest.id, contest.id);
        assert_eq!(result.census, 0);
        assert_eq!(result.auditable_votes, 0);
        assert_eq!(result.total_votes, 0);
        assert_eq!(result.total_valid_votes, 0);
        assert_eq!(result.total_invalid_votes, 0);
        assert_eq!(result.total_blank_votes, 0);
        assert_eq!(result.blank_votes.total(), 0);
        assert_eq!(result.invalid_votes.total(), 0);
        assert_eq!(result.percentage_census, 0.0);
        assert_eq!(result.percentage_total_votes, 0.0);
        assert_eq!(result.percentage_total_valid_votes, 0.0);
        assert_eq!(result.percentage_total_invalid_votes, 0.0);
        assert_eq!(result.percentage_total_blank_votes, 0.0);
        assert_eq!(result.process_results, None);
        assert_eq!(
            result
                .candidate_result
                .iter()
                .map(|candidate| candidate.candidate.id.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert!(result
            .candidate_result
            .iter()
            .all(|candidate| candidate.total_count == 0 && candidate.percentage_votes == 0.0));

        let metrics = result.extended_metrics.expect("extended metrics");
        assert_eq!(metrics.over_votes, 0);
        assert_eq!(metrics.under_votes, 0);
        assert_eq!(metrics.votes_actually, 0);
        assert_eq!(metrics.expected_votes, 0);
        assert_eq!(metrics.total_ballots, 0);
        assert_eq!(metrics.total_weight, 0);
        assert_eq!(metrics.total_declined_to_vote, 0);
        assert_eq!(metrics.total_blank_ballots, 0);
        assert!(metrics.votes_by_channel.is_empty());
    }
}
