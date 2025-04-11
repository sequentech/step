use crate::postgres::area::{get_area_by_id, get_areas_by_election_id};
// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::election::get_election_by_id;
use crate::postgres::results_area_contest::get_results_area_contest;
use crate::postgres::results_election::{
    get_election_results, get_results_election_by_results_event_id,
};
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::postgres::tally_session_execution::get_tally_session_executions;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::election_dates::get_election_dates;
use crate::services::election_event_status::get_election_event_status;
use crate::services::users::{
    count_keycloak_enabled_users, count_keycloak_enabled_users_by_attrs, AttributesFilterBy,
    AttributesFilterOption,
};
use crate::types::miru_plugin::MiruSbeiUser;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::StringifiedPeriodDates;
use sequent_core::types::hasura::core::{Area, Election, ElectionEvent};
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use sequent_core::types::scheduled_event::ScheduledEvent;
use sequent_core::util::temp_path::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use strand::hash::hash_b64;
use tracing::instrument;
// re-export for easy refactor:
pub use crate::services::users::{VALIDATE_ID_ATTR_NAME, VALIDATE_ID_REGISTERED_VOTER};
pub use sequent_core::util::date_time::get_date_and_time;

pub const DEFULT_CHAIRPERSON: &str = "Chairperson";
pub const DEFULT_POLL_CLERK: &str = "Poll Clerk";
pub const DEFULT_THIRD_MEMBER: &str = "Third Member";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionAnnotations {
    pub date_printed: String,
    pub report_hash: String,
    pub app_version: String,
    pub software_version: String,
    pub app_hash: String,
    pub executer_username: Option<String>,
    pub results_hash: Option<String>,
}

pub fn get_app_hash() -> String {
    env::var("APP_HASH").unwrap_or("-".to_string())
}

pub fn get_app_version() -> String {
    env::var("APP_VERSION").unwrap_or("-".to_string())
}

#[derive(Debug)]
pub struct ElectionVotesData {
    pub registered_voters: Option<i64>,
    pub total_ballots: Option<i64>,
    pub voters_turnout: Option<f64>,
}

#[instrument(err, skip_all)]
pub async fn generate_election_votes_data(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<ElectionVotesData> {
    let areas = get_areas_by_election_id(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .with_context(|| "Can't find areas")?;

    let registered_voters: i64 = areas
        .iter()
        .map(|area| {
            area.get_annotations_or_empty_values()
                .map(|annotations| annotations.registered_voters)
        })
        .collect::<Result<Vec<i64>>>()?
        .iter()
        .sum();
    // Fetch last election results created from tally session
    let election_results = get_election_results(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
    )
    .await
    .map_err(|e| anyhow!("Error fetching election results: {:?}", e))?;

    if let Some(result) = election_results.get(0) {
        let total_ballots = result.total_voters;
        let voters_turnout = if let Some(total_ballots) = total_ballots {
            calc_voters_turnout(total_ballots, registered_voters)?
        } else {
            None
        };

        Ok(ElectionVotesData {
            registered_voters: Some(registered_voters),
            total_ballots,
            voters_turnout,
        })
    } else {
        Ok(ElectionVotesData {
            registered_voters: None,
            total_ballots: None,
            voters_turnout: None,
        })
    }
}

#[instrument(err, skip_all)]
pub async fn generate_election_area_votes_data(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    contest_id: Option<&str>,
) -> Result<ElectionVotesData> {
    let area = get_area_by_id(hasura_transaction, tenant_id, area_id)
        .await?
        .ok_or(anyhow!("Can't find election"))?;
    let registered_voters = area
        .get_annotations_or_empty_values()
        .map(|annotations| annotations.registered_voters)
        .ok();
    // Fetch last election results created from tally session
    let area_results = get_results_area_contest(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_id,
        contest_id,
        area_id,
    )
    .await
    .map_err(|e| anyhow!("Error fetching election results: {:?}", e))?;

    if let Some(result) = area_results {
        let total_ballots = result.total_votes;
        let voters_turnout = if let (Some(registered_voters), Some(total_ballots)) =
            (registered_voters.clone(), total_ballots)
        {
            calc_voters_turnout(total_ballots, registered_voters)?
        } else {
            None
        };

        Ok(ElectionVotesData {
            registered_voters,
            total_ballots,
            voters_turnout,
        })
    } else {
        Ok(ElectionVotesData {
            registered_voters: None,
            total_ballots: None,
            voters_turnout: None,
        })
    }
}

#[instrument(err, skip_all)]
pub fn calc_voters_turnout(total_ballots: i64, registered_voters: i64) -> Result<Option<f64>> {
    if registered_voters == 0 {
        return Ok(Some(0.0));
    }

    let turnout = (total_ballots as f64 / registered_voters as f64) * 100.0;
    Ok(Some(turnout.clamp(0.0, 100.0)))
}

#[instrument(err, skip_all)]
pub async fn get_total_number_of_registered_voters_for_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
) -> Result<i64> {
    let mut attributes: HashMap<String, AttributesFilterOption> = HashMap::new();
    attributes.insert(
        AREA_ID_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: area_id.to_string(),
            filter_by: AttributesFilterBy::IsEqual,
        },
    );

    attributes.insert(
        VALIDATE_ID_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
            filter_by: AttributesFilterBy::IsEqual,
        },
    );
    let num_of_registered_voters_by_area_id =
        count_keycloak_enabled_users_by_attrs(&keycloak_transaction, &realm, Some(attributes))
            .await
            .map_err(|err| {
                anyhow!("Error getting count of enabled users by area_id attribute: {err}")
            })?;
    Ok(num_of_registered_voters_by_area_id)
}

