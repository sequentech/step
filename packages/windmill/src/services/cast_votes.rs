// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::database::PgConfig;
use super::sql_utils::escape_sql_literal;
use crate::postgres::cast_vote::{
    count_distinct_voters_by_channel_query, count_votes_per_day_query, CastVoteRelation,
};
use crate::services::electoral_log::ElectoralLog;
use crate::services::external::utils::{
    is_datafix_election_event_by_id, voted_via_not_internet_channel,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use deadpool_postgres::Transaction;
use futures::TryStreamExt;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::keycloak::{User, VotesInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use strum_macros::{Display, EnumString};
use tokio::fs::File;
use tokio::io::{copy, AsyncWriteExt, BufWriter};
use tokio_postgres::row::Row;
use tokio_util::io::StreamReader;
use tracing::{debug, info, instrument};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Display, EnumString, PartialEq, Eq)]
pub enum CastVoteStatus {
    #[serde(rename = "in-progress")]
    #[strum(serialize = "in-progress")]
    InProgress,
    #[serde(rename = "valid")]
    #[strum(serialize = "valid")]
    Valid,
    #[serde(rename = "discarded")]
    #[strum(serialize = "discarded")]
    Discarded,
}

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
    pub status: CastVoteStatus,
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
            status: CastVoteStatus::from_str(&item.try_get::<_, String>("status")?)
                .map_err(|err| anyhow!("Invalid cast vote status: {err}"))?,
        })
    }
}

/// Minimal identity of an `in-progress` cast vote, used to enqueue Datafix
/// processing without loading full ballot content just to schedule the work.
#[derive(Debug)]
pub struct InProgressCastVote {
    pub id: String,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub voter_id: String,
}

#[instrument(skip(hasura_transaction), err)]
pub async fn find_area_ballots(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    election_id: &str,
    output_file: &PathBuf,
) -> Result<()> {
    // COPY does not support parameters so we have to add them using format.
    // Validate as v4 UUIDs before interpolating into SQL.
    parse_uuid_v4(tenant_id)?;
    parse_uuid_v4(election_event_id)?;
    parse_uuid_v4(area_id)?;
    parse_uuid_v4(election_id)?;
    let tenant_id = escape_sql_literal(tenant_id);
    let election_event_id = escape_sql_literal(election_event_id);
    let area_id = escape_sql_literal(area_id);
    let election_id = escape_sql_literal(election_id);
    let status = escape_sql_literal(&CastVoteStatus::Valid.to_string());
    let default_channel = escape_sql_literal(&VotingStatusChannel::ONLINE.to_string());
    let areas_statement = format!(
        r#"
                    SELECT DISTINCT ON (election_id, voter_id_string)
                        voter_id_string,
                        content,
                        COALESCE(annotations->>'voting_channel', '{default_channel}') AS voting_channel
                    FROM "sequent_backend".cast_vote
                    WHERE
                        tenant_id = '{tenant_id}' AND
                        election_event_id = '{election_event_id}' AND
                        area_id = '{area_id}' AND
                        election_id = '{election_id}' AND
                        status = '{status}'
                    ORDER BY
                        election_id,
                        voter_id_string,
                        created_at DESC NULLS LAST,
                        id DESC
                "#
    );

    let tokio_temp_file = File::create(output_file)
        .await
        .expect("Could not create/open temporary file for tokio");

    let copy_out_query = format!("COPY ({}) TO STDOUT WITH (FORMAT CSV)", areas_statement);
    let mut writer = BufWriter::new(tokio_temp_file);

    debug!("copy_out_query: {copy_out_query}");

    let reader = hasura_transaction.copy_out(&copy_out_query).await?;

    let adapt_pg_error_to_io_error = |pg_err: tokio_postgres::Error| {
        std::io::Error::new(std::io::ErrorKind::Other, pg_err.to_string())
    };
    let io_error_stream = reader.map_err(adapt_pg_error_to_io_error);

    let async_reader = StreamReader::new(io_error_stream);
    tokio::pin!(async_reader);

    let bytes_copied = copy(&mut async_reader, &mut writer).await?;

    info!("ballot bytes_copied: {bytes_copied}");

    writer.flush().await?;

    Ok(())
}

/// Votes younger than this are skipped by the review beat: their
/// process_cast_vote task published directly by harvest is normally still in
/// flight, so re-enqueueing them would only produce redundant PgLock skips.
const IN_PROGRESS_ENQUEUE_GRACE_SECS: f64 = 90.0;

