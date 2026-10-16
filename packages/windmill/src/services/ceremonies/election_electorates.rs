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

/// The areas of each of `election_ids`: those with a ballot style for it
/// (`ballot_styles`, as `(election_id, area_id)`) within the session's areas
/// (`session_areas`, all of them when `None`), and those of its contest areas.
pub fn electorate_areas(
    election_ids: &[String],
    ballot_styles: &[(String, String)],
    session_areas: Option<&[String]>,
    contests: &[TallySessionContest],
) -> BTreeMap<String, Vec<String>> {
    election_ids
        .iter()
        .map(|election_id| {
            let areas: BTreeSet<&String> = ballot_styles
                .iter()
                .filter(|(election, area)| {
                    election == election_id
                        && session_areas.map_or(true, |session_areas| session_areas.contains(area))
                })
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
///
/// Elections are named as the ballot extraction names them, by their external
/// id or not at all, so the electorate has the voters whose ballots it counts.
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
    let styled: Vec<(String, String)> = ballot_styles
        .iter()
        .filter_map(|style| {
            style
                .area_id
                .clone()
                .map(|area_id| (style.election_id.clone(), area_id))
        })
        .collect();
    let aliases: HashMap<String, String> =
        get_election_event_elections(hasura_transaction, tenant_id, election_event_id)
            .await?
            .into_iter()
            .filter_map(|election| election.external_id.map(|alias| (election.id, alias)))
            .collect();
    let realm = get_event_realm(tenant_id, election_event_id);

    let mut electorates = ElectionElectorates::new();
    for (election_id, areas) in electorate_areas(
        &election_ids,
        &styled,
        tally_session.area_ids.as_deref(),
        contests,
    ) {
        let alias = aliases.get(&election_id).map(String::as_str).unwrap_or("");
        let electorate = count_keycloak_enabled_voters_of_areas(
            keycloak_transaction,
            &realm,
            &areas,
            &[],
            alias,
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

        let areas = electorate_areas(
            &ids(&["post", "acclaimed", "missing"]),
            &styles,
            None,
            &contests,
        );

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

    #[test]
    fn an_electorate_keeps_to_the_areas_of_the_session() {
        let styles = pairs(&[
            ("post", "north"),
            ("post", "south"),
            ("post", "acclaimed-area"),
        ]);
        let contests = vec![contest_area("post", "north", "c1", None)];
        let session_areas = ids(&["acclaimed-area", "north"]);

        let areas = electorate_areas(
            &ids(&["post"]),
            &styles,
            Some(session_areas.as_slice()),
            &contests,
        );

        assert_eq!(areas["post"], ids(&["acclaimed-area", "north"]));
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
}
