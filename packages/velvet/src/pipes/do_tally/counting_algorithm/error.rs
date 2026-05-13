// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The counting-algorithm error type lives in `velvet-core` so it can be
// shared with WASM consumers (the workbench). Re-exported here to preserve
// the existing `crate::pipes::do_tally::counting_algorithm::error::{Error,
// Result}` import paths inside velvet.
pub use velvet_core::counting::error::{Error, Result};
