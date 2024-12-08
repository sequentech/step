// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::report_variables::{
    get_total_number_of_registered_voters_for_area_id, VALIDATE_ID_ATTR_NAME,
    VALIDATE_ID_REGISTERED_VOTER,
};
use crate::services::users::{
    count_keycloak_enabled_users_by_attrs, AttributesFilterBy, AttributesFilterOption,
};
use crate::types::application::{ApplicationStatus, ApplicationType};
use crate::{
    postgres::application::get_applications, services::cast_votes::count_ballots_by_area_id,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::keycloak::AREA_ID_ATTR_NAME;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio_postgres::types::ToSql;
use tokio_postgres::Row;
use tracing::instrument;
use uuid::Uuid;
pub const SEX_ATTR_NAME: &str = "sex";
pub const FEMALE_VALE: &str = "F";
pub const MALE_VALE: &str = "M";
pub const POST_ATTR_NAME: &str = "embassy";
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
            VoterStatus::NotVoted => "Did Not Vote".to_string(),
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
    pub enrollment_date: Option<String>,
    pub verification_date: Option<String>, // for approval & disaproval
    pub verified_by: Option<String>,       // OFOV/SBEI/SYSTEM for approval & disaproval
    pub disapproval_reason: Option<String>, // for disapproval
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct VoteInfo {
    pub date_voted: Option<String>,
    pub status: Option<String>,
    // TODO: add more fields if needed for different reports
}

#[instrument(err, skip_all)]
pub async fn get_enrolled_voters(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    filters: Option<EnrollmentFilters>,
) -> Result<(Vec<Voter>, i64)> {
    let applications: Vec<Application> = get_applications(
        &hasura_transaction,
        &tenant_id,
        &election_event_id,
        &area_id,
        filters.as_ref(),
    )
    .await
    .map_err(|err| anyhow!("{}", err))?;

    let users = applications
        .into_iter()
        .map(|row| {
            let middle_name = row
                .applicant_data
                .get("middleName")
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            let first_name = row
                .applicant_data
                .get("firstName")
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            let last_name = row
                .applicant_data
                .get("lastName")
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            let suffix = row
                .applicant_data
                .get("suffix")
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            let status = if row.status == ApplicationStatus::ACCEPTED.to_string() {
                None
            } else {
                Some(VoterStatus::DidNotPreEnrolled.to_string())
            };

            Voter {
                id: Some(row.applicant_id),
                middle_name,
                first_name,
                last_name,
                suffix,
                status,
                date_voted: None,
                enrollment_date: row.created_at.map(|date| date.to_rfc3339()),
                verification_date: row.updated_at.map(|date| date.to_rfc3339()),
                verified_by: row
                    .annotations
                    .clone()
                    .unwrap_or_default()
                    .get("verified_by")
                    .and_then(|v| v.as_str().map(|s| s.to_string())),
                disapproval_reason: row
                    .annotations
                    .clone()
                    .unwrap_or_default()
                    .get("rejection_reason")
                    .and_then(|v| v.as_str().map(|s| s.to_string())),
            }
        })
        .collect::<Vec<Voter>>();

    let count = users.len() as i64;

    Ok((users, count))
}

#[instrument(err, skip_all)]
pub async fn get_voters_by_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
    attributes: HashMap<String, AttributesFilterOption>,
) -> Result<(Vec<Voter>, i64)> {
    let mut params: Vec<&(dyn ToSql + Sync)> = vec![&realm, &area_id];
    let mut dynamic_attr_conditions: Vec<String> = Vec::new();

    for (attr_name, attr_value) in attributes.iter() {
        let clause = attr_value.get_sql_filter_clause(params.len() + 2);
        params.push(attr_name);
        params.push(&attr_value.value);

        dynamic_attr_conditions.push(clause);
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
                enrollment_date: None,
                verification_date: None,
                verified_by: None,
                disapproval_reason: None,
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
             SELECT DISTINCT ON (v.voter_id_string) 
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
                date_voted: Some(last_voted_at.to_rfc3339()),
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
pub struct EnrollmentFilters {
    pub status: ApplicationStatus,
    pub verification_type: Option<ApplicationType>,
}

#[derive(Debug, Clone)]
pub struct FilterListVoters {
    pub enrolled: Option<EnrollmentFilters>,
    pub has_voted: Option<bool>,
    pub voters_sex: Option<String>,
    pub post: Option<String>,
    pub landbased_or_seafarer: Option<String>,
    pub verified: Option<bool>,
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
    let mut attributes: HashMap<String, AttributesFilterOption> = HashMap::new();

    match voters_filter.voters_sex {
        Some(voters_sex) => {
            attributes.insert(
                SEX_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: voters_sex.to_string(),
                    filter_by: AttributesFilterBy::IsLike,
                },
            );
        }
        None => {}
    };

    match voters_filter.post {
        Some(post) => {
            attributes.insert(
                POST_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: post.to_string(),
                    filter_by: AttributesFilterBy::IsLike,
                },
            );
        }
        None => {}
    };

    match voters_filter.landbased_or_seafarer {
        Some(landbased_or_seafarer) => {
            attributes.insert(
                LANDBASED_OR_SEAFARER_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: landbased_or_seafarer,
                    filter_by: AttributesFilterBy::PartialLike,
                },
            );
        }
        None => {}
    };

    match voters_filter.verified {
        Some(true) => {
            attributes.insert(
                VALIDATE_ID_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
                    filter_by: AttributesFilterBy::IsEqual,
                },
            );
        }
        Some(false) => {
            attributes.insert(
                VALIDATE_ID_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
                    filter_by: AttributesFilterBy::NotExist,
                },
            );
        }
        None => {}
    };

    let (voters, voters_count) = match voters_filter.enrolled {
        Some(_) => {
            get_enrolled_voters(
                &hasura_transaction,
                &tenant_id,
                &election_event_id,
                &area_id,
                voters_filter.enrolled,
            )
            .await?
        }
        None => {
            get_voters_by_area_id(&keycloak_transaction, &realm, &area_id, attributes.clone())
                .await?
        }
    };

    let (mut voters, voter_who_voted_count) = match with_vote_info {
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

    sort_voters(&mut voters);

    let total_not_voted = voters_count - &voter_who_voted_count;

    Ok(VotersData {
        total_voters: voters_count,
        total_voted: voter_who_voted_count,
        total_not_voted,
        voters,
    })
}

