// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Configurable monitoring dashboards: the configuration each election
//! event has, and every change to it.

/// The electoral log's record of each configuration change.
pub mod audit;

/// Reading an event's configuration, and saving, resetting and switching it.
pub mod config_store;

/// Sign-in attempts counted from the electoral log's Keycloak events.
pub mod login_counter;

/// `monitoring_voter`: what each voter counts as, refreshed from Keycloak,
/// their applications and their votes.
pub mod projection;

/// What each data source counts, from what a snapshot pass read.
pub mod producers;

/// The snapshot job's passes, and reading what they counted.
pub mod snapshot;
