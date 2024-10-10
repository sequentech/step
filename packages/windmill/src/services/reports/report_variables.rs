// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::database::get_keycloak_pool;
use crate::services::users::count_keycloak_enabled_users_by_attr;
use crate::{
    postgres::area_contest::get_areas_by_contest_id,
    services::users::count_keycloak_enabled_users_by_areas_id,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client as DbClient, Transaction};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::Contest;
use serde_json::{from_value, Value};
use tracing::instrument;

pub const COUNTRY_ATTR_NAME: &str = "country";

#[instrument(err, skip_all)]
pub async fn genereate_total_number_of_registered_voters_by_contest(
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
pub async fn genereate_total_number_of_expected_votes_for_contest(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    tenant_id: &str,
    election_event_id: &str,
    contest_id: &str,
    contest: &Contest,
) -> Result<i64> {
    let total_number_of_expected_votes: i64 =
        genereate_total_number_of_registered_voters_by_contest(
            &hasura_transaction,
            &keycloak_transaction,
            &realm,
            tenant_id,
            election_event_id,
            contest_id,
        )
        .await
        .map_err(|err| anyhow!("Error getting total number of expected votes: {err}"))?;

    match contest.max_votes {
        Some(max_votes) => Ok(total_number_of_expected_votes * max_votes),
        None => Ok(total_number_of_expected_votes),
    }
}

#[instrument(err, skip_all)]
pub async fn genereate_total_number_of_under_votes(
    results_area_contest_annotations: &Value,
) -> Result<(i64)> {
    let annotitions = results_area_contest_annotations.clone();
    let blank_votes = annotitions
        .get("blank_votes")
        .unwrap_or(&serde_json::Value::from(0))
        .as_i64()
        .unwrap_or(0);
    let implicit_invalid_votes = annotitions
        .get("implicit_invalid_votes")
        .unwrap_or(&serde_json::Value::from(0))
        .as_i64()
        .unwrap_or(0);
    let explicit_invalid_votes = annotitions
        .get("explicit_invalid_votes")
        .unwrap_or(&serde_json::Value::from(0))
        .as_i64()
        .unwrap_or(0);

    let under_votes = annotitions
        .get("extended_metrics")
        .and_then(|extended_metric| extended_metric.get("under_votes"))
        .and_then(|under_vote| under_vote.as_i64())
        .unwrap_or(0);

    let total_under_votes =
        blank_votes + implicit_invalid_votes + explicit_invalid_votes + under_votes;
    Ok(total_under_votes)
}

#[instrument(err, skip_all)]
pub async fn genereate_fill_up_rate(
    results_area_contest_annotations: &Value,
    nun_of_expected_voters: &i64,
) -> Result<(i64)> {
    let annotitions = results_area_contest_annotations.clone();
    let total_votes = annotitions
        .get("total_votes")
        .unwrap_or(&serde_json::Value::from(0))
        .as_i64()
        .unwrap_or(0);

    let fill_up_rate = (total_votes / nun_of_expected_voters) * 100;
    Ok(fill_up_rate)
}

#[instrument(err, skip_all)]
pub async fn genereate_voters_turnout(
    results_area_contest_annotations: &Value,
    number_of_registered_voters: &i64,
) -> Result<(i64)> {
    let annotitions = results_area_contest_annotations.clone();
    let number_of_ballots = annotitions
        .get("extended_metrics")
        .and_then(|extended_metric| extended_metric.get("ballots"))
        .and_then(|under_vote| under_vote.as_i64())
        .unwrap_or(0);

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