// Helper function to generate the sorting key for a voter
fn generate_sort_key(voter: &Voter) -> String {
    let mut key = String::new();
    if let Some(last_name) = &voter.last_name {
        key.push_str(last_name);
    }
    if let Some(first_name) = &voter.first_name {
        key.push_str(first_name);
    }
    if let Some(suffix) = &voter.suffix {
        key.push_str(suffix);
    }
    if let Some(middle_name) = &voter.middle_name {
        key.push_str(middle_name);
    }
    key.trim().to_string()
}

// Helper function to sort voters using precompute keys
fn sort_voters(voters: &mut Vec<Voter>) {
    let mut voters_with_keys: Vec<(String, &Voter)> = voters
        .iter()
        .map(|v| (generate_sort_key(v).to_lowercase(), v))
        .collect();

    voters_with_keys.sort_by(|(key_a, _), (key_b, _)| key_a.cmp(key_b));

    *voters = voters_with_keys
        .into_iter()
        .map(|(_, voter)| voter.clone())
        .collect();
}

#[instrument(err, skip_all)]
pub async fn count_voters_by_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
    post: Option<String>,
    pre_enrolled: Option<bool>,
) -> Result<i64> {
    let mut attributes: HashMap<String, AttributesFilterOption> = HashMap::new();
    attributes.insert(
        AREA_ID_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: area_id.to_string(),
            filter_by: AttributesFilterBy::IsEqual,
        },
    );

    match post {
        Some(post) => {
            attributes.insert(
                POST_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: post,
                    filter_by: AttributesFilterBy::IsLike,
                },
            );
        }
        None => {}
    }

    match pre_enrolled {
        Some(false) => {
            attributes.insert(
                VALIDATE_ID_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
                    filter_by: AttributesFilterBy::NotExist,
                },
            );
        }
        Some(true) => {
            attributes.insert(
                VALIDATE_ID_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
                    filter_by: AttributesFilterBy::IsEqual,
                },
            );
        }
        _ => {}
    }

    let total_not_pre_enrolled = count_keycloak_enabled_users_by_attrs(
        &keycloak_transaction,
        &realm,
        Some(attributes.clone()),
    )
    .await?;

    Ok(total_not_pre_enrolled)
}

#[instrument(err, skip_all)]
pub async fn get_not_enrolled_voters_by_area_id(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    area_id: &str,
) -> Result<Vec<Voter>> {
    let mut attributes: HashMap<String, AttributesFilterOption> = HashMap::new();
    attributes.insert(
        VALIDATE_ID_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
            filter_by: AttributesFilterBy::NotExist,
        },
    );

    let (voters, _voters_count) =
        get_voters_by_area_id(&keycloak_transaction, &realm, &area_id, attributes.clone()).await?;

    Ok(voters)
}

pub struct VotersBySex {
    pub total_female: i64,
    pub total_male: i64,
    pub overall_total: i64,
}

