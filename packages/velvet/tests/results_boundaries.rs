// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Public results are consumed from disk as well as in memory. These tests
//! exercise both paths and compare identities, totals and published positions.

mod support;

use sequent_core::ballot::Weight;
use sequent_core::types::ceremonies::{ScopeOperation, TallyOperation};
use std::fs;
use support::*;
use tempfile::tempdir;
use velvet::pipes::do_tally::counting_algorithm::{
    plurality_at_large::PluralityAtLarge, CountingAlgorithm,
};
use velvet::pipes::do_tally::tally::Tally;
use velvet::pipes::do_tally::{
    CandidateResult, ContestResult, OUTPUT_BREAKDOWNS_FOLDER, OUTPUT_CONTEST_RESULT_FILE,
};
use velvet::pipes::mark_winners::{MarkWinners, OUTPUT_WINNERS};

fn election_result(counts: &[(&str, u64)]) -> ContestResult {
    let contest = contest();
    ContestResult {
        candidate_result: counts
            .iter()
            .map(|(id, count)| CandidateResult {
                candidate: contest
                    .candidates
                    .iter()
                    .find(|c| c.id == *id)
                    .unwrap()
                    .clone(),
                total_count: *count,
                percentage_votes: 0.0,
            })
            .collect(),
        contest,
        ..Default::default()
    }
}

#[test]
fn winners_use_counts_then_names_and_never_elect_ballot_markers() {
    // Marker totals deliberately exceed every candidate. Bea and Ada tie;
    // their input order must not decide which one gets the first position.
    let result = election_result(&[
        ("blank", 100),
        ("invalid", 100),
        ("bea", 6),
        ("ada", 6),
        ("cam", 1),
    ]);
    let winners = MarkWinners::get_winners(&result);
    let positions: Vec<_> = winners
        .iter()
        .map(|w| (w.candidate.id.as_str(), w.total_count, w.winning_position))
        .collect();
    assert_eq!(positions, [("ada", 6, 1), ("bea", 6, 2)]);
}

#[test]
fn zero_seats_and_more_seats_than_candidates_have_bounded_outputs() {
    let mut result = election_result(&[("ada", 2), ("bea", 1)]);
    result.contest.winning_candidates_num = 0;
    assert!(MarkWinners::get_winners(&result).is_empty());

    result.contest.winning_candidates_num = 10;
    let winners = MarkWinners::get_winners(&result);
    assert_eq!(winners.len(), 2);
    assert_eq!(winners[1].winning_position, 2);
}

#[test]
fn acclaimed_winners_preserve_candidate_result_order_and_have_no_vote_totals() {
    let mut result = election_result(&[("bea", 1), ("blank", 50), ("ada", 99), ("invalid", 50)]);
    result.contest.is_acclaimed = Some(true);
    let winners = MarkWinners::get_winners(&result);

    assert_eq!(
        winners
            .iter()
            .map(|w| w.candidate.id.as_str())
            .collect::<Vec<_>>(),
        ["bea", "ada"]
    );
    assert!(winners.iter().all(|w| w.total_count == 0));
    assert_eq!(winners[0].winning_position, 1);
    assert_eq!(winners[1].winning_position, 2);
}

#[test]
fn aggregation_is_partition_independent_for_counts_and_percentages() {
    let first = PluralityAtLarge::new(tally(vec![(ballot(&["ada", "bea"]), weight(2))]))
        .tally()
        .unwrap();
    let second = PluralityAtLarge::new(tally(vec![(ballot(&["bea"]), Weight::default())]))
        .tally()
        .unwrap();
    let together = PluralityAtLarge::new(tally(vec![
        (ballot(&["ada", "bea"]), weight(2)),
        (ballot(&["bea"]), Weight::default()),
    ]))
    .tally()
    .unwrap();

    // A change in partition or fold order may reorder the result vector but
    // must not change the election. Census summation is checked separately.
    for aggregate in [
        first.aggregate(&second, false),
        second.aggregate(&first, false),
    ] {
        assert_eq!(candidate_counts(&aggregate), candidate_counts(&together));
        assert_eq!(aggregate.total_valid_votes, 2);
        assert_percentage(candidate_percentage(&aggregate, "ada"), 40.0);
        assert_percentage(candidate_percentage(&aggregate, "bea"), 60.0);
        assert_eq!(aggregate.extended_metrics.as_ref().unwrap().total_weight, 5);
    }
}

