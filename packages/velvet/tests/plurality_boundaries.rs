// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Hand-counted elections protect the distinction between ballots, candidate
//! marks and weighted marks. Percentages must use the matching denominator.

mod support;

use sequent_core::ballot::Weight;
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use support::*;
use velvet::pipes::do_tally::counting_algorithm::{
    plurality_at_large::PluralityAtLarge, CountingAlgorithm,
};

#[test]
fn area_weights_apply_to_marks_without_multiplying_participation() {
    // Two ballots: Ada+Bea with weight 3, then Bea with weight 2.
    // Ada has 3 marks, Bea has 5, but exactly two people participated.
    let result = PluralityAtLarge::new(tally(vec![
        (ballot(&["ada", "bea"]), weight(3)),
        (ballot(&["bea"]), weight(2)),
    ]))
    .tally()
    .unwrap();

    let counts = candidate_counts(&result);
    assert_eq!(counts["ada"], 3);
    assert_eq!(counts["bea"], 5);
    assert_eq!(counts["cam"], 0);
    assert_eq!(result.total_votes, 2);
    assert_eq!(result.total_valid_votes, 2);
    assert_percentage(result.percentage_total_votes, 20.0);
    assert_percentage(result.percentage_auditable_votes, 10.0);
    assert_percentage(candidate_percentage(&result, "ada"), 37.5);
    assert_percentage(candidate_percentage(&result, "bea"), 62.5);

    let metrics = result.extended_metrics.unwrap();
    assert_eq!(metrics.total_ballots, 2);
    assert_eq!(metrics.total_weight, 8);
    assert_eq!(metrics.expected_votes, 4);
    assert_eq!(metrics.votes_actually, 3);
    assert_eq!(metrics.under_votes, 1);
}

#[test]
fn zero_weight_keeps_the_ballot_but_adds_no_candidate_marks() {
    let result = PluralityAtLarge::new(tally(vec![(ballot(&["ada"]), weight(0))]))
        .tally()
        .unwrap();

    assert_eq!(result.total_valid_votes, 1);
    assert_eq!(candidate_counts(&result)["ada"], 0);
    assert_percentage(candidate_percentage(&result, "ada"), 0.0);
    assert_eq!(result.extended_metrics.unwrap().total_weight, 0);
}

#[test]
fn explicit_markers_and_declined_ballots_do_not_acquire_area_weight() {
    let mut invalid = ballot(&[]);
    invalid.is_explicit_invalid = true;
    let mut declined = ballot(&[]);
    declined.is_decline_to_vote = true;
    let mut blank_ballot = ballot(&[]);
    blank_ballot.is_blank_ballot = true;

    let result = PluralityAtLarge::new(tally(vec![
        (ballot(&["blank"]), weight(7)),
        (invalid, weight(7)),
        (declined, weight(7)),
        (blank_ballot, weight(7)),
    ]))
    .tally()
    .unwrap();

    assert_eq!(result.total_votes, 3); // The declined ballot is reported separately.
    assert_eq!(result.total_valid_votes, 2);
    assert_eq!(result.total_invalid_votes, 1);
    assert_eq!(result.blank_votes.explicit, 1);
    assert_eq!(result.blank_votes.implicit, 1);
    assert_eq!(result.invalid_votes.explicit, 1);
    assert_eq!(candidate_counts(&result)["blank"], 1);
    assert_eq!(candidate_counts(&result)["invalid"], 1);
    assert_percentage(candidate_percentage(&result, "blank"), 25.0);
    assert_percentage(candidate_percentage(&result, "invalid"), 25.0);

    let metrics = result.extended_metrics.unwrap();
    assert_eq!(metrics.total_ballots, 4);
    assert_eq!(metrics.total_weight, 0);
    assert_eq!(metrics.total_declined_to_vote, 1);
    assert_eq!(metrics.total_blank_ballots, 1);
}

#[test]
fn invalid_combinations_cannot_contribute_votes_to_selected_candidates() {
    let mut declined_with_content = ballot(&["ada"]);
    declined_with_content.is_decline_to_vote = true;
    let result = PluralityAtLarge::new(tally(vec![
        (ballot(&["ada", "blank"]), Weight::default()),
        (declined_with_content, Weight::default()),
    ]))
    .tally()
    .unwrap();

    assert_eq!(result.total_valid_votes, 0);
    assert_eq!(result.invalid_votes.implicit, 2);
    assert_eq!(candidate_counts(&result)["ada"], 0);
    assert_eq!(candidate_counts(&result)["blank"], 0);
    assert_eq!(result.extended_metrics.unwrap().total_declined_to_vote, 0);
}

