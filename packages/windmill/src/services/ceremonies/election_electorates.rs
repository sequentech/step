// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Each election's electorate, recorded by a tally session when it extracts
//! the ballots.

use crate::postgres::tally_session::merge_tally_session_annotation;
use crate::services::election::get_election_event_elections;
use crate::services::users::count_keycloak_enabled_voters_of_areas;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::{TallySession, TallySessionContest};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tracing::instrument;

/// The tally session annotation that records each election's electorate, as
/// a JSON string like the other values of those annotations.
pub const ELECTION_ELECTORATES_ANNOTATION: &str = "election_electorates";

/// Each election's eligible voters, by election id: its enabled voters in
/// the areas of its contest areas, each counted once.
pub type ElectionElectorates = BTreeMap<String, u64>;

/// The areas of each of `election_ids`, from the session's contest areas.
/// Every area-contest association of the session's elections has one.
pub fn electorate_areas(
    election_ids: &[String],
    contests: &[TallySessionContest],
) -> BTreeMap<String, Vec<String>> {
    election_ids
        .iter()
        .map(|election_id| {
            let areas: BTreeSet<&String> = contests
                .iter()
                .filter(|contest| &contest.election_id == election_id)
                .map(|contest| &contest.area_id)
                .collect();
            (election_id.clone(), areas.into_iter().cloned().collect())
        })
        .collect()
}

/// How a voter's authorized elections name an election, as Keycloak's
/// authorized-elections mapper does: by its alias, or by its id when it has
/// none.
pub fn authorization_alias(election_id: &str, alias: Option<&str>) -> String {
    alias
        .filter(|alias| !alias.is_empty())
        .unwrap_or(election_id)
        .to_string()
}

/// Whether this run records the session's electorates: only when it takes the
/// census of the session's contest areas for the first time (`contests` as the
/// run found them, `dumped` when it extracted ballots). A session that took
/// its census before electorates were recorded keeps that census on every
/// later run.
pub fn records_electorates(recorded: bool, contests: &[TallySessionContest], dumped: bool) -> bool {
    !recorded
        && dumped
        && !contests.is_empty()
        && contests.iter().all(|contest| contest.annotations.is_none())
}

/// Counts and records the electorate of each election of the session.
#[instrument(skip_all, err)]
pub async fn record_election_electorates(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tally_session: &TallySession,
    contests: &[TallySessionContest],
) -> Result<ElectionElectorates> {
    let tenant_id = &tally_session.tenant_id;
    let election_event_id = &tally_session.election_event_id;
    let election_ids = tally_session.election_ids.clone().unwrap_or_default();
    let aliases: HashMap<String, String> =
        get_election_event_elections(hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .map(|election| {
                let alias = authorization_alias(&election.id, election.external_id.as_deref());
                (election.id, alias)
            })
            .collect();
    let realm = get_event_realm(tenant_id, election_event_id);

    let mut electorates = ElectionElectorates::new();
    for (election_id, areas) in electorate_areas(&election_ids, contests) {
        let alias = aliases
            .get(&election_id)
            .cloned()
            .unwrap_or_else(|| election_id.clone());
        let electorate = count_keycloak_enabled_voters_of_areas(
            keycloak_transaction,
            &realm,
            &areas,
            &[],
            &alias,
        )
        .await?;
        electorates.insert(election_id, electorate);
    }
    merge_tally_session_annotation(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &tally_session.id,
        ELECTION_ELECTORATES_ANNOTATION,
        Value::String(serde_json::to_string(&electorates)?),
    )
    .await?;
    Ok(electorates)
}

/// The annotations of an imported tally session, with the elections
/// its recorded electorates name replaced through `replacement`, the import's id
/// map, as the import replaces them everywhere else. An id the map does not
/// have is an error, as it is for the session's own elections and areas.
pub fn remap_recorded_ids(
    annotations: Option<Value>,
    replacement: &HashMap<String, String>,
) -> Result<Option<Value>> {
    let Some(mut annotations) = annotations else {
        return Ok(None);
    };
    let replace = |id: &String| {
        replacement
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("Can't find id: {id} in replacement map"))
    };
    if let Some(electorates) = read_recorded::<ElectionElectorates>(
        Some(&annotations),
        ELECTION_ELECTORATES_ANNOTATION,
        "being imported",
    )? {
        let electorates = electorates
            .into_iter()
            .map(|(election_id, electorate)| Ok((replace(&election_id)?, electorate)))
            .collect::<Result<ElectionElectorates>>()?;
        annotations[ELECTION_ELECTORATES_ANNOTATION] =
            Value::String(serde_json::to_string(&electorates)?);
    }
    Ok(Some(annotations))
}

