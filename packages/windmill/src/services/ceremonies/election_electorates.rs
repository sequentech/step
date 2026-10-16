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
use sequent_core::types::ceremonies::TallyRunReason;
use sequent_core::types::hasura::core::{
    BallotStyle, TallySession, TallySessionContest, TallySessionExecution,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tracing::instrument;

/// The tally session annotation that records each election's electorate, as
/// a JSON string like the other values of those annotations.
pub const ELECTION_ELECTORATES_ANNOTATION: &str = "election_electorates";

/// Each election's eligible voters, by election id: its enabled voters in
/// every area of the session with a ballot style for it, whether or not that
/// area has anything to tally.
pub type ElectionElectorates = BTreeMap<String, u64>;

/// The tally session annotation that records, when the session is created,
/// the areas of each election's ballot styles its contest areas are made
/// from, as a JSON string.
pub const ELECTION_AREAS_ANNOTATION: &str = "election_areas";

/// The areas of each election, by election id.
pub type ElectionAreas = BTreeMap<String, BTreeSet<String>>;

/// The areas of each election of `styles`, as `(election_id, area_id)`.
pub fn election_areas<'a>(styles: impl IntoIterator<Item = (&'a str, &'a str)>) -> ElectionAreas {
    let mut areas = ElectionAreas::new();
    for (election_id, area_id) in styles {
        areas
            .entry(election_id.to_string())
            .or_default()
            .insert(area_id.to_string());
    }
    areas
}

/// The election areas the session recorded when it was created, or `None` for
/// a session created before they were recorded.
pub fn recorded_election_areas(tally_session: &TallySession) -> Result<Option<ElectionAreas>> {
    tally_session
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(ELECTION_AREAS_ANNOTATION))
        .map(|areas| {
            areas
                .as_str()
                .ok_or_else(|| anyhow!("not a string"))
                .and_then(|areas| Ok(serde_json::from_str(areas)?))
                .with_context(|| {
                    format!("the election areas of tally session {}", tally_session.id)
                })
        })
        .transpose()
}

/// The `(election_id, area_id)` of the ballot styles that existed when the
/// session was created, for a session that did not record them: those created
/// no later than the session.
pub fn published_for_session(
    ballot_styles: &[BallotStyle],
    tally_session: &TallySession,
) -> Vec<(String, String)> {
    ballot_styles
        .iter()
        .filter(|style| match (style.created_at, tally_session.created_at) {
            (Some(style_created), Some(session_created)) => style_created <= session_created,
            _ => true,
        })
        .filter_map(|style| {
            style
                .area_id
                .clone()
                .map(|area_id| (style.election_id.clone(), area_id))
        })
        .collect()
}

/// The areas of each of `election_ids`: those with a ballot style for it
/// (`ballot_styles`, as `(election_id, area_id)`) and those of its contest
/// areas.
pub fn electorate_areas(
    election_ids: &[String],
    ballot_styles: &[(String, String)],
    contests: &[TallySessionContest],
) -> BTreeMap<String, Vec<String>> {
    election_ids
        .iter()
        .map(|election_id| {
            let areas: BTreeSet<&String> = ballot_styles
                .iter()
                .filter(|(election, _)| election == election_id)
                .map(|(_, area)| area)
                .chain(
                    contests
                        .iter()
                        .filter(|contest| &contest.election_id == election_id)
                        .map(|contest| &contest.area_id),
                )
                .collect();
            (election_id.clone(), areas.into_iter().cloned().collect())
        })
        .collect()
}

/// How a voter's authorized elections name an election, as Keycloak's
/// authorized-elections mapper does: by its external id, or by its id when it
/// has none.
pub fn authorization_alias(election_id: &str, external_id: Option<&str>) -> String {
    external_id
        .filter(|external_id| !external_id.is_empty())
        .unwrap_or(election_id)
        .to_string()
}

/// Whether the session was counted before this run: it has results, or this
/// run counts it again. A recount's execution row asks for the recount
/// without the results it replaces, and an error keeps both from the row
/// before it.
pub fn counted_before(execution: &TallySessionExecution) -> bool {
    execution.results_event_id.is_some() || execution.run_reason() != TallyRunReason::NORMAL
}

/// Whether this run records the session's electorates: only when it takes the
/// census of the session's contest areas for the first time (`contests` as the
/// run found them, `dumped` when it extracted ballots), or, for a session with
/// no contest area, when it was not counted before. A session that took its
/// census before electorates were recorded keeps that census on every later
/// run.
pub fn records_electorates(
    recorded: bool,
    contests: &[TallySessionContest],
    dumped: bool,
    counted_before: bool,
) -> bool {
    if recorded {
        false
    } else if contests.is_empty() {
        !counted_before
    } else {
        dumped && contests.iter().all(|contest| contest.annotations.is_none())
    }
}

