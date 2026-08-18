// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::types::ceremonies::CountingAlgType;
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
pub fn effective_max_marks_per_ballot_typed(
    max_votes: Option<i64>,
    counting_algorithm: CountingAlgType,
    cumulative_number_of_checkboxes: Option<u64>,
) -> u64 {
    max_marks_per_ballot(
        max_votes,
        counting_algorithm.is_cumulative(),
        cumulative_number_of_checkboxes,
    )
}

fn max_marks_per_ballot(
    max_votes: Option<i64>,
    is_cumulative: bool,
    cumulative_number_of_checkboxes: Option<u64>,
) -> u64 {
    let base = max_votes
        .filter(|value| *value > 0)
        .map(|value| value as u64)
        .unwrap_or(1);
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
            HashMap::new(),
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
                HashMap::from([
                    ("blankBallots".to_string(), value.to_string()),
                    ("lowerBound".to_string(), lower_bound.to_string()),
                    ("upperBound".to_string(), upper_bound.to_string()),
                ]),
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

    use crate::types::ceremonies::CountingAlgType;
    use crate::types::tally_sheets::{
        AreaContestResults, CandidateResults, InvalidVotes,
    };

    use super::{
        effective_max_marks_per_ballot_typed, validate_area_contest_results,
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
            blank_ballots: None,
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
            blank_ballots: None,
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
            blank_ballots: None,
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
        assert_eq!(
            effective_max_marks_per_ballot_typed(
                None,
                CountingAlgType::PluralityAtLarge,
                None
            ),
            1
        );
    }

    #[test]
    fn effective_max_marks_floors_non_positive_max_votes_to_one() {
        assert_eq!(
            effective_max_marks_per_ballot_typed(
                Some(0),
                CountingAlgType::PluralityAtLarge,
                None
            ),
            1
        );
        assert_eq!(
            effective_max_marks_per_ballot_typed(
                Some(-1),
                CountingAlgType::PluralityAtLarge,
                None
            ),
            1
        );
    }

    #[test]
    fn effective_max_marks_uses_max_votes_for_non_cumulative_contests() {
        assert_eq!(
            effective_max_marks_per_ballot_typed(
                Some(3),
                CountingAlgType::PluralityAtLarge,
                None
            ),
            3
        );
    }

    #[test]
    fn effective_max_marks_multiplies_by_checkboxes_for_cumulative_contests() {
        assert_eq!(
            effective_max_marks_per_ballot_typed(
                Some(2),
                CountingAlgType::Cumulative,
                Some(3)
            ),
            6
        );
    }

    #[test]
    fn effective_max_marks_defaults_checkboxes_to_one_when_absent() {
        assert_eq!(
            effective_max_marks_per_ballot_typed(
                Some(2),
                CountingAlgType::Cumulative,
                None
            ),
            2
        );
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
            annotations: None,
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
