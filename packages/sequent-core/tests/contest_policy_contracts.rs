// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Configuration and decoded-vote checks must report the right kind of problem
//! without discarding earlier errors or changing the original voter choices.

#![cfg(feature = "default_features")]

use sequent_core::ballot::*;
use sequent_core::ballot_codec::checker::*;
use sequent_core::plaintext::*;
use sequent_core::services::error_checker::{
    check_contest, check_max_selections_per_type,
};
use sequent_core::types::ceremonies::{
    CountingAlgType, TallySessionResolutionData, TieBreakingMethod,
};

fn contest() -> Contest {
    Contest {
        id: "council".into(),
        max_votes: 3,
        min_votes: 1,
        candidates: ["a", "b", "c"]
            .into_iter()
            .map(|id| Candidate {
                id: id.into(),
                candidate_type: Some("district".into()),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn vote() -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: "council".into(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: ["a", "b", "c"]
            .into_iter()
            .map(|id| DecodedVoteChoice {
                id: id.into(),
                selected: 0,
                write_in_text: None,
            })
            .collect(),
    }
}

#[test]
fn invalid_numeric_configuration_is_an_encoding_error_not_a_voter_warning() {
    let (max, min, result) = check_max_min_votes_policy(-1, -2);
    assert_eq!((max, min), (None, None));
    assert!(result.invalid_alerts.is_empty());
    assert_eq!(result.invalid_errors.len(), 2);
    assert_eq!(
        result.invalid_errors[0].message.as_deref(),
        Some("errors.encoding.invalidMaxVotes")
    );
    assert_eq!(result.invalid_errors[0].message_map["max"], "-1");
    assert_eq!(result.invalid_errors[1].message_map["min"], "-2");
    assert!(result
        .invalid_errors
        .iter()
        .all(|error| error.error_type
            == InvalidPlaintextErrorType::EncodingError));
    let (max, min, result) = check_max_min_votes_policy(3, 1);
    assert_eq!((max, min), (Some(3), Some(1)));
    assert!(result.invalid_errors.is_empty());
}

#[test]
fn selection_limits_count_only_selected_candidates_of_the_relevant_type() {
    let mut contest = contest();
    let mut vote = vote();
    assert!(check_max_selections_per_type(&contest, &vote).is_empty());
    contest.presentation = Some(ContestPresentation {
        max_selections_per_type: Some(2),
        ..Default::default()
    });
    let errors = check_max_selections_per_type(&contest, &vote);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].message.as_deref(),
        Some("errors.implicit.maxSelectionsPerType")
    );
    assert_eq!(errors[0].message_map["type"], "district");
    assert_eq!(errors[0].message_map["numSelected"], "3");
    assert_eq!(errors[0].message_map["max"], "2");
    vote.invalid_errors = errors.clone();
    let checked = check_contest(&contest, &vote);
    assert_eq!(checked.invalid_errors.len(), 2);
    assert_eq!(vote.invalid_errors.len(), 1);
    assert_eq!(checked.choices, vote.choices);
    vote.choices[2].selected = -1;
    assert!(check_max_selections_per_type(&contest, &vote).is_empty());
    vote.choices[2].selected = 0;
    contest.candidates[2].candidate_type = None;
    assert!(check_max_selections_per_type(&contest, &vote).is_empty());
}

#[test]
fn warning_policies_preserve_the_expected_error_and_alert_channels() {
    for policy in [
        EOverVotePolicy::ALLOWED,
        EOverVotePolicy::ALLOWED_WITH_MSG,
        EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT,
        EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT,
        EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE,
    ] {
        let presentation = ContestPresentation {
            over_vote_policy: Some(policy.clone()),
            ..Default::default()
        };
        let result = check_over_vote_policy(&presentation, 3, 2);
        assert!(
            !result.invalid_errors.is_empty(),
            "an overvote remains invalid even when the UI permits continuing"
        );
        assert_eq!(
            result.invalid_alerts.is_empty(),
            policy == EOverVotePolicy::ALLOWED
        );
        assert!(check_over_vote_policy(&presentation, 2, 2)
            .invalid_errors
            .is_empty());
    }
    for policy in [
        EPreferenceGapsPolicy::ALLOWED_WARN_AND_DIALOG,
        EPreferenceGapsPolicy::NOT_ALLOWED_WARN_AND_DIALOG,
    ] {
        let presentation = ContestPresentation {
            preference_gaps_policy: Some(policy),
            ..Default::default()
        };
        assert_eq!(
            check_preference_gaps_policy(&presentation).invalid_errors[0]
                .message
                .as_deref(),
            Some("errors.implicit.preferenceOrderWithGaps")
        );
    }
    let presentation = ContestPresentation {
        invalid_vote_policy: Some(
            InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT,
        ),
        ..Default::default()
    };
    let result = check_invalid_vote_policy(&presentation, true);
    assert_eq!(
        result.invalid_alerts[0].message.as_deref(),
        Some("errors.explicit.alert")
    );
}

#[test]
fn contest_mark_limits_and_tie_resolutions_preserve_configured_election_rules()
{
    let mut contest = contest();
    assert_eq!(contest.max_marks_per_ballot(), 3);
    assert_eq!(contest.cumulative_number_of_checkboxes(), 1);
    assert_eq!(
        contest.get_invalid_vote_policy(),
        InvalidVotePolicy::ALLOWED
    );
    assert_eq!(contest.get_tie_breaking_policy(), TieBreakingPolicy::RANDOM);
    assert!(contest.get_tie_resolutions().is_empty());
    Contest::insert_tie_resolutions(&mut contest, &vec![]).unwrap();
    assert_eq!(contest.annotations, None);
    contest.counting_algorithm = Some(CountingAlgType::Cumulative);
    contest.presentation = Some(ContestPresentation {
        cumulative_number_of_checkboxes: Some(4),
        invalid_vote_policy: Some(InvalidVotePolicy::NOT_ALLOWED),
        ..Default::default()
    });
    assert_eq!(contest.max_marks_per_ballot(), 12);
    assert_eq!(contest.cumulative_number_of_checkboxes(), 4);
    assert_eq!(
        contest.get_invalid_vote_policy(),
        InvalidVotePolicy::NOT_ALLOWED
    );
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);
    assert_eq!(
        contest.get_tie_breaking_policy(),
        TieBreakingPolicy::EXTERNAL_PROCEDURE
    );
    contest.annotations = Some(
        [
            ("keep".into(), "unchanged".into()),
            ("tie_resolutions".into(), "malformed".into()),
        ]
        .into(),
    );
    assert!(contest.get_tie_resolutions().is_empty());
    let resolution = TallySessionResolutionData {
        round_number: Some(2),
        tied_candidate_ids: vec!["a".into(), "b".into()],
        vote_count: 17,
        method_used: TieBreakingMethod::ExternalProcedure,
        resolved_by_candidate_id: Some("b".into()),
    };
    Contest::insert_tie_resolutions(&mut contest, &vec![resolution.clone()])
        .unwrap();
    assert_eq!(contest.get_tie_resolutions(), vec![resolution]);
    assert_eq!(contest.annotations.as_ref().unwrap()["keep"], "unchanged");
    assert_eq!(vote().is_decline_to_vote(), false);
    assert!(vote().choices[0].is_selected());
    assert!(!Candidate::default().is_category_list());
    let category = Candidate {
        presentation: Some(CandidatePresentation {
            is_category_list: Some(true),
            ..CandidatePresentation::new()
        }),
        ..Default::default()
    };
    assert!(category.is_category_list());
    assert!(!AreaPresentation::default().is_early_voting());
    assert!(AreaPresentation {
        allow_early_voting: Some(EarlyVotingPolicy::AllowEarlyVoting)
    }
    .is_early_voting());
}

