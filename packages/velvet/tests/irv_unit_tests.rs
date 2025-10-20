// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::plaintext::DecodedVoteChoice;
use std::collections::HashMap;
use velvet::pipes::do_tally::counting_algorithm::instant_runoff::*;

/// Helper function to create a test RunoffStatus
fn create_runoff_status_simple() -> RunoffStatus {
    RunoffStatus {
        candidates_status: CandidatesStatus::default(),
        round_count: 0,
        rounds: Vec::new(),
    }
}

/// Helper function to create a test RunoffStatus with specific candidates
fn create_runoff_status(active_candidate_ids: Vec<&str>) -> RunoffStatus {
    let mut candidates_status = CandidatesStatus::default();
    for id in active_candidate_ids {
        candidates_status.insert(id.to_string(), ECandidateStatus::Active);
    }
    RunoffStatus {
        candidates_status,
        round_count: 0,
        rounds: Vec::new(),
    }
}

/// Helper function to create a DecodedVoteChoice
fn create_choice(id: &str, selected: i64) -> DecodedVoteChoice {
    DecodedVoteChoice {
        id: id.to_string(),
        selected,
        write_in_text: None,
    }
}

// ============================================================================
// Tests for filter_candidates_by_number_of_wins
// ============================================================================

#[test]
fn test_filter_by_exact_number_of_wins() {
    let runoff = create_runoff_status_simple();

    // Create a map with various candidates and their win counts
    let mut candidates_wins: HashMap<String, u64> = HashMap::new();
    candidates_wins.insert("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string(), 10);
    candidates_wins.insert("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string(), 25);
    candidates_wins.insert("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string(), 10);
    candidates_wins.insert("d4e5f6a7-b8c9-4d5e-8f9a-3b4c5d6e7f8a".to_string(), 50);
    candidates_wins.insert("e5f6a7b8-c9d0-4e5f-8a9b-4c5d6e7f8a9b".to_string(), 25);

    // Filter candidates with exactly 25 wins
    let result = runoff.filter_candidates_by_number_of_wins(&candidates_wins, 25);

    // Should return the two candidates with 25 wins
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string()));
    assert!(result.contains(&"e5f6a7b8-c9d0-4e5f-8a9b-4c5d6e7f8a9b".to_string()));
}

// ============================================================================
// Tests for find_first_active_choice
// ============================================================================

#[test]
fn test_first_preference_is_active() {
    let runoff = create_runoff_status(vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d",
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e",
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f",
    ]);
    let active_candidates = vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string(),
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string(),
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string(),
    ];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0), // First preference
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 1), // Second preference
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", 2), // Third preference
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string())
    );
}

#[test]
fn test_first_preference_eliminated_returns_second() {
    let runoff = create_runoff_status(vec![
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e",
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f",
    ]);
    let active_candidates = vec![
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string(),
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string(),
    ];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0), // First preference (eliminated)
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 1), // Second preference (active)
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", 2), // Third preference (active)
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string())
    );
}

#[test]
fn test_multiple_eliminated_skips_to_first_active() {
    let runoff = create_runoff_status(vec!["c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f"]);
    let active_candidates = vec!["c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string()];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0), // First preference (eliminated)
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 1), // Second preference (eliminated)
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", 2), // Third preference (active)
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string())
    );
}

#[test]
fn test_no_active_candidates_returns_none() {
    let runoff = create_runoff_status(vec![]);
    let active_candidates = vec![];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0),
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 1),
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", 2),
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(result, None);
}

#[test]
fn test_all_choices_eliminated_returns_none() {
    let runoff = create_runoff_status(vec!["d4e5f6a7-b8c9-4d5e-8f9a-3b4c5d6e7f8a"]);
    let active_candidates = vec!["d4e5f6a7-b8c9-4d5e-8f9a-3b4c5d6e7f8a".to_string()];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0),
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 1),
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", 2),
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(result, None);
}

#[test]
fn test_empty_choices_returns_none() {
    let runoff = create_runoff_status(vec!["a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d"]);
    let active_candidates = vec!["a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string()];

    let choices = vec![];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(result, None);
}

#[test]
fn test_all_choices_unselected_returns_none() {
    let runoff = create_runoff_status(vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d",
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e",
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f",
    ]);
    let active_candidates = vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string(),
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string(),
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string(),
    ];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", -1), // Not selected
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", -1), // Not selected
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", -1), // Not selected
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(result, None);
}

