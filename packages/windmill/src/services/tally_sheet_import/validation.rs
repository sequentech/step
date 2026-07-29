// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use sequent_core::services::tally_sheet_validation::{
    effective_max_marks_per_ballot, validate_area_contest_results,
};
use sequent_core::types::hasura::core::Contest;
use sequent_core::types::tally_sheet_import::TallySheetImportValidationError;
use sequent_core::types::tally_sheets::{AreaContestResults, VotingChannel};
use tracing::instrument;

/// Computes the maximum number of candidate marks a single non-blank
/// ballot can legitimately contribute for the given contest, from its
/// `max_votes`, `counting_algorithm`, and (for cumulative contests) the
/// per-candidate checkbox budget stored in `presentation`.
pub fn contest_max_marks_per_ballot(contest: &Contest) -> Option<u64> {
    let cumulative_number_of_checkboxes = contest
        .presentation
        .as_ref()
        .and_then(|value| value.get("cumulative_number_of_checkboxes"))
        .and_then(|value| value.as_u64());
    Some(effective_max_marks_per_ballot(
        contest.max_votes,
        contest.counting_algorithm.as_deref(),
        cumulative_number_of_checkboxes,
    ))
}

#[instrument(skip_all)]
pub fn validate_import_content(
    channel: &VotingChannel,
    area_name: &str,
    contest_external_id: &str,
    content: &AreaContestResults,
    max_marks_per_ballot: Option<u64>,
) -> Vec<TallySheetImportValidationError> {
    validate_area_contest_results(content, max_marks_per_ballot)
        .into_iter()
        .map(|shared_error| {
            error(
                &shared_error.code,
                shared_error.message,
                channel,
                area_name,
                contest_external_id,
                &shared_error.field,
                shared_error.params,
            )
        })
        .collect()
}

fn error(
    code: &str,
    message: String,
    channel: &VotingChannel,
    area_name: &str,
    contest_external_id: &str,
    field: &str,
    params: HashMap<String, String>,
) -> TallySheetImportValidationError {
    TallySheetImportValidationError {
        code: code.to_string(),
        message,
        channel: Some(channel.clone()),
        area_name: Some(area_name.to_string()),
        contest_external_id: Some(contest_external_id.to_string()),
        candidate_external_id: None,
        field: Some(field.to_string()),
        params,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use sequent_core::types::tally_sheets::{CandidateResults, InvalidVotes};

    use super::*;

    #[test]
    fn accepts_consistent_vote_bucket_arithmetic() {
        let content = AreaContestResults {
            area_id: "area-1".to_string(),
            contest_id: "contest-1".to_string(),
            total_votes: Some(15),
            total_valid_votes: Some(12),
            invalid_votes: Some(InvalidVotes {
                total_invalid: Some(3),
                implicit_invalid: Some(1),
                explicit_invalid: Some(2),
            }),
            total_blank_votes: Some(2),
            census: Some(20),
            candidate_results: HashMap::from([(
                "candidate-1".to_string(),
                CandidateResults {
                    candidate_id: "candidate-1".to_string(),
                    total_votes: Some(10),
                },
            )]),
            annotations: None,
        };

        let errors = validate_import_content(
            &VotingChannel::PAPER,
            "Precinct 1",
            "contest-1",
            &content,
            None,
        );

        assert!(errors.is_empty());
    }

    #[test]
    fn reports_all_inconsistent_vote_bucket_totals() {
        let content = AreaContestResults {
            area_id: "area-1".to_string(),
            contest_id: "contest-1".to_string(),
            total_votes: Some(30),
            total_valid_votes: Some(11),
            invalid_votes: Some(InvalidVotes {
                total_invalid: Some(8),
                implicit_invalid: Some(1),
                explicit_invalid: Some(2),
            }),
            total_blank_votes: Some(2),
            census: Some(20),
            candidate_results: HashMap::from([(
                "candidate-1".to_string(),
                CandidateResults {
                    candidate_id: "candidate-1".to_string(),
                    total_votes: Some(10),
                },
            )]),
            annotations: None,
        };

        let errors = validate_import_content(
            &VotingChannel::PAPER,
            "Precinct 1",
            "contest-1",
            &content,
            None,
        );
        let codes = errors
            .into_iter()
            .map(|error| error.code)
            .collect::<Vec<_>>();

        assert_eq!(
            codes,
            vec![
                "invalid_total_invalid",
                "invalid_total_valid_votes",
                "invalid_total_votes",
                "total_votes_exceeds_census"
            ]
        );
    }

    #[test]
    fn accepts_vote_for_n_contest_within_bound() {
        let content = AreaContestResults {
            area_id: "area-1".to_string(),
            contest_id: "contest-1".to_string(),
            total_votes: Some(10),
            total_valid_votes: Some(10),
            invalid_votes: Some(InvalidVotes {
                total_invalid: Some(0),
                implicit_invalid: Some(0),
                explicit_invalid: Some(0),
            }),
            total_blank_votes: Some(0),
            census: Some(20),
            candidate_results: HashMap::from([
                (
                    "candidate-1".to_string(),
                    CandidateResults {
                        candidate_id: "candidate-1".to_string(),
                        total_votes: Some(8),
                    },
                ),
                (
                    "candidate-2".to_string(),
                    CandidateResults {
                        candidate_id: "candidate-2".to_string(),
                        total_votes: Some(7),
                    },
                ),
            ]),
            annotations: None,
        };

        let errors = validate_import_content(
            &VotingChannel::PAPER,
            "Precinct 1",
            "contest-1",
            &content,
            Some(2),
        );

        assert!(errors.is_empty());
    }

    #[test]
    fn rejects_vote_for_n_contest_exceeding_bound() {
        let content = AreaContestResults {
            area_id: "area-1".to_string(),
            contest_id: "contest-1".to_string(),
            total_votes: Some(10),
            total_valid_votes: Some(10),
            invalid_votes: Some(InvalidVotes {
                total_invalid: Some(0),
                implicit_invalid: Some(0),
                explicit_invalid: Some(0),
            }),
            total_blank_votes: Some(0),
            census: Some(30),
            candidate_results: HashMap::from([
                (
                    "candidate-1".to_string(),
                    CandidateResults {
                        candidate_id: "candidate-1".to_string(),
                        total_votes: Some(12),
                    },
                ),
                (
                    "candidate-2".to_string(),
                    CandidateResults {
                        candidate_id: "candidate-2".to_string(),
                        total_votes: Some(9),
                    },
                ),
            ]),
            annotations: None,
        };

        let errors = validate_import_content(
            &VotingChannel::PAPER,
            "Precinct 1",
            "contest-1",
            &content,
            Some(2),
        );
        let codes = errors
            .into_iter()
            .map(|error| error.code)
            .collect::<Vec<_>>();

        assert_eq!(codes, vec!["invalid_total_valid_votes"]);
    }
}
