// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::postgres::contest::get_contest_by_election_id;
use crate::postgres::results_area_contest::{get_results_area_contest, ResultsAreaContest};
use crate::services::users::count_keycloak_enabled_users_by_attr;
use crate::{
    postgres::area_contest::get_areas_by_contest_id,
    services::users::count_keycloak_enabled_users_by_areas_id,
};
use anyhow::{anyhow, Context, Result};
use chrono::Local;
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::{Contest, Election};
use serde_json::Value;
use tracing::instrument;

pub const COUNTRY_ATTR_NAME: &str = "country";

#[instrument(err, skip_all)]
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

    let total_number_of_expected_votes: i64 =
        count_keycloak_enabled_users_by_areas_id(&keycloak_transaction, &realm, &contest_areas_id)
            .await
            .map_err(|err| anyhow!("Error getting count of enabeld users by areas id: {err}"))?;

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
    nun_of_expected_voters: &i64,
) -> Result<(i64)> {
    let total_votes = results_area_contest.total_votes.unwrap_or(-1);
    let fill_up_rate = (total_votes / nun_of_expected_voters) * 100;
    Ok(fill_up_rate)
}

#[instrument(err, skip_all)]
pub async fn get_total_number_of_ballots(
    results_area_contest: &ResultsAreaContest,
) -> Result<(i64)> {
    let annotitions = results_area_contest.annotations.clone();
    match &annotitions {
        Some(annotitions) => Ok(annotitions
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
    let voters_turnout = (number_of_ballots / number_of_registered_voters) * 100;
    Ok(voters_turnout)
}

#[instrument(err, skip_all)]
pub async fn get_total_number_of_registered_voters_for_country(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    country: &str,
) -> Result<i64> {
    let num_of_registerd_voters_by_country = count_keycloak_enabled_users_by_attr(
        &keycloak_transaction,
        &realm,
        COUNTRY_ATTR_NAME,
        &country,
    )
    .await
    .map_err(|err| anyhow!("Error getting count of enabeld users by country attribute: {err}"))?;
    Ok(num_of_registerd_voters_by_country)
}
pub struct ElectionData {
    pub country: String,
    pub geographical_region: String,
    pub voting_center: String,
    pub clustered_precinct_id: String,
    pub post: String,
}

#[instrument(err, skip_all)]
pub async fn extract_eleciton_data(election: &Election) -> Result<ElectionData> {
    let annotitions: Option<Value> = election.annotations.clone();
    let mut geographical_region = "";
    let mut voting_center = "";
    let mut clustered_precinct_id = "";
    let mut country = "";
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
            country = annotitions
                .get("country")
                .and_then(|country| country.as_str())
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
        country: country.to_string(),
        geographical_region: geographical_region.to_string(),
        voting_center: voting_center.to_string(),
        clustered_precinct_id: clustered_precinct_id.to_string(),
        post,
    })
}

pub fn get_date_and_time() -> (String, String) {
    let current_date_time = Local::now();
    let date = current_date_time
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let time = current_date_time.time().format("%H:%M:%S").to_string();
    (date, time)
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
        if let Some(results_area_contest) = get_results_area_contest(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election_id,
            &contest.id.clone(),
        )
        .await?
        {
            // fetch the amount of ballot counted in the contest
            ballots_counted += get_total_number_of_ballots(&results_area_contest).await?;
            results_area_contests.push(results_area_contest.clone());
        }
    }
    Ok((ballots_counted, results_area_contests, contests))
}