/// Returns a batch of `in-progress` cast votes using keyset pagination on
/// `(tenant_id, election_event_id, election_id, voter_id_string)`: pass the
/// last returned identity as `after` to
/// fetch the next batch (offset pagination is unsafe while workers update
/// statuses). `status` is inlined as a literal so the planner can match the
/// partial index `idx_cast_vote_in_progress`. `DISTINCT ON` keeps only the
/// newest vote per voter; older stacked re-votes drain on later beat cycles.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_in_progress_cast_votes_batch(
    hasura_transaction: &Transaction<'_>,
    limit: i64,
    after: Option<(Uuid, Uuid, Uuid, String)>,
) -> Result<Option<Vec<InProgressCastVote>>> {
    let (after_tenant_id, after_event_id, after_election_id, after_voter_id) = match after {
        Some((tenant_id, event_id, election_id, voter_id)) => (
            Some(tenant_id),
            Some(event_id),
            Some(election_id),
            Some(voter_id),
        ),
        None => (None, None, None, None),
    };
    let in_progress_status = CastVoteStatus::InProgress.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                    SELECT DISTINCT ON (tenant_id, election_event_id, election_id, voter_id_string)
                        id,
                        tenant_id,
                        election_event_id,
                        election_id,
                        voter_id_string
                    FROM "sequent_backend".cast_vote cv
                    WHERE
                        cv.status = $6 AND
                        cv.election_id IS NOT NULL AND
                        cv.voter_id_string IS NOT NULL AND
                        cv.created_at < NOW() - make_interval(secs => $7) AND
                        ($1::UUID IS NULL OR
                            (cv.tenant_id, cv.election_event_id, cv.election_id, cv.voter_id_string) >
                            ($1::UUID, $2::UUID, $3::UUID, $4::VARCHAR))
                    ORDER BY cv.tenant_id, cv.election_event_id, cv.election_id, cv.voter_id_string, cv.created_at DESC
                    LIMIT $5
                "#,
        )
        .await?;
    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &after_tenant_id,
                &after_event_id,
                &after_election_id,
                &after_voter_id,
                &limit,
                &in_progress_status,
                &IN_PROGRESS_ENQUEUE_GRACE_SECS,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error running the CastVote query: {}", err))?;

    let cast_votes = rows
        .into_iter()
        .map(|row| {
            Ok(InProgressCastVote {
                id: row.try_get::<_, Uuid>("id")?.to_string(),
                tenant_id: row.try_get("tenant_id")?,
                election_event_id: row.try_get("election_event_id")?,
                election_id: row.try_get("election_id")?,
                voter_id: row.try_get("voter_id_string")?,
            })
        })
        .collect::<Result<Vec<InProgressCastVote>>>()?;
    match cast_votes.is_empty() {
        true => Ok(None),
        false => Ok(Some(cast_votes)),
    }
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

const MAX_VOTES_TIME_BUCKETS: i32 = 1000;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum VotesTimeResolution {
    Minute,
    Hour,
    #[default]
    Day,
}

