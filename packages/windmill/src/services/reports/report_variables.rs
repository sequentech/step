// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::contest::get_contest_by_election_id;
use crate::postgres::results_area_contest::{get_results_area_contest, ResultsAreaContest};
use crate::postgres::tally_session::get_tally_sessions_by_election_event_id;
use crate::services::consolidation::{
    create_transmission_package_service::download_to_file, transmission_package::read_temp_file,
};
use crate::services::database::get_hasura_pool;
use crate::services::database::{get_keycloak_pool, PgConfig};
use crate::services::users::{count_keycloak_enabled_users, count_keycloak_enabled_users_by_attr};
use crate::{
    postgres::area_contest::get_areas_by_contest_id,
    services::users::count_keycloak_enabled_users_by_area_id,
};
use anyhow::{anyhow, Context, Result};
use chrono::Local;
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::{Client, Transaction};
use sequent_core::types::hasura::core::{Contest, Election};
use serde_json::Value;
use std::env;
use strand::hash::hash_b64;
use tracing::instrument;

// re-export for easy refactor:
pub use sequent_core::util::date_time::get_date_and_time;

pub const AREA_ID_ATTR_NAME: &str = "area_id";

pub fn get_app_hash() -> String {
    env::var("APP_HASH").unwrap_or("-".to_string())
}

pub fn get_app_version() -> String {
    env::var("APP_VERSION").unwrap_or("-".to_string())
}

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_total_number_of_registered_voters_by_contest(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    tenant_id: &str,
    election_event_id: &str,
    contest_id: &str,
) -> Result<i64> {
    let contest_areas_id = get_areas_by_contest_id(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        contest_id,
    )
    .await
    .map_err(|err| anyhow!("Error getting area by contest id: {err}"))?;

    let contest_areas_id: Vec<&str> = contest_areas_id.iter().map(|s| s.as_str()).collect();

    let mut total_number_of_expected_votes: i64 = 0;
    for area_id in &contest_areas_id {
        total_number_of_expected_votes +=
            count_keycloak_enabled_users_by_area_id(&keycloak_transaction, &realm, &area_id)
                .await
                .map_err(|err| anyhow!("Error getting count of enabled by area id: {err}"))?;
    }

    Ok(total_number_of_expected_votes)
}

#[instrument(err, skip_all)]
pub async fn generate_total_number_of_expected_votes_for_contest(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    tenant_id: &str,
    election_event_id: &str,
    contest: &Contest,
) -> Result<i64> {
    let total_number_of_expected_votes: i64 =
        generate_total_number_of_registered_voters_by_contest(
            &hasura_transaction,
            &keycloak_transaction,
            &realm,
            tenant_id,
            election_event_id,
            &contest.id.clone(),
        )
        .await
        .map_err(|err| anyhow!("Error getting total number of expected votes: {err}"))?;

    match contest.max_votes {
        Some(max_votes) => Ok(total_number_of_expected_votes * max_votes),
        None => Ok(total_number_of_expected_votes),
    }
}

#[instrument(err, skip_all)]
pub async fn generate_total_number_of_under_votes(
    results_area_contest: &ResultsAreaContest,
) -> Result<(i64)> {
    let blank_votes = results_area_contest.blank_votes.unwrap_or(-1);
    let implicit_invalid_votes = results_area_contest.implicit_invalid_votes.unwrap_or(-1);
    let explicit_invalid_votes = results_area_contest.explicit_invalid_votes.unwrap_or(-1);

    let annotitions = results_area_contest.annotations.clone();

    let under_votes = annotitions
        .as_ref()
        .and_then(|annotations| annotations.get("extended_metrics"))
        .and_then(|extended_metric| extended_metric.get("under_votes"))
        .and_then(|under_vote| under_vote.as_i64())
        .unwrap_or(0);

    let total_under_votes =
        blank_votes + implicit_invalid_votes + explicit_invalid_votes + under_votes;
    Ok(total_under_votes)
}

