// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Re-exports of the counting-algorithm trait and error type now defined
//! in `velvet-core`. Kept here so existing in-crate import paths
//! (`crate::pipes::do_tally::counting_algorithm::*`) continue to work.

pub use super::error::{Error, Result};
pub use velvet_core::counting::CountingAlgorithm;
