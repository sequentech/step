// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Vitrine, the transparency logs of election events: each transparent event
//! has a `ballots` log, which the electoral log's ballot box feeds through the
//! sidecar's queue, and an `events` log, where windmill appends the
//! configuration and milestone statements the booth and verifiers check.

pub mod client;
pub mod config;
pub mod contact;
pub mod hub_sites;
pub mod policy;
pub mod quorum;
pub mod statements;
pub mod transparency;
pub mod voter;
pub mod window;

use anyhow::{Context, Result};
use strum_macros::{AsRefStr, Display, EnumIter, EnumString};
use uuid::Uuid;

/// The logs of a transparent election event, by their names in its policy.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Display,
    EnumString,
    AsRefStr,
    EnumIter,
)]
#[strum(serialize_all = "lowercase")]
pub enum LogName {
    Ballots,
    Events,
}

/// An ID in the form the logs' entries and names use: a lowercase,
/// hyphenated UUID.
pub fn canonical_id(id: &str) -> Result<String> {
    Ok(Uuid::parse_str(id)
        .with_context(|| format!("{id:?} isn't a UUID"))?
        .hyphenated()
        .to_string())
}

/// A log's name on the sidecar, the path under its base URL:
/// `<tenant id>/<event id>/<log>`.
pub fn sidecar_name(tenant_id: &str, election_event_id: &str, log: LogName) -> Result<String> {
    Ok(format!(
        "{}/{}/{log}",
        canonical_id(tenant_id)?,
        canonical_id(election_event_id)?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sidecar_names_are_canonical_paths() {
        let name = sidecar_name(
            "90505C8A-23A9-4CDF-A26B-4E19F6A097D5",
            "{e7a2b3c4-0000-4000-8000-000000000001}",
            LogName::Events,
        )
        .unwrap();
        assert_eq!(
            name,
            "90505c8a-23a9-4cdf-a26b-4e19f6a097d5/e7a2b3c4-0000-4000-8000-000000000001/events"
        );
        assert!(sidecar_name("t", "e", LogName::Ballots).is_err());
    }
}