#[instrument(err, skip_all)]
pub async fn generate_fill_up_rate(
    results_area_contest: &ResultsAreaContest,
    num_of_expected_voters: &i64,
) -> Result<i64> {
    let total_votes = results_area_contest.total_votes.unwrap_or(-1);
    let fill_up_rate = if *num_of_expected_voters == 0 {
        0
    } else {
        (total_votes / num_of_expected_voters) * 100
    };
    Ok(fill_up_rate)
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
    let num_of_registered_voters_by_area_id = count_keycloak_enabled_users_by_attr(
        &keycloak_transaction,
        &realm,
        AREA_ID_ATTR_NAME,
        &area_id,
    )
    .await
    .map_err(|err| anyhow!("Error getting count of enabled users by area_id attribute: {err}"))?;
    Ok(num_of_registered_voters_by_area_id)
}

#[instrument(err, skip_all)]
pub async fn get_total_number_of_registered_voters(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
) -> Result<i64> {
    let num_of_registered_voters_by_area_id =
        count_keycloak_enabled_users(&keycloak_transaction, &realm)
            .await
            .map_err(|err| anyhow!("Error getting count of enabled users: {err}"))?;
    Ok(num_of_registered_voters_by_area_id)
}

pub struct ElectionData {
    pub area_id: String,
    pub geographical_region: String,
    pub voting_center: String,
    pub clustered_precinct_id: String,
    // FIXME: remove precinct_code? Is it redundant with clustered_precinct_id?
    pub precinct_code: String,
    pub post: String,
}

#[instrument(err, skip_all)]
pub async fn extract_election_data(election: &Election) -> Result<ElectionData> {
    let annotitions: Option<Value> = election.annotations.clone();
    let mut geographical_region = "";
    let mut voting_center = "";
    let mut clustered_precinct_id = "";
    let mut area_id = "";
    match &annotitions {
        Some(annotitions) => {
            geographical_region = annotitions
                .get("geographical_region")
                .and_then(|geographical_region| geographical_region.as_str())
                .unwrap_or("");
            voting_center = annotitions
                .get("voting_center")
                .and_then(|voting_center| voting_center.as_str())
                .unwrap_or("");
            clustered_precinct_id = annotitions
                .get("clustered_precinct_id")
                .and_then(|clustered_precinct_id| clustered_precinct_id.as_str())
                .unwrap_or("");
            area_id = annotitions
                .get("area_id")
                .and_then(|area_id| area_id.as_str())
                .unwrap_or("");
        }
        None => {}
    }
    let post = election
        .name
        .clone()
        .split("-")
        .next()
        .unwrap_or("")
        .trim_end()
        .to_string();

    Ok(ElectionData {
        area_id: area_id.to_string(),
        geographical_region: geographical_region.to_string(),
        voting_center: voting_center.to_string(),
        clustered_precinct_id: clustered_precinct_id.to_string(),
        precinct_code: clustered_precinct_id.to_string(),
        post,
    })
}

#[instrument(err, skip_all)]
pub async fn get_election_contests_area_results_and_total_ballot_counted(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<(i64, Vec<ResultsAreaContest>, Vec<Contest>)> {
    let contests: Vec<Contest> = get_contest_by_election_id(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await
    .with_context(|| "Error obtaining contests")?;

    let mut ballots_counted = 0;
    let mut results_area_contests: Vec<ResultsAreaContest> = vec![];
    for contest in contests.clone() {
        // fetch area contest for the contest of the election
        let Some(results_area_contest) = get_results_area_contest(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election_id,
            &contest.id.clone(),
        )
        .await
        .map_err(|e| anyhow::anyhow!(format!("Error getting results area contest {e:?}")))?
        else {
            continue;
        };
        // fetch the amount of ballot counted in the contest
        ballots_counted += get_total_number_of_ballots(&results_area_contest)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error getting number of ballots {e:?}")))?;
        results_area_contests.push(results_area_contest.clone());
    }
    Ok((ballots_counted, results_area_contests, contests))
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
