// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Configurable monitoring dashboards: the configuration each election
//! event has, and every change to it.

/// The electoral log's record of each configuration change.
pub mod audit;

/// Reading an event's configuration, and saving, resetting and switching it.
pub mod config_store;
