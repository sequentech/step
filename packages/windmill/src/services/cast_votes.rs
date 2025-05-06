// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::database::PgConfig;
use crate::services::datafix::utils::{
    is_datafix_election_event_by_id, voted_via_not_internet_channel,
};
use crate::services::electoral_log::ElectoralLog;
use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::types::keycloak::{User, VotesInfo};
use serde::{Deserialize, Serialize};
use tokio::io::{copy, BufWriter};
use std::collections::HashMap;
use tokio::fs::File;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use tokio_postgres::row::Row;
use tracing::{info, instrument};
use uuid::Uuid;
use tokio_util::io::StreamReader;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CastVote {
    pub id: String,
    pub tenant_id: String,
    pub election_id: Option<String>,
    pub area_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub last_updated_at: Option<DateTime<Utc>>,
    pub content: Option<String>,
    pub voter_id_string: Option<String>,
    pub election_event_id: String,
    pub ballot_id: Option<String>,
    pub cast_ballot_signature: Option<Vec<u8>>,
}

impl TryFrom<Row> for CastVote {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(CastVote {
            id: item.try_get::<_, Uuid>("id")?.to_string(),
            tenant_id: item.try_get::<_, Uuid>("tenant_id")?.to_string(),
            election_id: item
                .try_get::<_, Option<Uuid>>("election_id")?
                .map(|val| val.to_string()),
            area_id: item
                .try_get::<_, Option<Uuid>>("area_id")?
                .map(|val| val.to_string()),
            created_at: item.get("created_at"),
            last_updated_at: item.get("last_updated_at"),
            content: item.try_get("content")?,
            cast_ballot_signature: item.try_get("cast_ballot_signature")?,
            voter_id_string: item.try_get("voter_id_string")?,
            election_event_id: item.try_get::<_, Uuid>("election_event_id")?.to_string(),
            ballot_id: item.try_get("ballot_id")?,
        })
    }
}

#[instrument(err)]
pub async fn find_area_ballots(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    output_file: &mut File,
) -> Result<()> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid = Uuid::parse_str(area_id)
        .map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;
    let areas_statement = 
            r#"
                    SELECT DISTINCT ON (election_id, voter_id_string)
                        id,
                        election_id,
                        content,
                    FROM "sequent_backend".cast_vote
                    WHERE
                        tenant_id = $1 AND
                        election_event_id = $2 AND
                        area_id = $3
                    ORDER BY election_id, voter_id_string, created_at DESC
                    LIMIT $4 OFFSET $5
                "#;

    let copy_out_query = format!(
        "COPY ({}) TO STDOUT WITH (FORMAT CSV)",
        areas_statement
    );
    let mut writer = BufWriter::new(output_file); 

    let mut reader = hasura_transaction
        .copy_out(copy_out_query.as_str())
        .await
        .map_err(|err| anyhow!("Error running the areas query: {}", err))?;

    let mut async_reader = StreamReader::new(reader);

    let bytes_copied = copy(&mut async_reader, &mut writer).await.expect("Failed to write data to file");

    Ok(())
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ElectionCastVotes {
    pub election_id: String,
    pub census: i64,
    pub cast_votes: i64,
}

impl TryFrom<Row> for ElectionCastVotes {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(ElectionCastVotes {
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            census: 0,
            cast_votes: item.get("cast_votes"),
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CastVotesPerDay {
    pub day: String,
    pub day_count: i64,
}

impl TryFrom<Row> for CastVotesPerDay {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(CastVotesPerDay {
            day: item.try_get::<_, chrono::NaiveDate>("day")?.to_string(),
            day_count: item.try_get::<_, i64>("day_count")?,
        })
    }
}

#[instrument(err)]
pub async fn count_cast_votes_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    is_test_election: Option<bool>,
) -> Result<Vec<ElectionCastVotes>> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;

