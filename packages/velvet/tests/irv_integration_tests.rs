// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Integration tests for Instant Runoff Voting algorithm
// These tests verify that multiple components work together correctly

use std::collections::HashMap;
use velvet::pipes::do_tally::counting_algorithm::instant_runoff::*;

/// Helper function to create a candidate UUID with a specific suffix
fn candidate_id(suffix: &str) -> String {
    let prefix = match suffix.chars().next().unwrap_or('a') {
        'a' => "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4",
        'b' => "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5",
        'c' => "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6",
        'd' => "d4e5f6a7-b8c9-4d5e-8f9a-3b4c5d6e7",
        'e' => "e5f6a7b8-c9d0-4e5f-8a9b-4c5d6e7f8",
        'f' => "f6a7b8c9-d0e1-4f5a-8b9c-5d6e7f8a9",
        'g' => "a6b7c8d9-e0f1-4a6b-8c9d-5e6f7a8b9",
        'h' => "b7c8d9e0-f1a2-4b7c-8d9e-6f7a8b9c0",
        'i' => "c8d9e0f1-a2b3-4c8d-8e9f-7a8b9c0d1",
        'j' => "d9e0f1a2-b3c4-4d9e-8f9a-8b9c0d1e2",
        'k' => "e0f1a2b3-c4d5-4e0f-8a9b-9c0d1e2f3",
        'l' => "f1a2b3c4-d5e6-4f1a-8b9c-0d1e2f3a4",
        'm' => "a2b3c4d5-e6f7-4a2b-8c9d-1e2f3a4b5",
        'n' => "b3c4d5e6-f7a8-4b3c-8d9e-2f3a4b5c6",
        _ => "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4",
    };
    format!("{}{}", prefix, suffix)
}

/// Helper function to create a HashMap of candidate wins from a slice of (suffix, votes) pairs
fn create_wins(candidates_votes: &[(&str, u64)]) -> HashMap<String, u64> {
    candidates_votes
        .iter()
        .map(|(suffix, votes)| (candidate_id(suffix), *votes))
        .collect()
}

/// Helper function to create a Round with specific candidate wins
fn create_round(candidates_wins: HashMap<String, u64>, active_count: u64) -> Round {
    Round {
        winner: None,
        candidates_wins,
        eliminated_candidates: None,
        active_count,
    }
}

// ============================================================================
// Integration Tests for find_single_candidate_to_eliminate
// ============================================================================

#[test]
fn test_tie_breaking_using_previous_round() {
    // Setup: Create a realistic IRV scenario with multiple rounds
    //
    // Round 1 (initial round):
    //   - Candidate A: 35 votes
    //   - Candidate B: 40 votes
    //   - Candidate C: 50 votes
    //   - Candidate D: 25 votes (eliminated - had fewest votes)
    //
    // Round 2 (current round after D was eliminated and votes redistributed):
    //   - Candidate A: 40 votes (gained 5 from D)
    //   - Candidate B: 40 votes (stayed the same)
    //   - Candidate C: 70 votes (gained 20 from D)
    //   - TIE between A and B!
    //
    // Expected: The algorithm should look back to Round 1 to break the tie.
    // In Round 1, A had 35 votes and B had 40 votes, so A should be eliminated.

    let round1_wins = create_wins(&[("caa", 35), ("cab", 40), ("cac", 50), ("cad", 25)]);
    let round2_wins = create_wins(&[("caa", 40), ("cab", 40), ("cac", 70)]);

    let runoff = RunoffStatus {
        candidates_status: CandidatesStatus::default(),
        round_count: 2,
        rounds: vec![
            create_round(round1_wins, 4), // Round 1 had 4 active candidates
            create_round(round2_wins, 3), // Round 2 has 3 active candidates (D was eliminated)
        ],
    };

    // The candidates tied in the current round (Round 2)
    let candidates_to_eliminate = vec![candidate_id("caa"), candidate_id("cab")];

    let result = runoff.find_single_candidate_to_eliminate(&candidates_to_eliminate);

    // Expected: Should return only candidate A (who had fewer votes in Round 1)
    assert_eq!(result.len(), 1);
    assert_eq!(result.first(), Some(&candidate_id("caa")));
}

