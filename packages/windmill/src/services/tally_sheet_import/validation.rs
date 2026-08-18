// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

use sequent_core::services::tally_sheet_validation::{
    effective_max_marks_per_ballot_typed, validate_area_contest_results,
    validate_ballot_box_blank_ballots,
};
use sequent_core::types::ceremonies::CountingAlgType;
use sequent_core::types::hasura::core::Contest;
use sequent_core::types::tally_sheet_import::TallySheetImportValidationError;
use sequent_core::types::tally_sheets::{AreaContestResults, VotingChannel};
use tracing::instrument;

/// Computes the maximum number of candidate marks a single non-blank
/// ballot can legitimately contribute for the given contest, from its
/// `max_votes`, `counting_algorithm`, and (for cumulative contests) the
/// per-candidate checkbox budget stored in `presentation`.
#[instrument(skip_all)]
pub fn contest_max_marks_per_ballot(contest: &Contest) -> Option<u64> {
    let cumulative_number_of_checkboxes = contest
        .presentation
        .as_ref()
        .and_then(|value| value.get("cumulative_number_of_checkboxes"))
        .and_then(|value| value.as_u64());
    let counting_algorithm = contest
        .counting_algorithm
        .as_deref()
        .and_then(|value| value.parse::<CountingAlgType>().ok())
        .unwrap_or_default();
    Some(effective_max_marks_per_ballot_typed(
        contest.max_votes,
        counting_algorithm,
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
                Some(contest_external_id),
                &shared_error.field,
                shared_error.params,
            )
        })
        .collect()
}

pub struct BallotBoxContentCheck {
    pub errors: Vec<TallySheetImportValidationError>,
    /// The value implied when a box's per-contest blank vote counts pinch
    /// the possible range of `blank_ballots` to exactly one integer. `None`
    /// when the box has no such implied value.
    pub pre_filled_blank_ballots: Option<u64>,
}

/// Validates a ballot box's `blank_ballots` value against every contest
/// sheet of the box (not just the sheets in the current import batch --
/// see `preview_tally_sheet_import`, which fills in sheets for contests
/// missing from the batch before calling this). Box-scoped errors have no
/// single `contest_external_id`, unlike `validate_import_content`'s
/// per-sheet errors.
#[instrument(skip_all)]
pub fn validate_ballot_box_content(
    channel: &VotingChannel,
    area_name: &str,
    contest_sheets: &[&AreaContestResults],
) -> BallotBoxContentCheck {
    let check = validate_ballot_box_blank_ballots(contest_sheets);
    let errors = check
        .errors
        .into_iter()
        .map(|shared_error| {
            error(
                &shared_error.code,
                shared_error.message,
                channel,
                area_name,
                None,
                &shared_error.field,
                shared_error.params,
            )
        })
        .collect();

    BallotBoxContentCheck {
        errors,
        pre_filled_blank_ballots: check.pre_filled_value,
    }
}