    let test_elections_clause = match is_test_election {
        Some(true) => "AND el.name ILIKE '%Test%'".to_string(),
        Some(false) => "AND el.name NOT ILIKE '%Test%'".to_string(),
        None => "".to_string(),
    };

    let statement_str = format!(
        r#"
            SELECT el.id AS election_id, COUNT(DISTINCT cv.voter_id_string) AS cast_votes
            FROM sequent_backend.election el
            LEFT JOIN (
                SELECT DISTINCT election_id, voter_id_string
                FROM sequent_backend.cast_vote
            ) cv ON el.id = cv.election_id
            WHERE
                el.tenant_id = $1 AND
                el.election_event_id = $2
                {test_elections_clause}
            GROUP BY
                el.id
            "#
    );

    let statement = hasura_transaction.prepare(statement_str.as_str()).await?;

    let rows: Vec<Row> = hasura_transaction
        .query(&statement, &[&tenant_uuid, &election_event_uuid])
        .await
        .map_err(|err| anyhow!("Error running the query: {}", err))?;
    let count_data = rows
        .into_iter()
        .map(|row| -> Result<ElectionCastVotes> { row.try_into() })
        .collect::<Result<Vec<ElectionCastVotes>>>()?;

    Ok(count_data)
}

#[instrument(skip(transaction), err)]
pub async fn get_count_votes_per_day(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    start_date: &str,
    end_date: &str,
    election_id: Option<String>,
    user_timezone: &str,
) -> Result<Vec<CastVotesPerDay>> {
    let start_date_naive = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .with_context(|| "Error parsing start_date")?;
    let end_date_naive = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .with_context(|| "Error parsing end_date")?;
    let election_uuid = match election_id {
        Some(ref election_id_r) => Some(Uuid::parse_str(election_id_r.as_str())?),
        None => None,
    };
    let total_areas_statement = transaction
        .prepare(
            format!(
                r#"
            WITH date_series AS (
                SELECT
                    (t.day)::date AS day
                FROM 
                    generate_series(
                        $3::date,
                        $4::date,
                        interval '1 day'
                    ) AS t(day)
            )
            SELECT
                ds.day,
                COALESCE(
                    COUNT(
                        CASE 
                            WHEN DATE(v.created_at AT TIME ZONE $5) = ds.day THEN 1 
                            ELSE NULL 
                        END
                    ), 
                    0
                ) AS day_count
            FROM
                date_series ds
            LEFT JOIN sequent_backend.cast_vote v ON ds.day = DATE(v.created_at AT TIME ZONE $5)
                AND v.tenant_id = $1
                AND v.election_event_id = $2
                AND (v.election_id = $6 OR $6 IS NULL)
            WHERE
                (
                    DATE(v.created_at AT TIME ZONE $5) >= $3 AND
                    DATE(v.created_at AT TIME ZONE $5) <= $4
                )
                OR v.created_at IS NULL
            GROUP BY ds.day
            ORDER BY ds.day;
            "#
            )
            .as_str(),
        )
        .await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_areas_statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &start_date_naive,
                &end_date_naive,
                &user_timezone,
                &election_uuid,
            ],
        )
        .await?;

    let cast_votes_by_day = rows
        .into_iter()
        .map(|row| -> Result<CastVotesPerDay> { row.try_into() })
        .collect::<Result<Vec<CastVotesPerDay>>>()?;

    Ok(cast_votes_by_day)
}

