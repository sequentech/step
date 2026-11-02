// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The monitoring cadence this process runs with, read once from the
//! environment. What the settings mean and their bounds:
//! [`sequent_core::monitoring::cadence`].

use sequent_core::monitoring::cadence::{Cadence, SNAPSHOT_INTERVAL_ENV, VOTER_FULL_PASS_ENV};
use std::sync::OnceLock;
use tracing::{info, warn};

/// The cadence as the environment sets it, each value that is not used as
/// configured logged once.
pub fn from_env() -> Cadence {
    let cadence = Cadence::parse(
        std::env::var(SNAPSHOT_INTERVAL_ENV).ok().as_deref(),
        std::env::var(VOTER_FULL_PASS_ENV).ok().as_deref(),
    );
    log(&cadence);
    cadence
}

/// Logs what the cadence is, and every value it had to replace.
pub fn log(cadence: &Cadence) {
    for warning in cadence.warnings() {
        warn!("Monitoring cadence: {warning}");
    }
    info!(
        "Monitoring cadence: {}, {}",
        cadence.snapshot_interval, cadence.voter_full_pass
    );
}

/// This process's cadence: [`from_env`], read the first time it is asked.
pub fn configured() -> &'static Cadence {
    static CADENCE: OnceLock<Cadence> = OnceLock::new();
    CADENCE.get_or_init(from_env)
}
