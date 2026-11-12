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

// Both overflow cases merge PAPER counts. Velvet's error Display delegates to
// Debug, so the counting-algorithm message wraps the pipe error; match the
// overflow and its channel rather than that wrapper.
const PAPER_CHANNEL_OVERFLOW: &str = "Voting channel count overflow for PAPER";

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
fn area_result_aggregation_rejects_channel_overflow_before_combining_totals() {
    use sequent_core::types::participation::{ParticipationChannel, VotesByChannel};
    use sequent_core::types::tally_sheets::VotingChannel;
    use velvet::pipes::do_tally::ExtendedMetricsContest;

    let channel = ParticipationChannel::from(VotingChannel::PAPER);
    let result = |count| ContestResult {
        contest: contest(),
        census: count,
        total_votes: count,
        total_valid_votes: count,
        extended_metrics: Some(ExtendedMetricsContest {
            total_ballots: count,
            votes_by_channel: VotesByChannel::from([(channel.clone(), count)]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut tally = Tally::new(
        &contest(),
        ScopeOperation::Contest(TallyOperation::AggregateResults),
        vec![],
        0,
        0,
        vec![],
        vec![result(u64::MAX - 1), result(1)],
    )
    .unwrap();
    // These are already-counted area results, not u64::MAX allocated ballots.
    // The public aggregation entry point must accept the exact boundary first.
    let combined = tally.aggregate_results().unwrap();
    assert_eq!(combined.total_votes, u64::MAX);
    assert_eq!(
        combined.extended_metrics.unwrap().votes_by_channel[&channel],
        u64::MAX
    );

    tally.tally_results[1] = result(2);
    let error = tally.aggregate_results().unwrap_err();
    assert!(matches!(error,
        velvet::pipes::do_tally::counting_algorithm::Error::UnexpectedError(message)
        if message.contains(PAPER_CHANNEL_OVERFLOW)));
    assert_eq!(tally.tally_results[0].total_votes, u64::MAX - 1);
}

#[test]
fn counting_algorithms_reject_overflow_when_merging_paper_sheets() {
    use sequent_core::types::ceremonies::CountingAlgType;
    use sequent_core::types::participation::{ParticipationChannel, VotesByChannel};
    use sequent_core::types::tally_sheets::VotingChannel;
    use velvet::pipes::do_tally::counting_algorithm::{instant_runoff::InstantRunoff, Error};
    use velvet::pipes::do_tally::ExtendedMetricsContest;

    let channel = ParticipationChannel::from(VotingChannel::PAPER);
    for algorithm in [
        CountingAlgType::PluralityAtLarge,
        CountingAlgType::InstantRunoff,
    ] {
        let mut contest = contest();
        contest.counting_algorithm = Some(algorithm);
        contest.winning_candidates_num = 1;
        for last_count in [1, 2] {
            let sheets = [u64::MAX - 1, last_count].map(|count| ContestResult {
                contest: contest.clone(),
                total_votes: count,
                total_valid_votes: count,
                extended_metrics: Some(ExtendedMetricsContest {
                    total_ballots: count,
                    votes_by_channel: VotesByChannel::from([(channel.clone(), count)]),
                    ..Default::default()
                }),
                ..Default::default()
            });
            let tally = Tally::new(
                &contest,
                ScopeOperation::Area(TallyOperation::ProcessBallotsAll),
                vec![],
                0,
                0,
                sheets.into(),
                vec![],
            )
            .unwrap();
            let combined = match algorithm {
                CountingAlgType::PluralityAtLarge => PluralityAtLarge::new(tally).tally(),
                CountingAlgType::InstantRunoff => InstantRunoff::new(tally).tally(),
                _ => unreachable!(),
            };
            if last_count == 1 {
                let combined = combined.unwrap();
                assert_eq!(combined.total_votes, u64::MAX);
                assert_eq!(
                    combined.extended_metrics.unwrap().votes_by_channel[&channel],
                    u64::MAX
                );
            } else {
                assert!(
                    matches!(combined.unwrap_err(), Error::UnexpectedError(message)
                    if message.contains(PAPER_CHANNEL_OVERFLOW))
                );
            }
        }
    }
}

/// A result whose only nonzero counter is `counter`, set to `count`.
fn result_with_counter(counter: &str, count: u64) -> ContestResult {
    use sequent_core::types::participation::{ParticipationChannel, VotesByChannel};
    use sequent_core::types::tally_sheets::VotingChannel;
    use velvet::pipes::do_tally::{BlankVotes, ExtendedMetricsContest, InvalidVotes};

    let mut result = ContestResult {
        contest: contest(),
        extended_metrics: Some(ExtendedMetricsContest::default()),
        ..Default::default()
    };
    let metrics = result
        .extended_metrics
        .as_mut()
        .expect("the metrics were just set");
    match counter {
        "over_votes" => metrics.over_votes = count,
        "under_votes" => metrics.under_votes = count,
        "votes_actually" => metrics.votes_actually = count,
        "expected_votes" => metrics.expected_votes = count,
        "total_ballots" => metrics.total_ballots = count,
        "total_weight" => metrics.total_weight = count,
        "total_declined_to_vote" => metrics.total_declined_to_vote = count,
        "total_blank_ballots" => metrics.total_blank_ballots = count,
        "PAPER" => {
            metrics.votes_by_channel =
                VotesByChannel::from([(ParticipationChannel::from(VotingChannel::PAPER), count)])
        }
        "census" => result.census = count,
        "auditable_votes" => result.auditable_votes = count,
        "total_votes" => result.total_votes = count,
        "total_valid_votes" => result.total_valid_votes = count,
        "total_invalid_votes" => result.total_invalid_votes = count,
        "total_blank_votes" => result.total_blank_votes = count,
        "blank explicit" => result.blank_votes = BlankVotes::new(count, 0),
        "blank implicit" => result.blank_votes = BlankVotes::new(0, count),
        "invalid explicit" => result.invalid_votes = InvalidVotes::new(count, 0),
        "invalid implicit" => result.invalid_votes = InvalidVotes::new(0, count),
        "ada" => {
            result.candidate_result = vec![CandidateResult {
                candidate: candidate("ada", "Ada"),
                percentage_votes: 0.0,
                total_count: count,
            }]
        }
        _ => panic!("unknown counter {counter}"),
    }
    result
}

#[test]
fn aggregation_rejects_every_counter_beyond_the_counter_range() {
    // (counter set on both results, the counter named in the overflow error)
    let counters = [
        ("over_votes", "over_votes"),
        ("under_votes", "under_votes"),
        ("votes_actually", "votes_actually"),
        ("expected_votes", "expected_votes"),
        ("total_ballots", "total_ballots"),
        ("total_weight", "total_weight"),
        ("total_declined_to_vote", "total_declined_to_vote"),
        ("total_blank_ballots", "total_blank_ballots"),
        ("PAPER", "PAPER"),
        ("census", "census"),
        ("auditable_votes", "auditable_votes"),
        ("total_votes", "total_votes"),
        ("total_valid_votes", "total_valid_votes"),
        ("total_invalid_votes", "total_invalid_votes"),
        ("total_blank_votes", "total_blank_votes"),
        ("blank explicit", "explicit"),
        ("blank implicit", "implicit"),
        ("invalid explicit", "explicit"),
        ("invalid implicit", "implicit"),
        ("ada", "ada"),
    ];
    for (counter, reported) in counters {
        let exact = result_with_counter(counter, u64::MAX - 1)
            .aggregate(&result_with_counter(counter, 1), true)
            .expect("a total of exactly u64::MAX fits");
        assert_eq!(
            exact.candidate_result.len(),
            usize::from(counter == "ada"),
            "{counter}"
        );

        let error = result_with_counter(counter, u64::MAX)
            .aggregate(&result_with_counter(counter, 1), true)
            .expect_err("a total beyond u64::MAX must fail instead of wrapping");
        assert!(
            error
                .to_string()
                .contains(&format!("overflow for {reported}")),
            "{counter}: {error}"
        );
    }
}

fn paper_sheet(
    candidate_votes: &[(&str, u64)],
    total_valid_votes: u64,
    (explicit_invalid, implicit_invalid): (u64, u64),
) -> sequent_core::types::hasura::core::TallySheet {
    use sequent_core::types::tally_sheets::{
        AreaContestResults, CandidateResults, InvalidVotes as SheetInvalidVotes, TallySheetStatus,
    };

    sequent_core::types::hasura::core::TallySheet {
        id: "paper-batch".into(),
        tenant_id: "tenant".into(),
        election_event_id: "event".into(),
        election_id: "election".into(),
        contest_id: CONTEST_ID.into(),
        area_id: "area".into(),
        created_at: None,
        last_updated_at: None,
        labels: None,
        annotations: None,
        reviewed_at: None,
        reviewed_by_user_id: None,
        content: Some(AreaContestResults {
            area_id: "area".into(),
            contest_id: CONTEST_ID.into(),
            total_valid_votes: Some(total_valid_votes),
            invalid_votes: Some(SheetInvalidVotes {
                total_invalid: None,
                implicit_invalid: Some(implicit_invalid),
                explicit_invalid: Some(explicit_invalid),
            }),
            candidate_results: candidate_votes
                .iter()
                .map(|(id, votes)| {
                    (
                        (*id).to_string(),
                        CandidateResults {
                            candidate_id: (*id).to_string(),
                            total_votes: Some(*votes),
                        },
                    )
                })
                .collect(),
            ..Default::default()
        }),
        channel: Some("PAPER".into()),
        deleted_at: None,
        created_by_user_id: "test-operator".into(),
        status: TallySheetStatus::APPROVED,
        version: 1,
        import_id: None,
    }
}

#[test]
fn tally_sheet_totals_beyond_the_counter_range_fail_instead_of_wrapping() {
    use velvet::pipes::do_tally::tally::process_tally_sheet;

    let contest = contest();
    let exact = process_tally_sheet(
        &paper_sheet(&[("ada", u64::MAX - 1), ("bea", 1)], u64::MAX - 1, (0, 1)),
        &contest,
    )
    .expect("totals of exactly u64::MAX fit");
    assert_eq!(exact.total_votes, u64::MAX);
    assert_eq!(
        exact
            .extended_metrics
            .expect("a sheet result has metrics")
            .total_weight,
        u64::MAX
    );

    for (sheet, counter) in [
        (
            paper_sheet(&[("ada", u64::MAX), ("bea", 1)], 1, (0, 0)),
            "candidate votes",
        ),
        (
            paper_sheet(&[("ada", 1)], 1, (u64::MAX, 1)),
            "total_invalid",
        ),
        (paper_sheet(&[("ada", 1)], u64::MAX, (0, 1)), "total_votes"),
    ] {
        let error = process_tally_sheet(&sheet, &contest)
            .expect_err("a total beyond u64::MAX must fail instead of wrapping");
        assert!(
            error
                .to_string()
                .contains(&format!("Tally count overflow for {counter}")),
            "{counter}: {error}"
        );
    }
}

#[test]
fn a_tally_sheet_cannot_wrap_the_electronic_candidate_total() {
    use velvet::pipes::do_tally::counting_algorithm::Error;
    use velvet::pipes::do_tally::tally::process_tally_sheet;

    // Three electronic votes for Ada, then one approved paper sheet for her.
    let area_with_sheet = |ada_paper_votes: u64| {
        let mut area = tally(vec![(ballot(&["ada"]), Weight::default()); 3]);
        area.tally_sheet_results = vec![process_tally_sheet(
            &paper_sheet(&[("ada", ada_paper_votes)], ada_paper_votes, (0, 0)),
            &contest(),
        )
        .expect("a self-consistent sheet processes")];
        PluralityAtLarge::new(area).tally()
    };

    let combined = area_with_sheet(4).expect("small counts aggregate");
    assert_eq!(candidate_counts(&combined)["ada"], 7);

    let error = area_with_sheet(u64::MAX - 1)
        .expect_err("a total beyond u64::MAX must fail instead of wrapping");
    assert!(
        matches!(&error, Error::UnexpectedError(message)
            if message.contains("Tally count overflow for")),
        "{error}"
    );
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
        first.aggregate(&second, false).expect("counts fit"),
        second.aggregate(&first, false).expect("counts fit"),
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

#[test]
fn breakdown_publication_reports_blocked_output_files_and_directories() {
    let input = tempdir().unwrap();
    let output = tempdir().unwrap();
    let source = input.path().join(OUTPUT_BREAKDOWNS_FOLDER).join("north");
    fs::create_dir_all(&source).unwrap();
    fs::write(
        source.join(OUTPUT_CONTEST_RESULT_FILE),
        serde_json::to_vec(&election_result(&[("ada", 8), ("bea", 2)])).unwrap(),
    )
    .unwrap();
    let publish = || {
        MarkWinners::create_breakdown_winners(
            &input.path().to_path_buf(),
            &output.path().to_path_buf(),
        )
    };
    publish().unwrap();
    let destination = output.path().join(OUTPUT_BREAKDOWNS_FOLDER).join("north");
    let winners = destination.join(OUTPUT_WINNERS);
    let control: serde_json::Value = serde_json::from_slice(&fs::read(&winners).unwrap()).unwrap();
    assert_eq!(control[0]["candidate"]["id"], "ada");

    // A directory at the file path fails even when tests run as root, unlike
    // permission-bit fixtures. No input or candidate count has changed.
    fs::remove_file(&winners).unwrap();
    fs::create_dir(&winners).unwrap();
    let error = publish().unwrap_err();
    assert!(
        matches!(error, velvet::pipes::error::Error::IO(_)),
        "{error}"
    );
    assert!(winners.is_dir());

    fs::remove_dir_all(&destination).unwrap();
    fs::write(&destination, b"keep this operator file").unwrap();
    let error = publish().unwrap_err();
    assert!(
        matches!(error, velvet::pipes::error::Error::IO(_)),
        "{error}"
    );
    assert_eq!(fs::read(&destination).unwrap(), b"keep this operator file");
}