impl VotesTimeResolution {
    fn as_sql(self) -> &'static str {
        match self {
            Self::Minute => "minute",
            Self::Hour => "hour",
            Self::Day => "day",
        }
    }

    fn seconds(self) -> i64 {
        match self {
            Self::Minute => 60,
            Self::Hour => 60 * 60,
            Self::Day => 24 * 60 * 60,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct CastVotesPerDay {
    pub day: String,
    pub bucket: String,
    pub channel: String,
    pub day_count: i64,
}

impl TryFrom<Row> for CastVotesPerDay {
    type Error = anyhow::Error;
    fn try_from(item: Row) -> Result<Self> {
        Ok(CastVotesPerDay {
            day: item.try_get::<_, NaiveDate>("day")?.to_string(),
            bucket: item
                .try_get::<_, NaiveDateTime>("bucket")?
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            channel: item.try_get("channel")?,
            day_count: item.try_get::<_, i64>("day_count")?,
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct VotersByChannel {
    pub channel: String,
    pub count: i64,
}

impl TryFrom<Row> for VotersByChannel {
    type Error = anyhow::Error;

    fn try_from(item: Row) -> Result<Self> {
        Ok(VotersByChannel {
            channel: item.try_get("channel")?,
            count: item.try_get("count")?,
        })
    }
}

/// Counts each voter once under the channel of their latest valid vote.
/// Votes created before the channel annotation was introduced are online.
#[instrument(skip(transaction), err)]
pub async fn get_count_distinct_voters_by_channel(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<Vec<VotersByChannel>> {
    get_count_distinct_voters_by_channel_from_relation(
        transaction,
        tenant_id,
        election_event_id,
        election_id,
        CastVoteRelation::Production,
    )
    .await
}

async fn get_count_distinct_voters_by_channel_from_relation(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    cast_vote_relation: CastVoteRelation,
) -> Result<Vec<VotersByChannel>> {
    let election_id = election_id.map(parse_uuid_v4).transpose()?;
    let status = CastVoteStatus::Valid.to_string();
    let default_channel = VotingStatusChannel::ONLINE.to_string();
    let sql = count_distinct_voters_by_channel_query(cast_vote_relation, election_id.is_some());
    let statement = transaction.prepare(&sql).await?;

    let tenant_id = parse_uuid_v4(tenant_id)?;
    let election_event_id = parse_uuid_v4(election_event_id)?;
    let rows = match election_id {
        Some(election_id) => {
            transaction
                .query(
                    &statement,
                    &[
                        &tenant_id,
                        &election_event_id,
                        &status,
                        &default_channel,
                        &election_id,
                    ],
                )
                .await?
        }
        None => {
            transaction
                .query(
                    &statement,
                    &[&tenant_id, &election_event_id, &status, &default_channel],
                )
                .await?
        }
    };

    rows.into_iter().map(TryInto::try_into).collect()
}

#[instrument(err)]
pub async fn count_cast_votes_election(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    is_test_election: Option<bool>,
) -> Result<Vec<ElectionCastVotes>> {
    let tenant_uuid: uuid::Uuid = parse_uuid_v4(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;

    let test_elections_clause = match is_test_election {
        Some(true) => "AND el.name ILIKE '%Test%'".to_string(),
        Some(false) => "AND el.name NOT ILIKE '%Test%'".to_string(),
        None => "".to_string(),
    };
    let status = CastVoteStatus::Valid.to_string();
    let statement_str = format!(
        r#"
            SELECT el.id AS election_id, COUNT(DISTINCT cv.voter_id_string) AS cast_votes
            FROM sequent_backend.election el
            LEFT JOIN (
                SELECT DISTINCT election_id, voter_id_string
                FROM sequent_backend.cast_vote
                WHERE status = $3
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
        .query(&statement, &[&tenant_uuid, &election_event_uuid, &status])
        .await
        .map_err(|err| anyhow!("Error running the query: {}", err))?;
    let count_data = rows
        .into_iter()
        .map(|row| -> Result<ElectionCastVotes> { row.try_into() })
        .collect::<Result<Vec<ElectionCastVotes>>>()?;

    Ok(count_data)
}

fn parse_votes_time_boundary(value: &str, end_of_day: bool) -> Result<NaiveDateTime> {
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(value) = NaiveDateTime::parse_from_str(value, format) {
            return Ok(value);
        }
    }

    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .with_context(|| format!("Error parsing time boundary: {value}"))?;
    let time = if end_of_day {
        NaiveTime::from_hms_micro_opt(23, 59, 59, 999_999)
    } else {
        NaiveTime::from_hms_opt(0, 0, 0)
    }
    .ok_or_else(|| anyhow!("Error building time boundary"))?;

    Ok(date.and_time(time))
}

fn validate_votes_time_range(
    start: NaiveDateTime,
    end: NaiveDateTime,
    resolution: VotesTimeResolution,
    bucket_count: Option<i32>,
) -> Result<()> {
    if end < start {
        return Err(anyhow!("end_date must not be earlier than start_date"));
    }

    let requested_buckets = match bucket_count {
        Some(count) if (1..=MAX_VOTES_TIME_BUCKETS).contains(&count) => i64::from(count),
        Some(_) => {
            return Err(anyhow!(
                "bucket_count must be between 1 and {MAX_VOTES_TIME_BUCKETS}"
            ))
        }
        None => end
            .signed_duration_since(start)
            .num_seconds()
            .checked_div(resolution.seconds())
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| anyhow!("Unable to calculate requested time buckets"))?,
    };

    if requested_buckets > i64::from(MAX_VOTES_TIME_BUCKETS) {
        return Err(anyhow!(
            "Requested {requested_buckets} time buckets; maximum is {MAX_VOTES_TIME_BUCKETS}"
        ));
    }

    Ok(())
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
    resolution: VotesTimeResolution,
    bucket_count: Option<i32>,
) -> Result<Vec<CastVotesPerDay>> {
    get_count_votes_per_day_from_relation(
        transaction,
        tenant_id,
        election_event_id,
        start_date,
        end_date,
        election_id,
        user_timezone,
        resolution,
        bucket_count,
        CastVoteRelation::Production,
    )
    .await
}

async fn get_count_votes_per_day_from_relation(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    start_date: &str,
    end_date: &str,
    election_id: Option<String>,
    user_timezone: &str,
    resolution: VotesTimeResolution,
    bucket_count: Option<i32>,
    cast_vote_relation: CastVoteRelation,
) -> Result<Vec<CastVotesPerDay>> {
    let start_date_naive =
        parse_votes_time_boundary(start_date, false).with_context(|| "Error parsing start_date")?;
    let end_date_naive =
        parse_votes_time_boundary(end_date, true).with_context(|| "Error parsing end_date")?;
    validate_votes_time_range(start_date_naive, end_date_naive, resolution, bucket_count)?;

    let election_uuid = match election_id {
        Some(ref election_id_r) => Some(parse_uuid_v4(election_id_r.as_str())?),
        None => None,
    };
    let status = CastVoteStatus::Valid.to_string();
    let default_channel = VotingStatusChannel::ONLINE.to_string();
    let resolution_sql = resolution.as_sql();
    let sql = count_votes_per_day_query(cast_vote_relation);
    let total_areas_statement = transaction.prepare(&sql).await?;

    let rows: Vec<Row> = transaction
        .query(
            &total_areas_statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &start_date_naive,
                &end_date_naive,
                &user_timezone,
                &election_uuid,
                &status,
                &default_channel,
                &resolution_sql,
                &bucket_count,
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
        parse_uuid_v4(tenant_id).with_context(|| "Error parsing tenant_id as UUID")?;
    let election_event_uuid = parse_uuid_v4(election_event_id)
        .with_context(|| "Error parsing election_event_id as UUID")?;

    let election_uuid = match election_id {
        Some(ref election_id_s) => Some(
            parse_uuid_v4(election_id_s)
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
    let discarded_status = CastVoteStatus::Discarded.to_string();
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
            AND v.status <> $5
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
                &discarded_status,
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

    Ok(users)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CastVoteCountByIp {
    id: String,
    ip: Option<String>,
    country: Option<String>,
    vote_count: Option<i64>,
    election_presentation: Option<Value>,
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
            election_presentation: item.try_get("election_presentation")?,
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
        match parse_uuid_v4(&election_id_val) {
            Ok(uuid) => Some(uuid),
            Err(e) => None,
        }
    } else {
        None
    };
    let status = CastVoteStatus::Valid.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT
                ROW_NUMBER() OVER (ORDER BY vote_count DESC) AS id,
                *
            FROM (
                SELECT
                    cv.annotations->>'ip' AS ip,
                    cv.annotations->>'country' AS country,
                    array_agg(COALESCE(cv.voter_id_string, '')) AS voters_id,
                    cv.election_id,
                    COUNT(*) AS vote_count,
                    e.presentation AS election_presentation
                FROM sequent_backend.cast_vote cv
                JOIN sequent_backend.election e ON cv.election_id = e.id
                WHERE
                    cv.tenant_id = $1
                    AND cv.election_event_id = $2
                    AND cv.annotations ? 'ip'
                    AND cv.annotations ? 'country'
                    AND ($3::VARCHAR IS NULL OR cv.annotations->>'ip' ILIKE $3)
                    AND ($4::VARCHAR IS NULL OR cv.annotations->>'country' ILIKE $4)
                    AND ($5::UUID IS NULL OR cv.election_id = $5)
                    AND cv.status = $8
                GROUP BY
                    cv.annotations->>'ip',
                    cv.annotations->>'country',
                    cv.election_id,
                    e.presentation
            ) t
            ORDER BY vote_count DESC
            LIMIT $6 OFFSET $7;
            "#,
        )
        .await
        .map_err(|err| anyhow!("Error preparing the statement: {err}"))?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &ip_pattern,
                &country_pattern,
                &election_id_pattern,
                &query_limit,
                &query_offset,
                &status,
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
    let tenant_uuid: uuid::Uuid = parse_uuid_v4(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = parse_uuid_v4(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let status = CastVoteStatus::Valid.to_string();

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
                        election_id = $3 AND
                        status = $4
                    ORDER BY voter_id_string, area_id, created_at DESC
                ) AS latest_votes
            "#,
        )
        .await?;

    let row = hasura_transaction
        .query_one(
            &statement,
            &[&tenant_uuid, &election_event_uuid, &election_uuid, &status],
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
    let tenant_uuid: uuid::Uuid = parse_uuid_v4(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;
    let election_uuid: uuid::Uuid = parse_uuid_v4(election_id)
        .map_err(|err| anyhow!("Error parsing election_id as UUID: {}", err))?;
    let area_uuid: uuid::Uuid =
        parse_uuid_v4(area_id).map_err(|err| anyhow!("Error parsing area_id as UUID: {}", err))?;
    let status = CastVoteStatus::Valid.to_string();

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
                        area_id = $4 AND
                        status = $5
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
                &status,
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
    let tenant_uuid: uuid::Uuid = parse_uuid_v4(tenant_id)
        .map_err(|err| anyhow!("Error parsing tenant_id as UUID: {}", err))?;
    let election_event_uuid: uuid::Uuid = parse_uuid_v4(election_event_id)
        .map_err(|err| anyhow!("Error parsing election_event_id as UUID: {}", err))?;

    let test_elections_clause = match is_test_election {
        Some(true) => "AND el.name ILIKE '%Test%'".to_string(),
        Some(false) => "AND el.name NOT ILIKE '%Test%'".to_string(),
        None => "".to_string(),
    };
    let status = CastVoteStatus::Valid.to_string();
    let statement_str = format!(
        r#"
            SELECT COUNT(DISTINCT cv.voter_id_string) AS voter_count
            FROM sequent_backend.election el
            JOIN sequent_backend.cast_vote cv ON el.id = cv.election_id
            WHERE 
                cv.voter_id_string IS NOT NULL AND
                cv.status = $3 AND
                el.tenant_id = $1 AND 
                el.election_event_id = $2
                {test_elections_clause};
            "#
    );

    let statement = hasura_transaction.prepare(statement_str.as_str()).await?;

    let rows: Row = hasura_transaction
        .query_one(&statement, &[&tenant_uuid, &election_event_uuid, &status])
        .await
        .map_err(|err| anyhow!("Error running the query: {}", err))?;

    let count = rows.try_get::<_, i64>("voter_count")?;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::database::generate_hasura_pool;

    const TENANT_ID: &str = "10000000-0000-4000-8000-000000000001";
    const ELECTION_EVENT_ID: &str = "10000000-0000-4000-8000-000000000002";
    const ELECTION_ID: &str = "10000000-0000-4000-8000-000000000003";

    fn counts_by_channel(rows: Vec<VotersByChannel>) -> HashMap<String, i64> {
        rows.into_iter()
            .map(|row| (row.channel, row.count))
            .collect()
    }

    fn counts_by_day_and_channel(rows: Vec<CastVotesPerDay>) -> HashMap<(String, String), i64> {
        rows.into_iter()
            .map(|row| ((row.day, row.channel), row.day_count))
            .collect()
    }

    #[test]
    fn accepts_supported_time_resolutions_and_bounded_ranges() {
        let start = parse_votes_time_boundary("2026-01-01T10:15:00", false).unwrap();
        let end = parse_votes_time_boundary("2026-01-01T11:14:59", true).unwrap();

        assert!(
            validate_votes_time_range(start, end, VotesTimeResolution::Minute, Some(60),).is_ok()
        );
        assert!(validate_votes_time_range(start, end, VotesTimeResolution::Hour, None,).is_ok());
    }

    #[test]
    fn rejects_invalid_or_excessive_time_ranges() {
        let start = parse_votes_time_boundary("2026-01-01", false).unwrap();
        let end = parse_votes_time_boundary("2026-01-02", true).unwrap();

        assert!(validate_votes_time_range(start, end, VotesTimeResolution::Minute, None).is_err());
        assert!(validate_votes_time_range(end, start, VotesTimeResolution::Day, Some(2)).is_err());
        assert!(
            validate_votes_time_range(start, end, VotesTimeResolution::Day, Some(1001)).is_err()
        );
    }

    #[tokio::test]
    #[ignore = "requires PostgreSQL configured through HASURA_DB__*; exercised by the dedicated CI job"]
    async fn voters_by_channel_defaults_legacy_votes_and_uses_latest_valid_revote() {
        let pool = generate_hasura_pool().await.unwrap();
        let mut client = pool.get().await.unwrap();
        let transaction = client.transaction().await.unwrap();

        transaction
            .batch_execute(
                r#"
                CREATE TEMP TABLE cast_vote_stats_test (
                    id UUID PRIMARY KEY,
                    tenant_id UUID NOT NULL,
                    election_event_id UUID NOT NULL,
                    election_id UUID NOT NULL,
                    voter_id_string TEXT,
                    status TEXT NOT NULL,
                    annotations JSONB,
                    created_at TIMESTAMPTZ
                );

                INSERT INTO cast_vote_stats_test (
                    id,
                    tenant_id,
                    election_event_id,
                    election_id,
                    voter_id_string,
                    status,
                    annotations,
                    created_at
                ) VALUES
                    ('10000000-0000-4000-8000-000000000010', '10000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000003', 'legacy-voter', 'valid', '{}', '2026-01-01T00:00:00Z'),
                    ('10000000-0000-4000-8000-000000000011', '10000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000003', 'revoting-voter', 'valid', '{"voting_channel":"KIOSK"}', '2026-01-01T00:00:00Z'),
                    ('10000000-0000-4000-8000-000000000012', '10000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000003', 'revoting-voter', 'valid', '{"voting_channel":"TELEPHONE"}', '2026-01-02T00:00:00Z'),
                    ('10000000-0000-4000-8000-000000000013', '10000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000003', 'discarded-revote-voter', 'valid', '{"voting_channel":"KIOSK"}', '2026-01-01T00:00:00Z'),
                    ('10000000-0000-4000-8000-000000000014', '10000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000003', 'discarded-revote-voter', 'discarded', '{"voting_channel":"TELEPHONE"}', '2026-01-02T00:00:00Z'),
                    ('10000000-0000-4000-8000-000000000015', '10000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000004', 'second-election-voter', 'valid', '{"voting_channel":"ONLINE"}', '2026-01-01T00:00:00Z'),
                    ('10000000-0000-4000-8000-000000000016', '10000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000003', NULL, 'valid', '{"voting_channel":"ONLINE"}', '2026-01-01T00:00:00Z'),
                    ('10000000-0000-4000-8000-000000000017', '20000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000003', 'other-tenant-voter', 'valid', '{"voting_channel":"ONLINE"}', '2026-01-01T00:00:00Z');
                "#,
            )
            .await
            .unwrap();

        let event_counts = counts_by_channel(
            get_count_distinct_voters_by_channel_from_relation(
                &transaction,
                TENANT_ID,
                ELECTION_EVENT_ID,
                None,
                CastVoteRelation::StatisticsTest,
            )
            .await
            .unwrap(),
        );
        assert_eq!(event_counts.get("ONLINE"), Some(&2));
        assert_eq!(event_counts.get("KIOSK"), Some(&1));
        assert_eq!(event_counts.get("TELEPHONE"), Some(&1));

        let election_counts = counts_by_channel(
            get_count_distinct_voters_by_channel_from_relation(
                &transaction,
                TENANT_ID,
                ELECTION_EVENT_ID,
                Some(ELECTION_ID),
                CastVoteRelation::StatisticsTest,
            )
            .await
            .unwrap(),
        );
        assert_eq!(election_counts.get("ONLINE"), Some(&1));
        assert_eq!(election_counts.get("KIOSK"), Some(&1));
        assert_eq!(election_counts.get("TELEPHONE"), Some(&1));

        let votes_per_day = counts_by_day_and_channel(
            get_count_votes_per_day_from_relation(
                &transaction,
                TENANT_ID,
                ELECTION_EVENT_ID,
                "2026-01-01",
                "2026-01-03",
                Some(ELECTION_ID.to_string()),
                "UTC",
                VotesTimeResolution::Day,
                None,
                CastVoteRelation::StatisticsTest,
            )
            .await
            .unwrap(),
        );
        assert_eq!(
            votes_per_day.get(&("2026-01-01".to_string(), "ONLINE".to_string())),
            Some(&2)
        );
        assert_eq!(
            votes_per_day.get(&("2026-01-01".to_string(), "KIOSK".to_string())),
            Some(&2)
        );
        assert_eq!(
            votes_per_day.get(&("2026-01-02".to_string(), "TELEPHONE".to_string())),
            Some(&1)
        );
        assert_eq!(
            votes_per_day.get(&("2026-01-03".to_string(), "ONLINE".to_string())),
            Some(&0)
        );

        transaction.rollback().await.unwrap();
    }
}
