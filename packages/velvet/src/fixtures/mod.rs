//! Test fixtures for Velvet.
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// Area configuration fixtures.
mod areas;
/// Ballot style configuration fixtures.
pub mod ballot_styles;
/// Candidate configuration fixtures.
mod candidates;
/// Contest configuration fixtures.
mod contests;
/// Election configuration fixtures.
pub mod elections;
/// Test fixture management.
#[allow(clippy::module_inception)]
mod fixtures;

pub use fixtures::*;
