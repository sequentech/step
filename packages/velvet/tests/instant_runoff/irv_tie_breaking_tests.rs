// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use sequent_core::ballot::{Candidate, Contest, TieBreakingPolicy};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use velvet::pipes::do_tally::counting_algorithm::instant_runoff::{
    BallotsStatus, RunoffResult, RunoffStatus,
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
        counting_algorithm: Some(sequent_core::ballot::CountingAlgType::INSTANT_RUNOFF),
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
fn create_vote(preferences: &[&str]) -> (DecodedVoteContest, Option<u64>) {
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
            choices,
            is_explicit_invalid: false,
        },
        Some(1), // weight
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
    let result = runoff.run_with_policy(&mut ballots_status, &TieBreakingPolicy::RANDOM);

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
                winner_id == "candidate_a" || winner_id == "candidate_b" || winner_id == "candidate_c",
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
    );

    // Should require external input
    match result {
        RunoffResult::RequiresExternalInput { state, tie_info } => {
            assert_eq!(tie_info.tied_candidate_ids.len(), 3);
            assert_eq!(tie_info.round_number, 1);
            assert_eq!(tie_info.vote_counts, vec![1, 1, 1]);

            // Verify all candidates are tied
            assert!(tie_info.tied_candidate_ids.contains(&"candidate_a".to_string()));
            assert!(tie_info.tied_candidate_ids.contains(&"candidate_b".to_string()));
            assert!(tie_info.tied_candidate_ids.contains(&"candidate_c".to_string()));
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
    );

    // Should complete normally without pausing
    match result {
        RunoffResult::Completed(state) => {
            let winner = state.get_last_round().unwrap().winner.as_ref().unwrap();
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

    // Verify only candidate_a is active
    let active_candidates = paused_state.candidates_status.get_active_candidate_ids();
    assert_eq!(active_candidates.len(), 1);
    assert_eq!(active_candidates[0], "candidate_a");

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

    let votes = vec![
        create_vote(&["a", "b", "c"]),
        create_vote(&["b", "c", "a"]),
    ];

    let mut ballots_status = BallotsStatus::initialize_ballots_status(&votes, &contest);
    let mut runoff = RunoffStatus::initialize_runoff(&contest);

    runoff.run_with_policy(&mut ballots_status, &TieBreakingPolicy::EXTERNAL_PROCEDURE);

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
    );

    let mut paused_state = match result {
        RunoffResult::RequiresExternalInput { state, .. } => state,
        _ => panic!("Expected pause"),
    };

    // Apply decision
    paused_state.apply_external_tie_decision("candidate_b")?;

    // Resume - should complete now
    let result2 = paused_state.run_with_policy(
        &mut ballots_status,
        &TieBreakingPolicy::EXTERNAL_PROCEDURE,
    );

    match result2 {
        RunoffResult::Completed(final_state) => {
            let winner = final_state.get_last_round().unwrap().winner.as_ref().unwrap();
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
