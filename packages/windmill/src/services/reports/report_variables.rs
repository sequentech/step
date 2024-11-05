// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::services::consolidation::eml_generator::{
    find_miru_annotation_opt, ValidateAnnotations, MIRU_GEOGRAPHICAL_REGION, MIRU_PRECINCT_CODE,
    MIRU_VOTING_CENTER,
};
use crate::services::consolidation::{
    create_transmission_package_service::download_to_file, transmission_package::read_temp_file,
};
use crate::services::election_event_status::get_election_event_status;
use crate::services::users::{count_keycloak_enabled_users, count_keycloak_enabled_users_by_attrs};
use crate::types::miru_plugin::MiruSbeiUser;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ElectionEventStatus, PeriodDates, PeriodDatesStrings};
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

#[instrument(err, skip_all)]
pub async fn generate_voters_turnout(
    number_of_ballots: &i64,
    number_of_registered_voters: &i64,
) -> Result<f64> {
    let total_voters = *number_of_registered_voters;
    let total_ballots = *number_of_ballots;

    let voters_turnout = if total_voters == 0 {
        0.0
    } else {
        (total_ballots as f64 / total_voters as f64) * 100.0
    };

    Ok(voters_turnout)
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

    let area_sbei_usernames = annotations.sbei_usernames.clone();

    let inspectors: Vec<InspectorData> = match (
        area_sbei_usernames.is_empty(),
        election_event_sbei_users.is_empty(),
    ) {
        (false, false) => election_event_sbei_users
            .into_iter()
            .filter_map(|user: MiruSbeiUser| {
                if area_sbei_usernames.contains(&user.username) {
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
pub async fn get_election_dates(election: &Election) -> Result<PeriodDatesStrings> {
    let status: ElectionEventStatus =
        get_election_event_status(election.status.clone()).unwrap_or_default();
    let period_dates: PeriodDates = status.voting_period_dates;
    let dates = period_dates.to_string_fields("-");
    Ok(dates)
}
