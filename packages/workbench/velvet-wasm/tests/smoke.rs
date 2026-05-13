// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! End-to-end WASM smoke test for the workbench pipeline:
//! `Contest` JSON + plaintext ballot strings → tally → `ContestResult`.
//!
//! Run with `wasm-pack test --node -p velvet-wasm`. This compiles to
//! `wasm32-unknown-unknown` and executes inside Node's wasm runtime —
//! no browser needed for the smoke check.

use sequent_core::ballot::Contest;
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::fixtures::ballot_codec::get_test_contest;
use velvet_wasm::tally_plaintext_ballots;
use wasm_bindgen_test::*;

// wasm-bindgen-test runs in Node by default; no configure needed.

/// Encode a `DecodedVoteContest` (with the given candidate ids selected
/// in preference order) into the decimal-`BigUint` string format that
/// the ballot decoder expects.
fn encode_selection(contest: &Contest, selected_candidate_ids: &[&str]) -> String {
    use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};

    let choices: Vec<DecodedVoteChoice> = contest
        .candidates
        .iter()
        .map(|c| {
            let selected = selected_candidate_ids
                .iter()
                .position(|id| *id == c.id)
                .map(|i| i as i64)
                .unwrap_or(-1);
            DecodedVoteChoice {
                id: c.id.clone(),
                selected,
                write_in_text: None,
            }
        })
        .collect();

    let decoded = DecodedVoteContest {
        contest_id: contest.id.clone(),
        choices,
        is_explicit_invalid: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
    };

    contest
        .encode_plaintext_contest_bigint(&decoded)
        .expect("encoding decoded ballot must succeed")
        .to_str_radix(10)
}

#[wasm_bindgen_test]
fn plurality_smoke_tally_runs_end_to_end() {
    let contest = get_test_contest();
    let contest_json = serde_json::to_string(&contest).expect("serialise contest");

    // First two candidates from the fixture
    let cand_a = contest.candidates[0].id.clone();
    let cand_b = contest.candidates[1].id.clone();

    // Three ballots: A, A, B — expect A wins.
    let ballots = vec![
        encode_selection(&contest, &[&cand_a]),
        encode_selection(&contest, &[&cand_a]),
        encode_selection(&contest, &[&cand_b]),
    ];

    let result_json = tally_plaintext_ballots(&contest_json, ballots)
        .expect("tally must succeed in wasm runtime");

    // Parse the result back as untyped JSON and sanity-check counts.
    let result: serde_json::Value =
        serde_json::from_str(&result_json).expect("result must be valid JSON");

    let total_valid = result["total_valid_votes"].as_u64().unwrap();
    assert_eq!(total_valid, 3, "expected 3 valid ballots");

    let cand_results = result["candidate_result"].as_array().unwrap();
    let a_count = cand_results
        .iter()
        .find(|c| c["candidate"]["id"].as_str() == Some(&cand_a))
        .and_then(|c| c["total_count"].as_u64())
        .unwrap();
    let b_count = cand_results
        .iter()
        .find(|c| c["candidate"]["id"].as_str() == Some(&cand_b))
        .and_then(|c| c["total_count"].as_u64())
        .unwrap();

    assert_eq!(a_count, 2, "candidate A should have 2 votes");
    assert_eq!(b_count, 1, "candidate B should have 1 vote");
}