/// Reads the session annotation `key`, a JSON string, as `T`.
fn read_recorded<T: serde::de::DeserializeOwned>(
    annotations: Option<&Value>,
    key: &str,
    tally_session_id: &str,
) -> Result<Option<T>> {
    annotations
        .and_then(|annotations| annotations.get(key))
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| anyhow!("not a string"))
                .and_then(|value| Ok(serde_json::from_str(value)?))
                .with_context(|| format!("the {key} of tally session {tally_session_id}"))
        })
        .transpose()
}

/// Refuses a recorded map that names elections other than the session's own:
/// it was recorded for the session's elections, so one that names others is
/// inconsistent, and counting it, or counting without it, would change the
/// session's census silently.
fn of_session<'a>(
    tally_session: &TallySession,
    key: &str,
    mut elections: impl Iterator<Item = &'a String>,
) -> Result<()> {
    let session_elections = tally_session.election_ids.as_deref().unwrap_or_default();
    match elections.find(|election_id| !session_elections.contains(election_id)) {
        None => Ok(()),
        Some(election_id) => Err(anyhow!(
            "The {key} of tally session {} name election {election_id}, which is not one of \
             its elections",
            tally_session.id
        )),
    }
}

/// The electorates the session recorded for its elections, or `None` for a
/// session that recorded none.
pub fn recorded_election_electorates(
    tally_session: &TallySession,
) -> Result<Option<ElectionElectorates>> {
    let electorates = read_recorded::<ElectionElectorates>(
        tally_session.annotations.as_ref(),
        ELECTION_ELECTORATES_ANNOTATION,
        &tally_session.id,
    )?;
    if let Some(electorates) = &electorates {
        of_session(
            tally_session,
            ELECTION_ELECTORATES_ANNOTATION,
            electorates.keys(),
        )?;
    }
    Ok(electorates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::consolidation::eml_generator::{
        prepend_miru_annotation, ValidateAnnotations, MIRU_TALLY_SESSION_DATA,
    };

    fn contest_area(
        election_id: &str,
        area_id: &str,
        contest_id: &str,
        annotations: Option<Value>,
    ) -> TallySessionContest {
        TallySessionContest {
            id: format!("{election_id}-{area_id}-{contest_id}"),
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            area_id: area_id.into(),
            contest_id: Some(contest_id.into()),
            session_id: 1,
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations,
            tally_session_id: "session".into(),
            election_id: election_id.into(),
        }
    }

    fn census() -> Option<Value> {
        Some(serde_json::json!({
            "casted_ballots": 0,
            "elegible_voters": 1,
            "ballots_without_voter": 0
        }))
    }

    fn session(annotations: Option<Value>) -> TallySession {
        TallySession {
            id: "session".into(),
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations,
            election_ids: Some(vec!["other".into(), "post".into()]),
            area_ids: None,
            is_execution_completed: false,
            keys_ceremony_id: "keys".into(),
            execution_status: None,
            threshold: 2,
            configuration: None,
            tally_type: None,
            permission_label: None,
        }
    }

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn recorded(electorates: &ElectionElectorates) -> Value {
        serde_json::json!({
            "executer_username": "admin",
            ELECTION_ELECTORATES_ANNOTATION: serde_json::to_string(electorates).unwrap(),
        })
    }

    #[test]
    fn an_electorate_has_each_area_of_the_election_once() {
        // `post` has two contests in north, so two contest areas there.
        let contests = vec![
            contest_area("post", "north", "c1", None),
            contest_area("post", "north", "c2", None),
            contest_area("post", "south", "c1", None),
            contest_area("other", "west", "c3", None),
        ];

        let areas = electorate_areas(&ids(&["post", "other", "missing"]), &contests);

        assert_eq!(
            areas,
            BTreeMap::from([
                ("post".to_string(), ids(&["north", "south"])),
                ("other".to_string(), ids(&["west"])),
                ("missing".to_string(), vec![]),
            ])
        );
    }

    #[test]
    fn an_election_is_named_as_keycloak_authorizes_it() {
        assert_eq!(
            authorization_alias("election-uuid", Some("post-1")),
            "post-1"
        );
        // Keycloak names an election without an alias by its id, and so do
        // the voters' authorized elections.
        assert_eq!(authorization_alias("election-uuid", None), "election-uuid");
        assert_eq!(
            authorization_alias("election-uuid", Some("")),
            "election-uuid"
        );
    }

    #[test]
    fn electorates_are_recorded_only_with_the_first_census_of_the_session() {
        let fresh = vec![
            contest_area("post", "north", "c1", None),
            contest_area("post", "south", "c1", None),
        ];
        let partly_dumped = vec![
            contest_area("post", "north", "c1", census()),
            contest_area("post", "south", "c1", None),
        ];
        let dumped = vec![
            contest_area("post", "north", "c1", census()),
            contest_area("post", "south", "c1", census()),
        ];

        // The run that takes the session's first census.
        assert!(records_electorates(false, &fresh, true));
        // Already recorded: a retry or a later run.
        assert!(!records_electorates(true, &fresh, true));
        assert!(!records_electorates(true, &dumped, false));
        // Areas dumped before electorates were recorded, then the rest: some
        // of the session's census predates them, so none is recorded.
        assert!(!records_electorates(false, &partly_dumped, true));
        // Every area dumped before electorates were recorded: a later run keeps
        // the census it had.
        assert!(!records_electorates(false, &dumped, false));
        // Nothing to extract.
        assert!(!records_electorates(false, &[], false));
        assert!(!records_electorates(false, &[], true));
    }

    #[test]
    fn the_recorded_electorates_are_read_back() {
        let electorates = ElectionElectorates::from([("post".to_string(), 7)]);
        assert_eq!(recorded_election_electorates(&session(None)).unwrap(), None);
        assert_eq!(
            recorded_election_electorates(&session(Some(
                serde_json::json!({"executer_username": "admin"})
            )))
            .unwrap(),
            None
        );
        assert_eq!(
            recorded_election_electorates(&session(Some(recorded(&electorates)))).unwrap(),
            Some(electorates)
        );
        for unreadable in [
            serde_json::json!({"post": 7}),
            serde_json::json!("{\"post\": \"many\"}"),
        ] {
            assert!(
                recorded_election_electorates(&session(Some(serde_json::json!({
                    ELECTION_ELECTORATES_ANNOTATION: unreadable
                }))))
                .is_err()
            );
        }
    }

    #[test]
    fn the_transmission_packages_still_read_a_session_with_electorates() {
        // The transmission packages read the session's annotations as
        // strings, and one that is not a string hides every package.
        let electorates = ElectionElectorates::from([("post".to_string(), 7)]);
        let mut annotations = recorded(&electorates);
        annotations[prepend_miru_annotation(MIRU_TALLY_SESSION_DATA)] = serde_json::json!([{
            "election_id": "post",
            "area_id": "north",
            "servers": [],
            "documents": [],
            "logs": [],
            "threshold": 2
        }])
        .to_string()
        .into();
        let with_packages = session(Some(annotations));

        for packages in [
            with_packages.get_annotations().unwrap(),
            with_packages.get_annotations_or_empty_values().unwrap(),
        ] {
            assert_eq!(packages.len(), 1);
            assert_eq!(packages[0].area_id, "north");
        }
        assert_eq!(
            recorded_election_electorates(&with_packages).unwrap(),
            Some(electorates)
        );
    }

    #[test]
    fn a_recorded_map_naming_other_elections_is_refused() {
        // Recorded for elections the session does not have: inconsistent.
        let electorates = ElectionElectorates::from([("exported-post".to_string(), 7)]);
        let imported = session(Some(serde_json::json!({
            ELECTION_ELECTORATES_ANNOTATION: serde_json::to_string(&electorates).unwrap(),
        })));
        assert!(recorded_election_electorates(&imported).is_err());
    }
}