#[test]
fn test_mixed_selected_and_unselected() {
    let runoff = create_runoff_status(vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d",
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e",
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f",
    ]);
    let active_candidates = vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string(),
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string(),
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string(),
    ];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", -1), // Not selected
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 0),  // First preference
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", -1), // Not selected
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string())
    );
}

#[test]
fn test_unordered_selected_values_sorted_correctly() {
    let runoff = create_runoff_status(vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d",
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e",
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f",
    ]);
    let active_candidates = vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string(),
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string(),
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string(),
    ];

    // Choices provided in random order
    let choices = vec![
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 2), // Third preference
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0), // First preference
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", 1), // Second preference
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string())
    );
}

#[test]
fn test_gap_in_selected_values() {
    let runoff = create_runoff_status(vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d",
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e",
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f",
    ]);
    let active_candidates = vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string(),
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string(),
        "c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f".to_string(),
    ];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0), // First preference
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 5), // Some later preference (gap is OK)
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", -1), // Not selected
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string())
    );
}

#[test]
fn test_only_second_preference_active() {
    let runoff = create_runoff_status(vec!["b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e"]);
    let active_candidates = vec!["b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string()];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0), // First preference (eliminated)
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 1), // Second preference (active)
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", -1), // Not selected
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string())
    );
}

#[test]
fn test_single_choice_active() {
    let runoff = create_runoff_status(vec!["a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d"]);
    let active_candidates = vec!["a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string()];

    let choices = vec![create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0)];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string())
    );
}

#[test]
fn test_single_choice_not_active() {
    let runoff = create_runoff_status(vec!["b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e"]);
    let active_candidates = vec!["b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string()];

    let choices = vec![create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0)];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(result, None);
}

#[test]
fn test_single_choice_unselected() {
    let runoff = create_runoff_status(vec!["a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d"]);
    let active_candidates = vec!["a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string()];

    let choices = vec![create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", -1)];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(result, None);
}

#[test]
fn test_many_candidates_first_active_with_highest_preference() {
    let runoff = create_runoff_status(vec![
        "e5f6a7b8-c9d0-4e5f-8a9b-4c5d6e7f8a9b",
        "f6a7b8c9-d0e1-4f5a-8b9c-5d6e7f8a9b0c",
    ]);
    let active_candidates = vec![
        "e5f6a7b8-c9d0-4e5f-8a9b-4c5d6e7f8a9b".to_string(),
        "f6a7b8c9-d0e1-4f5a-8b9c-5d6e7f8a9b0c".to_string(),
    ];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0),
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 1),
        create_choice("c3d4e5f6-a7b8-4c5d-8e9f-2a3b4c5d6e7f", 2),
        create_choice("d4e5f6a7-b8c9-4d5e-8f9a-3b4c5d6e7f8a", 3),
        create_choice("e5f6a7b8-c9d0-4e5f-8a9b-4c5d6e7f8a9b", 4), // First active candidate
        create_choice("a6b7c8d9-e0f1-4a6b-8c9d-5e6f7a8b9c0d", 5),
        create_choice("b7c8d9e0-f1a2-4b7c-8d9e-6f7a8b9c0d1e", 6),
        create_choice("c8d9e0f1-a2b3-4c8d-8e9f-7a8b9c0d1e2f", 7),
        create_choice("d9e0f1a2-b3c4-4d9e-8f9a-8b9c0d1e2f3a", 8),
        create_choice("f6a7b8c9-d0e1-4f5a-8b9c-5d6e7f8a9b0c", 9), // Second active candidate
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    assert_eq!(
        result,
        Some("e5f6a7b8-c9d0-4e5f-8a9b-4c5d6e7f8a9b".to_string())
    );
}

#[test]
fn test_duplicate_selected_values() {
    // Although this shouldn't happen in practice, the function should handle it
    let runoff = create_runoff_status(vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d",
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e",
    ]);
    let active_candidates = vec![
        "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string(),
        "b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string(),
    ];

    let choices = vec![
        create_choice("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d", 0),
        create_choice("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e", 0), // Same selected value
    ];

    let result = runoff.find_first_active_choice(&choices, &active_candidates);
    // Should return one of them (order depends on sort stability)
    assert!(
        result == Some("a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d".to_string())
            || result == Some("b2c3d4e5-f6a7-4b5c-8d9e-1f2a3b4c5d6e".to_string())
    );
}
