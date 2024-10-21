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
use chrono::{DateTime, Local, Utc};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::{Contest, Election};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strand::hash::hash_sha256;
use tokio_postgres::types::ToSql;
use tokio_postgres::Row;
use tracing::instrument;
use uuid::Uuid;

pub const COUNTRY_ATTR_NAME: &str = "country";
pub const VALIDATE_ID_ATTR_NAME: &str = "sequent.read-only.id-card-number-validated";
pub const VALIDATE_ID_PRE_ENROLLED_VALUE: &str = "VERIFIED";

enum VoterStatus {
    Voted,
    NotVoted,
}

impl VoterStatus {
    pub fn to_string(&self) -> String {
        match self {
            VoterStatus::Voted => "Voted".to_string(),
            VoterStatus::NotVoted => "Did Not Voted".to_string(),
        }
    }
}

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
) -> Result<i64> {
    let total_votes = results_area_contest.total_votes.unwrap_or(-1);
    let fill_up_rate = (total_votes / nun_of_expected_voters) * 100;
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
pub async fn extract_election_data(election: &Election) -> Result<ElectionData> {
    let annotations: Option<Value> = election.annotations.clone();
    let mut geographical_region = "";
    let mut voting_center = "";
    let mut clustered_precinct_id = "";
    let mut country = "";
    match &annotations {
        Some(annotations) => {
            geographical_region = annotations
                .get("geographical_region")
                .and_then(|geographical_region| geographical_region.as_str())
                .unwrap_or("");
            voting_center = annotations
                .get("voting_center")
                .and_then(|voting_center| voting_center.as_str())
                .unwrap_or("");
            clustered_precinct_id = annotations
                .get("clustered_precinct_id")
                .and_then(|clustered_precinct_id| clustered_precinct_id.as_str())
                .unwrap_or("");
            country = annotations // == area name.
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
        let results_area_contest = get_results_area_contest(
            &hasura_transaction,
            &tenant_id,
            &election_event_id,
            &election_id,
            &contest.id.clone(),
        )
        .await
        .map_err(|e| anyhow::anyhow!(format!("Error getting results area contest {:?}", e)))?;
        // fetch the amount of ballot counted in the contest
        ballots_counted += get_total_number_of_ballots(&results_area_contest)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Error getting number of ballots {:?}", e)))?;
        results_area_contests.push(results_area_contest.clone());
    }
    Ok((ballots_counted, results_area_contests, contests))
}

#[derive(Debug)]
pub struct ReportData {
    pub report_hash: String,
    pub ovcs_version: String,
    pub system_hash: String,
}

#[instrument(err, skip_all)]
pub async fn get_report_system_vals(report_type: String) -> Result<ReportData> {
    let ovcs_version = std::env::var("APP_VERSION")
        .map_err(|e| anyhow::anyhow!(format!("Missing APP_VERSION env variable {:?}", e)))?;

    let system_hash = std::env::var("APP_HASH")
        .map_err(|e| anyhow::anyhow!(format!("Missing APP_HASH env variable {:?}", e)))?;

    //TODO: needed?
    let datetime_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let report_datetime = format!("{}{}", report_type, datetime_str);
    let report_hash = hash_sha256(report_datetime.as_bytes())
        .with_context(|| "Error hashing report type XML")?
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect();

    Ok(ReportData {
        report_hash,
        system_hash,
        ovcs_version,
    })
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Voter {
    pub id: Option<String>,
    pub middle_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub suffix: Option<String>,
    pub status: Option<String>,
    pub date_voted: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct VoteInfo {
    pub date_voted: Option<String>,
    pub status: Option<String>,
}

pub async fn get_voters_by_user_attributes(
    keycloak_transaction: &Transaction<'_>,
    attributes: HashMap<String, String>,
    realm: &str,
) -> Result<(Vec<Voter>, i32)> {
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&realm];
    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = vec![];

    let mut attr_placeholder_count = 2;

    for (key, value) in attributes.clone() {
        dynamic_attr_conditions.push(format!(
                "EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE ${})",
                attr_placeholder_count,
                attr_placeholder_count + 1
            ));
        let val = Some(format!("%{value}%"));
        let formatted_keyy = key.trim_matches('\'').to_string();
        dynamic_attr_params.push(Some(formatted_keyy.clone()));
        dynamic_attr_params.push(val.clone());
        attr_placeholder_count += 2;
    }

    let dynamic_attr_clause = if !dynamic_attr_conditions.is_empty() {
        dynamic_attr_conditions.join(" AND ")
    } else {
        "1=1".to_string() // Always true if no dynamic attributes are specified
    };

    let statement = keycloak_transaction
    .prepare(
        format!(
            r#"
            SELECT 
                u.id, 
                u.first_name,
                u.last_name,
                COALESCE(attr_json.attributes ->> 'middleName', '') AS middle_name,
                COALESCE(attr_json.attributes ->> 'suffix', '') AS suffix,
                COUNT(u.id) OVER() AS total_count
            FROM 
                user_entity u
            INNER JOIN
                realm AS ra ON ra.id = u.realm_id
            LEFT JOIN LATERAL (
                SELECT
                    json_object_agg(ua.name, ua.value) AS attributes
                FROM user_attribute ua
                WHERE ua.user_id = u.id
                GROUP BY ua.user_id -- Grouping by user_id ensures we aggregate attributes for the specific user
            ) attr_json ON true
            WHERE
                ra.name = $1
            AND ({dynamic_attr_clause})
            "#
        )
        .as_str(),
    )
    .await?;

    for value in &dynamic_attr_params {
        params.push(value);
    }

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let count: i32 = if rows.is_empty() {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_count")?.try_into()?
    };

    let users = rows
        .into_iter()
        .map(|row| {
            let user = Voter {
                id: row.get("id"),
                middle_name: row.get("middle_name"),
                first_name: row.get("first_name"),
                last_name: row.get("last_name"),
                suffix: row.get("suffix"),
                status: None,
                date_voted: None,
            };
            user
        })
        .collect::<Vec<Voter>>();

    Ok((users, count))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_voters_with_vote_info(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    users: Vec<Voter>,
    filter_by_has_voted: Option<bool>,
) -> Result<(Vec<Voter>, i32)> {
    let tenant_uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;

    // Prepare the list of user IDs for the query
    let user_ids: Vec<String> = users
        .iter()
        .map(|user| {
            user.id
                .clone()
                .ok_or_else(|| anyhow!("Encountered a user without an ID"))
        })
        .collect::<Result<Vec<String>>>()
        .with_context(|| "Error extracting user IDs")?;

    let vote_info_statement = hasura_transaction
        .prepare(
            r#"
            SELECT 
                v.voter_id_string AS voter_id_string, 
                MAX(v.last_updated_at) AS last_voted_at,
                COUNT(v.voter_id_string) OVER() AS total_count
            FROM 
                sequent_backend.cast_vote v
            WHERE 
                v.tenant_id = $1 AND
                v.election_event_id = $2 AND
                v.voter_id_string = ANY($3)
            GROUP BY 
                v.voter_id_string, v.election_id;
            "#,
        )
        .await
        .with_context(|| "Error preparing the vote info statement")?;

    let rows = hasura_transaction
        .query(
            &vote_info_statement,
            &[&tenant_uuid, &election_event_uuid, &user_ids],
        )
        .await
        .with_context(|| "Error executing the vote info query")?;

    let mut user_votes_map: HashMap<String, VoteInfo> = users
        .iter()
        .map(|user| {
            let user_id = user
                .id
                .clone()
                .ok_or_else(|| anyhow!("Encountered a user without an ID"))?;
            Ok((
                user_id,
                VoteInfo {
                    date_voted: None,
                    status: None,
                },
            ))
        })
        .collect::<Result<_>>()
        .with_context(|| "Error processing users for user_votes_map")?;

    let count: i32 = if rows.is_empty() {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_count")?.try_into()?
    };

    for row in rows {
        let voter_id_string: String = row
            .try_get("voter_id_string")
            .with_context(|| "Error getting voter_id_string from row")?;
        let last_voted_at: DateTime<Utc> = row
            .try_get("last_voted_at")
            .with_context(|| "Error getting last_voted_at from row")?;

        if let Some(user_votes_info) = user_votes_map.get_mut(&voter_id_string) {
            VoteInfo {
                date_voted: Some(last_voted_at.to_string()),
                status: Some(VoterStatus::Voted.to_string()),
            };
        } else {
            return Err(anyhow!("Not found user for voter-id={voter_id_string}"));
        }
    }

    // Construct the final Vec<Voter> in the same order as the input users
    let mut filtered_users: Vec<Voter> = Vec::new();
    for user in users.iter() {
        let user_id = user
            .id
            .clone()
            .ok_or_else(|| anyhow!("Encountered a user without an ID"))?;

        let votes_info = user_votes_map
            .get(&user_id)
            .cloned()
            .ok_or_else(|| anyhow!("Missing vote info for user ID {}", user_id))?;

        match filter_by_has_voted {
            Some(has_voted) => match votes_info.status.clone() {
                Some(status) => {
                    if has_voted {
                        filtered_users.push(Voter {
                            status: Some(status),
                            date_voted: votes_info.date_voted,
                            ..user.clone()
                        })
                    }
                }
                None => {
                    if !has_voted {
                        filtered_users.push(Voter {
                            status: Some(VoterStatus::NotVoted.to_string()),
                            date_voted: None,
                            ..user.clone()
                        })
                    }
                }
            },
            None => filtered_users.push(Voter {
                status: Some(
                    votes_info
                        .status
                        .unwrap_or(VoterStatus::NotVoted.to_string())
                        .to_string(),
                ),
                date_voted: votes_info.date_voted,
                ..user.clone()
            }),
        }
    }

    Ok((filtered_users, count))
}

#[derive(Debug)]
pub struct VotersData {
    pub total_ov: i32,
    pub total_ov_voted: i32,
    pub total_ov_not_voted: i32,
    pub voters: Vec<Voter>,
}
#[instrument(err, skip_all)]
pub async fn get_voters_data(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    tenant_id: &str,
    election_event_id: &str,
    country: &str,
) -> Result<VotersData> {
    let mut attributes: HashMap<String, String> = HashMap::new();
    attributes.insert(COUNTRY_ATTR_NAME.to_string(), country.to_string());

    let (voters, voters_count) =
        get_voters_by_user_attributes(&keycloak_transaction, attributes.clone(), &realm).await?;

    let (voters, voter_who_voted_count) = get_voters_with_vote_info(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        voters.clone(),
        None,
    )
    .await?;

    let total_ov_not_voted = voters_count - &voter_who_voted_count;

    Ok(VotersData {
        total_ov: voters_count,
        total_ov_voted: voter_who_voted_count,
        total_ov_not_voted,
        voters,
    })
}