fn error(
    code: &str,
    message: String,
    channel: &VotingChannel,
    area_name: &str,
    contest_external_id: Option<&str>,
    field: &str,
    params: HashMap<String, String>,
) -> TallySheetImportValidationError {
    TallySheetImportValidationError {
        code: code.to_string(),
        message,
        channel: Some(channel.clone()),
        area_name: Some(area_name.to_string()),
        contest_external_id: contest_external_id.map(str::to_string),
        candidate_external_id: None,
        field: Some(field.to_string()),
        params,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use sequent_core::types::tally_sheets::{CandidateResults, InvalidVotes};
    use serde_json::json;

    use super::*;

    fn contest(
        max_votes: Option<i64>,
        counting_algorithm: Option<&str>,
        presentation: Option<serde_json::Value>,
    ) -> Contest {
        Contest {
            id: "contest-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            election_event_id: "event-1".to_string(),
            election_id: "election-1".to_string(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            is_acclaimed: None,
            is_active: None,
            description: None,
            presentation,
            min_votes: None,
            max_votes,
            winning_candidates_num: None,
            voting_type: None,
            counting_algorithm: counting_algorithm.map(str::to_string),
            is_encrypted: None,
            tally_configuration: None,
            image_document_id: None,
            conditions: None,
            external_id: None,
        }
    }

    #[test]
    fn max_marks_is_one_for_single_choice_contest() {
        let contest = contest(Some(1), Some("plurality-at-large"), None);
        assert_eq!(contest_max_marks_per_ballot(&contest), Some(1));
    }

    #[test]
    fn max_marks_is_max_votes_for_vote_for_n_contest() {
        let contest = contest(Some(4), Some("plurality-at-large"), None);
        assert_eq!(contest_max_marks_per_ballot(&contest), Some(4));
    }

    #[test]
    fn max_marks_multiplies_checkboxes_for_cumulative_contest() {
        let contest = contest(
            Some(3),
            Some("cumulative"),
            Some(json!({"cumulative_number_of_checkboxes": 5})),
        );
        assert_eq!(contest_max_marks_per_ballot(&contest), Some(15));
    }

    #[test]
    fn max_marks_ignores_checkboxes_for_non_cumulative_contest() {
        let contest = contest(
            Some(3),
            Some("plurality-at-large"),
            Some(json!({"cumulative_number_of_checkboxes": 5})),
        );
        assert_eq!(contest_max_marks_per_ballot(&contest), Some(3));
    }

    #[test]
    fn max_marks_falls_back_to_max_votes_when_cumulative_checkboxes_unusable() {
        // Absent, wrong-typed, and non-positive checkbox counts all collapse
        // to a multiplier of 1 rather than zeroing the bound out.
        for presentation in [
            None,
            Some(json!({})),
            Some(json!({"cumulative_number_of_checkboxes": "5"})),
            Some(json!({"cumulative_number_of_checkboxes": 0})),
        ] {
            let contest = contest(Some(3), Some("cumulative"), presentation);
            assert_eq!(contest_max_marks_per_ballot(&contest), Some(3));
        }
    }

    #[test]
    fn max_marks_defaults_to_one_when_max_votes_absent_or_non_positive() {
        for max_votes in [None, Some(0), Some(-2)] {
            let contest = contest(max_votes, Some("plurality-at-large"), None);
            assert_eq!(contest_max_marks_per_ballot(&contest), Some(1));
        }
    }

    #[test]
    fn max_marks_defaults_to_one_when_counting_algorithm_absent() {
        let contest = contest(None, None, None);
        assert_eq!(contest_max_marks_per_ballot(&contest), Some(1));
    }

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
            blank_ballots: None,
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
            blank_ballots: None,
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
            blank_ballots: None,
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
            blank_ballots: None,
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

    fn contest_sheet(total_votes: u64, total_blank_votes: u64) -> AreaContestResults {
        AreaContestResults {
            area_id: "area-1".to_string(),
            contest_id: "contest-1".to_string(),
            total_votes: Some(total_votes),
            total_valid_votes: Some(total_votes),
            invalid_votes: None,
            total_blank_votes: Some(total_blank_votes),
            blank_ballots: None,
            census: None,
            candidate_results: HashMap::new(),
            annotations: None,
        }
    }

    #[test]
    fn attaches_channel_and_area_but_no_contest_to_box_level_errors() {
        let mut sheet_a = contest_sheet(10, 3);
        sheet_a.blank_ballots = Some(2);
        let mut sheet_b = contest_sheet(10, 5);
        sheet_b.blank_ballots = Some(3);
        let sheets = [&sheet_a, &sheet_b];

        let check = validate_ballot_box_content(&VotingChannel::PAPER, "Precinct 1", &sheets);

        assert_eq!(check.errors.len(), 1);
        let error = &check.errors[0];
        assert_eq!(error.code, "inconsistent_blank_ballots");
        assert_eq!(error.channel, Some(VotingChannel::PAPER));
        assert_eq!(error.area_name, Some("Precinct 1".to_string()));
        assert_eq!(error.contest_external_id, None);
    }

    #[test]
    fn surfaces_the_pre_filled_value_when_bounds_pinch() {
        let sheet_a = contest_sheet(10, 0);
        let sheet_b = contest_sheet(10, 5);
        let sheets = [&sheet_a, &sheet_b];

        let check = validate_ballot_box_content(&VotingChannel::PAPER, "Precinct 1", &sheets);

        assert!(check.errors.is_empty());
        assert_eq!(check.pre_filled_blank_ballots, Some(0));
    }
}
