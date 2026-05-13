// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Re-export of the instant-runoff algorithm, now defined in
//! `velvet-core`. Kept here so existing in-crate import paths
//! (`crate::pipes::do_tally::counting_algorithm::instant_runoff::*`)
//! continue to work.

pub use velvet_core::counting::instant_runoff::*;
