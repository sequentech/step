// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Effect-dependency analysis of the spec (lane A of
//! `characterization/effect-dependencies.mjs`).
//!
//! Enumerates the spec's full valid input domain and computes, for every
//! effect COMPONENT (each scalar effect plus the presence of each message
//! key in each of errors / alerts / inline.voting / inline.review):
//!
//!   - its SUPPORT: which input dimensions can change it at all
//!     (for dimension Y: does any fiber — an assignment of every other
//!     dimension — yield different component values as Y varies?);
//!   - its CONDITIONAL PROFILE: for each supporting Y and each other
//!     dimension Z, the projection of the Y-sensitive fibers onto Z. A
//!     strict subset reads "this component depends on Y only when Z ∈ S" —
//!     a conditional-independence statement (necessary condition, exact as
//!     a projection);
//!   - a WITNESS per (component, Y): one concrete fiber and two Y values
//!     with differing component values — an executable claim, re-runnable
//!     against production by the calling runner (lane B).
//!
//! Everything here is a statement about the SPEC. Fidelity to production
//! is exactly what the witnesses + the calling runner's evidence labels
//! establish (or defer) per claim.
//!
//! Domain note: bounds are constrained to min ≤ max — outside it,
//! production emits encoding errors the spec deliberately does not model
//! (a named scope boundary). Deterministic: no clock, no randomness.

use std::io::Write;
use validation_spec::*;

const INVALID: [InvalidVotePolicy; 4] = [
    InvalidVotePolicy::Allowed,
    InvalidVotePolicy::Warn,
    InvalidVotePolicy::WarnInvalidImplicitAndExplicit,
    InvalidVotePolicy::NotAllowed,
];
const BLANK: [BlankVotePolicy; 4] = [
    BlankVotePolicy::Allowed,
    BlankVotePolicy::Warn,
    BlankVotePolicy::WarnOnlyInReview,
    BlankVotePolicy::NotAllowed,
];
const OVER: [OverVotePolicy; 5] = [
    OverVotePolicy::Allowed,
    OverVotePolicy::AllowedWithMsg,
    OverVotePolicy::AllowedWithMsgAndAlert,
    OverVotePolicy::NotAllowedWithMsgAndAlert,
    OverVotePolicy::NotAllowedWithMsgAndDisable,
];
const UNDER: [UnderVotePolicy; 4] = [
    UnderVotePolicy::Allowed,
    UnderVotePolicy::Warn,
    UnderVotePolicy::WarnOnlyInReview,
    UnderVotePolicy::WarnAndAlert,
];
const RANK: [RankPolicy; 2] = [
    RankPolicy::AllowedWarnAndDialog,
    RankPolicy::NotAllowedWarnAndDialog,
];

/// The nine message keys, indexed for bitmasking.
const KEYS: [&str; 9] = [
    SELECTED_MAX,
    SELECTED_MIN,
    BLANK_VOTE,
    UNDER_VOTE,
    OVER_VOTE_DISABLED,
    DUPLICATED_POSITION,
    PREFERENCE_ORDER_WITH_GAPS,
    EXPLICIT_NOT_ALLOWED,
    EXPLICIT_ALERT,
];

/// Input dimensions, in a fixed order shared with the JSON output.
const DIMS: [(&str, usize); 14] = [
    ("invalid_vote_policy", 4),
    ("blank_vote_policy", 4),
    ("over_vote_policy", 5),
    ("under_vote_policy", 4),
    ("duplicated_rank_policy", 2),
    ("preference_gaps_policy", 2),
    ("min_votes", 4),  // 0..=3
    ("max_votes", 4),  // 0..=3; cells with min > max are excluded
    ("regulars", 5),   // 0..=4
    ("blank_marker", 2),
    ("explicit_invalid", 2),
    ("decline", 2),
    ("duplicate_ranks", 2),
    ("rank_gaps", 2),
];
const N_DIMS: usize = 14;

fn dim_value_label(dim: usize, v: usize) -> String {
    let wire = |s: &str| s.to_string();
    match dim {
        0 => wire(serde_json::to_value(INVALID[v]).unwrap().as_str().unwrap()),
        1 => wire(serde_json::to_value(BLANK[v]).unwrap().as_str().unwrap()),
        2 => wire(serde_json::to_value(OVER[v]).unwrap().as_str().unwrap()),
        3 => wire(serde_json::to_value(UNDER[v]).unwrap().as_str().unwrap()),
        4 | 5 => wire(serde_json::to_value(RANK[v]).unwrap().as_str().unwrap()),
        6 | 7 | 8 => v.to_string(),
        _ => (v == 1).to_string(),
    }
}

