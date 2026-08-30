// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The fix ledger's per-fix witnesses: `f_fixed` (production's rules,
//! evaluated through `validation_adapters::f_fixed`) against the frozen
//! oracle `f`, one test per intended divergence — moved here with the
//! rationalized implementation when it folded into sequent-core. The
//! exhaustive version of this comparison is `characterization/fix-diff.md`.

use validation_adapters::f_fixed;
use validation_spec::{
    f, BallotClass, Config, Gate, InvalidVotePolicy, OverVotePolicy, Policies, UnderVotePolicy,
    VoteState, SELECTED_MAX, SELECTED_MIN, UNDER_VOTE,
};

fn cfg(min: u32, max: u32) -> Config {
    Config {
        min,
        max,
        policies: Policies::default(),
    }
}

/// Off the fix cells, the fixed implementation still equals the oracle
/// field-for-field — the fork changes only what the fixes touch. (A
/// min-vote cell under invalid=warn: n = 0 with min = 1, so neither the
/// S6 count nor the S4 zero-zone is in play, and warn keeps the cell off
/// the S1 mute — the selectedMin error renders in BOTH implementations.)
#[test]
fn fixed_matches_oracle_off_the_fix_cells() {
    let config = Config {
        min: 1,
        max: 2,
        policies: Policies {
            invalid: InvalidVotePolicy::Warn,
            ..Policies::default()
        },
    };
    let vs = VoteState {
        regulars: 0,
        ..VoteState::default()
    };
    assert_eq!(f_fixed(&config, &vs), f(&config, &vs));
}

/// S6 fixed: a ranked ballot of two candidates with max = 1 is an
/// over-vote to the checker (it emits selectedMax) AND now to the gate.
/// The oracle gate counted only the single first preference, so it stayed
/// open while the checker flagged the error — the silent-discount shape.
#[test]
fn s6_gate_counts_all_ranked_selections() {
    let config = Config {
        min: 0,
        max: 1,
        policies: Policies {
            over: OverVotePolicy::NotAllowedWithMsgAndAlert,
            ..Policies::default()
        },
    };
    // Two candidates ranked; one sits at rank 0 (an ordinary ranking).
    let vs = VoteState {
        regulars: 2,
        first_preferences: Some(1),
        ..VoteState::default()
    };
    let oracle = f(&config, &vs);
    let fixed = f_fixed(&config, &vs);
    // The checker sees the over-vote in both.
    assert!(oracle.emissions.errors.iter().any(|m| m == SELECTED_MAX));
    assert!(fixed.emissions.errors.iter().any(|m| m == SELECTED_MAX));
    // But only the oracle gate misses it (counted 1, not 2).
    assert!(!oracle.gate.hard);
    assert!(fixed.gate.hard);
}

/// S1 fixed by judgment: the oracle mutes the selectedMin error under
/// invalid=allowed — the voter is told nothing and the ballot is
/// discarded — while the fixed implementation renders it at both casting
/// points. Gates and tally are identical in both: the fix restores
/// information, not friction ("informed but uninterrupted").
#[test]
fn s1_muted_errors_now_render() {
    // One regular selection below min = 2 (NOT a deliberate blank — that
    // cell belongs to the S2/S3 verdict, tested below).
    let config = cfg(2, 3);
    let vs = VoteState {
        regulars: 1,
        ..VoteState::default()
    };
    let oracle = f(&config, &vs);
    let fixed = f_fixed(&config, &vs);
    assert!(oracle.emissions.errors.iter().any(|m| m == SELECTED_MIN));
    assert!(oracle.inline.voting.is_empty() && oracle.inline.review.is_empty());
    assert!(fixed.inline.voting.iter().any(|m| m == SELECTED_MIN));
    assert!(fixed.inline.review.iter().any(|m| m == SELECTED_MIN));
    assert_eq!(fixed.gate, oracle.gate);
    assert_eq!(fixed.dialog, oracle.dialog);
    assert_eq!(fixed.tally, oracle.tally);
}

/// S2/S3 fixed by judgment (2026-08-28): a deliberate blank is not
/// subject to the min-vote rule. The oracle books the min=2 marker-only
/// ballot ImplicitInvalid (with the selectedMin error muted under the
/// default invalid=allowed — the original S2 cell); the fixed
/// implementation emits no error at all and reports it as what the voter
/// declared: ExplicitBlank. No gates in either (nothing to warn about).
#[test]
fn s2s3_deliberate_blank_exempt_from_min_vote() {
    let config = cfg(2, 3);
    let vs = VoteState {
        blank_marker: true,
        ..VoteState::default()
    };
    let oracle = f(&config, &vs);
    let fixed = f_fixed(&config, &vs);
    assert!(oracle.emissions.errors.iter().any(|m| m == SELECTED_MIN));
    assert_eq!(oracle.tally, BallotClass::ImplicitInvalid);
    assert!(fixed.emissions.errors.is_empty());
    assert_eq!(fixed.tally, BallotClass::ExplicitBlank);
    assert_eq!(
        fixed.gate,
        Gate {
            hard: false,
            soft: false
        }
    );
    // The null-vote shape is NOT an explicit blank: the flag keeps the
    // ballot inside the min-vote rule (verdict-scoped exemption).
    let null_vs = VoteState {
        explicit_invalid: true,
        ..VoteState::default()
    };
    assert!(f_fixed(&config, &null_vs)
        .emissions
        .errors
        .iter()
        .any(|m| m == SELECTED_MIN));
}

/// S4 fixed: the empty ballot is not an under-vote. With min = 0 the
/// oracle checker emitted an under-vote alert on the empty ballot
/// (overlapping the blank rule); the fixed checker does not.
#[test]
fn s4_empty_ballot_is_not_an_undervote() {
    let config = Config {
        min: 0,
        max: 2,
        policies: Policies {
            under: UnderVotePolicy::WarnAndAlert,
            ..Policies::default()
        },
    };
    let vs = VoteState {
        regulars: 0,
        ..VoteState::default()
    };
    assert!(f(&config, &vs)
        .emissions
        .alerts
        .iter()
        .any(|m| m == UNDER_VOTE));
    assert!(!f_fixed(&config, &vs)
        .emissions
        .alerts
        .iter()
        .any(|m| m == UNDER_VOTE));
}
