// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![doc = include_str!("../lib.docs.md")]

/// Command-line interface for the `velvet` binary: argument parsing, per-stage execution state, errors, and the `test-all` validation harness.
pub mod cli;
/// Election and pipeline configuratio.
pub mod config;
/// Test fixtures—synthetic elections, areas, contests, candidates, and ballot styles shared across `velvet` tests.
pub mod fixtures;
/// Tally pipeline stages and orchestration.
pub mod pipes;
/// Shared helpers for pipeline stages.
pub mod utils;