#[instrument(skip(hasura_transaction, users), err)]
pub async fn get_users_with_vote_info(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<String>,
    mut users: Vec<User>,
    filter_by_has_voted: Option<bool>,
) -> Result<Vec<User>> {
    let tenant_uuid =
        Uuid::parse_str(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid = Uuid::parse_str(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;

    let election_uuid = match election_id {
        Some(ref election_id_s) => Some(
            Uuid::parse_str(election_id_s)
                .with_context(|| format!("Error parsing election_id {election_id_s} as UUID"))?,
        ),
        None => None,
    };

    let is_datafix_event =
        is_datafix_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
            .await
            .with_context(|| "Error checking if is datafix election event")?;

    // Collect user IDs (and verify all have an ID)
    let user_ids: Vec<String> = users
        .iter()
        .map(|user| {
            user.id
                .clone()
                .ok_or_else(|| anyhow!("Encountered a user without an ID"))
        })
        .collect::<Result<Vec<String>>>()
        .with_context(|| "Error extracting user IDs")?;

    // If no users, we can return early
    if user_ids.is_empty() {
        return Ok(vec![]);
    }

    let vote_info_statement = hasura_transaction
        .prepare(
            r#"
        SELECT
            v.voter_id_string AS voter_id_string,
            v.election_id     AS election_id,
            COUNT(v.id)       AS num_votes,
            MAX(v.created_at) AS last_voted_at
        FROM sequent_backend.cast_vote v
        WHERE
            v.tenant_id        = $1::uuid
            AND v.election_event_id = $2::uuid
            AND v.voter_id_string   = ANY($3::text[])
            AND ($4::uuid IS NULL OR v.election_id = $4::uuid)
        GROUP BY
            v.voter_id_string, v.election_id
        "#,
        )
        .await?;

    let rows = hasura_transaction
        .query(
            &vote_info_statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &user_ids,
                &election_uuid,
            ],
        )
        .await
        .with_context(|| "Error executing the vote info query")?;

    // Build a map from user_id -> Vec<VotesInfo> only for users who have votes
    let mut user_votes_map = HashMap::<String, Vec<VotesInfo>>::with_capacity(rows.len());

    for row in rows {
        let voter_id_string: String = row
            .try_get("voter_id_string")
            .with_context(|| "Error getting voter_id_string from row")?;
        let election_id: Uuid = row
            .try_get("election_id")
            .with_context(|| "Error getting election_id from row")?;
        let num_votes: i64 = row
            .try_get("num_votes")
            .with_context(|| "Error getting num_votes from row")?;
        let last_voted_at: DateTime<Utc> = row
            .try_get("last_voted_at")
            .with_context(|| "Error getting last_voted_at from row")?;

        user_votes_map
            .entry(voter_id_string)
            .or_insert_with(Vec::new)
            .push(VotesInfo {
                election_id: election_id.to_string(),
                num_votes: num_votes as usize,
                last_voted_at: last_voted_at.to_string(),
            });
    }

    // Attach votes_info to each user in-place. Then do datafix logic if needed.
    // keep the same user order by iterating in place.
    for user in &mut users {
        let user_id = user
            .id
            .as_ref()
            .ok_or_else(|| anyhow!("Encountered a user without an ID"))?;

        // Get the collected VotesInfo from the map, or empty Vec if none
        let mut votes_info = user_votes_map.remove(user_id).unwrap_or_default();

        // If this is a "datafix" event, adjust the votes_info by checking the user's attributes
        if is_datafix_event {
            if let Some(attributes) = &user.attributes {
                if voted_via_not_internet_channel(&attributes) {
                    votes_info = vec![VotesInfo {
                        election_id: "".to_string(), // Not used for datafix
                        num_votes: 1,
                        last_voted_at: "".to_string(), // Not used for datafix
                    }];
                }
            }
        }

        user.votes_info = Some(votes_info);
    }

    // filter by has_voted, if needed - keep only users with at least one vote
    if let Some(has_voted) = filter_by_has_voted {
        users.retain(|user| {
            let info_count = user.votes_info.as_ref().map(|v| v.len()).unwrap_or(0);
            if has_voted {
                info_count > 0
            } else {
                info_count == 0
            }
        });
    }

    Ok(users)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CastVoteCountByIp {
    id: String,
    ip: Option<String>,
    country: Option<String>,
    vote_count: Option<i64>,
    election_name: String,
    election_id: String,
    voters_id: Vec<String>,
}
impl TryFrom<Row> for CastVoteCountByIp {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(CastVoteCountByIp {
            id: item.try_get::<_, i64>("id")?.to_string(),
            ip: item.try_get("ip").unwrap_or(None),
            country: item.try_get("country").unwrap_or(None),
            vote_count: item.try_get("vote_count")?,
            election_name: item.try_get("election_name")?,
            election_id: item.try_get::<_, Uuid>("election_id")?.to_string(),
            voters_id: item.try_get("voters_id")?,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ListCastVotesByIpFilter {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub ip: Option<String>,
    pub country: Option<String>,
    pub election_id: Option<String>,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_top_count_votes_by_ip(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    filter: ListCastVotesByIpFilter,
) -> Result<(Vec<CastVoteCountByIp>, i32)> {
    let low_sql_limit = PgConfig::from_env()?.low_sql_limit;
    let default_sql_limit = PgConfig::from_env()?.default_sql_limit;
    let query_limit: i64 =
        std::cmp::min(low_sql_limit, filter.limit.unwrap_or(default_sql_limit)).into();
    let query_offset: i64 = if let Some(offset_val) = filter.offset {
        offset_val.into()
    } else {
        0
    };

    let ip_pattern: Option<String> = if let Some(ip_val) = filter.ip {
        Some(format!("%{ip_val}%"))
    } else {
        None
    };

    let country_pattern: Option<String> = if let Some(country_val) = filter.country {
        Some(format!("%{country_val}%"))
    } else {
        None
    };
    let election_id_pattern: Option<Uuid> = if let Some(election_id_val) = filter.election_id {
        match Uuid::parse_str(&election_id_val) {
            Ok(uuid) => Some(uuid),
            Err(e) => None,
        }
    } else {
        None
    };

    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT 
                ROW_NUMBER() OVER (ORDER BY COUNT(*) DESC) AS id,
                cv.annotations->>'ip' AS ip,         
                cv.annotations->>'country' AS country,
                array_agg(COALESCE(cv.voter_id_string, '')) AS voters_id,
                cv.election_id as election_id,
                COUNT(*) AS vote_count,
                e.name AS election_name
            FROM 
                sequent_backend.cast_vote cv
            JOIN 
                sequent_backend.election e ON cv.election_id = e.id
            WHERE 
                cv.tenant_id = $1
                AND cv.election_event_id = $2
                AND cv.annotations ? 'ip'                
                AND cv.annotations ? 'country'    
                AND ($3::VARCHAR IS NULL OR cv.annotations->>'ip' ILIKE $3)
                AND ($4::VARCHAR IS NULL OR cv.annotations->>'country' ILIKE $4)
                AND ($5::UUID IS NULL OR cv.election_id = $5)
            GROUP BY 
                cv.annotations->>'ip',               
                cv.annotations->>'country',     
                cv.election_id,
                e.name
            ORDER BY 
                vote_count DESC
            LIMIT $6 OFFSET $7;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the statement: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &ip_pattern,
                &country_pattern,
                &election_id_pattern,
                &query_limit,
                &query_offset,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error getting cast votes: {err}"))?;

    let count: i32 = rows
        .len()
        .try_into()
        .map_err(|err| anyhow!("Error counting: {err}"))?;

    let cast_votes_by_ip: Vec<CastVoteCountByIp> = rows
        .into_iter()
        .map(|row| -> Result<CastVoteCountByIp> { row.try_into() })
        .collect::<Result<Vec<CastVoteCountByIp>>>()
        .map_err(|err| anyhow!("Error collecting the votes: {err}"))?;

    Ok((cast_votes_by_ip, count))
}

#[instrument(err)]
pub async fn count_ballots_by_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<i64> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;

    // Prepare and execute the statement
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT COUNT(*)
                FROM (
                    SELECT DISTINCT ON (voter_id_string, area_id) voter_id_string, area_id
                    FROM "sequent_backend".cast_vote
                    WHERE
                        tenant_id = $1 AND
                        election_event_id = $2 AND
                        election_id = $3
                    ORDER BY voter_id_string, area_id, created_at DESC
                ) AS latest_votes
            "#,
        )
        .await?;

    let row = hasura_transaction
        .query_one(
            &statement,
            &[&tenant_uuid, &election_event_uuid, &election_uuid],
        )
        .await
        .map_err(|err| anyhow!("Error running the count query: {}", err))?;

    let vote_count: i64 = row.get(0); // Get the count from the first column

    Ok(vote_count)
}

#[instrument(err)]
pub async fn count_ballots_by_area_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
) -> Result<i64> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = Uuid::parse_str(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid = Uuid::parse_str(area_id)
        .map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;

    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT COUNT(*)
                FROM (
                    SELECT DISTINCT ON (voter_id_string, area_id) voter_id_string, area_id
                    FROM "sequent_backend".cast_vote
                    WHERE
                        tenant_id = $1 AND
                        election_event_id = $2 AND
                        election_id = $3 AND
                        area_id = $4
                    ORDER BY voter_id_string, area_id, created_at DESC
                ) AS latest_votes
            "#,
        )
        .await?;

    let row = hasura_transaction
        .query_one(
            &statement,
            &[
                &tenant_uuid,
                &election_event_uuid,
                &election_uuid,
                &area_uuid,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the count query: {}", err))?;

    let vote_count: i64 = row.get(0);

    Ok(vote_count)
}

#[instrument(err)]
pub async fn count_cast_votes_election_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    is_test_election: Option<bool>,
) -> Result<i64> {
    let tenant_uuid: uuid::Uuid = Uuid::parse_str(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = Uuid::parse_str(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;

    let test_elections_clause = match is_test_election {
        Some(true) => "AND el.name ILIKE '%Test%'".to_string(),
        Some(false) => "AND el.name NOT ILIKE '%Test%'".to_string(),
        None => "".to_string(),
    };

    let statement_str = format!(
        r#"
            SELECT COUNT(DISTINCT cv.voter_id_string) AS voter_count
            FROM sequent_backend.election el
            JOIN sequent_backend.cast_vote cv ON el.id = cv.election_id
            WHERE 
                cv.voter_id_string IS NOT NULL AND
                el.tenant_id = $1 AND 
                el.election_event_id = $2
                {test_elections_clause};
            "#
    );

    let statement = hasura_transaction.prepare(statement_str.as_str()).await?;

    let rows: Row = hasura_transaction
        .query_one(&statement, &[&tenant_uuid, &election_event_uuid])
        .await
        .map_err(|err| anyhow!("Error running the query: {}", err))?;

    let count = rows.try_get::<_, i64>("voter_count")?;

    Ok(count)
}

/// Returns the private signing key for the given voter.
///
/// The private key is generated and a log post
/// is published with the corresponding public key
/// (with StatementType::AdminPublicKey).
///
/// There is a possibility that the private key is created
/// but the notification fails. This is logged in
/// electorallog::post_voter_pk
#[instrument(err)]
pub async fn get_voter_signing_key(
    hasura_transaction: &Transaction<'_>,
    elog_database: &str,
    tenant_id: &str,
    event_id: &str,
    user_id: &str,
    area_id: &str,
) -> Result<StrandSignatureSk> {
    info!("Generating private signing key for voter {}", user_id);
    let sk = StrandSignatureSk::gen()?;
    let pk = StrandSignaturePk::from_sk(&sk)?;
    let pk = pk.to_der_b64_string()?;

    ElectoralLog::post_voter_pk(
        hasura_transaction,
        elog_database,
        tenant_id,
        event_id,
        user_id,
        &pk,
        area_id,
    )
    .await?;

    Ok(sk)
}