#[test]
fn test_tie_persists_through_lookback() {
    // Setup: Create a scenario where tie-breaking lookback fails to resolve the tie
    //
    // Round 1 (14 candidates):
    //   - Candidates A, B, C: 10 votes each (3-way TIE for lowest) - ELIMINATED
    //   - Candidates D, E, F: 15 votes each
    //   - Candidates G, H, I: 20 votes each
    //   - Candidates J, K, L: 25 votes each
    //   - Candidates M, N: 30 votes each
    //
    // Round 2 (11 candidates, A/B/C eliminated):
    //   - Candidates D, E, F: 18 votes each (now tied for lowest) - ELIMINATED
    //   - Candidates G, H, I: 25 votes each
    //   - Candidates J, K, L: 32 votes each
    //   - Candidates M, N: 38 votes each
    //
    // Round 3 (8 candidates, D/E/F eliminated):
    //   - Candidates G, H: 30 votes each (tied for lowest)
    //   - Candidate I: 35 votes
    //   - Candidates J, K, L: 40 votes each
    //   - Candidates M, N: 45 votes each
    //
    // Expected: In Round 3, G and H are tied. Look back to Round 2 - both weren't
    // in the elimination set. Look back to Round 1 - both had 20 votes (tied).
    // Since the tie persists back to Round 1, return both

    // Round 1: A, B, C tied for lowest (eliminated)
    let round1_wins = create_wins(&[
        ("caa", 10),
        ("cab", 10),
        ("cac", 10), // Tied for lowest - eliminated
        ("cad", 15),
        ("cae", 15),
        ("caf", 15),
        ("cag", 20),
        ("cah", 20),
        ("cai", 20),
        ("caj", 25),
        ("cak", 25),
        ("cal", 25),
        ("cam", 30),
        ("can", 30),
    ]);

    // Round 2: D, E, F now tied for lowest (A, B, C already eliminated)
    let round2_wins = create_wins(&[
        ("cad", 18),
        ("cae", 18),
        ("caf", 18), // Tied for lowest - eliminated
        ("cag", 25),
        ("cah", 25),
        ("cai", 25),
        ("caj", 32),
        ("cak", 32),
        ("cal", 32),
        ("cam", 38),
        ("can", 38),
    ]);

    // Round 3: I is the lowest (A-F already eliminated)
    let round3_wins = create_wins(&[
        ("cag", 30),
        ("cah", 30), // Next to be eliminated on the next round - tied for lowest
        ("cai", 25), // To be eliminated
        ("caj", 40),
        ("cak", 40),
        ("cal", 40),
        ("cam", 45),
        ("can", 45),
    ]);

    let runoff = RunoffStatus {
        candidates_status: CandidatesStatus::default(),
        round_count: 3,
        rounds: vec![
            create_round(round1_wins, 14), // Round 1: 14 active candidates
            create_round(round2_wins, 11), // Round 2: 11 active (A, B, C eliminated)
            create_round(round3_wins, 8),  // Round 3: 8 active (D, E, F also eliminated)
        ],
    };

    // When round3 is processed, I is eliminated.
    // Then G and H are tied for the lowest
    // The 2 candidates tied in Round 3 persist tied through the lookback
    let candidates_to_eliminate = vec![candidate_id("cag"), candidate_id("cah")];

    // Call the function under test
    let result = runoff.find_single_candidate_to_eliminate(&candidates_to_eliminate);

    // Expected: Algorithm looks back through rounds 3, 2, 1
    // Since the tie persists back to Round 1, return both
    assert_eq!(result.len(), 2);
    assert!(result.contains(&candidate_id("cag")));
    assert!(result.contains(&candidate_id("cah")));
}

// ============================================================================
// Integration Tests for do_round_eliminations
// ============================================================================