/// Counts and records the electorate of each election of the session.
#[instrument(skip_all, err)]
pub async fn record_election_electorates(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    tally_session: &TallySession,
    ballot_styles: &[BallotStyle],
    contests: &[TallySessionContest],
) -> Result<ElectionElectorates> {
    let tenant_id = &tally_session.tenant_id;
    let election_event_id = &tally_session.election_event_id;
    let election_ids = tally_session.election_ids.clone().unwrap_or_default();
    let published: Vec<(String, String)> = match recorded_election_areas(tally_session)? {
        Some(recorded) => recorded
            .into_iter()
            .flat_map(|(election_id, areas)| {
                areas
                    .into_iter()
                    .map(move |area_id| (election_id.clone(), area_id))
            })
            .collect(),
        None => published_for_session(ballot_styles, tally_session),
    };
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
    for (election_id, areas) in electorate_areas(&election_ids, &published, contests) {
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

/// The electorates the session recorded, or `None` for a session that
/// recorded none.
pub fn recorded_election_electorates(
    tally_session: &TallySession,
) -> Result<Option<ElectionElectorates>> {
    tally_session
        .annotations
        .as_ref()
        .and_then(|annotations| annotations.get(ELECTION_ELECTORATES_ANNOTATION))
        .map(|electorates| {
            electorates
                .as_str()
                .ok_or_else(|| anyhow!("not a string"))
                .and_then(|electorates| Ok(serde_json::from_str(electorates)?))
                .with_context(|| {
                    format!(
                        "the election electorates of tally session {}",
                        tally_session.id
                    )
                })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::consolidation::eml_generator::{
        prepend_miru_annotation, ValidateAnnotations, MIRU_TALLY_SESSION_DATA,
    };
    use chrono::{DateTime, Local};

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
            election_ids: None,
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

    fn pairs(values: &[(&str, &str)]) -> Vec<(String, String)> {
        values
            .iter()
            .map(|(election, area)| (election.to_string(), area.to_string()))
            .collect()
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
    fn an_electorate_has_every_area_of_the_election_including_those_with_nothing_to_tally() {
        // `post` has ballot styles in north, south and an area whose contests
        // are all acclaimed, which therefore has no contest area in the
        // session. north has two contests, so two contest areas. `other` has
        // its own area; `acclaimed` is acclaimed everywhere.
        let styles = pairs(&[
            ("post", "north"),
            ("post", "south"),
            ("post", "acclaimed-area"),
            ("other", "west"),
            ("acclaimed", "east"),
        ]);
        let contests = vec![
            contest_area("post", "north", "c1", None),
            contest_area("post", "north", "c2", None),
            contest_area("post", "south", "c1", None),
            contest_area("other", "west", "c3", None),
        ];

        let areas = electorate_areas(&ids(&["post", "acclaimed", "missing"]), &styles, &contests);

        assert_eq!(
            areas,
            BTreeMap::from([
                (
                    "post".to_string(),
                    ids(&["acclaimed-area", "north", "south"])
                ),
                ("acclaimed".to_string(), ids(&["east"])),
                // No ballot style: no electorate.
                ("missing".to_string(), vec![]),
            ])
        );
    }

    fn at(time: &str) -> Option<DateTime<Local>> {
        Some(
            DateTime::parse_from_rfc3339(time)
                .unwrap()
                .with_timezone(&Local),
        )
    }

    fn style(area_id: &str, created_at: Option<DateTime<Local>>) -> BallotStyle {
        BallotStyle {
            id: format!("style-{area_id}"),
            tenant_id: "tenant".into(),
            election_id: "post".into(),
            area_id: Some(area_id.into()),
            created_at,
            last_updated_at: None,
            labels: None,
            annotations: None,
            ballot_eml: None,
            ballot_signature: None,
            status: None,
            election_event_id: "event".into(),
            deleted_at: None,
            ballot_publication_id: "publication".into(),
        }
    }

    #[test]
    fn the_areas_are_those_published_when_the_session_was_created() {
        // The session's area-contest links no longer include the acclaimed
        // area, whose link was removed after it was published: it is still
        // published, so it is still the election's.
        let mut session = session(None);
        session.created_at = at("2026-10-10T12:00:00Z");
        session.area_ids = Some(ids(&["north"]));
        let styles = vec![
            style("north", at("2026-10-01T09:00:00Z")),
            style("acclaimed-area", at("2026-10-01T09:00:00Z")),
            // Published after the session was created.
            style("later", at("2026-10-11T09:00:00Z")),
            // Without a time, as far as can be told it was there.
            style("untimed", None),
        ];

        assert_eq!(
            published_for_session(&styles, &session),
            pairs(&[
                ("post", "north"),
                ("post", "acclaimed-area"),
                ("post", "untimed"),
            ])
        );
    }

    #[test]
    fn an_election_is_named_as_keycloak_authorizes_it() {
        assert_eq!(
            authorization_alias("election-uuid", Some("post-1")),
            "post-1"
        );
        // Keycloak names an election without an external id by its id, and so
        // do the voters' authorized elections.
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
        assert!(records_electorates(false, &fresh, true, false));
        // Already recorded: a retry, a recount or a tie-break rerun.
        assert!(!records_electorates(true, &fresh, true, false));
        assert!(!records_electorates(true, &dumped, false, true));
        // Areas dumped before electorates were recorded, then the rest:
        // some of the session's census predates them, so none is recorded.
        assert!(!records_electorates(false, &partly_dumped, true, false));
        // Every area dumped before electorates were recorded: a recount keeps
        // the census it had.
        assert!(!records_electorates(false, &dumped, false, true));
        assert!(!records_electorates(false, &dumped, false, false));
        // Nothing to extract: recorded on its first run, not on a session
        // that was counted before.
        assert!(records_electorates(false, &[], false, false));
        assert!(!records_electorates(false, &[], false, true));
        assert!(!records_electorates(true, &[], false, false));
    }

    fn execution(
        results_event_id: Option<&str>,
        run_reason: Option<TallyRunReason>,
    ) -> TallySessionExecution {
        TallySessionExecution {
            id: "execution".into(),
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            created_at: None,
            last_updated_at: None,
            labels: None,
            annotations: None,
            current_message_id: -1,
            tally_session_id: "session".into(),
            session_ids: None,
            status: None,
            results_event_id: results_event_id.map(str::to_string),
            documents: None,
            run_reason: run_reason.map(|reason| reason.to_string()),
        }
    }

    #[test]
    fn a_session_with_nothing_to_extract_records_its_electorates_only_on_its_first_count() {
        // The rows a session's execution goes through, as they are written.
        let created = execution(None, Some(TallyRunReason::NORMAL));
        let before_the_reason_existed = execution(None, None);
        let completed = execution(Some("results"), Some(TallyRunReason::NORMAL));
        // `request_recount`: the previous row's status, no results.
        let recount_requested = execution(None, Some(TallyRunReason::RECOUNT));
        // `handle_tally_session_error` carries both fields forward.
        let recount_failed_once = execution(None, Some(TallyRunReason::RECOUNT));
        let tie_break = execution(Some("results"), Some(TallyRunReason::TIE_BREAK_RERUN));
        let failed_before_results = execution(None, Some(TallyRunReason::NORMAL));

        for first in [&created, &before_the_reason_existed, &failed_before_results] {
            assert!(!counted_before(first));
            assert!(records_electorates(
                false,
                &[],
                false,
                counted_before(first)
            ));
        }
        for rerun in [
            &completed,
            &recount_requested,
            &recount_failed_once,
            &tie_break,
        ] {
            assert!(counted_before(rerun));
            assert!(!records_electorates(
                false,
                &[],
                false,
                counted_before(rerun)
            ));
        }
    }

    #[test]
    fn the_recorded_electorates_are_read_back() {
        let electorates =
            ElectionElectorates::from([("acclaimed".to_string(), 4), ("post".to_string(), 7)]);
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
    fn the_election_areas_recorded_at_creation_are_read_back() {
        // What the session's contest areas were made from, including an area
        // with nothing to tally, each area once.
        let areas = election_areas([
            ("post", "north"),
            ("post", "acclaimed-area"),
            ("post", "north"),
            ("other", "west"),
        ]);
        assert_eq!(
            areas,
            BTreeMap::from([
                ("other".to_string(), BTreeSet::from(["west".to_string()])),
                (
                    "post".to_string(),
                    BTreeSet::from(["acclaimed-area".to_string(), "north".to_string()])
                ),
            ])
        );

        assert_eq!(recorded_election_areas(&session(None)).unwrap(), None);
        let recorded = session(Some(serde_json::json!({
            "executer_username": "admin",
            ELECTION_AREAS_ANNOTATION: serde_json::to_string(&areas).unwrap(),
        })));
        assert_eq!(recorded_election_areas(&recorded).unwrap(), Some(areas));
        assert!(recorded_election_areas(&session(Some(serde_json::json!({
            ELECTION_AREAS_ANNOTATION: {"post": ["north"]}
        }))))
        .is_err());
    }
}
