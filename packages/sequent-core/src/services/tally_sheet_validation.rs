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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BallotBoxBlankBallotsCheck {
    pub errors: Vec<TallySheetValidationError>,
    /// The value implied by the bounds when they pinch to exactly one
    /// integer -- covers the common "some contest reports zero blanks, so
    /// the ballot-box total must be zero too" case. `None` when the bounds
    /// don't uniquely determine a value; the caller decides whether to
    /// offer it as a suggestion or apply it automatically.
    pub pre_filled_value: Option<u64>,
}

/// Validates (and where possible, pre-fills) a ballot box's `blank_ballots`
/// value against the per-contest blank-vote counts of every contest sheet
/// in the box.
///
/// `blank_ballots` is a ballot-box property replicated onto every contest
/// sheet of the box (the same way `total_declined_to_vote` already is), so
/// it must be:
/// - the same across every sheet that supplies it (sheets that omit it are
///   ignored for this check, since the field is optional per sheet);
/// - within `max(0, Σbᵢ − (n−1)·T) ≤ v ≤ min(bᵢ)`, where `bᵢ` is contest
///   `i`'s `total_blank_votes`, `n` is the number of contests in the box,
///   and `T` is the box's total ballot count. The upper bound holds
///   because a ballot blank everywhere is blank in the contest with the
///   fewest blanks; the lower bound is an inclusion-exclusion count: at
///   most `T - bᵢ` ballots are non-blank in contest `i`, so at most
///   `Σ(T - bᵢ) = nT - Σbᵢ` ballots are non-blank in *some* contest, which
///   leaves at least `T - (nT - Σbᵢ) = Σbᵢ - (n-1)T` ballots blank in all
///   of them (floored at 0).
///
/// `T` should be uniform across a box's sheets by construction (every
/// ballot in a box carries every one of the box's contests), but this
/// function tolerates disagreement defensively by using the largest
/// reported `total_votes`: a ballot counted in any one contest's turnout
/// is definitely present in the box, so the maximum is the safest
/// (most conservative) estimate of the box's true ballot count -- a
/// smaller value would tighten the lower bound incorrectly and risk
/// rejecting a genuinely valid entry.
pub fn validate_ballot_box_blank_ballots(
    contest_sheets: &[&AreaContestResults],
) -> BallotBoxBlankBallotsCheck {
    if contest_sheets.is_empty() {
        return BallotBoxBlankBallotsCheck {
            errors: Vec::new(),
            pre_filled_value: None,
        };
    }

    let mut errors = Vec::new();

    let distinct_values: std::collections::BTreeSet<u64> = contest_sheets
        .iter()
        .filter_map(|sheet| sheet.blank_ballots)
        .collect();
    if distinct_values.len() > 1 {
        errors.push(error(
            "inconsistent_blank_ballots",
            "Every contest sheet of a ballot box that supplies blank_ballots must carry the same value"
                .to_string(),
            "blank_ballots",
        ));
    }
    // Ambiguous when sheets disagree: don't guess which one is right.
    let box_blank_ballots = (distinct_values.len() == 1)
        .then(|| distinct_values.into_iter().next())
        .flatten();

    let contest_count = contest_sheets.len() as u64;
    let blank_votes_per_contest: Vec<u64> = contest_sheets
        .iter()
        .map(|sheet| sheet.total_blank_votes.unwrap_or(0))
        .collect();
    let sum_blank_votes: u64 = blank_votes_per_contest.iter().sum();
    let min_blank_votes =
        blank_votes_per_contest.iter().copied().min().unwrap_or(0);
    let total_ballots = contest_sheets
        .iter()
        .filter_map(|sheet| sheet.total_votes)
        .max()
        .unwrap_or(0);

    let lower_bound = sum_blank_votes
        .saturating_sub(contest_count.saturating_sub(1) * total_ballots);
    let upper_bound = min_blank_votes;

    if let Some(value) = box_blank_ballots {
        if value < lower_bound || value > upper_bound {
            errors.push(error(
                "blank_ballots_out_of_bounds",
                format!(
                    "blank_ballots ({value}) must be between {lower_bound} and {upper_bound} given the box's per-contest blank vote counts"
                ),
                "blank_ballots",
            ));
        }
    }

    let pre_filled_value = (lower_bound == upper_bound).then_some(lower_bound);

    BallotBoxBlankBallotsCheck {
        errors,
        pre_filled_value,
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
            blank_ballots: None,
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
            blank_ballots: None,
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
            blank_ballots: None,
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

    fn contest_sheet(
        contest_id: &str,
        total_votes: u64,
        total_blank_votes: u64,
        blank_ballots: Option<u64>,
    ) -> AreaContestResults {
        AreaContestResults {
            area_id: "area-1".to_string(),
            contest_id: contest_id.to_string(),
            total_votes: Some(total_votes),
            total_valid_votes: Some(total_votes),
            invalid_votes: None,
            total_blank_votes: Some(total_blank_votes),
            blank_ballots,
            census: None,
            candidate_results: HashMap::new(),
        }
    }

    mod ballot_box_blank_ballots {
        use super::contest_sheet;
        use crate::services::tally_sheet_validation::validate_ballot_box_blank_ballots;

        #[test]
        fn pre_fills_zero_when_a_contest_has_zero_blanks() {
            let sheets = [
                contest_sheet("contest-1", 10, 0, None),
                contest_sheet("contest-2", 10, 5, None),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            assert!(check.errors.is_empty());
            assert_eq!(check.pre_filled_value, Some(0));
        }

        #[test]
        fn pre_fills_a_nonzero_value_when_bounds_pinch() {
            // n=2, T=9, b=[9,9]: every ballot is blank in both contests,
            // so the box-level value must be exactly 9.
            let sheets = [
                contest_sheet("contest-1", 9, 9, None),
                contest_sheet("contest-2", 9, 9, None),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            assert!(check.errors.is_empty());
            assert_eq!(check.pre_filled_value, Some(9));
        }

        #[test]
        fn does_not_pre_fill_when_bounds_leave_a_range() {
            let sheets = [
                contest_sheet("contest-1", 10, 3, Some(2)),
                contest_sheet("contest-2", 10, 5, Some(2)),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            assert!(check.errors.is_empty());
            assert_eq!(check.pre_filled_value, None);
        }

        #[test]
        fn accepts_a_value_within_bounds() {
            let sheets = [
                contest_sheet("contest-1", 10, 8, Some(7)),
                contest_sheet("contest-2", 10, 9, Some(7)),
                contest_sheet("contest-3", 10, 9, Some(7)),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            assert!(check.errors.is_empty());
        }

        #[test]
        fn rejects_a_value_above_the_upper_bound() {
            let sheets = [
                contest_sheet("contest-1", 10, 3, Some(4)),
                contest_sheet("contest-2", 10, 5, Some(4)),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            let codes: Vec<_> =
                check.errors.iter().map(|e| e.code.as_str()).collect();
            assert_eq!(codes, vec!["blank_ballots_out_of_bounds"]);
        }

        #[test]
        fn rejects_a_value_below_the_lower_bound() {
            // n=2, T=10, b=[9,9]: lower bound is max(0, 18-10)=8.
            let sheets = [
                contest_sheet("contest-1", 10, 9, Some(5)),
                contest_sheet("contest-2", 10, 9, Some(5)),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            let codes: Vec<_> =
                check.errors.iter().map(|e| e.code.as_str()).collect();
            assert_eq!(codes, vec!["blank_ballots_out_of_bounds"]);
        }

        #[test]
        fn rejects_sheets_that_disagree_on_the_value() {
            let sheets = [
                contest_sheet("contest-1", 10, 3, Some(2)),
                contest_sheet("contest-2", 10, 5, Some(3)),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            let codes: Vec<_> =
                check.errors.iter().map(|e| e.code.as_str()).collect();
            assert_eq!(codes, vec!["inconsistent_blank_ballots"]);
        }

        #[test]
        fn stays_unavailable_when_no_sheet_supplies_a_value() {
            let sheets = [
                contest_sheet("contest-1", 10, 3, None),
                contest_sheet("contest-2", 10, 5, None),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            assert!(check.errors.is_empty());
        }

        #[test]
        fn ignores_sheets_that_omit_the_value_when_checking_agreement() {
            let sheets = [
                contest_sheet("contest-1", 10, 3, Some(2)),
                contest_sheet("contest-2", 10, 5, None),
            ];
            let refs: Vec<&_> = sheets.iter().collect();

            let check = validate_ballot_box_blank_ballots(&refs);

            assert!(check.errors.is_empty());
        }

        #[test]
        fn returns_nothing_for_an_empty_box() {
            let check = validate_ballot_box_blank_ballots(&[]);

            assert!(check.errors.is_empty());
            assert_eq!(check.pre_filled_value, None);
        }
    }
}