pub async fn count_voters_by_their_sex(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    post: &str,
    landbased_or_seafarer: Option<&str>,
    not_pre_enrolled: bool,
) -> Result<VotersBySex> {
    let mut attributes: HashMap<String, AttributesFilterOption> = HashMap::new();
    attributes.insert(
        POST_ATTR_NAME.to_string(),
        AttributesFilterOption {
            value: post.to_string(),
            filter_by: AttributesFilterBy::IsLike,
        },
    );

    match not_pre_enrolled {
        true => {
            attributes.insert(
                VALIDATE_ID_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: VALIDATE_ID_REGISTERED_VOTER.to_string(),
                    filter_by: AttributesFilterBy::NotExist,
                },
            );
        }
        false => {}
    }

    match landbased_or_seafarer {
        Some(landbased_or_seafarer) => {
            attributes.insert(
                LANDBASED_OR_SEAFARER_ATTR_NAME.to_string(),
                AttributesFilterOption {
                    value: landbased_or_seafarer.to_string(),
                    filter_by: AttributesFilterBy::PartialLike,
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

    Ok(VotersBySex {
        total_female,
        total_male,
        overall_total,
    })
}

pub fn calc_percentage(count: i64, total: i64) -> f64 {
    match total == 0 {
        true => 0.0,
        false => (count as f64 / total as f64) * 100.0,
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VotersStatsData {
    pub total_male_landbased: i64,
    pub total_female_landbased: i64,
    pub total_landbased: i64,
    pub total_male_seafarer: i64,
    pub total_female_seafarer: i64,
    pub total_seafarer: i64,
    pub total_male: i64,
    pub total_female: i64,
    pub overall_total: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostData {
    pub post: String,
    pub stats: VotersStatsData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionData {
    pub geographical_region: String,
    pub posts: Vec<PostData>,
    pub stats: VotersStatsData,
}

pub async fn set_up_region_voters_data(
    keycloak_transaction: &Transaction<'_>,
    realm: &str,
    region: &str,
    posts: Vec<String>,
    not_pre_enrolled: bool,
) -> Result<RegionData> {
    let mut posts_data: Vec<PostData> = vec![];

    let mut region_overall_total_male_landbased: i64 = 0;
    let mut region_overall_total_female_landbased: i64 = 0;
    let mut region_overall_total_landbased: i64 = 0;
    let mut region_overall_total_male_seafarer: i64 = 0;
    let mut region_overall_total_female_seafarer: i64 = 0;
    let mut region_overall_total_seafarer: i64 = 0;
    let mut region_overall_total_male: i64 = 0;
    let mut region_overall_total_female: i64 = 0;
    let mut region_overall_total: i64 = 0;

    for post in posts {
        let landbased = count_voters_by_their_sex(
            &keycloak_transaction,
            &realm,
            &post,
            Some(LANDBASED_VALUE),
            not_pre_enrolled.clone(),
        )
        .await
        .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;
        let seafarer = count_voters_by_their_sex(
            &keycloak_transaction,
            &realm,
            &post,
            Some(SEAFARER_VALUE),
            not_pre_enrolled.clone(),
        )
        .await
        .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;
        let general =
            count_voters_by_their_sex(&keycloak_transaction, &realm, &post, None, not_pre_enrolled)
                .await
                .map_err(|err| anyhow!("Error count_voters_by_their_sex, landbase {err}"))?;

        region_overall_total_male_landbased += landbased.total_male;
        region_overall_total_female_landbased += landbased.total_female;
        region_overall_total_landbased += landbased.overall_total;

        region_overall_total_male_seafarer += seafarer.total_male;
        region_overall_total_female_seafarer += seafarer.total_female;
        region_overall_total_seafarer += seafarer.overall_total;

        region_overall_total_male += general.total_male;
        region_overall_total_female += general.total_female;
        region_overall_total += general.overall_total;

        posts_data.push(PostData {
            post: post.to_string(),
            stats: VotersStatsData {
                total_male_landbased: landbased.total_male,
                total_female_landbased: landbased.total_female,
                total_landbased: landbased.overall_total,
                total_male_seafarer: seafarer.total_male,
                total_female_seafarer: seafarer.total_female,
                total_seafarer: seafarer.overall_total,
                total_male: general.total_male,
                total_female: general.total_female,
                overall_total: general.overall_total,
            },
        })
    }
    Ok(RegionData {
        geographical_region: region.to_string(),
        posts: posts_data,
        stats: VotersStatsData {
            total_male_landbased: region_overall_total_male_landbased,
            total_female_landbased: region_overall_total_female_landbased,
            total_landbased: region_overall_total_landbased,
            total_male_seafarer: region_overall_total_male_seafarer,
            total_female_seafarer: region_overall_total_female_seafarer,
            total_seafarer: region_overall_total_seafarer,
            total_male: region_overall_total_male,
            total_female: region_overall_total_female,
            overall_total: region_overall_total,
        },
    })
}
