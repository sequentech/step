// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Adapter conformance — the native analogue of the wasm sweep, with the
//! adapters in the loop, expectations split by injection status:
//!
//!   production decode  ≡  ORACLE `f` ∘ (contest_config, vote_state)
//!       — decode is NOT injected: it must still match the bug-compatible
//!         oracle's emissions;
//!   production gates   ≡  RATIONALIZED `f_fixed` ∘ (…)
//!       — the gates ARE injected (voting_screen.rs routes through the
//!         query-provider), so production now carries the ledger's gate
//!         fixes and must match `f_fixed`.
//!
//! For every cell of a policy × vote-state matrix mirroring the seven
//! characterization grids (on the real bundled-fixture contests), the wire
//! selection is round-tripped through production's own codec
//! (`encode_plaintext_contest_bigint` → `decode_plaintext_contest_bigint`)
//! and production's own gate functions, and compared per the split above.
//!
//! Also asserted per cell: deriving the `VoteState` from the PRE-decode wire
//! selection and from the POST-decode record gives the same answer (the
//! marker/flag route convergence the adapter mirrors).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sequent_core::ballot::{
    Contest, EBlankVotePolicy, EDuplicatedRankPolicy, EOverVotePolicy, EPreferenceGapsPolicy,
    EUnderVotePolicy, InvalidVotePolicy,
};
use sequent_core::ballot_codec::BigUIntCodec;
use sequent_core::plaintext::{DecodedVoteChoice, DecodedVoteContest};
use sequent_core::util::voting_screen::{
    check_voting_error_dialog_util, check_voting_not_allowed_next_util,
};
use validation_adapters::{contest_config, for_ballot, vote_state, AdapterError};
use validation_spec::{f, f_fixed};

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

/// Round-trip through production's codec, evaluate production's gates, and
/// compare everything against the oracle through the adapters.
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

    let config = contest_config(contest).unwrap_or_else(|e| panic!("{label}: {e}"));
    let vs = vote_state(contest, &decoded);
    // Route convergence: the pre-decode wire selection derives the same
    // VoteState as the canonical decoded record.
    assert_eq!(
        vote_state(contest, input),
        vs,
        "{label}: pre-decode and post-decode derivations disagree"
    );

    // Decode is not injected: emissions match the bug-compatible oracle.
    let oracle = f(&config, &vs);
    assert_eq!(
        oracle.emissions.errors,
        keys(&decoded.invalid_errors),
        "{label}: errors disagree"
    );
    assert_eq!(
        oracle.emissions.alerts,
        keys(&decoded.invalid_alerts),
        "{label}: alerts disagree"
    );
    // The gates are injected: production matches the rationalized f_fixed.
    let fixed = f_fixed(&config, &vs);
    assert_eq!(fixed.gate.hard, prod_hard, "{label}: hard gate disagrees");
    assert_eq!(fixed.gate.soft, prod_soft, "{label}: soft gate disagrees");
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
            for (state, ranks) in [("valid_full", [0, 1, 2]), ("duplicate", [0, 0, -1])] {
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
