// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_postgres::types::ToSql;
use tokio_postgres::Row;
use tracing::instrument;
use uuid::Uuid;

use crate::services::{
    cast_votes::count_ballots_by_area_id,
    users::{count_keycloak_enabled_users_by_attrs, AttributesFilterBy, AttributesFilterOption},
};

use super::report_variables::{VALIDATE_ID_ATTR_NAME, VALIDATE_ID_REGISTERED_VOTER};

pub const SEX_ATTR_NAME: &str = "sex";
pub const FEMALE_VALE: &str = "F";
pub const MALE_VALE: &str = "M";
pub const LANDBASED_OR_SEAFARER_ATTR_NAME: &str = "landBasedOrSeafarer";
pub const LANDBASED_VALUE: &str = "land";
pub const SEAFARER_VALUE: &str = "sea";
enum VoterStatus {
    Voted,
    NotVoted,
    DidNotPreEnrolled,
}

impl VoterStatus {
    pub fn to_string(&self) -> String {
        match self {
            VoterStatus::Voted => "Voted".to_string(),
            VoterStatus::NotVoted => "Did Not Voted".to_string(),
            VoterStatus::DidNotPreEnrolled => "Did Not Pre-enrolled".to_string(),
        }
    }
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
    // TODO: add more fields if needed for different reports
}

pub async fn get_voters_by_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
    attributes: HashMap<String, (String, bool)>, // actual value = String, search for exist value = bool (true)
) -> Result<(Vec<Voter>, i64)> {
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &area_id];
    let mut dynamic_attr_conditions: Vec<String> = Vec::new();
    let mut dynamic_attr_params: Vec<Option<String>> = vec![];

    let mut attr_placeholder_count = 3;

    for (key, value) in attributes.clone() {
        if value.1 == false {
            dynamic_attr_conditions.push(format!(
                "NOT EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE ${})",
                attr_placeholder_count,
                attr_placeholder_count + 1
            ));
        } else {
            dynamic_attr_conditions.push(format!(
                "EXISTS (SELECT 1 FROM user_attribute ua WHERE ua.user_id = u.id AND ua.name = ${} AND ua.value ILIKE ${})",
                attr_placeholder_count,
                attr_placeholder_count + 1
            ));
        }
        let val = value.0;
        let val = Some(format!("%{val}%"));
        let formatted_key = key.trim_matches('\'').to_string();
        dynamic_attr_params.push(Some(formatted_key.clone()));
        dynamic_attr_params.push(val.clone());
        attr_placeholder_count += 2;
    }

    let dynamic_attr_clause = if !dynamic_attr_conditions.is_empty() {
        dynamic_attr_conditions.join(" AND ")
    } else {
        "1=1".to_string() // Always true if no dynamic attributes are specified
    };

    let statement = keycloak_transaction
        .prepare(&format!(
            r#"
        SELECT 
            u.id, 
            u.first_name,
            u.last_name,
            COALESCE(attr_json.attributes ->> 'middleName', '') AS middle_name,
            COALESCE(attr_json.attributes ->> 'suffix', '') AS suffix,
            COALESCE(attr_json.attributes ->> '{VALIDATE_ID_ATTR_NAME}', '') AS validate_id,
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
            GROUP BY ua.user_id
        ) attr_json ON true
        WHERE
            ra.name = $1 AND
            EXISTS (
                SELECT 1 
                FROM user_attribute ua 
                WHERE ua.user_id = u.id 
                AND ua.name = '{AREA_ID_ATTR_NAME}' 
                AND ua.value = $2
            )
            AND ({dynamic_attr_clause})
        "#,
        ))
        .await?;

    for value in &dynamic_attr_params {
        params.push(value);
    }

    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let count: i64 = if rows.is_empty() {
        0
    } else {
        rows[0].try_get::<&str, i64>("total_count")?.try_into()?
    };

    let users = rows
        .into_iter()
        .map(|row| {
            let validate_id = row.get("validate_id");
            let status = match validate_id {
                Some(VALIDATE_ID_REGISTERED_VOTER) => None,
                _ => Some(VoterStatus::DidNotPreEnrolled.to_string()),
            };
            let user = Voter {
                id: row.get("id"),
                middle_name: row.get("middle_name"),
                first_name: row.get("first_name"),
                last_name: row.get("last_name"),
                suffix: row.get("suffix"),
                status: status,
                date_voted: None,
            };
            user
        })
        .collect::<Vec<Voter>>();

    Ok((users, count))
}

