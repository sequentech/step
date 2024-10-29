// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::contest::get_contest_by_election_id;
use crate::postgres::results_area_contest::{get_results_area_contest, ResultsAreaContest};
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::services::consolidation::eml_generator::{
    find_miru_annotation, find_miru_annotation_opt, ValidateAnnotations, MIRU_GEOGRAPHICAL_REGION,
    MIRU_PRECINCT_CODE, MIRU_VOTING_CENTER,
};
use crate::services::consolidation::{
    create_transmission_package_service::download_to_file, transmission_package::read_temp_file,
};
use crate::services::database::get_hasura_pool;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::users::{count_keycloak_enabled_users, count_keycloak_enabled_users_by_attrs};
use anyhow::{anyhow, Context, Result};
use chrono::Local;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::{Client, Transaction};
use sequent_core::types::hasura::core::{Area, Contest, Election};
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use strand::hash::hash_b64;
use tracing::instrument;

pub const VALIDATE_ID_ATTR_NAME: &str = "sequent.read-only.id-card-number-validated";
pub const VALIDATE_ID_REGISTERED_VOTER: &str = "VERIFIED";

pub fn get_app_hash() -> String {
    env::var("APP_HASH").unwrap_or("-".to_string())
}

pub fn get_app_version() -> String {
    env::var("APP_VERSION").unwrap_or("-".to_string())
}

#[instrument(err, skip_all)]
pub async fn get_total_number_of_ballots(
    results_area_contest: &ResultsAreaContest,
) -> Result<(i64)> {
    let annotations = results_area_contest.annotations.clone();
    match &annotations {
        Some(annotations) => Ok(annotations
            .get("extended_metrics")
            .and_then(|extended_metric| extended_metric.get("ballots"))
            .and_then(|under_vote| under_vote.as_i64())
            .unwrap_or(-1)),
        None => Ok(-1),
    }
}

#[instrument(err, skip_all)]
pub async fn generate_voters_turnout(
    number_of_ballots: &i64,
    number_of_registered_voters: &i64,
) -> Result<(i64)> {
    let voters_turnout = if *number_of_registered_voters == 0 {
        0
    } else {
        (number_of_ballots / number_of_registered_voters) * 100
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
        count_keycloak_enabled_users_by_attrs(&keycloak_transaction, &realm, None)
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
    let annotations = election.get_valid_annotations()?;
    let geographical_region = find_miru_annotation_opt(MIRU_GEOGRAPHICAL_REGION, &annotations)?
        .unwrap_or("-".to_string());
    let voting_center =
        find_miru_annotation_opt(MIRU_VOTING_CENTER, &annotations)?.unwrap_or("-".to_string());
    let precinct_code =
        find_miru_annotation_opt(MIRU_PRECINCT_CODE, &annotations)?.unwrap_or("-".to_string());
    let area_id = "";

    let election_alias_or_name = election.alias.as_deref().unwrap_or(&election.name);

    let post = election_alias_or_name
        .split('-')
        .next()
        .map(|s| s.trim_end().to_string())
        .with_context(|| format!("error parsing election name"))?;

    Ok(ElectionData {
        area_id: area_id.to_string(),
        geographical_region,
        voting_center,
        precinct_code,
        post,
    })
}

#[instrument(err, skip_all)]
pub async fn get_post(election: &Election) -> Result<String> {
    let election_alias_or_name = election.alias.as_deref().unwrap_or(&election.name);

    let post = election_alias_or_name
        .split('-')
        .next()
        .map(|s| s.trim_end().to_string())
        .with_context(|| format!("error parsing election name"))?;
    Ok(post)
}

pub struct AreaData {
    pub geographical_region: String,
    pub voting_center: String,
    pub precinct_code: String,
}

#[instrument(err, skip_all)]
pub async fn extract_area_data(area: &Area) -> Result<AreaData> {
    let annotations = area.get_valid_annotations()?;
    let geographical_region = find_miru_annotation_opt(MIRU_GEOGRAPHICAL_REGION, &annotations)?
        .unwrap_or("-".to_string());
    let voting_center =
        find_miru_annotation_opt(MIRU_VOTING_CENTER, &annotations)?.unwrap_or("-".to_string());
    let precinct_code =
        find_miru_annotation_opt(MIRU_PRECINCT_CODE, &annotations)?.unwrap_or("-".to_string());

    Ok(AreaData {
        geographical_region,
        voting_center,
        precinct_code,
    })
}

pub fn get_date_and_time() -> String {
    let current_date_time = Local::now();
    let printed_datetime = current_date_time.to_rfc3339();
    printed_datetime
}

#[instrument(err, skip_all)]
pub async fn get_election_contests_area_results(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
) -> Result<(Vec<ResultsAreaContest>, Vec<Contest>)> {
    let contests: Vec<Contest> = get_contest_by_election_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await
    .map_err(|e| anyhow::anyhow!(format!("Error getting results contests {e:?}")))?;

    let mut results_area_contests: Vec<ResultsAreaContest> = vec![];
    for contest in contests.clone() {
        // fetch area contest for the contest of the election
        let Some(results_area_contest) = get_results_area_contest(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election_id,
            &contest.id.clone(),
            &area_id,
        )
        .await
        .map_err(|e| anyhow::anyhow!(format!("Error getting results area contest {e:?}")))?
        else {
            continue;
        };

        results_area_contests.push(results_area_contest.clone());
    }
    Ok((results_area_contests, contests))
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
