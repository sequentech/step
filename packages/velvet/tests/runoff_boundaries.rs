// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Small round histories make transfers and operator tie resolutions auditable.

mod support;

use sequent_core::types::ceremonies::{TallySessionResolutionData, TieBreakingMethod};
use std::collections::HashMap;
use velvet::pipes::do_tally::counting_algorithm::instant_runoff::{
    CandidateOutcome, ECandidateStatus, Round, RunoffStatus,
};

fn outcomes(counts: &[(&str, u64)]) -> HashMap<String, CandidateOutcome> {
    counts
        .iter()
        .map(|(id, wins)| {
            (
                (*id).into(),
                CandidateOutcome {
                    wins: *wins,
                    ..Default::default()
                },
            )
        })
        .collect()
}

fn runoff() -> RunoffStatus {
    RunoffStatus::initialize_runoff(&support::contest())
}

fn resolution() -> TallySessionResolutionData {
    TallySessionResolutionData {
        round_number: Some(1),
        // The operator can return the tied candidates in either order.
        tied_candidate_ids: vec!["bea".into(), "ada".into()],
        vote_count: 4,
        method_used: TieBreakingMethod::ExternalProcedure,
        resolved_by_candidate_id: Some("bea".into()),
    }
}

#[test]
fn transfers_are_signed_changes_from_the_latest_round_and_preserve_counts() {
    let mut status = runoff();
    let current = outcomes(&[("ada", 6), ("bea", 0), ("cam", 2)]);
    assert!(status
        .calculate_transferences(&current)
        .values()
        .all(|v| v.transference == 0));
    status.rounds.push(Round {
        candidates_wins: outcomes(&[("ada", 1), ("bea", 7)]),
        ..Default::default()
    });
    status.rounds.push(Round {
        candidates_wins: outcomes(&[("ada", 4), ("bea", 3)]),
        ..Default::default()
    });
    let transferred = status.calculate_transferences(&current);
    for (id, count, delta) in [("ada", 6, 2), ("bea", 0, -3), ("cam", 2, 2)] {
        assert_eq!(transferred[id].wins, count);
        assert_eq!(transferred[id].transference, delta);
        assert_eq!(
            current[id].transference, 0,
            "do not rewrite the input round"
        );
    }
}

#[test]
fn lookback_uses_the_most_recent_decisive_round_and_only_the_tied_candidates() {
    let mut status = runoff();
    let tied = vec!["ada".into(), "bea".into()];
    assert_eq!(status.find_single_candidate_to_eliminate(&tied), tied);
    for counts in [
        vec![("ada", 1), ("bea", 5)],
        vec![("ada", 4), ("bea", 2), ("cam", 0)],
        vec![("ada", 4), ("bea", 4)],
    ] {
        status.rounds.push(Round {
            candidates_wins: outcomes(&counts),
            ..Default::default()
        });
    }
    assert_eq!(status.find_single_candidate_to_eliminate(&tied), ["bea"]);
}

#[test]
fn matching_external_resolution_elects_only_its_winner_and_eliminates_the_other_tie() {
    let mut status = runoff();
    status.tie_resolutions.push(resolution());
    let (winner, eliminated) = status
        .determine_winner_by_external_procedure(
            &vec!["ada".into(), "bea".into()],
            &outcomes(&[("ada", 4), ("bea", 4)]),
        )
        .unwrap();
    assert_eq!((winner.id.as_str(), winner.name.as_str()), ("bea", "Bea"));
    assert_eq!(
        eliminated.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ["ada"]
    );
    assert_eq!(
        status.candidates_status["ada"],
        ECandidateStatus::Eliminated
    );
    assert_eq!(status.candidates_status["bea"], ECandidateStatus::Active);
    assert_eq!(status.candidates_status["cam"], ECandidateStatus::Active);
    assert!(status.pending_tie_resolution.is_none());
}

fn assert_unresolved(invalid: TallySessionResolutionData) {
    let mut status = runoff();
    status.tie_resolutions.push(invalid);
    let tied = vec!["ada".into(), "bea".into()];
    assert!(status
        .determine_winner_by_external_procedure(&tied, &outcomes(&[("ada", 4), ("bea", 4)]),)
        .is_none());
    assert!(status
        .candidates_status
        .values()
        .all(|s| *s == ECandidateStatus::Active));
    let pending = status.pending_tie_resolution.unwrap();
    assert_eq!(pending.round_number, Some(1));
    assert_eq!(pending.tied_candidate_ids, tied);
    assert_eq!(pending.vote_count, 4);
    assert!(pending.resolved_by_candidate_id.is_none());
}

#[test]
fn an_external_resolution_for_another_round_or_candidate_set_stays_pending() {
    let mut wrong_round = resolution();
    wrong_round.round_number = Some(2);
    assert_unresolved(wrong_round);
    let mut wrong_set = resolution();
    wrong_set.tied_candidate_ids = vec!["ada".into(), "cam".into()];
    assert_unresolved(wrong_set);
    let mut incomplete = resolution();
    incomplete.resolved_by_candidate_id = None;
    assert_unresolved(incomplete);
}

#[test]
fn duplicate_candidate_ids_cannot_impersonate_the_complete_tied_set() {
    let mut invalid = resolution();
    invalid.tied_candidate_ids = vec!["ada".into(), "ada".into()];
    assert_unresolved(invalid);
}

#[test]
fn an_external_resolution_cannot_elect_a_candidate_outside_the_tie() {
    let mut invalid = resolution();
    invalid.resolved_by_candidate_id = Some("cam".into());
    assert_unresolved(invalid);
}
