// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/// `SQLite` access layer for areas.
pub mod area;

/// `SQLite` access layer for the relationship between areas and contests.
pub mod area_contest;

/// `SQLite` access layer for candidates participating in contests.
pub mod candidate;

/// `SQLite` access layer for contest.
pub mod contests;

/// `SQLite` access layer for election.
pub mod election;

/// `SQLite` access layer for election events.
pub mod election_event;

/// `SQLite` access layer for aggregated results per area and contest.
pub mod results_area_contest;

/// `SQLite` access layer for candidate-level results within an area contest.
pub mod results_area_contest_candidate;

/// `SQLite` access layer for overall contest results.
pub mod results_contest;

/// `SQLite` access layer for candidate-level results within a contest.
pub mod results_contest_candidate;

/// `SQLite` access layer for overall election results.
pub mod results_election;

/// `SQLite` access layer for results aggregated by election and area.
pub mod results_election_area;

/// `SQLite` access layer for event-based results and updates.
pub mod results_event;

/// `SQLite` access layer for tally session resolutions and outcomes.
pub mod tally_session_resolution;

/// Shared helpers
pub mod utils;
