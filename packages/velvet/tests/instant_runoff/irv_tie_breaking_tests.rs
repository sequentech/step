// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sequent_core::ballot::{Candidate, Contest, TieBreakingPolicy, Weight};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::CountingAlgType;
use sequent_core::types::ceremonies::{TallySessionResolutionData, TieBreakingMethod};
use velvet::pipes::do_tally::counting_algorithm::instant_runoff::{BallotsStatus, RunoffStatus};

/// Helper: Create a simple 3-candidate contest
fn create_test_contest_3_candidates() -> Contest {
    Contest {
        id: "contest1".to_string(),
        tenant_id: "tenant1".to_string(),
        election_event_id: "event1".to_string(),
        election_id: "election1".to_string(),
        name: Some("Test Contest".to_string()),
        name_i18n: None,
        description: None,
        description_i18n: None,
        alias: Some("test-contest".to_string()),
        alias_i18n: None,
        max_votes: 1,
        min_votes: 1,
        winning_candidates_num: 1,
        voting_type: Some("instant-runoff".to_string()),
        counting_algorithm: Some(CountingAlgType::InstantRunoff),
        is_encrypted: false,
        candidates: vec![
            Candidate {
                id: "candidate_a".to_string(),
                name: Some("Candidate A".to_string()),
                ..Default::default()
            },
            Candidate {
                id: "candidate_b".to_string(),
                name: Some("Candidate B".to_string()),
                ..Default::default()
            },
            Candidate {
                id: "candidate_c".to_string(),
                name: Some("Candidate C".to_string()),
                ..Default::default()
            },
        ],
        presentation: None,
        created_at: None,
        annotations: None,
        tie_breaking_policy: None, // Will be set per test
    }
}

/// Helper: Create a vote with preference order
fn create_vote(preferences: &[&str]) -> (DecodedVoteContest, Weight) {
    let choices: Vec<DecodedVoteChoice> = preferences
        .iter()
        .enumerate()
        .map(|(i, id)| DecodedVoteChoice {
            id: format!("candidate_{}", id),
            selected: i as i64,
            write_in_text: None,
        })
        .collect();

    (
        DecodedVoteContest {
            contest_id: "contest1".to_string(),
            choices,
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            invalid_errors: vec![],
            invalid_alerts: vec![],
        },
        Weight::default(),
    )
}

#[test]
fn test_tie_breaking_policy_default_is_random() {
    let contest = create_test_contest_3_candidates();
    assert_eq!(contest.get_tie_breaking_policy(), TieBreakingPolicy::RANDOM);
}

#[test]
fn test_full_tie_with_random_policy_completes() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::RANDOM);

    // Create perfect 3-way tie: each candidate gets 1 first-place vote
    let votes = vec![
        create_vote(&["a", "b", "c"]),
        create_vote(&["b", "c", "a"]),
        create_vote(&["c", "a", "b"]),
    ];

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    runoff.run(&mut ballots_status);

    // Should complete with a randomly selected winner
    assert!(
        runoff.pending_tie_resolution.is_none(),
        "RANDOM policy should not require external input"
    );
    let last_round = runoff.get_last_round().unwrap();
    assert!(
        last_round.winner.is_some(),
        "Should have a winner with RANDOM policy"
    );
    let winner_id = &last_round.winner.as_ref().unwrap().id;
    assert!(
        winner_id == "candidate_a" || winner_id == "candidate_b" || winner_id == "candidate_c",
        "Winner should be one of the tied candidates"
    );

    Ok(())
}

#[test]
fn test_full_tie_with_external_policy_pauses() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);

    // Create perfect 3-way tie
    let votes = vec![
        create_vote(&["a", "b", "c"]),
        create_vote(&["b", "c", "a"]),
        create_vote(&["c", "a", "b"]),
    ];

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    runoff.run(&mut ballots_status);

    // Should require external input
    let tie_info = runoff
        .pending_tie_resolution
        .expect("EXTERNAL_PROCEDURE policy should pause on tie");
    assert_eq!(tie_info.tied_candidate_ids.len(), 3);
    assert_eq!(tie_info.round_number, Some(1));
    assert_eq!(tie_info.vote_count, 1);

    // Verify all candidates are tied
    assert!(tie_info
        .tied_candidate_ids
        .contains(&"candidate_a".to_string()));
    assert!(tie_info
        .tied_candidate_ids
        .contains(&"candidate_b".to_string()));
    assert!(tie_info
        .tied_candidate_ids
        .contains(&"candidate_c".to_string()));

    Ok(())
}