#[test]
fn ballot_files_keep_their_area_weights_and_fail_on_missing_or_malformed_input() {
    let directory = tempdir().unwrap();
    let first = directory.path().join("first.json");
    let second = directory.path().join("second.json");
    fs::write(&first, serde_json::to_vec(&vec![ballot(&["ada"])]).unwrap()).unwrap();
    fs::write(
        &second,
        serde_json::to_vec(&vec![ballot(&["bea"])]).unwrap(),
    )
    .unwrap();
    let load = |files| {
        Tally::new(
            &contest(),
            ScopeOperation::Area(TallyOperation::ProcessBallotsAll),
            files,
            10,
            0,
            vec![],
            vec![],
        )
    };
    let loaded = load(vec![
        (first.clone(), weight(2)),
        (second.clone(), weight(3)),
    ])
    .unwrap();
    let result = PluralityAtLarge::new(loaded).tally().unwrap();
    assert_eq!(candidate_counts(&result)["ada"], 2);
    assert_eq!(candidate_counts(&result)["bea"], 3);

    fs::write(&second, br#"[{"contest_id": 7}]"#).unwrap();
    assert!(load(vec![(second, Weight::default())]).is_err());
    assert!(load(vec![(
        directory.path().join("missing.json"),
        Weight::default()
    )])
    .is_err());

    let mut missing_algorithm = contest();
    missing_algorithm.counting_algorithm = None;
    assert!(Tally::new(
        &missing_algorithm,
        ScopeOperation::Area(TallyOperation::ProcessBallotsAll),
        vec![],
        0,
        0,
        vec![],
        vec![]
    )
    .is_err());
}

#[test]
fn breakdown_winner_files_keep_each_area_separate() {
    let input = tempdir().unwrap();
    let output = tempdir().unwrap();
    for (area, result) in [
        ("north", election_result(&[("ada", 8), ("bea", 2)])),
        ("south", election_result(&[("ada", 1), ("bea", 9)])),
    ] {
        let directory = input.path().join(OUTPUT_BREAKDOWNS_FOLDER).join(area);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join(OUTPUT_CONTEST_RESULT_FILE),
            serde_json::to_vec(&result).unwrap(),
        )
        .unwrap();
    }
    MarkWinners::create_breakdown_winners(
        &input.path().to_path_buf(),
        &output.path().to_path_buf(),
    )
    .unwrap();

    for (area, expected) in [("north", "ada"), ("south", "bea")] {
        let path = output
            .path()
            .join(OUTPUT_BREAKDOWNS_FOLDER)
            .join(area)
            .join(OUTPUT_WINNERS);
        let winners: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(winners[0]["candidate"]["id"], expected);
        assert_eq!(winners[0]["winning_position"], 1);
        assert_eq!(winners.as_array().unwrap().len(), 2);
    }
}

#[test]
fn a_missing_or_corrupt_breakdown_result_does_not_publish_winners() {
    let input = tempdir().unwrap();
    let output = tempdir().unwrap();
    let area = input.path().join(OUTPUT_BREAKDOWNS_FOLDER).join("north");
    fs::create_dir_all(&area).unwrap();
    let publish = || {
        MarkWinners::create_breakdown_winners(
            &input.path().to_path_buf(),
            &output.path().to_path_buf(),
        )
    };
    assert!(publish().is_err());

    fs::write(area.join(OUTPUT_CONTEST_RESULT_FILE), b"{truncated").unwrap();
    assert!(publish().is_err());
    assert!(!output
        .path()
        .join(OUTPUT_BREAKDOWNS_FOLDER)
        .join("north")
        .join(OUTPUT_WINNERS)
        .exists());
}
