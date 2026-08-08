// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::services::tally_sheet_validation::{
    validate_area_contest_results, validate_ballot_box_blank_ballots,
};
use sequent_core::types::tally_sheet_import::TallySheetImportValidationError;
use sequent_core::types::tally_sheets::{AreaContestResults, VotingChannel};
use tracing::instrument;

#[instrument(skip_all)]
pub fn validate_import_content(
    channel: &VotingChannel,
    area_name: &str,
    contest_external_id: &str,
    content: &AreaContestResults,
) -> Vec<TallySheetImportValidationError> {
    validate_area_contest_results(content)
        .into_iter()
        .map(|shared_error| {
            error(
                &shared_error.code,
                shared_error.message,
                channel,
                area_name,
                Some(contest_external_id),
                &shared_error.field,
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
) -> TallySheetImportValidationError {
    TallySheetImportValidationError {
        code: code.to_string(),
        message,
        channel: Some(channel.clone()),
        area_name: Some(area_name.to_string()),
        contest_external_id: contest_external_id.map(str::to_string),
        candidate_external_id: None,
        field: Some(field.to_string()),
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

        let errors =
            validate_import_content(&VotingChannel::PAPER, "Precinct 1", "contest-1", &content);

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

        let errors =
            validate_import_content(&VotingChannel::PAPER, "Precinct 1", "contest-1", &content);
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
