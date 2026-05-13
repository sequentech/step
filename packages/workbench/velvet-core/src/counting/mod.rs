// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Counting algorithms.
//!
//! Pure-computation tally machinery: the shared error type, the
//! `CountingAlgorithm` trait, the `Tally` data context, per-ballot
//! extended-metrics accumulators, and individual algorithm
//! implementations. None of these touch the filesystem, async runtimes,
//! or threads — file I/O orchestration lives in the `velvet` crate.

pub mod algorithm;
pub mod error;
pub mod extended_metrics;
pub mod instant_runoff;
pub mod plurality_at_large;
pub mod tally;

pub use algorithm::CountingAlgorithm;
pub use error::{Error, Result};
pub use extended_metrics::update_extended_metrics;
pub use instant_runoff::InstantRunoff;
pub use plurality_at_large::PluralityAtLarge;
pub use tally::{process_tally_sheet, Tally};
