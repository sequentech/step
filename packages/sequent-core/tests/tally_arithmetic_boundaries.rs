// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A ballot count fits u64, but intermediate sums of marks or contest blanks
//! may not. Validate the mathematical totals without wrapping or rejecting a
//! legitimate multi-mark tally merely because its aggregate exceeds u64.

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

use sequent_core::services::tally_sheet_validation::{
    validate_area_contest_results, validate_ballot_box_blank_ballots,
};
use sequent_core::types::tally_sheets::{
    AreaContestResults, CandidateResults, InvalidVotes,
};
use std::collections::HashMap;

fn candidate_results(votes: &[u64]) -> HashMap<String, CandidateResults> {
    votes
        .iter()
        .enumerate()
        .map(|(index, &count)| {
            let id = format!("candidate-{index}");
            (
                id.clone(),
                CandidateResults {
                    candidate_id: id,
                    total_votes: Some(count),
                },
            )
        })
        .collect()
}

#[test]
fn invalid_vote_components_cannot_wrap_to_a_plausible_declared_total(
) -> TestResult {
    let sheet = AreaContestResults {
        total_votes: Some(0),
        total_valid_votes: Some(0),
        invalid_votes: Some(InvalidVotes {
            total_invalid: Some(0),
            implicit_invalid: Some(u64::MAX),
            explicit_invalid: Some(1),
        }),
        ..Default::default()
    };
    let errors = validate_area_contest_results(&sheet, None);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors
            .first()
            .ok_or("expected a tally validation error")?
            .code,
        "invalid_total_invalid"
    );
    Ok(())
}

#[test]
fn valid_and_invalid_ballots_cannot_wrap_to_a_zero_turnout() -> TestResult {
    let sheet = AreaContestResults {
        total_votes: Some(0),
        total_valid_votes: Some(u64::MAX),
        candidate_results: candidate_results(&[u64::MAX]),
        invalid_votes: Some(InvalidVotes {
            total_invalid: Some(1),
            implicit_invalid: Some(1),
            explicit_invalid: Some(0),
        }),
        ..Default::default()
    };
    let errors = validate_area_contest_results(&sheet, None);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors
            .first()
            .ok_or("expected a tally validation error")?
            .code,
        "invalid_total_votes"
    );
    Ok(())
}

#[test]
fn candidate_mark_sums_above_u64_are_valid_only_when_the_contest_allows_them(
) -> TestResult {
    let sheet = AreaContestResults {
        total_votes: Some(u64::MAX),
        total_valid_votes: Some(u64::MAX),
        candidate_results: candidate_results(&[u64::MAX, 1]),
        ..Default::default()
    };
    // MAX + 1 marks fit a two-mark contest with MAX ballots. Saturating the
    // sum would also wrongly accept it when only one mark is permitted.
    assert!(validate_area_contest_results(&sheet, Some(2)).is_empty());
    let errors = validate_area_contest_results(&sheet, Some(1));
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors
            .first()
            .ok_or("expected a tally validation error")?
            .code,
        "invalid_total_valid_votes"
    );
    assert_eq!(
        errors
            .first()
            .ok_or("expected a tally validation error")?
            .params
            .get("candidateVotesSum")
            .ok_or("candidate sum is missing")?,
        "18446744073709551616"
    );
    Ok(())
}

#[test]
fn the_largest_representable_single_mark_tally_remains_valid() {
    let sheet = AreaContestResults {
        total_votes: Some(u64::MAX),
        total_valid_votes: Some(u64::MAX),
        census: Some(u64::MAX),
        candidate_results: candidate_results(&[u64::MAX]),
        ..Default::default()
    };
    assert!(validate_area_contest_results(&sheet, None).is_empty());
}

#[test]
fn unanimous_blank_contests_keep_the_exact_box_total_at_the_numeric_limit() {
    let sheets: Vec<_> = (0..3)
        .map(|_| AreaContestResults {
            total_votes: Some(u64::MAX),
            total_blank_votes: Some(u64::MAX),
            blank_ballots: Some(u64::MAX),
            ..Default::default()
        })
        .collect();
    let check =
        validate_ballot_box_blank_ballots(&sheets.iter().collect::<Vec<_>>());
    assert!(check.errors.is_empty());
    assert_eq!(check.pre_filled_value, Some(u64::MAX));
}

#[test]
fn large_blank_contest_intersections_keep_their_real_bounds() -> TestResult {
    // With three contests each having one non-blank ballot, at most three
    // distinct ballots are non-blank somewhere: the intersection is MAX-3..MAX-1.
    let mut sheets: Vec<_> = (0..3)
        .map(|_| AreaContestResults {
            total_votes: Some(u64::MAX),
            total_blank_votes: Some(u64::MAX - 1),
            blank_ballots: Some(u64::MAX - 2),
            ..Default::default()
        })
        .collect();
    let valid =
        validate_ballot_box_blank_ballots(&sheets.iter().collect::<Vec<_>>());
    assert!(valid.errors.is_empty());
    assert_eq!(valid.pre_filled_value, None);

    for sheet in &mut sheets {
        sheet.blank_ballots = Some(u64::MAX - 4);
    }
    let invalid =
        validate_ballot_box_blank_ballots(&sheets.iter().collect::<Vec<_>>());
    assert_eq!(invalid.errors.len(), 1);
    assert_eq!(
        invalid
            .errors
            .first()
            .ok_or("expected a tally validation error")?
            .code,
        "blank_ballots_out_of_bounds"
    );
    assert_eq!(
        invalid
            .errors
            .first()
            .ok_or("expected a tally validation error")?
            .params
            .get("lowerBound")
            .ok_or("lower bound is missing")?,
        "18446744073709551612"
    );
    assert_eq!(
        invalid
            .errors
            .first()
            .ok_or("expected a tally validation error")?
            .params
            .get("upperBound")
            .ok_or("upperBound is missing")?,
        "18446744073709551614"
    );
    Ok(())
}
