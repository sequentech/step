// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sequent_core::ballot::{Candidate, Contest, TieBreakingPolicy, Weight};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::types::ceremonies::CountingAlgType;
use std::collections::HashMap;
use velvet::pipes::do_tally::counting_algorithm::instant_runoff::{
    BallotsStatus, RunoffResult, RunoffStatus, TieBreakingMethod,
};

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

    // Run with random policy
    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::RANDOM,
        &HashMap::new(),
    );

    // Should complete with a randomly selected winner
    match result {
        RunoffResult::Completed(state) => {
            let last_round = state.get_last_round().unwrap();
            assert!(
                last_round.winner.is_some(),
                "Should have a winner with RANDOM policy"
            );
            let winner_id = &last_round.winner.as_ref().unwrap().id;
            assert!(
                winner_id == "candidate_a"
                    || winner_id == "candidate_b"
                    || winner_id == "candidate_c",
                "Winner should be one of the tied candidates"
            );
        }
        RunoffResult::RequiresExternalInput { .. } => {
            panic!("RANDOM policy should not require external input");
        }
    }

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

    // Run with external procedure policy
    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &HashMap::new(),
    );

    // Should require external input
    match result {
        RunoffResult::RequiresExternalInput { state, tie_info } => {
            assert_eq!(tie_info.tied_candidate_ids.len(), 3);
            assert_eq!(tie_info.round_number, 1);
            assert_eq!(tie_info.vote_counts, vec![1, 1, 1]);

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
        }
        RunoffResult::Completed(_) => {
            panic!("EXTERNAL_PROCEDURE policy should pause on tie");
        }
    }

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

    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &HashMap::new(),
    );

    // Should complete normally without pausing
    match result {
        RunoffResult::Completed(state) => {
            let last_round = state.get_last_round().unwrap();
            let winner = last_round.winner.as_ref().unwrap();
            assert_eq!(winner.id, "candidate_a");
        }
        RunoffResult::RequiresExternalInput { .. } => {
            panic!("Should complete without pausing when there's a clear winner");
        }
    }

    Ok(())
}

#[test]
fn test_apply_external_tie_decision() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);

    // Create tie
    let votes = vec![
        create_vote(&["a", "b", "c"]),
        create_vote(&["b", "c", "a"]),
        create_vote(&["c", "a", "b"]),
    ];

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);

    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &HashMap::new(),
    );

    // Get the paused state
    let mut paused_state = match result {
        RunoffResult::RequiresExternalInput { state, .. } => state,
        _ => panic!("Expected paused state"),
    };

    // Apply external decision: choose candidate_a
    paused_state
        .apply_external_tie_decision("candidate_a")
        .expect("Should apply decision successfully");

    // Verify last round shows the winner
    let last_round = paused_state.get_last_round().unwrap();
    assert!(last_round.winner.is_some());
    assert_eq!(last_round.winner.as_ref().unwrap().id, "candidate_a");

    Ok(())
}

#[test]
fn test_apply_external_tie_decision_invalid_candidate() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);

    let votes = vec![create_vote(&["a", "b", "c"]), create_vote(&["b", "c", "a"])];

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);

    runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &HashMap::new(),
    );

    // Try to apply decision with invalid candidate
    let result = runoff.apply_external_tie_decision("candidate_xyz");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid candidate ID"));

    Ok(())
}

#[test]
fn test_resume_after_external_decision() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);

    // Create tie
    let votes = vec![
        create_vote(&["a", "b", "c"]),
        create_vote(&["b", "c", "a"]),
        create_vote(&["c", "a", "b"]),
    ];

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);

    // First run - pause on tie
    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &HashMap::new(),
    );

    let mut paused_state = match result {
        RunoffResult::RequiresExternalInput { state, .. } => state,
        _ => panic!("Expected pause"),
    };

    // Apply decision
    paused_state
        .apply_external_tie_decision("candidate_b")
        .map_err(|e| anyhow::anyhow!(e))?;

    // Resume - should complete now
    let result2 = paused_state.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &HashMap::new(),
    );

    match result2 {
        RunoffResult::Completed(final_state) => {
            let last_round = final_state.get_last_round().unwrap();
            let winner = last_round.winner.as_ref().unwrap();
            assert_eq!(winner.id, "candidate_b");
        }
        _ => panic!("Should complete after applying decision"),
    }

    Ok(())
}