#[test]
fn stored_counting_algorithms_determine_mark_budgets_and_tally_operations() {
    use sequent_core::services::tally_sheet_validation::{
        resolve_max_marks_per_ballot, UNKNOWN_COUNTING_ALGORITHM,
    };
    use sequent_core::types::ceremonies::{CountingAlgType, TallyOperation};

    assert_eq!(
        resolve_max_marks_per_ballot(Some(3), None, Some(4)).unwrap(),
        3
    );
    assert_eq!(
        resolve_max_marks_per_ballot(Some(3), Some("cumulative"), Some(4))
            .unwrap(),
        12
    );
    let error =
        resolve_max_marks_per_ballot(Some(3), Some("not-an-algorithm"), None)
            .unwrap_err();
    assert_eq!(error.code, UNKNOWN_COUNTING_ALGORITHM);
    assert_eq!(error.field, "counting_algorithm");
    assert_eq!(error.params["countingAlgorithm"], "not-an-algorithm");

    // Preferential tallies need all ballots at contest level; area totals must
    // not invent candidate aggregates for an algorithm that transfers votes.
    for algorithm in [CountingAlgType::InstantRunoff, CountingAlgType::Borda] {
        assert_eq!(
            algorithm.get_default_tally_operation_for_contest(),
            TallyOperation::ProcessBallotsAll
        );
        assert_eq!(
            algorithm.get_default_tally_operation_for_area(),
            TallyOperation::SkipCandidateResults
        );
    }
    for algorithm in [
        CountingAlgType::PluralityAtLarge,
        CountingAlgType::Cumulative,
    ] {
        assert_eq!(
            algorithm.get_default_tally_operation_for_contest(),
            TallyOperation::AggregateResults
        );
        assert_eq!(
            algorithm.get_default_tally_operation_for_area(),
            TallyOperation::ProcessBallotsAll
        );
    }
}

#[test]
fn weighted_batches_reconstruct_each_voters_weight_without_shift_overflow() {
    use sequent_core::types::keycloak::{
        weight_bit_multiplier, weight_has_bit, MAX_VOTE_WEIGHT,
        VOTE_WEIGHT_BATCHES,
    };
    for weight in [0, 1, 2, 3, 5, 255, 256, MAX_VOTE_WEIGHT] {
        let reconstructed: u64 = (0..VOTE_WEIGHT_BATCHES)
            .filter(|bit| weight_has_bit(weight, *bit))
            .map(|bit| weight_bit_multiplier(bit).unwrap())
            .sum();
        assert_eq!(reconstructed, weight);
    }
    for bit in [VOTE_WEIGHT_BATCHES, u32::MAX] {
        assert!(!weight_has_bit(u64::MAX, bit));
        assert_eq!(weight_bit_multiplier(bit), None);
    }
}
