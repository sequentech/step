// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `velvet-core` — pure-computation subset of the `velvet` tally crate.
//!
//! This crate is intended to compile to `wasm32-unknown-unknown` so the
//! ballot decoder and counting algorithms can run client-side in the
//! workbench. It must not depend (transitively or otherwise) on filesystem
//! I/O, async runtimes, sqlite, threads, or wall-clock time.
//!
//! Step 2a: extract tally result types and counting-algorithm error type.

pub mod counting;
pub mod decode;
pub mod result;

pub use result::{
    CandidateResult, ContestResult, ExtendedMetricsContest, InvalidVotes,
};