#[test]
fn test_no_tie_with_external_policy_completes() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);

    // Candidate A gets 2 first-place votes (clear winner)
    let votes = vec![
        create_vote(&["a", "b", "c"]),
        create_vote(&["a", "c", "b"]),
        create_vote(&["b", "c", "a"]),
    ];

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    runoff.run(&mut ballots_status);

    // Should complete normally without pausing
    assert!(
        runoff.pending_tie_resolution.is_none(),
        "Should complete without pausing when there's a clear winner"
    );
    let last_round = runoff.get_last_round().unwrap();
    let winner = last_round.winner.as_ref().unwrap();
    assert_eq!(winner.id, "candidate_a");

    Ok(())
}

/// Ballot helper: builds an 8-vote set where B is eliminated in Round 1 (3 vs 3 vs 2 votes)
/// and the redistributed B-votes split equally, creating an A/C tie in Round 2.
fn create_two_round_tie_votes() -> Vec<(DecodedVoteContest, Weight)> {
    vec![
        create_vote(&["a", "c", "b"]),
        create_vote(&["a", "c", "b"]),
        create_vote(&["a", "c", "b"]),
        create_vote(&["c", "a", "b"]),
        create_vote(&["c", "a", "b"]),
        create_vote(&["c", "a", "b"]),
        create_vote(&["b", "a", "c"]), // B eliminated → redistributed to A
        create_vote(&["b", "c", "a"]), // B eliminated → redistributed to C
    ]
}

/// Round 1: A=3, B=2, C=3 → B is eliminated (no tie needed).
/// Round 2: A=4, C=4 (B's two votes split one each) → full tie requiring external input.
/// Asserts the algorithm pauses at Round 2 without a resolution, and completes
/// with A winning when a round-2 resolution is provided.
#[test]
fn test_multi_round_tie_with_external_policy() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);
    let votes = create_two_round_tie_votes();

    // Without any resolution: algorithm should pause at Round 2
    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    runoff.run(&mut ballots_status);
    let tie_info = runoff
        .pending_tie_resolution
        .expect("Expected pause at Round 2, got completion");
    assert_eq!(
        tie_info.round_number,
        Some(2),
        "Tie should occur at Round 2"
    );
    assert_eq!(tie_info.tied_candidate_ids.len(), 2);
    assert!(tie_info
        .tied_candidate_ids
        .contains(&"candidate_a".to_string()));
    assert!(tie_info
        .tied_candidate_ids
        .contains(&"candidate_c".to_string()));

    // With resolution for Round 2: algorithm completes with A winning
    let mut ballots_status2 = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff2 = RunoffStatus::initialize_runoff(&contest);
    runoff2.tie_resolutions.push(TallySessionResolutionData {
        round_number: Some(2),
        tied_candidate_ids: vec!["candidate_a".to_string(), "candidate_c".to_string()],
        vote_count: 0,
        method_used: TieBreakingMethod::ExternalProcedure,
        resolved_by_candidate_id: Some("candidate_a".to_string()),
    });
    runoff2.run(&mut ballots_status2);
    assert!(
        runoff2.pending_tie_resolution.is_none(),
        "Expected completion when Round 2 resolution is provided"
    );
    let last_round = runoff2.get_last_round().unwrap();
    let winner = last_round.winner.as_ref().unwrap();
    assert_eq!(winner.id, "candidate_a");

    Ok(())
}

/// A resolution that names a candidate not present in the current tie group must be
/// ignored: the algorithm falls through and returns with a pending tie.
#[test]
fn test_ignored_resolution_for_non_tied_candidate() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);
    let votes = create_two_round_tie_votes();

    // candidate_b was already eliminated in Round 1 — not in the Round 2 tie [A, C]
    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    // tied_candidate_ids references B, which is not in the actual Round 2 tie [A, C],
    // so the resolution does not match and must be ignored.
    runoff.tie_resolutions.push(TallySessionResolutionData {
        round_number: Some(2),
        tied_candidate_ids: vec!["candidate_a".to_string(), "candidate_b".to_string()],
        vote_count: 0,
        method_used: TieBreakingMethod::ExternalProcedure,
        resolved_by_candidate_id: Some("candidate_b".to_string()),
    });
    runoff.run(&mut ballots_status);
    let tie_info = runoff
        .pending_tie_resolution
        .expect("Should have ignored the invalid resolution and paused");
    assert_eq!(tie_info.round_number, Some(2));
    assert!(
        !tie_info
            .tied_candidate_ids
            .contains(&"candidate_b".to_string()),
        "B should not be in the Round 2 tie"
    );

    Ok(())
}