#[instrument(err, skip_all)]
pub async fn get_total_number_of_registered_voters(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
) -> Result<i64> {
    let num_of_registered_voters = count_keycloak_enabled_users(&keycloak_transaction, &realm)
        .await
        .map_err(|err| anyhow!("Error getting count of enabled users: {err}"))?;
    Ok(num_of_registered_voters)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ElectionData {
    pub geographical_region: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub pollcenter_code: String,
    pub post: String,
}

#[instrument(err, skip_all)]
pub async fn extract_election_data(election: &Election) -> Result<ElectionData> {
    let annotations: crate::services::consolidation::eml_generator::MiruElectionAnnotations =
        election.get_annotations_or_empty_values()?;

    Ok(ElectionData {
        geographical_region: annotations.geographical_area.clone(),
        voting_center: annotations.post.clone(),
        precinct_code: annotations.precinct_code.clone(),
        pollcenter_code: annotations.pollcenter_code.clone(),
        post: annotations.post.clone(),
    })
}

pub struct ElectionEventAnnotation {
    pub sbei_users: Vec<MiruSbeiUser>,
}

#[instrument(err, skip_all)]
pub async fn extract_election_event_annotations(
    election_event: &ElectionEvent,
) -> Result<ElectionEventAnnotation> {
    let annotations: crate::services::consolidation::eml_generator::MiruElectionEventAnnotations =
        election_event.get_annotations_or_empty_values()?;

    Ok(ElectionEventAnnotation {
        sbei_users: annotations.sbei_users.clone(),
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InspectorData {
    pub role: String,
    pub name: String,
}

pub struct AreaData {
    pub inspectors: Vec<InspectorData>,
    pub registered_voters: i64,
}

#[instrument(err, skip_all)]
pub async fn extract_area_data(
    area: &Area,
    election_event_sbei_users: Vec<MiruSbeiUser>,
) -> Result<AreaData> {
    let annotations = area.get_annotations_or_empty_values()?;

    let area_sbei_ids = annotations.sbei_ids.clone();

    let inspectors: Vec<InspectorData> = match (
        area_sbei_ids.is_empty(),
        election_event_sbei_users.is_empty(),
    ) {
        (false, false) => {
            let mut seen_ids = HashSet::new();
            election_event_sbei_users
                .into_iter()
                .filter_map(|user| {
                    if area_sbei_ids.contains(&user.miru_id)
                        && seen_ids.insert(user.miru_id.clone())
                    {
                        Some(InspectorData {
                            role: user.miru_name.clone(),
                            name: user.miru_name,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        }
        _ => vec![
            InspectorData {
                role: "".to_string(),
                name: DEFULT_CHAIRPERSON.to_string(),
            },
            InspectorData {
                role: "".to_string(),
                name: DEFULT_POLL_CLERK.to_string(),
            },
            InspectorData {
                role: "".to_string(),
                name: DEFULT_THIRD_MEMBER.to_string(),
            },
        ],
    };

    Ok(AreaData {
        inspectors,
        registered_voters: annotations.registered_voters,
    })
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_results_hash(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<String> {
    let tally_sessions = get_tally_sessions_by_election_event_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        false,
    )
    .await
    .map_err(|err| anyhow!("Error getting the tally sessions: {err:?}"))?;

    // filter tally sessions that holds the current election_id
    let tally_sessions = tally_sessions
        .into_iter()
        .filter(|tally_session| {
            tally_session
                .election_ids
                .as_ref()
                .map(|ids| ids.contains(&election_id.to_string()))
                .unwrap_or(false)
                && tally_session.tally_type.clone().unwrap_or_default()
                    == String::from("ELECTORAL_RESULTS")
        })
        .collect::<Vec<_>>();

    // the first tally session is the latest one
    let tally_session_id = if !tally_sessions.is_empty() {
        &tally_sessions[0].id
    } else {
        return Err(anyhow!("No tally session yet"));
    };

    let tally_session_executions = get_tally_session_executions(
        hasura_transaction,
        tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .map_err(|err| anyhow!("Error getting the tally session executions"))?;

    // the first execution is the latest one
    let tally_session_execution = tally_session_executions
        .first()
        .ok_or_else(|| anyhow!("No tally session executions found"))?;

    let results_event_id = tally_session_execution
        .results_event_id
        .clone()
        .ok_or_else(|| anyhow!("Missing results_event_id in tally session execution"))?; // here im failing

    let result_election = get_results_election_by_results_event_id(
        hasura_transaction,
        tenant_id,
        election_id,
        &results_event_id,
    )
    .await
    .map_err(|err| anyhow!("Error getting the results election: {err:?}"))?;

    let results_hash = result_election
        .annotations
        .and_then(|annotations| annotations.get("results_hash").cloned());

    let results_hash = results_hash
        .map(|hash| hash.to_string().replace("\"", " ").trim().to_string())
        .unwrap_or_default();

    Ok(results_hash)
}

#[instrument(err, skip_all)]
pub async fn get_report_hash(report_type: &str) -> Result<String> {
    let date_and_time = get_date_and_time();
    let report_date_time = format!("{}{}", report_type, date_and_time);
    let report_hash = hash_b64(report_date_time.as_bytes())
        .map_err(|err| anyhow!("Error hashing report hash: {err:?}"))?;
    Ok(report_hash)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataElection {
    pub election_dates: StringifiedPeriodDates,
    pub election_name: String,
    pub election_annotations: ElectionData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDataElections {
    pub regions: Vec<(String, Vec<String>)>,
    pub elections: Vec<UserDataElection>,
}

#[instrument(err, skip_all)]
pub async fn process_elections(
    elections: Vec<Election>,
    scheduled_events: Vec<ScheduledEvent>,
) -> Result<UserDataElections> {
    let mut region_posts_map: HashMap<String, HashSet<String>> = HashMap::new();

    let mut elections_data: Vec<UserDataElection> = vec![];

    for election in elections {
        let election_general_data = extract_election_data(&election)
            .await
            .map_err(|err| anyhow!("Error extract election annotations {err}"))?;

        region_posts_map
            .entry(election_general_data.geographical_region.clone())
            .or_insert_with(HashSet::new)
            .insert(election_general_data.post.clone());

        let election_dates = get_election_dates(&election, scheduled_events.clone())
            .map_err(|e| anyhow::anyhow!("Error getting election dates {e}"))?;

        elections_data.push(UserDataElection {
            election_dates,
            election_name: election.alias.unwrap_or(election.name),
            election_annotations: election_general_data,
        });
    }

    // Convert HashMap into a Vec<(String, Vec<String>)>
    let regions: Vec<(String, Vec<String>)> = region_posts_map
        .into_iter()
        .map(|(region, posts)| (region, posts.into_iter().collect()))
        .collect();

    Ok(UserDataElections {
        regions,
        elections: elections_data,
    })
}
