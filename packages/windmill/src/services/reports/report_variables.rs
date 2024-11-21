use crate::postgres::results_election::get_election_results;
// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::services::consolidation::eml_generator::ValidateAnnotations;
use crate::services::consolidation::{
    create_transmission_package_service::download_to_file, transmission_package::read_temp_file,
};
use crate::services::users::{count_keycloak_enabled_users, count_keycloak_enabled_users_by_attrs};
use crate::types::miru_plugin::MiruSbeiUser;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::{Area, Election, ElectionEvent};
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use strand::hash::hash_b64;
use tracing::instrument;
// re-export for easy refactor:
pub use sequent_core::util::date_time::get_date_and_time;

pub const VALIDATE_ID_ATTR_NAME: &str = "sequent.read-only.id-card-number-validated";
pub const VALIDATE_ID_REGISTERED_VOTER: &str = "VERIFIED";

pub const DEFULT_CHAIRPERSON: &str = "Chairperson";
pub const DEFULT_POLL_CLERK: &str = "Poll Clerk";
pub const DEFULT_THIRD_MEMBER: &str = "Third Member";

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
        let registered_voters = result.elegible_census;
        let total_ballots = result.total_voters;
        let voters_turnout = if let (Some(registered_voters), Some(total_ballots)) =
            (registered_voters, total_ballots)
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
    Ok(Some(turnout))
}

#[instrument(err, skip_all)]
pub async fn get_total_number_of_registered_voters_for_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
) -> Result<i64> {
    let mut attributes: HashMap<String, String> = HashMap::new();
    attributes.insert(AREA_ID_ATTR_NAME.to_string(), area_id.to_string());
    attributes.insert(
        VALIDATE_ID_ATTR_NAME.to_string(),
        VALIDATE_ID_REGISTERED_VOTER.to_string(),
    );
    let num_of_registered_voters_by_area_id =
        count_keycloak_enabled_users_by_attrs(&keycloak_transaction, &realm, Some(attributes))
            .await
            .map_err(|err| {
                anyhow!("Error getting count of enabled users by area_id attribute: {err}")
            })?;
    Ok(num_of_registered_voters_by_area_id)
}

pub struct ElectionData {
    pub area_id: String,
    pub geographical_region: String,
    pub voting_center: String,
    pub precinct_code: String,
    pub post: String,
}

#[instrument(err, skip_all)]
pub async fn extract_election_data(election: &Election) -> Result<ElectionData> {
    let annotations: crate::services::consolidation::eml_generator::MiruElectionAnnotations =
        election.get_annotations_or_empty_values()?;
    let area_id = "";

    Ok(ElectionData {
        area_id: area_id.to_string(),
        geographical_region: annotations.geographical_area.clone(),
        voting_center: annotations.post.clone(),
        precinct_code: annotations.precinct_code.clone(),
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
        (false, false) => election_event_sbei_users
            .into_iter()
            .filter_map(|user: MiruSbeiUser| {
                if area_sbei_ids.contains(&user.miru_id) {
                    Some(InspectorData {
                        role: user.miru_name.clone(),
                        name: user.miru_name,
                    })
                } else {
                    None
                }
            })
            .collect(),
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

    Ok(AreaData { inspectors })
}

#[instrument(err, skip(hasura_transaction))]
pub async fn get_results_hash(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<String> {
    let tally_sessions = get_tally_sessions_by_election_event_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        false,
    )
    .await
    .map_err(|err| anyhow!("Error getting the tally sessions: {err:?}"))?;

    let tally_session_id = if !tally_sessions.is_empty() {
        &tally_sessions[0].id
    } else {
        return Err(anyhow!("No tally session yet"));
    };

    let mut results_temp_file = download_to_file(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &tally_session_id,
    )
    .await
    .map_err(|err| anyhow!("Error getting the results file: {err:?}"))?;

    let file_data = read_temp_file(&mut results_temp_file)
        .map_err(|err| anyhow!("Error reading the results file: {err:?}"))?;

    let file_hash =
        hash_b64(&file_data).map_err(|err| anyhow!("Error hashing the results file: {err:?}"))?;

    Ok(file_hash)
}

#[instrument(err, skip_all)]
pub async fn get_report_hash(report_type: &str) -> Result<String> {
    let date_and_time = get_date_and_time();
    let report_date_time = format!("{}{}", report_type, date_and_time);
    let report_hash = hash_b64(report_date_time.as_bytes())
        .map_err(|err| anyhow!("Error hashing report hash: {err:?}"))?;
    Ok(report_hash)
}
