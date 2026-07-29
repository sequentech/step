// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::tally_sheets::AreaContestResults;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// `message` is a pre-formatted English sentence kept for logs, CLI output,
/// and any other non-translated context. `code` + `params` are what a UI
/// should use to render a translated message (`t(code, params)`); `params`
/// holds every number referenced by `message`, stringified.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TallySheetValidationError {
    pub code: String,
    pub message: String,
    pub field: String,
    pub params: HashMap<String, String>,
}

/// Computes the maximum number of candidate marks a single non-blank
/// ballot can legitimately contribute, given a contest's `max_votes` and
/// counting algorithm. Absent or non-positive `max_votes` defaults to `1`
/// (single-choice), which reproduces the strict one-mark-per-ballot
/// behavior for ordinary contests. For `cumulative` contests, a voter may
/// give multiple points to the same candidate, so the bound is further
/// multiplied by the number of point checkboxes offered per candidate.
pub fn effective_max_marks_per_ballot(
    max_votes: Option<i64>,
    counting_algorithm: Option<&str>,
    cumulative_number_of_checkboxes: Option<u64>,
) -> u64 {
    let base = max_votes
        .filter(|value| *value > 0)
        .map(|value| value as u64)
        .unwrap_or(1);
    let is_cumulative = counting_algorithm
        .map(|value| value.eq_ignore_ascii_case("cumulative"))
        .unwrap_or(false);
    if is_cumulative {
        let checkboxes = cumulative_number_of_checkboxes
            .filter(|value| *value > 0)
            .unwrap_or(1);
        base.saturating_mul(checkboxes)
    } else {
        base
    }
}

pub fn validate_area_contest_results(
    content: &AreaContestResults,
    max_marks_per_ballot: Option<u64>,
) -> Vec<TallySheetValidationError> {
    let mut errors = Vec::new();
    let invalid_votes = content.invalid_votes.clone().unwrap_or_default();
    let implicit_invalid = invalid_votes.implicit_invalid.unwrap_or(0);
    let explicit_invalid = invalid_votes.explicit_invalid.unwrap_or(0);
    let total_invalid = invalid_votes.total_invalid.unwrap_or(0);
    let total_valid_votes = content.total_valid_votes.unwrap_or(0);
    let total_blank_votes = content.total_blank_votes.unwrap_or(0);
    let total_votes = content.total_votes.unwrap_or(0);
    let candidate_votes_sum: u64 = content
        .candidate_results
        .values()
        .map(|candidate_result| candidate_result.total_votes.unwrap_or(0))
        .sum();

    if total_invalid != implicit_invalid + explicit_invalid {
        errors.push(error(
            "invalid_total_invalid",
            format!(
                "total_invalid ({total_invalid}) must equal implicit_invalid ({implicit_invalid}) + explicit_invalid ({explicit_invalid})"
            ),
            "total_invalid",
            HashMap::from([
                ("totalInvalid".to_string(), total_invalid.to_string()),
                ("implicitInvalid".to_string(), implicit_invalid.to_string()),
                ("explicitInvalid".to_string(), explicit_invalid.to_string()),
            ]),
        ));
    }

    // A voter may be allowed to mark more than one candidate per ballot
    // (e.g. plurality-at-large "vote for N", or cumulative voting), so
    // candidate_votes_sum isn't required to equal the ballot count — it
    // must fall between one mark per non-blank ballot and
    // max_marks_per_ballot marks per non-blank ballot.
    let non_blank_valid_votes =
        total_valid_votes.saturating_sub(total_blank_votes);
    let max_marks = max_marks_per_ballot.unwrap_or(1).max(1);
    let lower_bound = non_blank_valid_votes;
    let upper_bound = non_blank_valid_votes.saturating_mul(max_marks);

    if candidate_votes_sum < lower_bound || candidate_votes_sum > upper_bound {
        errors.push(error(
            "invalid_total_valid_votes",
            format!(
                "candidate votes ({candidate_votes_sum}) must be between {lower_bound} and {upper_bound} (non-blank valid votes {non_blank_valid_votes} \u{d7} up to {max_marks} marks per ballot)"
            ),
            "total_valid_votes",
            HashMap::from([
                ("candidateVotesSum".to_string(), candidate_votes_sum.to_string()),
                ("lowerBound".to_string(), lower_bound.to_string()),
                ("upperBound".to_string(), upper_bound.to_string()),
                ("nonBlankValidVotes".to_string(), non_blank_valid_votes.to_string()),
                ("maxMarks".to_string(), max_marks.to_string()),
            ]),
        ));
    }

    if total_votes != total_valid_votes + total_invalid {
        errors.push(error(
            "invalid_total_votes",
            format!(
                "total_votes ({total_votes}) must equal total_valid_votes ({total_valid_votes}) + total_invalid ({total_invalid})"
            ),
            "total_votes",
            HashMap::from([
                ("totalVotes".to_string(), total_votes.to_string()),
                ("totalValidVotes".to_string(), total_valid_votes.to_string()),
                ("totalInvalid".to_string(), total_invalid.to_string()),
            ]),
        ));
    }

    // Census is validated as a required field by the CSV import pipeline and the
    // manual entry form; skip this check rather than treating an absent census as 0.
    if let Some(census) = content.census {
        if total_votes > census {
            errors.push(error(
                "total_votes_exceeds_census",
                format!(
                    "total_votes ({total_votes}) must not be greater than census ({census})"
                ),
                "census",
                HashMap::from([
                    ("totalVotes".to_string(), total_votes.to_string()),
                    ("census".to_string(), census.to_string()),
                ]),
            ));
        }
    }

    errors
}

