// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! SQLite schema helpers and import routines for offline result storage.

/// Area hierarchy table creation and inserts.
pub mod area;
/// Area-to-contest assignment table creation and inserts.
pub mod area_contest;
/// Candidate table creation and CSV import.
pub mod candidate;
/// Contest table creation and inserts.
pub mod contests;
/// Election table creation and inserts.
pub mod election;
/// Election event table creation and inserts.
pub mod election_event;
/// Per-area contest results table creation and updates.
pub mod results_area_contest;
/// Per-candidate results within an area-contest.
pub mod results_area_contest_candidate;
/// Contest-level aggregated results.
pub mod results_contest;
/// Candidate-level contest results.
pub mod results_contest_candidate;
/// Election-level aggregated results.
pub mod results_election;
/// Election results broken down by area.
pub mod results_election_area;
/// Top-level results event metadata.
pub mod results_event;
/// Tally session resolution records.
pub mod tally_session_resolution;
/// Shared SQLite serialization helpers.
pub mod utils;
