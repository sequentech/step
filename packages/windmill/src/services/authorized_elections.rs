// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The `authorized-election-ids` voter attribute restricts a voter to some of
//! the election event's elections. The Keycloak token mapper looks each value
//! up among the elections' external IDs, and then among their IDs. A value that
//! several elections have as external ID names none of them.

use crate::services::election::ElectionHead;
use sequent_core::services::keycloak::MULTIVALUE_USER_ATTRIBUTE_SEPARATOR;
use std::collections::HashMap;
use std::fmt;

/// The external ID the token mapper looks `election` up by, if it has one.
fn external_id(election: &ElectionHead) -> Option<&str> {
    election
        .external_id
        .as_deref()
        .filter(|external_id| !external_id.is_empty())
}

/// Starts the values export writes for stored values that do not name a
/// single election. Import rejects them.
const QUOTE: char = '"';

/// A spreadsheet takes a cell that starts with one of these for a formula.
const FORMULA_PREFIXES: [char; 4] = ['=', '+', '-', '@'];

/// Keycloak keeps attribute values in `user_attribute.value`, a 255-character
/// column, which the voters import writes and the tally census reads.
const MAX_ATTRIBUTE_VALUE_CHARS: usize = 255;

/// Whether `value` can be stored as an `authorized-election-ids` value: it fits
/// in the attribute, and reads back unchanged from a voters CSV cell, whose
/// values are separated by `|` and trimmed, without a spreadsheet taking it for
/// a formula or import for a quoted value.
fn can_be_stored(value: &str) -> bool {
    value.chars().count() <= MAX_ATTRIBUTE_VALUE_CHARS
        && !value.is_empty()
        && value.trim() == value
        && !value.contains(MULTIVALUE_USER_ATTRIBUTE_SEPARATOR)
        && !value.starts_with(QUOTE)
        && !value.starts_with(FORMULA_PREFIXES)
}

/// How export writes a stored value that does not name a single election, so
/// that importing it into any election event fails: in double quotes.
pub(crate) fn unresolved_cell_value(value: &str) -> String {
    format!("{QUOTE}{}{QUOTE}", value.escape_debug())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnresolvedElection {
    NoElection,
    SeveralElections,
    NoStorableValue,
    Quoted,
}

impl fmt::Display for UnresolvedElection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnresolvedElection::NoElection => {
                "no election in this election event has that external ID or ID"
            }
            UnresolvedElection::SeveralElections => {
                "more than one election in this election event has that external ID or ID"
            }
            UnresolvedElection::NoStorableValue => {
                "the election it names can be stored neither by its external ID nor by its \
                 ID, which another election has as external ID"
            }
            UnresolvedElection::Quoted => {
                "a value that starts with a double quote names no election"
            }
        })
    }
}

/// Resolves `authorized-election-ids` values as the token mapper does, to the
/// value stored for the election each one names: its external ID, unless
/// another election shares it or it cannot be stored, and otherwise its ID.
#[derive(Debug, Default)]
pub struct AuthorizedElectionIds {
    /// The IDs of the elections each value names.
    elections_named: HashMap<String, Vec<String>>,
    /// The value stored for each election, by ID, if one names it alone.
    stored_values: HashMap<String, Option<String>>,
}

impl AuthorizedElectionIds {
    pub fn new(elections: &[ElectionHead]) -> Self {
        let mut elections_named: HashMap<String, Vec<String>> = HashMap::new();
        for election in elections {
            if let Some(external_id) = external_id(election) {
                elections_named
                    .entry(external_id.to_string())
                    .or_default()
                    .push(election.id.clone());
            }
        }
        // After the external IDs, which take precedence over an equal ID.
        for election in elections {
            elections_named
                .entry(election.id.clone())
                .or_insert_with(|| vec![election.id.clone()]);
        }
        let names_alone = |value: &str, election: &ElectionHead| {
            matches!(
                elections_named.get(value).map(Vec::as_slice),
                Some([id]) if *id == election.id
            )
        };
        let stored_values = elections
            .iter()
            .map(|election| {
                let value = election
                    .external_id
                    .iter()
                    .chain([&election.id])
                    .find(|value| can_be_stored(value) && names_alone(value.as_str(), election))
                    .cloned();
                (election.id.clone(), value)
            })
            .collect();
        AuthorizedElectionIds {
            elections_named,
            stored_values,
        }
    }

