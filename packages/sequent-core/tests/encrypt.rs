// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the encrypt module
#[cfg(feature = "default_features")]
use sequent_core::ballot::*;
#[cfg(feature = "default_features")]
use sequent_core::ballot_codec::multi_ballot::*;
#[cfg(feature = "default_features")]
use sequent_core::encrypt::*;
#[cfg(feature = "default_features")]
use sequent_core::plaintext::DecodedVoteContest;
#[cfg(feature = "default_features")]
use sequent_core::serialization::deserialize_with_path::deserialize_value;
use serde_json::json;

#[test]
fn test_multi_contest_reencoding_with_explicit_invalid() {
    // Create test data matching the scenario with explicit invalid candidates
    let ballot_selection_json = json!([{
        "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
        "is_explicit_invalid": true,
        "invalid_errors": [],
        "invalid_alerts": [],
        "choices": [
            {
                "id": "05614f41-720a-4fd5-842f-58355c0bbdc0",
                "selected": -1
            },
            {
                "id": "dfc5a43d-2276-4859-8f76-b0f18f859e59",
                "selected": -1
            }
        ]
    }]);

    // Create a minimal ballot style for testing
    let election_json = json!({
        "id": "b48da6fd-f7e5-4868-9abb-e23452f373ad",
        "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
        "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
        "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
        "public_key": {
            "public_key": "xEH1M/iIdDkZg1ENaP7yPZWtaOcnYLTmK+sFYmuDJVk",
            "is_demo": false
        },
        "area_id": "dcaf94aa-e2f8-460b-8da6-2a7907c04664",
        "contests": [{
            "id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
            "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
            "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
            "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
            "name": "Contest",
            "max_votes": 1,
            "min_votes": 0,
            "winning_candidates_num": 1,
            "voting_type": "non-preferential",
            "counting_algorithm": "plurality-at-large",
            "is_encrypted": true,
            "candidates": [
                {
                    "id": "05614f41-720a-4fd5-842f-58355c0bbdc0",
                    "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                    "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                    "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                    "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                    "name": "Null",
                    "presentation": {
                        "is_explicit_invalid": true
                    }
                },
                {
                    "id": "dfc5a43d-2276-4859-8f76-b0f18f859e59",
                    "tenant_id": "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
                    "election_event_id": "a6de87ab-6f00-4349-b8e3-7d0471e4a211",
                    "election_id": "15d8c59d-762e-4f43-b03f-e0c31f24d076",
                    "contest_id": "bb08a9eb-49c9-44d7-a25e-b2e142e17b0a",
                    "name": "A"
                }
            ]
        }],
        "election_event_presentation": {
            "contest_encryption_policy": "multiple-contests"
        }
    });

    let decoded_multi_contests: Vec<DecodedVoteContest> =
        deserialize_value(ballot_selection_json)
            .expect("Failed to parse ballot selection");
    let ballot_style: BallotStyle =
        deserialize_value(election_json).expect("Failed to parse election");

    // This test should pass now with the fix for explicit invalid candidates
    let result =
        test_multi_contest_reencoding(&decoded_multi_contests, &ballot_style);

    assert!(
        result.is_ok(),
        "Multi-contest reencoding with explicit invalid candidate failed: {:?}",
        result.err()
    );

    // Verify the output maintains the explicit invalid flag
    let output_contests = result.unwrap();
    assert_eq!(output_contests.len(), 1);
    assert_eq!(output_contests[0].is_explicit_invalid, true);
}
