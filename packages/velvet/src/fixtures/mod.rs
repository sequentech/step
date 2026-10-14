// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod areas;
pub mod ballot_styles;
mod candidates;
mod contests;
pub mod elections;
#[expect(
    clippy::module_inception,
    reason = "Preserve the existing module layout, re-exports and caller paths during the construct review"
)]
pub mod fixtures;

pub use fixtures::*;