/// A resolution keyed to Round 1 must not be applied to a Round 2 tie.
/// The algorithm should pause at Round 2 as if no resolution was provided.
#[test]
fn test_ignored_resolution_for_wrong_round() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);
    let votes = create_two_round_tie_votes();

    // Resolution exists only for Round 1; the actual tie is in Round 2
    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    runoff.tie_resolutions.push(TallySessionResolutionData {
        round_number: Some(1),
        tied_candidate_ids: vec!["candidate_a".to_string(), "candidate_c".to_string()],
        vote_count: 0,
        method_used: TieBreakingMethod::ExternalProcedure,
        resolved_by_candidate_id: Some("candidate_a".to_string()),
    });
    runoff.run(&mut ballots_status);
    let tie_info = runoff
        .pending_tie_resolution
        .expect("Round 1 resolution should not be used for a Round 2 tie");
    assert_eq!(
        tie_info.round_number,
        Some(2),
        "Round 1 resolution must not resolve the Round 2 tie"
    );

    Ok(())
}

/// Verifies that the RunoffStatus.tie_resolutions history is accurately populated
/// for both RANDOM and EXTERNAL_PROCEDURE tie-breaking methods.
#[test]
fn test_tie_breaking_state_history_recorded() -> Result<()> {
    // Three-way full tie: every candidate gets exactly 1 first-preference vote.
    let three_way_tie_votes = vec![
        create_vote(&["a", "b", "c"]),
        create_vote(&["b", "c", "a"]),
        create_vote(&["c", "a", "b"]),
    ];

    // --- RANDOM policy ---
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::RANDOM);
    let mut ballots_status =
        BallotsStatus::initialize_ballots_status(&three_way_tie_votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    runoff.run(&mut ballots_status);
    assert!(
        runoff.pending_tie_resolution.is_none(),
        "RANDOM policy on a full tie should always complete"
    );
    assert_eq!(
        runoff.tie_resolutions.len(),
        1,
        "Should record one tie resolution"
    );
    let entry = &runoff.tie_resolutions[0];
    assert_eq!(entry.round_number, Some(1));
    assert_eq!(entry.method_used, TieBreakingMethod::Random);
    assert!(
        entry.resolved_by_candidate_id.is_some(),
        "Random resolution must record a winner"
    );

    // --- EXTERNAL_PROCEDURE policy with a pre-configured resolution ---
    let mut contest2 = create_test_contest_3_candidates();
    contest2.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);
    let mut ballots_status2 =
        BallotsStatus::initialize_ballots_status(&three_way_tie_votes, &contest2);
    let mut runoff2 = RunoffStatus::initialize_runoff(&contest2);
    runoff2.tie_resolutions.push(TallySessionResolutionData {
        round_number: Some(1),
        tied_candidate_ids: vec![
            "candidate_a".to_string(),
            "candidate_b".to_string(),
            "candidate_c".to_string(),
        ],
        vote_count: 0,
        method_used: TieBreakingMethod::ExternalProcedure,
        resolved_by_candidate_id: Some("candidate_a".to_string()),
    });
    runoff2.run(&mut ballots_status2);
    assert!(
        runoff2.pending_tie_resolution.is_none(),
        "EXTERNAL_PROCEDURE with a valid resolution should complete"
    );
    assert_eq!(runoff2.tie_resolutions.len(), 1);
    let entry2 = &runoff2.tie_resolutions[0];
    assert_eq!(entry2.round_number, Some(1));
    assert_eq!(entry2.method_used, TieBreakingMethod::ExternalProcedure);
    assert_eq!(
        entry2.resolved_by_candidate_id.as_deref(),
        Some("candidate_a"),
        "Should record the externally resolved candidate"
    );

    Ok(())
}
