// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The voting screen distinguishes a warning the voter may accept from a
//! selection they must change. These cases keep those two decisions separate.

#![cfg(feature = "default_features")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(private_interfaces, private_bounds, unnameable_types)]
#![deny(rustdoc::missing_crate_level_docs, rustdoc::broken_intra_doc_links)]
#![deny(
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::unwrap_used,
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::indexing_slicing,
    clippy::future_not_send,
    clippy::arithmetic_side_effects,
    clippy::suspicious,
    clippy::complexity,
    clippy::style,
    clippy::perf,
    clippy::pedantic
)]

use support::TestResult;

mod support;

use sequent_core::ballot::*;
use sequent_core::plaintext::*;
use sequent_core::util::voting_screen::{
    check_voting_error_dialog_util, check_voting_not_allowed_next_util,
    get_contest_plurality,
};
use std::collections::HashMap;

fn contest() -> Contest {
    get_contest_plurality(
        EOverVotePolicy::ALLOWED,
        EBlankVotePolicy::ALLOWED,
        InvalidVotePolicy::ALLOWED,
        Some(1),
    )
}

/// Ranks are zero-based; negative values are unselected candidates.
fn decoded(contest: &Contest, ranks: &[i64]) -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: contest.id.clone(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: ranks
            .iter()
            .enumerate()
            .map(|(index, &selected)| DecodedVoteChoice {
                id: index.to_string(),
                selected,
                write_in_text: None,
            })
            .collect(),
    }
}

fn error(
    kind: InvalidPlaintextErrorType,
    message: &str,
) -> InvalidPlaintextError {
    InvalidPlaintextError {
        error_type: kind,
        candidate_id: None,
        message: Some(message.into()),
        message_map: HashMap::new(),
    }
}

/// Named outcomes make the policy tables readable without a pair of booleans.
#[derive(Debug, PartialEq)]
enum Decision {
    Continue,
    Warn,
    Block,
}

fn decision(contest: &Contest, vote: &DecodedVoteContest) -> Decision {
    let votes = HashMap::from([(contest.id.clone(), vote.clone())]);
    if check_voting_not_allowed_next_util(vec![contest.clone()], votes.clone())
    {
        Decision::Block
    } else if check_voting_error_dialog_util(vec![contest.clone()], votes) {
        Decision::Warn
    } else {
        Decision::Continue
    }
}

#[test]
fn a_missing_decoded_contest_blocks_but_an_acclaimed_contest_does_not() {
    let mut contest = contest();
    assert!(check_voting_not_allowed_next_util(
        vec![contest.clone()],
        HashMap::new(),
    ));
    assert!(!check_voting_error_dialog_util(
        vec![contest.clone()],
        HashMap::new(),
    ));

    // Acclaimed contests have no selectable candidates: applying a minimum
    // selection requirement to them would strand the voter permanently.
    contest.is_acclaimed = Some(true);
    assert!(!check_voting_not_allowed_next_util(
        vec![contest.clone()],
        HashMap::new(),
    ));
    assert!(!check_voting_error_dialog_util(
        vec![contest],
        HashMap::new()
    ));
}

#[test]
fn blank_policies_do_not_turn_a_warning_into_a_block() -> TestResult {
    for (policy, expected) in [
        (EBlankVotePolicy::ALLOWED, Decision::Continue),
        (EBlankVotePolicy::WARN, Decision::Warn),
        (EBlankVotePolicy::WARN_ONLY_IN_REVIEW, Decision::Continue),
        (EBlankVotePolicy::NOT_ALLOWED, Decision::Block),
    ] {
        let mut contest = contest();
        contest
            .presentation
            .as_mut()
            .ok_or("fixture is missing its presentation")?
            .blank_vote_policy = Some(policy);
        assert_eq!(
            decision(&contest, &decoded(&contest, &[-1, -1])),
            expected,
            "{policy:?}"
        );
        assert_eq!(
            decision(&contest, &decoded(&contest, &[0, -1])),
            Decision::Continue
        );
    }
    Ok(())
}

#[test]
fn overvote_dialog_policies_apply_only_above_the_limit() -> TestResult {
    for (policy, expected) in [
        (EOverVotePolicy::ALLOWED, Decision::Continue),
        (EOverVotePolicy::ALLOWED_WITH_MSG, Decision::Continue),
        (EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT, Decision::Warn),
        (
            EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT,
            Decision::Block,
        ),
    ] {
        let mut contest = contest();
        contest.max_votes = 2;
        contest
            .presentation
            .as_mut()
            .ok_or("fixture is missing its presentation")?
            .over_vote_policy = Some(policy);
        assert_eq!(
            decision(&contest, &decoded(&contest, &[0, 0])),
            Decision::Continue
        );
        assert_eq!(
            decision(&contest, &decoded(&contest, &[0, 0, 0])),
            expected,
            "{policy:?}"
        );
    }
    Ok(())
}

#[test]
fn an_undervote_warning_has_both_a_lower_and_an_upper_boundary() -> TestResult {
    let mut contest = contest();
    contest.min_votes = 2;
    contest.max_votes = 4;
    contest
        .presentation
        .as_mut()
        .ok_or("fixture is missing its presentation")?
        .under_vote_policy = Some(EUnderVotePolicy::WARN_AND_ALERT);

    for (selections, expected) in [
        (0, Decision::Continue),
        (1, Decision::Continue),
        (2, Decision::Warn),
        (3, Decision::Warn),
        (4, Decision::Continue),
    ] {
        // Minimum-vote errors come from decoding. This table isolates the
        // additional confirmation for valid selections below the maximum.
        assert_eq!(
            decision(&contest, &decoded(&contest, &vec![0; selections])),
            expected,
            "{selections} selections"
        );
    }
    Ok(())
}

