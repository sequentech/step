// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Adapter conformance — the native analogue of the wasm sweep, with the
//! adapters in the loop. Decode and the gates are both injected
//! (raw_ballot.rs and voting_screen.rs route through the query-provider),
//! so production's emissions AND gates must match the RATIONALIZED
//! `f_fixed` ∘ (contest_config, vote_state).
//!
//! For every cell of a policy × vote-state matrix mirroring the seven
//! characterization grids (on the real bundled-fixture contests), the wire
//! selection is round-tripped through production's own codec
//! (`encode_plaintext_contest_bigint` → `decode_plaintext_contest_bigint`)
//! and production's own gate functions, and compared against `f_fixed`.
//!
//! Also asserted per cell:
//!
//!   * route convergence — deriving the `VoteState` from the PRE-decode
//!     wire selection and from the POST-decode record gives the same
//!     answer;
//!   * record fidelity — the PRE-injection checker sequence (the checker
//!     functions in their raw_ballot.rs call order, reproduced verbatim in
//!     [`legacy_policy_checks`]), transformed by EXACTLY the fix ledger's
//!     two decode movements (S2S3: no `selectedMin` for a deliberate
//!     blank; S4: no `underVote` alert on the empty ballot), equals the
//!     provider's `policy_emissions` record for record — `error_type`,
//!     message key, `message_map`, order. The behaviour change at the
//!     decode site is those two movements and nothing else.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sequent_core::ballot::{
    Contest, EBlankVotePolicy, EDuplicatedRankPolicy, EOverVotePolicy, EPreferenceGapsPolicy,
    EUnderVotePolicy, InvalidVotePolicy,
};
use sequent_core::ballot_codec::{
    check_blank_vote_policy, check_duplicated_rank_policy, check_invalid_vote_policy,
    check_max_min_votes_policy, check_min_vote_policy, check_over_vote_policy,
    check_preference_gaps_policy, check_under_vote_policy, BigUIntCodec, CheckerResult,
};
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest, PreferencialOrderErrorType};
use sequent_core::util::voting_screen::{
    check_voting_error_dialog_util, check_voting_not_allowed_next_util,
};
use validation_adapters::{
    contest_config, f_fixed, for_ballot, policy_emissions, spec_config, spec_vote_state,
    vote_state, AdapterError,
};
use validation_spec::{selections_with_markers, SELECTED_MIN, UNDER_VOTE};

// ---------------------------------------------------------------------------
// Fixture loading — the same bundled snapshots the characterization uses
// ---------------------------------------------------------------------------

fn fixture_contests(file: &str) -> Vec<Contest> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../app/src/fixtures/snapshots")
        .join(file);
    let snap: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read fixture {}: {e}", path.display())),
    )
    .expect("fixture JSON");
    let styles = snap["state"]["ballotStyles"]
        .as_object()
        .expect("ballotStyles object");
    let eml = &styles.values().next().expect("one ballot style")["ballot_eml"];
    eml["contests"]
        .as_array()
        .expect("contests array")
        .iter()
        .map(|c| serde_json::from_value(c.clone()).expect("Contest deserializes"))
        .collect()
}

fn contest_with_marker(contests: &[Contest], blank: bool) -> Contest {
    contests
        .iter()
        .find(|c| {
            c.candidates.iter().any(|cand| {
                if blank {
                    cand.is_explicit_blank()
                } else {
                    cand.is_explicit_invalid()
                }
            })
        })
        .expect("marker contest")
        .clone()
}

fn marker_id(contest: &Contest, blank: bool) -> String {
    contest
        .candidates
        .iter()
        .find(|cand| {
            if blank {
                cand.is_explicit_blank()
            } else {
                cand.is_explicit_invalid()
            }
        })
        .expect("marker candidate")
        .id
        .clone()
}

