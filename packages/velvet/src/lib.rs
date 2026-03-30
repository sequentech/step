// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Velvet is a crate for running election tally pipelines.
/// Command-line interface and configuration components.
pub mod cli;
pub mod config;
pub mod fixtures;
/// Pipeline processing modules for election tallying.
pub mod pipes;
/// Utility functions and helpers.
pub mod utils;