    /// Also resolves the values an election event import replaced, given as a
    /// map from the exported value to the imported one, as the imported one.
    /// The import replaces every value shaped like an ID, external IDs too.
    pub fn with_replaced_ids(mut self, replaced_ids: &HashMap<String, String>) -> Self {
        let replaced: Vec<(String, Vec<String>)> = replaced_ids
            .iter()
            .filter_map(|(old_value, new_value)| {
                let election_ids = self.elections_named.get(new_value)?;
                Some((old_value.clone(), election_ids.clone()))
            })
            .collect();
        for (old_value, election_ids) in replaced {
            self.elections_named
                .entry(old_value)
                .or_insert(election_ids);
        }
        self
    }

    /// The value stored for the election that `reference` names.
    pub fn resolve(&self, reference: &str) -> Result<&str, UnresolvedElection> {
        match self.elections_named.get(reference).map(Vec::as_slice) {
            Some([id]) => self
                .stored_value(id)
                .ok_or(UnresolvedElection::NoStorableValue),
            Some([_, _, ..]) => Err(UnresolvedElection::SeveralElections),
            _ => Err(UnresolvedElection::NoElection),
        }
    }

    /// The value stored for the election that `reference`, read from a voters
    /// CSV cell, names. A reference that starts with a double quote names none,
    /// as export quotes values that do not name a single election.
    pub fn resolve_imported(&self, reference: &str) -> Result<&str, UnresolvedElection> {
        if reference.starts_with(QUOTE) {
            return Err(UnresolvedElection::Quoted);
        }
        self.resolve(reference)
    }

    /// The value stored for the election with ID `election_id`.
    pub fn stored_value(&self, election_id: &str) -> Option<&str> {
        self.stored_values
            .get(election_id)
            .and_then(Option::as_deref)
    }

    /// The values the token mapper resolves to `election`, and an external ID
    /// it shares with other elections, which earlier token mappers resolved to
    /// one of them.
    fn census_values(&self, election: &ElectionHead) -> Vec<String> {
        let mut values: Vec<String> = external_id(election)
            .into_iter()
            .chain([election.id.as_str()])
            .filter(|value| {
                self.elections_named
                    .get(*value)
                    .is_some_and(|ids| ids.contains(&election.id))
            })
            .map(str::to_string)
            .collect();
        values.dedup();
        values
    }
}