fn build(idx: &[usize; N_DIMS]) -> (Config, VoteState) {
    (
        Config {
            min: idx[6] as u32,
            max: idx[7] as u32,
            policies: Policies {
                invalid: INVALID[idx[0]],
                blank: BLANK[idx[1]],
                over: OVER[idx[2]],
                under: UNDER[idx[3]],
                dup: RANK[idx[4]],
                gap: RANK[idx[5]],
            },
        },
        VoteState {
            regulars: idx[8] as u32,
            blank_marker: idx[9] == 1,
            explicit_invalid: idx[10] == 1,
            decline: idx[11] == 1,
            duplicate_ranks: idx[12] == 1,
            rank_gaps: idx[13] == 1,
            // Not a dimension of this analysis: leaving it None makes the
            // gates read `regulars`, i.e. the plurality behaviour, exactly
            // as before quirk S6 existed. Adding first-preference counts as
            // a 15th dimension is analysis-layer work in its own right —
            // it would widen the ledger, not establish fidelity.
            first_preferences: None,
        },
    )
}

/// One evaluated cell, packed for fast per-component comparison.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
struct Packed {
    errors: u16,
    alerts: u16,
    voting: u16,
    review: u16,
    hard: bool,
    soft: bool,
    dialog: u8,
    reach: u8,
    tally: u8,
}

fn mask(keys: &[String]) -> u16 {
    let mut m = 0u16;
    for k in keys {
        if let Some(i) = KEYS.iter().position(|x| x == k) {
            m |= 1 << i;
        }
    }
    m
}

fn eval(idx: &[usize; N_DIMS]) -> Packed {
    let (config, vs) = build(idx);
    let e = f(&config, &vs);
    Packed {
        errors: mask(&e.emissions.errors),
        alerts: mask(&e.emissions.alerts),
        voting: mask(&e.inline.voting),
        review: mask(&e.inline.review),
        hard: e.gate.hard,
        soft: e.gate.soft,
        dialog: match e.dialog {
            Dialog::None => 0,
            Dialog::Dismissible => 1,
            Dialog::Blocking => 2,
        },
        reach: match e.reachability {
            Reachability::Yes => 0,
            Reachability::InputsDisabled => 1,
            Reachability::MarkerCleared => 2,
        },
        tally: match e.tally {
            BallotClass::Valid => 0,
            BallotClass::ExplicitInvalid => 1,
            BallotClass::ImplicitInvalid => 2,
            BallotClass::ExplicitBlank => 3,
            BallotClass::ImplicitBlank => 4,
            BallotClass::Declined => 5,
        },
    }
}

/// Component extractors: 4 sets × 9 key-presence booleans + 5 scalars.
const N_COMPONENTS: usize = 4 * 9 + 5;

fn component_name(c: usize) -> String {
    if c < 36 {
        let set = ["errors", "alerts", "inline.voting", "inline.review"][c / 9];
        let key = KEYS[c % 9].rsplit('.').next().unwrap();
        format!("{key} ∈ {set}")
    } else {
        ["gate.hard", "gate.soft", "dialog", "reachability", "tally"][c - 36].to_string()
    }
}

fn component_value(p: &Packed, c: usize) -> u16 {
    if c < 36 {
        let m = [p.errors, p.alerts, p.voting, p.review][c / 9];
        (m >> (c % 9)) & 1
    } else {
        match c - 36 {
            0 => p.hard as u16,
            1 => p.soft as u16,
            2 => p.dialog as u16,
            3 => p.reach as u16,
            _ => p.tally as u16,
        }
    }
}

fn component_value_label(c: usize, v: u16) -> String {
    if c < 36 {
        return (v == 1).to_string();
    }
    match c - 36 {
        0 | 1 => (v == 1).to_string(),
        2 => ["none", "dismissible", "blocking"][v as usize].to_string(),
        3 => ["yes", "inputs_disabled", "marker_cleared"][v as usize].to_string(),
        _ => [
            "Valid",
            "ExplicitInvalid",
            "ImplicitInvalid",
            "ExplicitBlank",
            "ImplicitBlank",
            "Declined",
        ][v as usize]
            .to_string(),
    }
}

#[derive(Clone)]
struct Witness {
    fiber: [usize; N_DIMS], // the varied dim's slot holds y1
    dim: usize,
    y1: usize,
    y2: usize,
    v1: u16,
    v2: u16,
}

/// Witness preference: the calling runner re-runs witnesses through the
/// production WASM, whose fixtures are plurality contests with two regular
/// candidates and sane bounds — prefer witnesses it can represent. A
/// component whose sensitivity requires an unrepresentable dimension keeps
/// the best available witness (the runner labels it deferred).
fn representability_score(fiber: &[usize; N_DIMS], dim: usize, y1: usize) -> u32 {
    let mut cell = *fiber;
    cell[dim] = y1;
    let mut s = 0;
    if cell[11] == 0 {
        s += 1; // no decline
    }
    if cell[12] == 0 && cell[13] == 0 {
        s += 1; // no preferential state
    }
    if cell[8] <= 2 {
        s += 1; // regulars within fixture candidates
    }
    if cell[7] >= 1 {
        s += 1; // max_votes ≥ 1 (max = 0 is a config-sanity boundary)
    }
    s
}