fn error(
    code: &str,
    message: String,
    field: &str,
    params: HashMap<String, String>,
) -> TallySheetValidationError {
    TallySheetValidationError {
        code: code.to_string(),
        message,
        field: field.to_string(),
        params,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::types::tally_sheets::{
        AreaContestResults, CandidateResults, InvalidVotes,
    };

    use super::{
        effective_max_marks_per_ballot, validate_area_contest_results,
    };

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

        let errors = validate_area_contest_results(&content, None);

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

        let errors = validate_area_contest_results(&content, None);
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
    fn skips_census_check_when_census_is_absent() {
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
            census: None,
            candidate_results: HashMap::from([(
                "candidate-1".to_string(),
                CandidateResults {
                    candidate_id: "candidate-1".to_string(),
                    total_votes: Some(10),
                },
            )]),
            annotations: None,
        };

        let errors = validate_area_contest_results(&content, None);

        assert!(errors.is_empty());
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

        let errors = validate_area_contest_results(&content, Some(2));

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

        let errors = validate_area_contest_results(&content, Some(2));
        let codes = errors
            .into_iter()
            .map(|error| error.code)
            .collect::<Vec<_>>();

        assert_eq!(codes, vec!["invalid_total_valid_votes"]);
    }

    #[test]
    fn rejects_vote_for_n_contest_below_lower_bound() {
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
            candidate_results: HashMap::from([(
                "candidate-1".to_string(),
                CandidateResults {
                    candidate_id: "candidate-1".to_string(),
                    total_votes: Some(5),
                },
            )]),
            annotations: None,
        };

        let errors = validate_area_contest_results(&content, Some(2));
        let codes = errors
            .into_iter()
            .map(|error| error.code)
            .collect::<Vec<_>>();

        assert_eq!(codes, vec!["invalid_total_valid_votes"]);
    }

    #[test]
    fn accepts_cumulative_contest_using_full_checkbox_budget() {
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
            census: Some(60),
            candidate_results: HashMap::from([(
                "candidate-1".to_string(),
                CandidateResults {
                    candidate_id: "candidate-1".to_string(),
                    total_votes: Some(40),
                },
            )]),
            annotations: None,
        };

        // max_votes=2, cumulative_number_of_checkboxes=3 -> 6 marks/ballot
        let errors = validate_area_contest_results(&content, Some(6));

        assert!(errors.is_empty());
    }

    #[test]
    fn effective_max_marks_defaults_to_one_when_max_votes_absent() {
        assert_eq!(effective_max_marks_per_ballot(None, None, None), 1);
    }

    #[test]
    fn effective_max_marks_floors_non_positive_max_votes_to_one() {
        assert_eq!(effective_max_marks_per_ballot(Some(0), None, None), 1);
        assert_eq!(effective_max_marks_per_ballot(Some(-1), None, None), 1);
    }

    #[test]
    fn effective_max_marks_uses_max_votes_for_non_cumulative_contests() {
        assert_eq!(
            effective_max_marks_per_ballot(
                Some(3),
                Some("plurality-at-large"),
                None
            ),
            3
        );
    }

    #[test]
    fn effective_max_marks_multiplies_by_checkboxes_for_cumulative_contests() {
        assert_eq!(
            effective_max_marks_per_ballot(
                Some(2),
                Some("cumulative"),
                Some(3)
            ),
            6
        );
        assert_eq!(
            effective_max_marks_per_ballot(
                Some(2),
                Some("Cumulative"),
                Some(3)
            ),
            6
        );
    }

    #[test]
    fn effective_max_marks_defaults_checkboxes_to_one_when_absent() {
        assert_eq!(
            effective_max_marks_per_ballot(Some(2), Some("cumulative"), None),
            2
        );
    }
}