/*Fill voters with voting info */
#[instrument(skip(hasura_transaction), err)]
pub async fn get_voters_with_vote_info(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    users: Vec<Voter>,
    filter_by_has_voted: Option<bool>,
) -> Result<(Vec<Voter>, i64)> {
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
                MAX(v.created_at) AS last_voted_at
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

    let count: i64 = rows.len().try_into()?;

    for row in rows {
        let voter_id_string: String = row
            .try_get("voter_id_string")
            .with_context(|| "Error getting voter_id_string from row")?;
        let last_voted_at: DateTime<Utc> = row
            .try_get("last_voted_at")
            .with_context(|| "Error getting last_voted_at from row")?;

        if let Some(user_votes_info) = user_votes_map.get_mut(&voter_id_string) {
            *user_votes_info = VoteInfo {
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
            None => {
                //if pre-enrolled => status = DidNotPreEnrolled else status = NotVoted/Voted
                let status = user.status.clone().unwrap_or(
                    votes_info
                        .status
                        .unwrap_or(VoterStatus::NotVoted.to_string()),
                );
                filtered_users.push(Voter {
                    status: Some(status),
                    date_voted: votes_info.date_voted,
                    ..user.clone()
                })
            }
        }
    }

    Ok((filtered_users, count))
}

#[derive(Debug, Clone)]
pub struct VotersData {
    pub total_voters: i64,
    pub total_voted: i64,
    pub total_not_voted: i64,
    pub voters: Vec<Voter>,
}

#[derive(Debug, Clone)]
pub struct FilterListVoters {
    pub pre_enrolled: bool,
    pub has_voted: Option<bool>, // put None if not to filter by has_voted
    pub voters_sex: Option<String>,
}

#[instrument(err, skip_all)]
pub async fn get_voters_data(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
    with_vote_info: bool,
    voters_filter: FilterListVoters,
) -> Result<VotersData> {
    let mut attributes: HashMap<String, (String, bool)> = HashMap::new();

    match voters_filter.voters_sex {
        Some(voters_sex) => {
            attributes.insert(SEX_ATTR_NAME.to_string(), (voters_sex.to_string(), true));
        }
        None => {}
    };

    let (voters, voters_count) =
        get_voters_by_area_id(&keycloak_transaction, &realm, &area_id, attributes.clone()).await?;

    let (voters, voter_who_voted_count) = match with_vote_info {
        true => {
            get_voters_with_vote_info(
                &hasura_transaction,
                &tenant_id,
                &election_event_id,
                voters.clone(),
                voters_filter.has_voted,
            )
            .await?
        }
        false => {
            let number_of_voted = count_ballots_by_area_id(
                &hasura_transaction,
                &tenant_id,
                &election_event_id,
                &election_id,
                &area_id,
            )
            .await?;
            (voters, number_of_voted)
        }
    };

    let total_not_voted = voters_count - &voter_who_voted_count;

    Ok(VotersData {
        total_voters: voters_count,
        total_voted: voter_who_voted_count,
        total_not_voted,
        voters,
    })
}

pub async fn count_not_enrolled_voters_by_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
) -> Result<i64> {
    let params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &area_id];
    let statement = keycloak_transaction
        .prepare(&format!(
            r#"
        SELECT 
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
            GROUP BY ua.user_id
        ) attr_json ON true
        WHERE
            ra.name = $1 AND
            EXISTS (
                SELECT 1 
                FROM user_attribute ua 
                WHERE ua.user_id = u.id 
                AND ua.name = '{AREA_ID_ATTR_NAME}' 
                AND ua.value = $2
            )
            AND NOT EXISTS (
                SELECT 1 
                FROM user_attribute ua 
                WHERE ua.user_id = u.id 
                AND ua.name = '{VALIDATE_ID_ATTR_NAME}' 
                AND ua.value = 'VERIFIED'
            )
        "#,
        ))
        .await?;
    let rows: Vec<Row> = keycloak_transaction
        .query(&statement, &params.as_slice())
        .await
        .map_err(|err| anyhow!("{}", err))?;

    let count: i64 = rows.len().try_into()?;

    Ok(count)
}

pub async fn get_not_enrolled_voters_by_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
) -> Result<Vec<Voter>> {
    let mut attributes: HashMap<String, (String, bool)> = HashMap::new();
    attributes.insert(
        VALIDATE_ID_ATTR_NAME.to_string(),
        (VALIDATE_ID_REGISTERED_VOTER.to_string(), false),
    );

    let (voters, _voters_count) =
        get_voters_by_area_id(&keycloak_transaction, &realm, &area_id, attributes.clone()).await?;

    Ok(voters)
}

pub struct VotersByGender {
    pub total_female: i64,
    pub total_male: i64,
    pub overall_total: i64,
}

pub async fn count_voters_by_their_sex(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
    bandbased_or_seafarer: Option<&str>,
) -> Result<VotersByGender> {
    let mut attributes: HashMap<String, AttributesFilterOption> = HashMap::new();
    attributes.insert(
        AREA_ID_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: area_id.to_string(),
            filter_by: AttributesFilterBy::IsEqual,
        },
    );

    match bandbased_or_seafarer {
        Some(bandbased_or_seafarer) => {
            attributes.insert(
                LANDBASED_OR_SEAFARER_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: bandbased_or_seafarer.to_string(),
                    filter_by: AttributesFilterBy::IsLike,
                },
            );
        }
        None => {}
    }

    let overall_total = count_keycloak_enabled_users_by_attrs(
        keycloak_transaction,
        realm,
        Some(attributes.clone()),
    )
    .await?;

    attributes.insert(
        SEX_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: FEMALE_VALE.to_string(),
            filter_by: AttributesFilterBy::IsEqual,
        },
    );

    let total_female = count_keycloak_enabled_users_by_attrs(
        keycloak_transaction,
        realm,
        Some(attributes.clone()),
    )
    .await?;

    attributes.insert(
        SEX_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: MALE_VALE.to_string(),
            filter_by: AttributesFilterBy::IsEqual,
        },
    );
    let total_male =
        count_keycloak_enabled_users_by_attrs(keycloak_transaction, realm, Some(attributes))
            .await?;

    Ok(VotersByGender {
        total_female,
        total_male,
        overall_total,
    })
}

pub fn calc_percentage(count: i64, total: i64) -> f64 {
    match total == 0 {
        true => -1.0,
        false => (count as f64 / total as f64) * 100.0,
    }
}