#[test]
fn test_backward_compatibility_with_run() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    // Don't set tie_breaking_policy - should default to RANDOM

    let votes = vec![
        create_vote(&["a", "b", "c"]),
        create_vote(&["b", "c", "a"]),
        create_vote(&["c", "a", "b"]),
    ];

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);

    // Use old run() method - should work without panicking
    runoff.run(&mut ballots_status);

    // Should have a winner (randomly selected)
    let last_round = runoff.get_last_round().unwrap();
    assert!(last_round.winner.is_some());

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
    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &HashMap::new(),
    );
    match result {
        RunoffResult::RequiresExternalInput { tie_info, .. } => {
            assert_eq!(tie_info.round_number, 2, "Tie should occur at Round 2");
            assert_eq!(tie_info.tied_candidate_ids.len(), 2);
            assert!(tie_info
                .tied_candidate_ids
                .contains(&"candidate_a".to_string()));
            assert!(tie_info
                .tied_candidate_ids
                .contains(&"candidate_c".to_string()));
        }
        RunoffResult::Completed(_) => panic!("Expected pause at Round 2, got completion"),
    }

    // With resolution for Round 2: algorithm completes with A winning
    let resolutions = HashMap::from([(2u64, "candidate_a".to_string())]);
    let mut ballots_status2 = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff2 = RunoffStatus::initialize_runoff(&contest);
    let result2 = runoff2.run_with_policy(
        &mut ballots_status2,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &resolutions,
    );
    match result2 {
        RunoffResult::Completed(state) => {
            let last_round = state.get_last_round().unwrap();
            let winner = last_round.winner.as_ref().unwrap();
            assert_eq!(winner.id, "candidate_a");
        }
        RunoffResult::RequiresExternalInput { .. } => {
            panic!("Expected completion when Round 2 resolution is provided")
        }
    }

    Ok(())
}

/// A resolution that names a candidate not present in the current tie group must be
/// ignored: the algorithm falls through and returns RequiresExternalInput.
#[test]
fn test_ignored_resolution_for_non_tied_candidate() -> Result<()> {
    let mut contest = create_test_contest_3_candidates();
    contest.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);
    let votes = create_two_round_tie_votes();

    // candidate_b was already eliminated in Round 1 — not in the Round 2 tie [A, C]
    let resolutions = HashMap::from([(2u64, "candidate_b".to_string())]);

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &resolutions,
    );
    match result {
        RunoffResult::RequiresExternalInput { tie_info, .. } => {
            assert_eq!(tie_info.round_number, 2);
            assert!(
                !tie_info
                    .tied_candidate_ids
                    .contains(&"candidate_b".to_string()),
                "B should not be in the Round 2 tie"
            );
        }
        RunoffResult::Completed(_) => {
            panic!("Should have ignored the invalid resolution and paused")
        }
    }

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
    let resolutions = HashMap::from([(1u64, "candidate_a".to_string())]);

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);
    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &resolutions,
    );
    match result {
        RunoffResult::RequiresExternalInput { tie_info, .. } => {
            assert_eq!(
                tie_info.round_number, 2,
                "Round 1 resolution must not resolve the Round 2 tie"
            );
        }
        RunoffResult::Completed(_) => {
            panic!("Round 1 resolution should not be used for a Round 2 tie")
        }
    }

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
    let result = runoff.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::RANDOM,
        &HashMap::new(),
    );
    let state = match result {
        RunoffResult::Completed(s) => s,
        _ => panic!("RANDOM policy on a full tie should always complete"),
    };
    assert_eq!(
        state.tie_resolutions.len(),
        1,
        "Should record one tie resolution"
    );
    let entry = &state.tie_resolutions[0];
    assert_eq!(entry.round_number, 1);
    assert_eq!(entry.method_used, TieBreakingMethod::Random);
    assert!(
        entry.resolved_by_candidate_id.is_some(),
        "Random resolution must record a winner"
    );

    // --- EXTERNAL_PROCEDURE policy with a pre-configured resolution ---
    let mut contest2 = create_test_contest_3_candidates();
    contest2.tie_breaking_policy = Some(TieBreakingPolicy::EXTERNAL_PROCEDURE);
    let resolutions = HashMap::from([(1u64, "candidate_a".to_string())]);
    let mut ballots_status2 =
        BallotsStatus::initialize_ballots_status(&three_way_tie_votes, &contest2);
    let mut runoff2 = RunoffStatus::initialize_runoff(&contest2);
    let result2 = runoff2.run_with_policy(
        &mut ballots_status2,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
        &resolutions,
    );
    let state2 = match result2 {
        RunoffResult::Completed(s) => s,
        _ => panic!("EXTERNAL_PROCEDURE with a valid resolution should complete"),
    };
    assert_eq!(state2.tie_resolutions.len(), 1);
    let entry2 = &state2.tie_resolutions[0];
    assert_eq!(entry2.round_number, 1);
    assert_eq!(entry2.method_used, TieBreakingMethod::ExternalProcedure);
    assert_eq!(
        entry2.resolved_by_candidate_id.as_deref(),
        Some("candidate_a"),
        "Should record the externally resolved candidate"
    );

    Ok(())
}
