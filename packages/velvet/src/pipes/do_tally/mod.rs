// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod counting_algorithm;
mod error;
pub mod tally;

#[expect(
    clippy::module_inception,
    reason = "Preserve the existing module layout, re-exports and caller paths during the construct review"
)]
mod do_tally;
pub use do_tally::*;