/// The `authorized-election-ids` values that each election's census matches,
/// by election ID.
pub fn census_values_by_election(elections: &[ElectionHead]) -> HashMap<String, Vec<String>> {
    let authorized_elections = AuthorizedElectionIds::new(elections);
    elections
        .iter()
        .map(|election| {
            (
                election.id.clone(),
                authorized_elections.census_values(election),
            )
        })
        .collect()
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
    fn elections_are_stored_by_external_id_or_by_id_without_one() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some("GIAMBI30-3-31")),
            election(ELECTION_B, None),
            election(ELECTION_C, Some("")),
        ]);

        assert_eq!(elections.resolve("GIAMBI30-3-31"), Ok("GIAMBI30-3-31"));
        assert_eq!(elections.resolve(ELECTION_A), Ok("GIAMBI30-3-31"));
        assert_eq!(elections.resolve(ELECTION_B), Ok(ELECTION_B));
        assert_eq!(elections.resolve(ELECTION_C), Ok(ELECTION_C));
        assert_eq!(elections.stored_value(ELECTION_A), Some("GIAMBI30-3-31"));
        assert_eq!(elections.stored_value(ELECTION_B), Some(ELECTION_B));
    }

    #[test]
    fn values_matching_no_election_are_not_resolved() {
        let elections = AuthorizedElectionIds::new(&[election(ELECTION_A, Some("GIAMBI30-3-31"))]);

        for value in [ELECTION_B, "giambi30-3-31", ""] {
            assert_eq!(
                elections.resolve(value),
                Err(UnresolvedElection::NoElection)
            );
        }
    }

    /// Stored, they would read back as other values, or as a blank cell that
    /// leaves the voter unrestricted.
    #[test]
    fn external_ids_that_do_not_fit_in_a_cell_are_stored_by_id() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some("GIAMBI30-3-31|GIAMBI30-3-32")),
            election(ELECTION_B, Some(" ")),
            election(ELECTION_C, Some(" GIAMBI30-3-31")),
        ]);

        assert_eq!(
            elections.resolve("GIAMBI30-3-31|GIAMBI30-3-32"),
            Ok(ELECTION_A)
        );
        assert_eq!(elections.resolve(" "), Ok(ELECTION_B));
        assert_eq!(elections.resolve(ELECTION_C), Ok(ELECTION_C));
        assert_eq!(elections.stored_value(ELECTION_C), Some(ELECTION_C));
    }

    /// Import rejects values that start with a double quote, which export
    /// writes for values that name no election.
    #[test]
    fn external_ids_starting_with_a_double_quote_are_stored_by_id() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some("\"GIAMBI30-3-31")),
            election(ELECTION_B, Some("GIAMBI30-3-31\"")),
        ]);

        assert_eq!(elections.resolve("\"GIAMBI30-3-31"), Ok(ELECTION_A));
        assert_eq!(elections.stored_value(ELECTION_A), Some(ELECTION_A));
        assert_eq!(elections.stored_value(ELECTION_B), Some("GIAMBI30-3-31\""));
    }

    /// A spreadsheet would take them for formulas.
    #[test]
    fn external_ids_starting_like_a_formula_are_stored_by_id() {
        for external_id in ["=1+1", "+34", "-1", "@SUM(A1)"] {
            let elections = AuthorizedElectionIds::new(&[election(ELECTION_A, Some(external_id))]);

            assert_eq!(elections.resolve(external_id), Ok(ELECTION_A));
            assert_eq!(elections.stored_value(ELECTION_A), Some(ELECTION_A));
        }
    }

    /// The voters import writes each value to a Keycloak attribute, which
    /// holds 255 characters, however many bytes they take.
    #[test]
    fn external_ids_longer_than_an_attribute_value_are_stored_by_id() {
        let longest = "é".repeat(MAX_ATTRIBUTE_VALUE_CHARS);
        let too_long = format!("{longest}1");
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some(&longest)),
            election(ELECTION_B, Some(&too_long)),
        ]);

        assert_eq!(elections.stored_value(ELECTION_A), Some(longest.as_str()));
        assert_eq!(elections.resolve(&too_long), Ok(ELECTION_B));
        assert_eq!(elections.stored_value(ELECTION_B), Some(ELECTION_B));
    }

    /// The token mapper resolves a shared external ID to none of them.
    #[test]
    fn elections_sharing_an_external_id_are_stored_by_id() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some("GIAMBI30-3-31")),
            election(ELECTION_B, Some("GIAMBI30-3-31")),
        ]);

        assert_eq!(elections.resolve(ELECTION_A), Ok(ELECTION_A));
        assert_eq!(elections.resolve(ELECTION_B), Ok(ELECTION_B));
        assert_eq!(
            elections.resolve("GIAMBI30-3-31"),
            Err(UnresolvedElection::SeveralElections)
        );
    }

    #[test]
    fn an_external_id_equal_to_another_elections_id_names_its_own_election() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some(ELECTION_B)),
            election(ELECTION_B, Some("GIAMBI30-3-31")),
        ]);

        assert_eq!(elections.resolve(ELECTION_B), Ok(ELECTION_B));
        assert_eq!(elections.resolve(ELECTION_A), Ok(ELECTION_B));
        assert_eq!(elections.stored_value(ELECTION_A), Some(ELECTION_B));
        assert_eq!(elections.resolve("GIAMBI30-3-31"), Ok("GIAMBI30-3-31"));
    }

    /// As in the token mapper, no value is left to name the other election.
    #[test]
    fn an_external_id_equal_to_the_id_of_an_election_without_one_names_its_own_election() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some(ELECTION_B)),
            election(ELECTION_B, None),
        ]);

        assert_eq!(elections.stored_value(ELECTION_A), Some(ELECTION_B));
        assert_eq!(elections.resolve(ELECTION_A), Ok(ELECTION_B));
        assert_eq!(elections.resolve(ELECTION_B), Ok(ELECTION_B));
        assert_eq!(elections.stored_value(ELECTION_B), None);
    }

    /// Its external ID cannot be stored, and its ID names the other election.
    #[test]
    fn an_election_with_no_value_that_can_be_stored_is_not_resolved() {
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some("=GIAMBI30-3-31")),
            election(ELECTION_B, Some(ELECTION_A)),
        ]);

        assert_eq!(elections.stored_value(ELECTION_A), None);
        assert_eq!(
            elections.resolve("=GIAMBI30-3-31"),
            Err(UnresolvedElection::NoStorableValue)
        );
        assert_eq!(elections.stored_value(ELECTION_B), Some(ELECTION_A));
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

        assert_eq!(elections.resolve(exported_a), Ok("GIAMBI30-3-31"));
        assert_eq!(elections.resolve(exported_b), Ok(ELECTION_B));
        assert_eq!(
            elections.resolve(exported_area),
            Err(UnresolvedElection::NoElection)
        );
    }

    /// An election event import replaces every value shaped like an ID,
    /// external IDs included, so each value named in the exported event what
    /// its replacement names in the imported one.
    #[test]
    fn values_replaced_by_an_event_import_resolve_as_their_replacements() {
        let exported_a = "1d2e3f4a-5b6c-4d7e-8f9a-0b1c2d3e4f5a";
        let exported_b = "2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b";
        let exported_external_id = "4a5b6c7d-8e9f-4a0b-9c1d-2e3f4a5b6c7d";
        let imported_external_id = "5b6c7d8e-9f0a-4b1c-8d2e-3f4a5b6c7d8e";
        let replaced_ids = HashMap::from([
            (exported_a.to_string(), ELECTION_A.to_string()),
            (exported_b.to_string(), ELECTION_B.to_string()),
            (
                exported_external_id.to_string(),
                imported_external_id.to_string(),
            ),
        ]);
        let elections = AuthorizedElectionIds::new(&[
            election(ELECTION_A, Some(ELECTION_B)),
            election(ELECTION_B, Some(imported_external_id)),
        ])
        .with_replaced_ids(&replaced_ids);

        assert_eq!(
            elections.resolve(exported_external_id),
            Ok(imported_external_id)
        );
        for (exported, imported) in &replaced_ids {
            assert_eq!(
                elections.resolve(exported),
                elections.resolve(imported),
                "{exported} must resolve as {imported}"
            );
        }
    }

    #[test]
    fn the_census_matches_the_external_id_and_the_id() {
        let census = census_values_by_election(&[
            election(ELECTION_A, Some("GIAMBI30-3-31")),
            election(ELECTION_B, None),
            election(ELECTION_C, Some("")),
        ]);

        assert_eq!(
            census,
            HashMap::from([
                (
                    ELECTION_A.to_string(),
                    vec!["GIAMBI30-3-31".to_string(), ELECTION_A.to_string()]
                ),
                (ELECTION_B.to_string(), vec![ELECTION_B.to_string()]),
                (ELECTION_C.to_string(), vec![ELECTION_C.to_string()]),
            ])
        );
    }

    /// The token mapper resolves the ID to the election whose external ID it
    /// is, so the other election's census must not match it.
    #[test]
    fn the_census_does_not_match_an_id_another_election_has_as_external_id() {
        let census = census_values_by_election(&[
            election(ELECTION_A, Some(ELECTION_B)),
            election(ELECTION_B, Some("GIAMBI30-3-31")),
        ]);

        assert_eq!(
            census[ELECTION_A],
            vec![ELECTION_B.to_string(), ELECTION_A.to_string()]
        );
        assert_eq!(census[ELECTION_B], vec!["GIAMBI30-3-31".to_string()]);
    }

    /// No value restricts a voter to the other election, so its census matches
    /// only unrestricted voters.
    #[test]
    fn the_census_of_an_election_no_value_names_matches_none() {
        let census = census_values_by_election(&[
            election(ELECTION_A, Some(ELECTION_B)),
            election(ELECTION_B, None),
        ]);

        assert_eq!(
            census[ELECTION_A],
            vec![ELECTION_B.to_string(), ELECTION_A.to_string()]
        );
        assert!(census[ELECTION_B].is_empty());
    }

    /// Earlier token mappers resolved a shared external ID to one of them, so
    /// both censuses match it rather than leave out the ballots cast then.
    #[test]
    fn the_census_of_elections_sharing_an_external_id_matches_it() {
        let census = census_values_by_election(&[
            election(ELECTION_A, Some("GIAMBI30-3-31")),
            election(ELECTION_B, Some("GIAMBI30-3-31")),
        ]);

        assert_eq!(
            census[ELECTION_A],
            vec!["GIAMBI30-3-31".to_string(), ELECTION_A.to_string()]
        );
        assert_eq!(
            census[ELECTION_B],
            vec!["GIAMBI30-3-31".to_string(), ELECTION_B.to_string()]
        );
    }
}