#[test]
fn invalid_vote_policy_controls_implicit_errors_but_never_encoding_errors(
) -> TestResult {
    for (policy, expected) in [
        (InvalidVotePolicy::ALLOWED, Decision::Continue),
        (
            InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT,
            Decision::Continue,
        ),
        (InvalidVotePolicy::WARN, Decision::Warn),
        (
            InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT,
            Decision::Warn,
        ),
        (InvalidVotePolicy::NOT_ALLOWED, Decision::Block),
    ] {
        let mut contest = contest();
        contest
            .presentation
            .as_mut()
            .ok_or("fixture is missing its presentation")?
            .invalid_vote_policy = Some(policy.clone());
        let mut vote = decoded(&contest, &[0]);
        vote.invalid_errors
            .push(error(InvalidPlaintextErrorType::Implicit, "invalid-choice"));
        assert_eq!(decision(&contest, &vote), expected, "{policy:?}");

        for kind in [
            InvalidPlaintextErrorType::Explicit,
            InvalidPlaintextErrorType::EncodingError,
        ] {
            vote.invalid_errors = vec![error(kind, "malformed-ballot")];
            assert_eq!(decision(&contest, &vote), Decision::Block);
        }
    }
    Ok(())
}

#[test]
fn explicit_invalid_markers_are_neither_blank_nor_counted_twice() -> TestResult
{
    let mut contest = contest();
    contest.max_votes = 1;
    let presentation = contest
        .presentation
        .as_mut()
        .ok_or("fixture is missing its presentation")?;
    presentation.blank_vote_policy = Some(EBlankVotePolicy::NOT_ALLOWED);
    presentation.over_vote_policy =
        Some(EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT);
    contest
        .candidates
        .first_mut()
        .ok_or("fixture candidate is missing")?
        .presentation
        .as_mut()
        .ok_or("fixture is missing its presentation")?
        .is_explicit_invalid = Some(true);

    // Multi-contest decoding includes the marker among the choices, whereas
    // single-contest decoding can carry it only as a separate flag.
    for ranks in [&[0][..], &[][..]] {
        let mut vote = decoded(&contest, ranks);
        vote.is_explicit_invalid = true;
        assert_eq!(decision(&contest, &vote), Decision::Continue);

        contest
            .presentation
            .as_mut()
            .ok_or("fixture is missing its presentation")?
            .invalid_vote_policy =
            Some(InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT);
        assert_eq!(decision(&contest, &vote), Decision::Warn);
        contest
            .presentation
            .as_mut()
            .ok_or("fixture is missing its presentation")?
            .invalid_vote_policy = Some(InvalidVotePolicy::ALLOWED);
    }
    Ok(())
}

#[test]
fn rank_errors_follow_their_own_warning_or_blocking_policy() -> TestResult {
    for message in [
        "errors.implicit.duplicatedPosition",
        "errors.implicit.preferenceOrderWithGaps",
    ] {
        let mut contest = contest();
        let mut vote = decoded(&contest, &[0, 0]);
        vote.invalid_errors
            .push(error(InvalidPlaintextErrorType::Implicit, message));
        assert_eq!(decision(&contest, &vote), Decision::Warn);

        let presentation = contest
            .presentation
            .as_mut()
            .ok_or("fixture is missing its presentation")?;
        presentation.duplicated_rank_policy =
            Some(EDuplicatedRankPolicy::NOT_ALLOWED_WARN_AND_DIALOG);
        presentation.preference_gaps_policy =
            Some(EPreferenceGapsPolicy::NOT_ALLOWED_WARN_AND_DIALOG);
        assert_eq!(decision(&contest, &vote), Decision::Block);

        // An unrelated error must not accidentally match either rank rule.
        vote.invalid_errors
            .first_mut()
            .ok_or("fixture error is missing")?
            .message = None;
        assert_eq!(decision(&contest, &vote), Decision::Continue);
    }
    Ok(())
}

#[test]
fn a_contest_without_presentation_uses_the_default_policies() {
    let mut contest = contest();
    contest.presentation = None;
    assert_eq!(
        decision(&contest, &decoded(&contest, &[])),
        Decision::Continue
    );
    assert_eq!(
        decision(&contest, &decoded(&contest, &[0, 0, 0, 0])),
        Decision::Warn
    );
}

#[test]
fn preference_validation_reports_duplicates_and_gaps_independently() {
    let contest = contest();
    for ranks in [vec![], vec![-1], vec![2, 0, -1, 1]] {
        assert_eq!(
            decoded(&contest, &ranks).validate_preferencial_order(),
            Ok(())
        );
    }
    for (ranks, errors) in [
        (
            vec![0, 0],
            vec![PreferencialOrderErrorType::DuplicatedPosition],
        ),
        (
            vec![0, 2],
            vec![PreferencialOrderErrorType::PreferenceOrderWithGaps],
        ),
        (
            vec![1, 1],
            vec![
                PreferencialOrderErrorType::DuplicatedPosition,
                PreferencialOrderErrorType::PreferenceOrderWithGaps,
            ],
        ),
    ] {
        assert_eq!(
            decoded(&contest, &ranks).validate_preferencial_order(),
            Err(errors)
        );
    }
}
