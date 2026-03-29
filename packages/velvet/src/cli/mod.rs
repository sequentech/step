// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Velvet CLI module: error handling, state, and test harness for CLI operations.
pub mod error;
pub mod state;
pub mod test_all;

mod cli;
pub use cli::*;
