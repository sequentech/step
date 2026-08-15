// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Grid evaluator for the conformance harness
//! (`characterization/rust-conformance.mjs`): reads a JSON array of cells
//! from stdin, writes the corresponding JSON array of outputs to stdout.
//! Two cell kinds:
//!
//!   {"kind": "f", "config": {...}, "voteState": {...}}
//!       → the full Effects record (same field names as spec.mjs `f`)
//!   {"kind": "classify", "decline": b, "flag": b, "hasErrors": b,
//!    "selection": "none|regular|marker|mixed"}
//!       → {"tally": "<BallotClass>"} — probes the classifier with
//!         synthetic error states, as classifier-table.mjs does
//!
//! Deterministic and side-effect free: no clock, no randomness, no files.

use serde::Deserialize;
use std::io::Read;
use validation_spec::{classify, f, Config, SelectionClass, VoteState};

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
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string(&outputs).expect("serialize outputs")
    );
}
