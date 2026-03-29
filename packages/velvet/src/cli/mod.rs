//! Velvet CLI module: error handling, state, and test harness for CLI operations.
// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod error;
pub mod state;
pub mod test_all;

/// Private CLI module containing command handling.
#[allow(clippy::module_inception)]
mod cli;
pub use cli::*;
