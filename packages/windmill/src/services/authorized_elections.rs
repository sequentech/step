// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The `authorized-election-ids` voter attribute restricts a voter to some of
//! the election event's elections. Each value names an election by its
//! external ID, or by its ID when it has none: that is what the Keycloak token
//! mapper resolves and what the tally census matches.

use crate::services::election::ElectionHead;
use std::collections::HashMap;

/// The value that authorizes a voter for `election`.
pub fn authorized_election_id(election: &ElectionHead) -> &str {
    election
        .external_id
        .as_deref()
        .filter(|external_id| !external_id.is_empty())
        .unwrap_or(&election.id)
}

/// Resolves a reference to one of an event's elections, by external ID or by
/// ID, to the value that authorizes a voter for it.
#[derive(Debug, Default)]
pub struct AuthorizedElectionIds {
    by_reference: HashMap<String, String>,
}

impl AuthorizedElectionIds {
    pub fn new(elections: &[ElectionHead]) -> Self {
        let mut by_reference: HashMap<String, String> = elections
            .iter()
            .map(|election| {
                (
                    election.id.clone(),
                    authorized_election_id(election).to_string(),
                )
            })
            .collect();
        // Inserted last so that an external ID equal to another election's ID
        // names the election it belongs to, as it does in the token mapper.
        for election in elections {
            let value = authorized_election_id(election);
            by_reference.insert(value.to_string(), value.to_string());
        }
        AuthorizedElectionIds { by_reference }
    }

    /// Also resolves the IDs an election event import replaced, given as a map
    /// from the exported ID to the imported one.
    pub fn with_replaced_ids(mut self, replaced_ids: &HashMap<String, String>) -> Self {
        for (old_id, new_id) in replaced_ids {
            if let Some(value) = self.by_reference.get(new_id).cloned() {
                self.by_reference.entry(old_id.clone()).or_insert(value);
            }
        }
        self
    }

    pub fn resolve(&self, reference: &str) -> Option<&str> {
        self.by_reference.get(reference).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ELECTION_A: &str = "6f1c2d3e-4a5b-4c6d-8e7f-0a1b2c3d4e5f";
    const ELECTION_B: &str = "7a2b3c4d-5e6f-4a7b-9c8d-1e2f3a4b5c6d";
    const ELECTION_C: &str = "8b3c4d5e-6f7a-4b8c-ad9e-2f3a4b5c6d7e";

    fn election(id: &str, external_id: Option<&str>) -> ElectionHead {
        ElectionHead {
            id: id.to_string(),
            name: "-".to_string(),
            alias: None,
            external_id: external_id.map(str::to_string),
        }
    }

    #[test]
    fn elections_are_authorized_by_external_id_or_by_id_without_one() {
        assert_eq!(
            authorized_election_id(&election(ELECTION_A, Some("GIAMBI30-3-31"))),
            "GIAMBI30-3-31"
        );
        assert_eq!(
            authorized_election_id(&election(ELECTION_B, None)),
            ELECTION_B
        );
        assert_eq!(
            authorized_election_id(&election(ELECTION_C, Some(""))),
            ELECTION_C
        );
    }

    #[test]
    fn external_ids_and_ids_resolve_to_the_authorizing_value() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some("GIAMBI30-3-31")),
            election(ELECTION_B, None),
        ]);

        assert_eq!(elections.resolve("GIAMBI30-3-31"), Some("GIAMBI30-3-31"));
        assert_eq!(elections.resolve(ELECTION_A), Some("GIAMBI30-3-31"));
        assert_eq!(elections.resolve(ELECTION_B), Some(ELECTION_B));
        assert_eq!(elections.resolve(ELECTION_C), None);
        assert_eq!(elections.resolve("giambi30-3-31"), None);
        assert_eq!(elections.resolve(""), None);
    }

    #[test]
    fn an_external_id_equal_to_another_elections_id_names_its_own_election() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some(ELECTION_B)),
            election(ELECTION_B, Some("GIAMBI30-3-31")),
        ]);

        assert_eq!(elections.resolve(ELECTION_B), Some(ELECTION_B));
        assert_eq!(elections.resolve(ELECTION_A), Some(ELECTION_B));
    }

    #[test]
    fn ids_replaced_by_an_event_import_resolve_to_the_imported_elections() {
        let exported_a = "1d2e3f4a-5b6c-4d7e-8f9a-0b1c2d3e4f5a";
        let exported_b = "2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b";
        let exported_area = "3f4a5b6c-7d8e-4f9a-ab1c-2d3e4f5a6b7c";
        let replaced_ids = HashMap::from([
            (exported_a.to_string(), ELECTION_A.to_string()),
            (exported_b.to_string(), ELECTION_B.to_string()),
            (exported_area.to_string(), ELECTION_C.to_string()),
        ]);
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some("GIAMBI30-3-31")),
            election(ELECTION_B, None),
        ])
        .with_replaced_ids(&replaced_ids);

        assert_eq!(elections.resolve(exported_a), Some("GIAMBI30-3-31"));
        assert_eq!(elections.resolve(exported_b), Some(ELECTION_B));
        assert_eq!(elections.resolve(exported_area), None);
    }
}
