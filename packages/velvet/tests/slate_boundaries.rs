// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A slate is a shortcut for marking candidates. The tally receives candidate
//! marks and counts those: a slate has no vote and no row of its own.

use sequent_core::ballot::{Candidate, Contest, Weight};
use sequent_core::ballot_codec::PlaintextCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::{CountingAlgType, ScopeOperation, TallyOperation};
use std::collections::BTreeMap;
use velvet::pipes::do_tally::counting_algorithm::{
    plurality_at_large::PluralityAtLarge, CountingAlgorithm,
};
use velvet::pipes::do_tally::tally::Tally;
use velvet::pipes::do_tally::ContestResult;

const TRUSTEES: &str = "trustees";
const UNITED: [&str; 3] = ["tr-united-1", "tr-united-2", "tr-united-3"];
const VOICES: [&str; 2] = ["tr-voices-1", "tr-voices-2"];
const INDEPENDENT: &str = "tr-independent";

/// Three seats contested by a full slate, a two-candidate slate and an
/// independent candidate.
fn trustees() -> Contest {
    Contest {
        id: TRUSTEES.into(),
        min_votes: 0,
        max_votes: 3,
        winning_candidates_num: 3,
        counting_algorithm: Some(CountingAlgType::PluralityAtLarge),
        candidates: UNITED
            .iter()
            .chain(&VOICES)
            .chain(&[INDEPENDENT])
            .map(|id| Candidate {
                id: (*id).into(),
                contest_id: TRUSTEES.into(),
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

/// A ballot as the tally receives it: marked in the Voting Portal, encoded
/// into the contest plaintext and decoded again.
fn cast(marks: &[&str]) -> DecodedVoteContest {
    let contest = trustees();
    let selection = DecodedVoteContest {
        contest_id: TRUSTEES.into(),
        is_explicit_invalid: false,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: contest
            .candidates
            .iter()
            .map(|candidate| DecodedVoteChoice {
                id: candidate.id.clone(),
                selected: if marks.contains(&candidate.id.as_str()) {
                    0
                } else {
                    -1
                },
                write_in_text: None,
            })
            .collect(),
    };
    let plaintext = contest.encode_plaintext_contest(&selection).unwrap();
    contest.decode_plaintext_contest(&plaintext).unwrap()
}

fn count(ballots: Vec<DecodedVoteContest>) -> ContestResult {
    PluralityAtLarge::new(Tally {
        id: CountingAlgType::PluralityAtLarge,
        scope_operation: ScopeOperation::Area(TallyOperation::ProcessBallotsAll),
        contest: trustees(),
        ballots: ballots
            .into_iter()
            .map(|ballot| (ballot, Weight::default()))
            .collect(),
        census: 10,
        auditable_votes: 0,
        tally_sheet_results: vec![],
        tally_results: vec![],
    })
    .tally()
    .unwrap()
}

fn candidate_counts(result: &ContestResult) -> BTreeMap<String, u64> {
    let counts: BTreeMap<String, u64> = result
        .candidate_result
        .iter()
        .map(|entry| (entry.candidate.id.clone(), entry.total_count))
        .collect();
    assert_eq!(counts.len(), result.candidate_result.len());
    counts
}

#[test]
fn slate_and_hand_marked_ballots_tally_identically() {
    // Two voters chose the United slate; two marked the same three trustees
    // one by one, in another order.
    let by_slate = count(vec![cast(&UNITED), cast(&UNITED)]);
    let by_hand = count(vec![
        cast(&["tr-united-3", "tr-united-1", "tr-united-2"]),
        cast(&["tr-united-2", "tr-united-3", "tr-united-1"]),
    ]);

    assert_eq!(candidate_counts(&by_slate), candidate_counts(&by_hand));
    assert_eq!(by_slate.total_votes, by_hand.total_votes);
    assert_eq!(by_slate.total_valid_votes, by_hand.total_valid_votes);
    assert_eq!(
        by_slate.extended_metrics.unwrap().votes_actually,
        by_hand.extended_metrics.unwrap().votes_actually
    );
}

#[test]
fn results_count_candidate_marks_and_list_only_candidates() {
    let result = count(vec![
        cast(&UNITED),
        // A partial slate uses two of the three trustee choices.
        cast(&VOICES),
        // A mixed ballot: one trustee from each slate and the independent.
        cast(&["tr-united-1", "tr-voices-2", INDEPENDENT]),
        cast(&[]),
    ]);

    let counts = candidate_counts(&result);
    assert_eq!(
        counts.keys().map(String::as_str).collect::<Vec<_>>(),
        trustees()
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
    );
    assert_eq!(counts["tr-united-1"], 2);
    assert_eq!(counts["tr-united-2"], 1);
    assert_eq!(counts["tr-united-3"], 1);
    assert_eq!(counts["tr-voices-1"], 1);
    assert_eq!(counts["tr-voices-2"], 2);
    assert_eq!(counts[INDEPENDENT], 1);

    // Eight marks on four ballots: choosing a slate added none.
    assert_eq!(counts.values().sum::<u64>(), 8);
    assert_eq!(result.total_votes, 4);
    assert_eq!(result.total_valid_votes, 4);
    assert_eq!(result.total_blank_votes, 1);
    let metrics = result.extended_metrics.unwrap();
    assert_eq!(metrics.votes_actually, 8);
    // One unused choice on the partial slate and three on the blank ballot.
    assert_eq!(metrics.under_votes, 4);
}