fn main() {
    // sensitivity[c][y]; profile[c][y][z] = bitmask of z-values seen in
    // Y-sensitive fibers; witness[c][y] = first sensitive fiber found.
    let mut sensitive = vec![[false; N_DIMS]; N_COMPONENTS];
    let mut profile = vec![[[0u8; N_DIMS]; N_DIMS]; N_COMPONENTS];
    let mut witness: Vec<Vec<Option<(u32, Witness)>>> = vec![vec![None; N_DIMS]; N_COMPONENTS];
    let mut cells_seen = 0u64;

    for y in 0..N_DIMS {
        // Odometer over every dimension except y; y is the inner sweep.
        let mut idx = [0usize; N_DIMS];
        loop {
            // Evaluate the fiber's cells across y (skipping invalid bounds).
            let mut vals: Vec<(usize, Packed)> = Vec::with_capacity(DIMS[y].1);
            for v in 0..DIMS[y].1 {
                idx[y] = v;
                if idx[6] > idx[7] {
                    continue; // min > max: outside the modelled domain
                }
                vals.push((v, eval(&idx)));
            }
            cells_seen += vals.len() as u64;
            if vals.len() >= 2 {
                for c in 0..N_COMPONENTS {
                    let first = component_value(&vals[0].1, c);
                    if let Some((y2, p2)) =
                        vals[1..].iter().find(|(_, p)| component_value(p, c) != first)
                    {
                        sensitive[c][y] = true;
                        for z in 0..N_DIMS {
                            if z != y {
                                profile[c][y][z] |= 1 << idx[z];
                            }
                        }
                        let score = representability_score(&idx, y, vals[0].0);
                        let better = match &witness[c][y] {
                            None => true,
                            Some((s, _)) => score > *s,
                        };
                        if better {
                            let mut fiber = idx;
                            fiber[y] = vals[0].0;
                            witness[c][y] = Some((
                                score,
                                Witness {
                                    fiber,
                                    dim: y,
                                    y1: vals[0].0,
                                    y2: *y2,
                                    v1: first,
                                    v2: component_value(p2, c),
                                },
                            ));
                        }
                    }
                }
            }
            // Advance the odometer (all dims except y).
            let mut d = N_DIMS;
            loop {
                if d == 0 {
                    break;
                }
                d -= 1;
                if d == y {
                    continue;
                }
                idx[d] += 1;
                if idx[d] < DIMS[d].1 {
                    break;
                }
                idx[d] = 0;
            }
            if idx.iter().enumerate().all(|(i, &v)| i == y || v == 0) {
                break;
            }
        }
    }

    // ---- JSON out ----------------------------------------------------------
    let dims_json: Vec<serde_json::Value> = DIMS
        .iter()
        .enumerate()
        .map(|(i, (name, card))| {
            serde_json::json!({
                "name": name,
                "values": (0..*card).map(|v| dim_value_label(i, v)).collect::<Vec<_>>(),
            })
        })
        .collect();

    let mut components = Vec::new();
    for c in 0..N_COMPONENTS {
        let support: Vec<&str> = (0..N_DIMS)
            .filter(|&y| sensitive[c][y])
            .map(|y| DIMS[y].0)
            .collect();
        let mut restrictions = Vec::new();
        for y in 0..N_DIMS {
            if !sensitive[c][y] {
                continue;
            }
            for z in 0..N_DIMS {
                if z == y {
                    continue;
                }
                let m = profile[c][y][z];
                let full = (1u8 << DIMS[z].1) - 1;
                if m != full {
                    let values: Vec<String> = (0..DIMS[z].1)
                        .filter(|&v| m & (1 << v) != 0)
                        .map(|v| dim_value_label(z, v))
                        .collect();
                    restrictions.push(serde_json::json!({
                        "depends_on": DIMS[y].0,
                        "only_when": DIMS[z].0,
                        "in": values,
                    }));
                }
            }
        }
        let witnesses: Vec<serde_json::Value> = (0..N_DIMS)
            .filter_map(|y| witness[c][y].as_ref())
            .map(|(_, w)| {
                let cell: serde_json::Value = DIMS
                    .iter()
                    .enumerate()
                    .map(|(i, (name, _))| {
                        (
                            name.to_string(),
                            serde_json::Value::String(dim_value_label(i, w.fiber[i])),
                        )
                    })
                    .collect::<serde_json::Map<_, _>>()
                    .into();
                serde_json::json!({
                    "varies": DIMS[w.dim].0,
                    "cell": cell,
                    "from": dim_value_label(w.dim, w.y1),
                    "to": dim_value_label(w.dim, w.y2),
                    "value_from": component_value_label(c, w.v1),
                    "value_to": component_value_label(c, w.v2),
                })
            })
            .collect();
        components.push(serde_json::json!({
            "component": component_name(c),
            "constant": support.is_empty(),
            "support": support,
            "restrictions": restrictions,
            "witnesses": witnesses,
        }));
    }

    let out = serde_json::json!({
        "domain": {
            "dims": dims_json,
            "constraint": "min_votes <= max_votes",
            "cells_evaluated": cells_seen,
        },
        "components": components,
    });
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, &out).expect("write JSON");
    stdout.flush().expect("flush");
}
