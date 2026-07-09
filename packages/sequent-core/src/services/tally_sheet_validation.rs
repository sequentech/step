// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::tally_sheets::AreaContestResults;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TallySheetValidationError {
    pub code: String,
    pub message: String,
    pub field: String,
}

pub fn validate_area_contest_results(
    content: &AreaContestResults,
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
        ));
    }

    if total_valid_votes != candidate_votes_sum + total_blank_votes {
        errors.push(error(
            "invalid_total_valid_votes",
            format!(
                "total_valid_votes ({total_valid_votes}) must equal candidate votes ({candidate_votes_sum}) + blank votes ({total_blank_votes})"
            ),
            "total_valid_votes",
        ));
    }

    if total_votes != total_valid_votes + total_invalid {
        errors.push(error(
            "invalid_total_votes",
            format!(
                "total_votes ({total_votes}) must equal total_valid_votes ({total_valid_votes}) + total_invalid ({total_invalid})"
            ),
            "total_votes",
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
            ));
        }
    }

    errors
}

fn error(
    code: &str,
    message: String,
    field: &str,
) -> TallySheetValidationError {
    TallySheetValidationError {
        code: code.to_string(),
        message,
        field: field.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::types::tally_sheets::{
        AreaContestResults, CandidateResults, InvalidVotes,
    };

    use super::validate_area_contest_results;

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
        };

        let errors = validate_area_contest_results(&content);

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
        };

        let errors = validate_area_contest_results(&content);
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
        };

        let errors = validate_area_contest_results(&content);

        assert!(errors.is_empty());
    }
}