fn regular_ids(contest: &Contest) -> Vec<String> {
    contest
        .candidates
        .iter()
        .filter(|c| !c.is_explicit_blank() && !c.is_explicit_invalid())
        .map(|c| c.id.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Cell machinery
// ---------------------------------------------------------------------------

/// A wire selection: `(candidate id, selected)` for the picked candidates,
/// everything else -1 (the runners' makeSelection).
fn wire(contest: &Contest, picked: &[(&str, i64)], flag: bool) -> DecodedVoteContest {
    DecodedVoteContest {
        contest_id: contest.id.clone(),
        is_explicit_invalid: flag,
        is_decline_to_vote: false,
        is_blank_ballot: false,
        invalid_errors: vec![],
        invalid_alerts: vec![],
        choices: contest
            .candidates
            .iter()
            .map(|c| DecodedVoteChoice {
                id: c.id.clone(),
                selected: picked
                    .iter()
                    .find(|(id, _)| *id == c.id)
                    .map(|(_, sel)| *sel)
                    .unwrap_or(-1),
                write_in_text: None,
            })
            .collect(),
    }
}

fn keys(errors: &[sequent_core::plaintext::InvalidPlaintextError]) -> Vec<String> {
    errors
        .iter()
        .map(|e| e.message.clone().unwrap_or_default())
        .collect()
}

/// The PRE-injection policy-check sequence of `raw_ballot.rs::decode`,
/// verbatim (the checker functions in their original call order, fed the
/// same marker-inclusive count) — the "before" leg of the record-fidelity
/// assertion. Bounds are assumed representable (every grid cell's are).
fn legacy_policy_checks(contest: &Contest, decoded: &DecodedVoteContest) -> CheckerResult {
    let presentation = contest.presentation.clone().unwrap_or_default();
    let is_explicit_invalid = decoded.is_explicit_invalid;
    let is_explicit_blank = decoded.choices.iter().any(|choice| {
        choice.selected > -1
            && contest
                .candidates
                .iter()
                .any(|c| c.id == choice.id && c.is_explicit_blank())
    });

    let mut result = CheckerResult::default();
    let mut push = |r: CheckerResult| {
        result.invalid_errors.extend(r.invalid_errors);
        result.invalid_alerts.extend(r.invalid_alerts);
    };

    push(check_invalid_vote_policy(
        &presentation,
        is_explicit_invalid,
    ));

    let num_selected_candidates = decoded
        .choices
        .iter()
        .filter(|choice| {
            choice.selected > -1
                && contest
                    .candidates
                    .iter()
                    .find(|candidate| candidate.id == choice.id)
                    .map(|candidate| !candidate.is_explicit_blank())
                    .unwrap_or(true)
        })
        .count();
    let (max_votes, min_votes, maxmin_errors) =
        check_max_min_votes_policy(contest.max_votes, contest.min_votes);
    push(maxmin_errors);
    let num_selected_with_markers =
        num_selected_candidates + usize::from(is_explicit_invalid) + usize::from(is_explicit_blank);
    if let Some(max_votes) = max_votes {
        push(check_over_vote_policy(
            &presentation,
            num_selected_with_markers,
            max_votes,
        ));
    }
    if let Some(min_votes) = min_votes {
        push(check_min_vote_policy(num_selected_with_markers, min_votes));
    }
    push(check_under_vote_policy(
        &presentation,
        num_selected_with_markers,
        max_votes,
        min_votes,
    ));
    push(check_blank_vote_policy(
        &presentation,
        num_selected_with_markers,
        is_explicit_invalid,
    ));
    if contest.get_counting_algorithm().is_preferential() {
        if let Err(errors) = decoded.validate_preferencial_order() {
            for error in errors {
                match error {
                    PreferencialOrderErrorType::PreferenceOrderWithGaps => {
                        push(check_preference_gaps_policy(&presentation))
                    }
                    PreferencialOrderErrorType::DuplicatedPosition => {
                        push(check_duplicated_rank_policy(&presentation))
                    }
                }
            }
        }
    }
    result
}

/// Round-trip through production's codec, evaluate production's gates, and
/// compare everything against `f_fixed` through the adapters.
fn assert_cell(contest: &Contest, input: &DecodedVoteContest, label: &str) {
    let bigint = contest
        .encode_plaintext_contest_bigint(input)
        .unwrap_or_else(|e| panic!("{label}: encode failed: {e:?}"));
    let decoded = contest
        .decode_plaintext_contest_bigint(&bigint)
        .unwrap_or_else(|e| panic!("{label}: decode failed: {e:?}"));

    let prod_hard = check_voting_not_allowed_next_util(
        vec![contest.clone()],
        HashMap::from([(contest.id.clone(), decoded.clone())]),
    );
    let prod_soft = check_voting_error_dialog_util(
        vec![contest.clone()],
        HashMap::from([(contest.id.clone(), decoded.clone())]),
    );

    let vs = vote_state(contest, &decoded);
    // Route convergence: the pre-decode wire selection derives the same
    // VoteState as the canonical decoded record.
    assert_eq!(
        vote_state(contest, input),
        vs,
        "{label}: pre-decode and post-decode derivations disagree"
    );

    // Decode and the gates are injected: production matches f_fixed.
    let spec_cfg = spec_config(contest).unwrap_or_else(|e| panic!("{label}: {e}"));
    let spec_vs = spec_vote_state(contest, &decoded);
    let fixed = f_fixed(&spec_cfg, &spec_vs);
    assert_eq!(
        fixed.emissions.errors,
        keys(&decoded.invalid_errors),
        "{label}: errors disagree"
    );
    assert_eq!(
        fixed.emissions.alerts,
        keys(&decoded.invalid_alerts),
        "{label}: alerts disagree"
    );
    assert_eq!(fixed.gate.hard, prod_hard, "{label}: hard gate disagrees");
    assert_eq!(fixed.gate.soft, prod_soft, "{label}: soft gate disagrees");

    // Record fidelity: the legacy checker sequence, transformed by EXACTLY
    // the fix ledger's two decode movements, equals the provider's record —
    // error_type, key, message_map and order included.
    let mut expected = legacy_policy_checks(contest, &decoded);
    let n = selections_with_markers(&spec_vs);
    let deliberate_blank = vs.blank_marker && vs.regulars == 0 && !vs.explicit_invalid;
    if deliberate_blank {
        // S2S3: a deliberate blank is not subject to the min-vote rule.
        expected
            .invalid_errors
            .retain(|e| e.message.as_deref() != Some(SELECTED_MIN));
    }
    if n == 0 {
        // S4: the empty ballot is not an under-vote (the blank rule's domain).
        expected
            .invalid_alerts
            .retain(|a| a.message.as_deref() != Some(UNDER_VOTE));
    }
    let provider = policy_emissions(contest, &decoded)
        .unwrap_or_else(|e| panic!("{label}: provider rejected the config: {e}"));
    assert_eq!(
        expected, provider,
        "{label}: provider record differs from legacy beyond the two named fixes"
    );
}

fn with_policies(
    contest: &Contest,
    min: Option<i64>,
    max: Option<i64>,
    set: impl FnOnce(&mut sequent_core::ballot::ContestPresentation),
) -> Contest {
    let mut c = contest.clone();
    if let Some(min) = min {
        c.min_votes = min;
    }
    if let Some(max) = max {
        c.max_votes = max;
    }
    let p = c.presentation.get_or_insert_with(Default::default);
    set(p);
    c
}

const INVALID: [InvalidVotePolicy; 5] = [
    InvalidVotePolicy::ALLOWED,
    InvalidVotePolicy::WARN,
    InvalidVotePolicy::WARN_INVALID_IMPLICIT_AND_EXPLICIT,
    InvalidVotePolicy::NOT_ALLOWED,
    InvalidVotePolicy::ALLOWED_WITH_EXCLUSIVE_EXPLICIT,
];

// ---------------------------------------------------------------------------
// The grids
// ---------------------------------------------------------------------------

#[test]
fn overvote_grid_matches_production() {
    let contests = fixture_contests("explicit-blank-invalid.json");
    let council = contest_with_marker(&contests, false);
    let regs = regular_ids(&council);
    const OVER: [EOverVotePolicy; 5] = [
        EOverVotePolicy::ALLOWED,
        EOverVotePolicy::ALLOWED_WITH_MSG,
        EOverVotePolicy::ALLOWED_WITH_MSG_AND_ALERT,
        EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_ALERT,
        EOverVotePolicy::NOT_ALLOWED_WITH_MSG_AND_DISABLE,
    ];
    for over in OVER {
        for invalid in INVALID {
            let c = with_policies(&council, Some(0), Some(1), |p| {
                p.over_vote_policy = Some(over.clone());
                p.invalid_vote_policy = Some(invalid.clone());
            });
            for (state, picked) in [
                ("empty", vec![]),
                ("at_max", vec![(regs[0].as_str(), 0)]),
                (
                    "over_max",
                    vec![(regs[0].as_str(), 0), (regs[1].as_str(), 0)],
                ),
            ] {
                assert_cell(
                    &c,
                    &wire(&c, &picked, false),
                    &format!("over {over:?}×{invalid:?}×{state}"),
                );
            }
        }
    }
}

#[test]
fn minvote_and_blank_and_undervote_grids_match_production() {
    let contests = fixture_contests("explicit-blank-invalid.json");
    let referendum = contest_with_marker(&contests, true);
    let regs = regular_ids(&referendum);
    let blank_id = marker_id(&referendum, true);

    // min-vote: min ∈ {1,2} × invalid × {none, one, marker_only}, max 3
    for min in [1, 2] {
        for invalid in INVALID {
            let c = with_policies(&referendum, Some(min), Some(3), |p| {
                p.invalid_vote_policy = Some(invalid.clone());
            });
            for (state, picked) in [
                ("none", vec![]),
                ("one", vec![(regs[0].as_str(), 0)]),
                ("marker_only", vec![(blank_id.as_str(), 0)]),
            ] {
                assert_cell(
                    &c,
                    &wire(&c, &picked, false),
                    &format!("min {min}×{invalid:?}×{state}"),
                );
            }
        }
    }

    // blank: blank(4) × invalid(5) × {empty, explicit_invalid, marker_only,
    // one_regular}, min 0 max 2
    const BLANK: [EBlankVotePolicy; 4] = [
        EBlankVotePolicy::ALLOWED,
        EBlankVotePolicy::WARN,
        EBlankVotePolicy::WARN_ONLY_IN_REVIEW,
        EBlankVotePolicy::NOT_ALLOWED,
    ];
    for blank in BLANK {
        for invalid in INVALID {
            let c = with_policies(&referendum, Some(0), Some(2), |p| {
                p.blank_vote_policy = Some(blank.clone());
                p.invalid_vote_policy = Some(invalid.clone());
            });
            for (state, picked, flag) in [
                ("empty", vec![], false),
                ("explicit_invalid", vec![], true),
                ("marker_only", vec![(blank_id.as_str(), 0)], false),
                ("one_regular", vec![(regs[0].as_str(), 0)], false),
            ] {
                assert_cell(
                    &c,
                    &wire(&c, &picked, flag),
                    &format!("blank {blank:?}×{invalid:?}×{state}"),
                );
            }
        }
    }

    // under-vote: under(4) × invalid(5) × {empty, under, full}, min 0 max 2
    const UNDER: [EUnderVotePolicy; 4] = [
        EUnderVotePolicy::ALLOWED,
        EUnderVotePolicy::WARN,
        EUnderVotePolicy::WARN_ONLY_IN_REVIEW,
        EUnderVotePolicy::WARN_AND_ALERT,
    ];
    for under in UNDER {
        for invalid in INVALID {
            let c = with_policies(&referendum, Some(0), Some(2), |p| {
                p.under_vote_policy = Some(under.clone());
                p.invalid_vote_policy = Some(invalid.clone());
            });
            for (state, picked) in [
                ("empty", vec![]),
                ("under", vec![(regs[0].as_str(), 0)]),
                ("full", vec![(regs[0].as_str(), 0), (regs[1].as_str(), 0)]),
            ] {
                assert_cell(
                    &c,
                    &wire(&c, &picked, false),
                    &format!("under {under:?}×{invalid:?}×{state}"),
                );
            }
        }
    }
}

#[test]
fn invalid_grid_matches_production_including_marker_routes() {
    let contests = fixture_contests("explicit-blank-invalid.json");
    let council = contest_with_marker(&contests, false);
    let regs = regular_ids(&council);
    let null_id = marker_id(&council, false);
    for invalid in INVALID {
        let c = with_policies(&council, Some(0), Some(2), |p| {
            p.invalid_vote_policy = Some(invalid.clone());
        });
        for (state, picked, flag) in [
            ("none", vec![], false),
            ("regular", vec![(regs[0].as_str(), 0)], false),
            ("flag_only", vec![], true),
            // the marker routes: the marker choice selected, flag set — the
            // booth reducer's behaviour; decode canonicalizes to flag-only
            ("marker", vec![(null_id.as_str(), 0)], true),
            (
                "marker_plus",
                vec![(null_id.as_str(), 0), (regs[0].as_str(), 0)],
                true,
            ),
        ] {
            assert_cell(
                &c,
                &wire(&c, &picked, flag),
                &format!("invalid {invalid:?}×{state}"),
            );
        }
    }
}

#[test]
fn preferential_grids_match_production() {
    let contests = fixture_contests("instant-runoff-3cand.json");
    let irv = contests
        .iter()
        .find(|c| c.get_counting_algorithm().is_preferential())
        .expect("IRV contest")
        .clone();
    let ids = regular_ids(&irv);
    const DUP: [EDuplicatedRankPolicy; 2] = [
        EDuplicatedRankPolicy::ALLOWED_WARN_AND_DIALOG,
        EDuplicatedRankPolicy::NOT_ALLOWED_WARN_AND_DIALOG,
    ];
    const GAP: [EPreferenceGapsPolicy; 2] = [
        EPreferenceGapsPolicy::ALLOWED_WARN_AND_DIALOG,
        EPreferenceGapsPolicy::NOT_ALLOWED_WARN_AND_DIALOG,
    ];
    for dup in DUP {
        for invalid in INVALID {
            let c = with_policies(&irv, None, None, |p| {
                p.duplicated_rank_policy = Some(dup.clone());
                p.invalid_vote_policy = Some(invalid.clone());
            });
            // both_defects (two candidates at rank 0, one at rank 2 — a
            // duplicate AND a gap) pins the EMISSION ORDER of the
            // preferential pair: duplicates before gaps, the order
            // validate_preferencial_order returns and decoding emits. The
            // record-fidelity assertion compares full ordered records, so
            // this is the one cell where that order is instrument-checked.
            for (state, ranks) in [
                ("valid_full", [0, 1, 2]),
                ("duplicate", [0, 0, -1]),
                ("both_defects", [0, 0, 2]),
            ] {
                let picked: Vec<(&str, i64)> = ids
                    .iter()
                    .zip(ranks)
                    .filter(|(_, r)| *r >= 0)
                    .map(|(id, r)| (id.as_str(), r))
                    .collect();
                assert_cell(
                    &c,
                    &wire(&c, &picked, false),
                    &format!("dup {dup:?}×{invalid:?}×{state}"),
                );
            }
        }
    }
    for gap in GAP {
        for invalid in INVALID {
            let c = with_policies(&irv, None, None, |p| {
                p.preference_gaps_policy = Some(gap.clone());
                p.invalid_vote_policy = Some(invalid.clone());
            });
            for (state, ranks) in [("valid_full", [0, 1, 2]), ("gap", [0, 2, -1])] {
                let picked: Vec<(&str, i64)> = ids
                    .iter()
                    .zip(ranks)
                    .filter(|(_, r)| *r >= 0)
                    .map(|(id, r)| (id.as_str(), r))
                    .collect();
                assert_cell(
                    &c,
                    &wire(&c, &picked, false),
                    &format!("gap {gap:?}×{invalid:?}×{state}"),
                );
            }
        }
    }
}

#[test]
fn ballot_composition_matches_production_gates() {
    let contests = fixture_contests("explicit-blank-invalid.json");
    let referendum = contest_with_marker(&contests, true);
    let council = contest_with_marker(&contests, false);

    // Referendum blocking (blank=not-allowed, empty); Council clean (one pick).
    let blocking = with_policies(&referendum, Some(0), Some(2), |p| {
        p.blank_vote_policy = Some(EBlankVotePolicy::NOT_ALLOWED);
    });
    let clean = with_policies(&council, Some(0), Some(1), |p| {
        p.invalid_vote_policy = Some(InvalidVotePolicy::ALLOWED);
    });
    let regs = regular_ids(&clean);

    let d_blocking = {
        let input = wire(&blocking, &[], false);
        let bigint = blocking.encode_plaintext_contest_bigint(&input).unwrap();
        blocking.decode_plaintext_contest_bigint(&bigint).unwrap()
    };
    let d_clean = {
        let input = wire(&clean, &[(regs[0].as_str(), 0)], false);
        let bigint = clean.encode_plaintext_contest_bigint(&input).unwrap();
        clean.decode_plaintext_contest_bigint(&bigint).unwrap()
    };

    let prod_hard = check_voting_not_allowed_next_util(
        vec![blocking.clone(), clean.clone()],
        HashMap::from([
            (blocking.id.clone(), d_blocking.clone()),
            (clean.id.clone(), d_clean.clone()),
        ]),
    );
    let prod_soft = check_voting_error_dialog_util(
        vec![blocking.clone(), clean.clone()],
        HashMap::from([
            (blocking.id.clone(), d_blocking.clone()),
            (clean.id.clone(), d_clean.clone()),
        ]),
    );

    let ballot = for_ballot([(&blocking, &d_blocking), (&clean, &d_clean)]).unwrap();
    assert_eq!(ballot.hard_gate(), prod_hard);
    assert_eq!(ballot.soft_gate(), prod_soft);
    assert!(
        ballot.hard_gate(),
        "the blocking contest must block the ballot"
    );
}

#[test]
fn unrepresentable_bounds_are_a_config_rejection() {
    let contests = fixture_contests("explicit-blank-invalid.json");
    let mut c = contests[0].clone();
    c.min_votes = -1;
    assert_eq!(
        contest_config(&c),
        Err(AdapterError::UnrepresentableBounds {
            min_votes: -1,
            max_votes: c.max_votes
        })
    );
}