#[test]
fn test_do_round_eliminations_with_tie_resolution() {
    // Setup: Create a simple IRV scenario with 6 candidates over 3 rounds
    //
    // Round 1 (6 candidates):
    //   - Candidate A: 10 votes (lowest) - ELIMINATED
    //   - Candidate B: 15 votes
    //   - Candidate C: 20 votes
    //   - Candidate D: 25 votes
    //   - Candidate E: 30 votes
    //   - Candidate F: 40 votes
    //
    // Round 2 (5 candidates, A eliminated):
    //   - Candidate B: 18 votes
    //   - Candidate C: 22 votes (lowest among remaining) - ELIMINATED
    //   - Candidate D: 28 votes
    //   - Candidate E: 32 votes
    //   - Candidate F: 50 votes
    //
    // Round 3 (4 candidates, A and C eliminated):
    //   - Candidate B: 25 votes
    //   - Candidate D: 30 votes
    //   - Candidate E: 35 votes
    //   - Candidate F: 60 votes
    //
    // Current state: Candidates B and D both have similar votes (tied for lowest)
    //
    // Expected: Look back to Round 2 where B had 18 votes and D had 28 votes.
    // Eliminate candidate B only.

    // Round 1: A has lowest (eliminated)
    let round1_wins = create_wins(&[
        ("caa", 10),
        ("cab", 15),
        ("cac", 20),
        ("cad", 25),
        ("cae", 30),
        ("caf", 40),
    ]);

    // Round 2: C has lowest among remaining (eliminated)
    let round2_wins = create_wins(&[
        ("cab", 18),
        ("cac", 22),
        ("cad", 28),
        ("cae", 32),
        ("caf", 50),
    ]);

    // Round 3: All remaining candidates
    let round3_wins = create_wins(&[
        ("cab", 25),
        ("cad", 30),
        ("cae", 20),
        ("caf", 60), // E will be eliminated before processing the next round
    ]);

    let mut runoff = RunoffStatus {
        candidates_status: CandidatesStatus::default(),
        round_count: 3,
        rounds: vec![
            create_round(round1_wins, 6), // Round 1: 6 active candidates
            create_round(round2_wins, 5), // Round 2: 5 active (A eliminated)
            create_round(round3_wins, 4), // Round 3: 4 active (A, C eliminated)
        ],
    };

    // Initialize remaining candidates as active
    for suffix in ["cab", "cad", "caf"] {
        runoff
            .candidates_status
            .insert(candidate_id(suffix), ECandidateStatus::Active);
    }

    // Current round wins: B and D tied for lowest
    let current_wins = create_wins(&[
        ("cab", 30),
        ("cad", 30), // B and D tied for lowest
        ("caf", 70),
    ]);

    // Candidates tied for lowest in current state
    let candidates_to_eliminate = vec![candidate_id("cab"), candidate_id("cad")];

    // Call the function under test - this is an integration test because it calls
    // find_single_candidate_to_eliminate internally (not mocked)
    let result = runoff.do_round_eliminations(&current_wins, &candidates_to_eliminate);

    // Expected: Should successfully eliminate candidate B (who had fewer in Round 2)
    assert!(result.is_some());
    assert_eq!(result, Some(candidate_id("cab")));

    // Verify candidate B's status was updated to Eliminated
    assert_eq!(
        runoff.candidates_status.get(&candidate_id("cab")),
        Some(&ECandidateStatus::Eliminated)
    );

    // Verify candidate D is still Active
    assert_eq!(
        runoff.candidates_status.get(&candidate_id("cad")),
        Some(&ECandidateStatus::Active)
    );
}

#[test]
fn test_do_round_eliminations_unbreakable_tie_random_removal() {
    // Setup: Create a scenario where all remaining candidates are tied throughout all rounds
    //
    // Round 1 (4 candidates):
    //   - Candidate A: 25 votes
    //   - Candidates B, C, D: 20 votes each (3-way TIE for lowest) - Should be eliminated but tie persists
    //
    // Current state: 3 candidates, 1 eliminated randomly.
    //
    // Expected: Since all active candidates are tied and lookback doesn't break the tie,
    // do_round_eliminations should remove a random candidate.

    // Round 1: B, C, D tied for lowest
    let round1_wins = create_wins(&[
        ("caa", 25),
        ("cab", 20),
        ("cac", 20),
        ("cad", 20), // 3-way tie
    ]);

    let mut runoff = RunoffStatus {
        candidates_status: CandidatesStatus::default(),
        round_count: 1,
        rounds: vec![
            create_round(round1_wins, 4), // Round 1: 4 active candidates
        ],
    };

    // Initialize all candidates as active
    for suffix in ["caa", "cab", "cac"] {
        runoff
            .candidates_status
            .insert(candidate_id(suffix), ECandidateStatus::Active);
    }

    // Current round wins: 2 last candidates tied
    let current_wins = create_wins(&[("caa", 45), ("cab", 20), ("cac", 20)]);

    // Candidates tied for lowest
    let candidates_to_eliminate = vec![candidate_id("cab"), candidate_id("cac")];

    // Call the function under test
    let result = runoff.do_round_eliminations(&current_wins, &candidates_to_eliminate);

    // Expected: Should return None because all active candidates are tied
    // (active_count == reduced_list.len() triggers the tie condition)
    assert!(result.is_some());

    // Verify that caa is Active
    assert_eq!(
        runoff.candidates_status.get(&candidate_id("caa")),
        Some(&ECandidateStatus::Active)
    );
    // The Other 2 candidates have different status, because one was eliminated randomly
    let st_cab = runoff.candidates_status.get(&candidate_id("cab"));
    let st_cac = runoff.candidates_status.get(&candidate_id("cac"));
    assert_ne!(st_cab, st_cac);
}
