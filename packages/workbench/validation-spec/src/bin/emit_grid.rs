// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Grid evaluator for the characterization runners (via
//! `characterization/rust-spec.mjs`): reads a JSON array of cells from stdin,
//! writes the corresponding JSON array of outputs to stdout. Cell kinds:
//!
//!   {"kind": "f", "config": {...}, "voteState": {...}}
//!       → the full Effects record from `f` — the FROZEN ORACLE (production's
//!         bug-compatible behaviour); the "before" leg of the diff report
//!   {"kind": "fixed", "config": {...}, "voteState": {...}}
//!       → the full Effects record from `f_fixed` — the RATIONALIZED
//!         implementation (the query-provider); the "after" leg, for
//!         fix-diff.mjs
//!   {"kind": "hybrid", "config": {...}, "voteState": {...}}
//!       → the full Effects record of PRODUCTION AS CURRENTLY INJECTED —
//!         the per-component expectation the runners compare against while
//!         the injection proceeds site by site: emissions, gates, dialog
//!         and tally from `f_fixed` (decode and the gates are injected;
//!         the tally classifies from the decoded record, so it moves with
//!         decode), inline from the UNINJECTED TypeScript filter — the
//!         oracle's `inline_views` — applied to the FIXED emissions;
//!         reachability is shared (no fix touches it). When the filter is
//!         injected (S1's display fix) this collapses into `f_fixed` and
//!         the kind can retire.
//!   {"kind": "classify", "decline": b, "flag": b, "hasErrors": b,
//!    "selection": "none|regular|marker|mixed"}
//!       → {"tally": "<BallotClass>"} — probes the classifier with
//!         synthetic error states, as classifier-table.mjs does
//!   {"kind": "ballot", "contests": [{config, voteState}, …]}
//!       → {"hard": b, "soft": b} — the ORACLE gates' cross-contest OR (the
//!         free functions, so this stays a production-fidelity check), for
//!         ballot-gate-composition.mjs
//!
//! Deterministic and side-effect free: no clock, no randomness, no files.

use serde::Deserialize;
use std::io::Read;
use validation_spec::{
    classify, emissions, f, f_fixed, hard_gate, inline_views, soft_gate, Config, SelectionClass,
    VoteState,
};

#[derive(Deserialize)]
struct ContestCell {
    config: Config,
    #[serde(rename = "voteState")]
    vote_state: VoteState,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum Cell {
    #[serde(rename = "f")]
    F {
        config: Config,
        #[serde(rename = "voteState")]
        vote_state: VoteState,
    },
    #[serde(rename = "classify")]
    Classify {
        decline: bool,
        flag: bool,
        #[serde(rename = "hasErrors")]
        has_errors: bool,
        selection: SelectionClass,
    },
    #[serde(rename = "fixed")]
    Fixed {
        config: Config,
        #[serde(rename = "voteState")]
        vote_state: VoteState,
    },
    #[serde(rename = "hybrid")]
    Hybrid {
        config: Config,
        #[serde(rename = "voteState")]
        vote_state: VoteState,
    },
    #[serde(rename = "ballot")]
    Ballot { contests: Vec<ContestCell> },
}

fn main() {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("read stdin");
    let cells: Vec<Cell> = serde_json::from_str(&input).expect("parse cells JSON");
    let outputs: Vec<serde_json::Value> = cells
        .iter()
        .map(|cell| match cell {
            Cell::F { config, vote_state } => {
                serde_json::to_value(f(config, vote_state)).expect("serialize Effects")
            }
            Cell::Classify {
                decline,
                flag,
                has_errors,
                selection,
            } => {
                serde_json::json!({
                    "tally": classify(*decline, *flag, *has_errors, *selection)
                })
            }
            Cell::Fixed { config, vote_state } => {
                serde_json::to_value(f_fixed(config, vote_state)).expect("serialize Effects")
            }
            Cell::Hybrid { config, vote_state } => {
                // Production as currently injected (see the module doc):
                // everything from f_fixed except inline, which is the
                // uninjected oracle filter over the FIXED emissions.
                let mut effects = f_fixed(config, vote_state);
                effects.inline = inline_views(
                    &config.policies,
                    &effects.emissions.errors,
                    &effects.emissions.alerts,
                );
                serde_json::to_value(effects).expect("serialize Effects")
            }
            Cell::Ballot { contests } => {
                // The ORACLE gates' cross-contest OR (free functions): this
                // stays a production-fidelity check, so it must not read the
                // rationalized provider (which is meant to diverge).
                let gates: Vec<(bool, bool)> = contests
                    .iter()
                    .map(|c| {
                        let em = emissions(&c.config, &c.vote_state);
                        (
                            hard_gate(&c.config, &c.vote_state, &em),
                            soft_gate(&c.config, &c.vote_state, &em),
                        )
                    })
                    .collect();
                serde_json::json!({
                    "hard": gates.iter().any(|(h, _)| *h),
                    "soft": gates.iter().any(|(_, s)| *s),
                })
            }
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string(&outputs).expect("serialize outputs")
    );
}