#[test]
fn suppressing_candidate_results_preserves_participation_statistics() {
    let algorithm = PluralityAtLarge::new(tally(vec![
        (ballot(&["ada", "bea"]), weight(4)),
        (ballot(&[]), Weight::default()),
    ]));
    let public = algorithm
        .process_ballots(TallyOperation::ProcessBallotsAll)
        .unwrap();
    let suppressed = algorithm
        .process_ballots(TallyOperation::SkipCandidateResults)
        .unwrap();

    assert!(suppressed.candidate_result.is_empty());
    assert_eq!(suppressed.total_votes, 2);
    assert_eq!(suppressed.total_blank_votes, 1);
    let mut public_json = serde_json::to_value(public).unwrap();
    public_json["candidate_result"] = serde_json::json!([]);
    assert_eq!(serde_json::to_value(suppressed).unwrap(), public_json);
}

#[test]
fn unknown_candidates_fail_instead_of_becoming_unpublished_votes() {
    let valid = PluralityAtLarge::new(tally(vec![(ballot(&["ada"]), Weight::default())]));
    assert!(valid.tally().is_ok());

    let unknown = PluralityAtLarge::new(tally(vec![(ballot(&["outsider"]), Weight::default())]));
    let error = unknown.tally().unwrap_err().to_string();
    assert!(
        error.contains("outsider"),
        "the failure should identify the unknown candidate: {error}"
    );
}

#[test]
fn an_empty_election_has_finite_zero_totals_and_keeps_every_candidate() {
    let mut input = tally(vec![]);
    input.census = 0;
    input.auditable_votes = 0;
    let result = PluralityAtLarge::new(input).tally().unwrap();

    assert_eq!(result.candidate_result.len(), 5);
    assert!(candidate_counts(&result).values().all(|count| *count == 0));
    assert_eq!(result.total_votes, 0);
    assert_percentage(result.percentage_total_votes, 0.0);
    assert_percentage(result.percentage_auditable_votes, 0.0);
    for candidate in &result.candidate_result {
        assert_percentage(candidate.percentage_votes, 0.0);
    }
}

#[test]
fn contest_aggregation_adds_area_censuses_but_area_aggregation_is_rejected() {
    let area = PluralityAtLarge::new(tally(vec![(ballot(&["ada"]), weight(2))]))
        .tally()
        .unwrap();
    let mut input = tally(vec![]);
    input.scope_operation = ScopeOperation::Contest(TallyOperation::AggregateResults);
    input.tally_results = vec![area.clone(), area];
    let result = PluralityAtLarge::new(input).tally().unwrap();
    assert_eq!(result.census, 20);
    assert_eq!(result.total_votes, 2);
    assert_eq!(candidate_counts(&result)["ada"], 4);

    let mut invalid = tally(vec![]);
    invalid.scope_operation = ScopeOperation::Area(TallyOperation::AggregateResults);
    assert!(PluralityAtLarge::new(invalid).tally().is_err());

    let mut no_results = tally(vec![]);
    no_results.scope_operation = ScopeOperation::Contest(TallyOperation::AggregateResults);
    assert!(PluralityAtLarge::new(no_results).tally().is_err());
}

#[test]
fn paper_results_add_marks_and_participation_without_counting_the_same_census_twice() {
    let paper = PluralityAtLarge::new(tally(vec![(ballot(&["bea"]), weight(3))]))
        .tally()
        .unwrap();
    let mut input = tally(vec![(ballot(&["ada"]), Weight::default())]);
    input.tally_sheet_results = vec![paper];
    let result = PluralityAtLarge::new(input).tally().unwrap();

    assert_eq!(result.census, 10);
    assert_eq!(result.total_votes, 2);
    assert_percentage(candidate_percentage(&result, "ada"), 25.0);
    assert_percentage(candidate_percentage(&result, "bea"), 75.0);
    assert_eq!(result.extended_metrics.unwrap().total_weight, 4);
}
