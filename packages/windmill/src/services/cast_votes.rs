// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::database::PgConfig;
use crate::services::ballot_box::{wait_for_sequencer, TALLY_SEQUENCER_WAIT};
use crate::services::ballot_box_reads::{ballot_box_time, get_event_ballot_box};
use crate::services::electoral_log::ElectoralLog;
use crate::services::external::utils::{
    is_datafix_election_event_by_id, voted_via_not_internet_channel,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use deadpool_postgres::Transaction;
use electoral_log::adapters::ballot_box_reads::{
    tally_ballots_query, BucketRange, IpBallotsFilter,
};
use electoral_log::adapters::postgres::PostgresStore;
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
use tokio_util::io::StreamReader;
use tracing::{info, instrument};
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

/// The tally input of an election's area, as CSV rows `(voter_id, content,
/// voting_channel)`, once every ballot the area accepted is in the electoral log.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_area_ballots(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    election_id: &str,
    output_file: &PathBuf,
) -> Result<()> {
    let ballot_box = get_event_ballot_box(hasura_transaction, tenant_id, election_event_id).await?;
    let store = &ballot_box.store;
    let waiting = wait_for_sequencer(TALLY_SEQUENCER_WAIT, || {
        store.unsequenced_count(election_event_id, election_id, area_id)
    })
    .await?;
    if waiting > 0 {
        return Err(anyhow!(
            "Refusing to extract ballots for election {election_id} area {area_id}: \
             {waiting} accepted ballot(s) are not in the electoral log yet. Tally again \
             once the sequencer has appended them"
        ));
    }
    let unrecorded = store
        .unrecorded_count(&ballot_box.board, election_event_id, election_id, area_id)
        .await?;
    if unrecorded > 0 {
        return Err(anyhow!(
            "Refusing to extract ballots for election {election_id} area {area_id}: \
             {unrecorded} ballot(s) have no cast-vote record in the electoral log"
        ));
    }
    let query = tally_ballots_query(election_event_id, election_id, area_id)?;
    let client = store.client().await?;
    let reader = client
        .copy_out(&format!("COPY ({query}) TO STDOUT WITH (FORMAT CSV)"))
        .await?;
    write_copy_out(reader, output_file).await
}

async fn write_copy_out(
    reader: tokio_postgres::CopyOutStream,
    output_file: &PathBuf,
) -> Result<()> {
    let tokio_temp_file = File::create(output_file)
        .await
        .with_context(|| format!("Error creating {output_file:?}"))?;
    let mut writer = BufWriter::new(tokio_temp_file);

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ElectionCastVotes {
    pub election_id: String,
    pub census: i64,
    pub cast_votes: i64,
}

const MAX_VOTES_TIME_BUCKETS: i32 = 1000;
/// How times are given to the ballot box.
const BALLOT_BOX_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.f";

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
    pub channel: VotingStatusChannel,
    pub day_count: i64,
}

fn parse_voting_channel(channel: &str) -> Result<VotingStatusChannel> {
    VotingStatusChannel::from_str(channel)
        .map_err(|error| anyhow!("Invalid voting channel {channel}: {error}"))
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct VotersByChannel {
    pub channel: VotingStatusChannel,
    pub count: i64,
}

/// Counts each voter once under the channel of their latest valid ballot.
#[instrument(skip(transaction), err)]
pub async fn get_count_distinct_voters_by_channel(
    transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<Vec<VotersByChannel>> {
    let election_id = election_id.map(parse_uuid_v4).transpose()?;
    get_event_ballot_box(transaction, tenant_id, election_event_id)
        .await?
        .store
        .voters_by_channel(
            election_event_id,
            election_id.map(|id| id.to_string()).as_deref(),
        )
        .await?
        .into_iter()
        .map(|(channel, count)| {
            Ok(VotersByChannel {
                channel: parse_voting_channel(&channel)?,
                count,
            })
        })
        .collect()
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
    let start =
        parse_votes_time_boundary(start_date, false).with_context(|| "Error parsing start_date")?;
    let end =
        parse_votes_time_boundary(end_date, true).with_context(|| "Error parsing end_date")?;
    validate_votes_time_range(start, end, resolution, bucket_count)?;
    let election_id = election_id
        .as_deref()
        .map(parse_uuid_v4)
        .transpose()?
        .map(|id| id.to_string());
    let start = start.format(BALLOT_BOX_TIME_FORMAT).to_string();
    let end = end.format(BALLOT_BOX_TIME_FORMAT).to_string();
    let range = BucketRange {
        resolution: resolution.as_sql(),
        time_zone: user_timezone,
        start: &start,
        end: &end,
        last_buckets: bucket_count,
    };
    get_event_ballot_box(transaction, tenant_id, election_event_id)
        .await?
        .store
        .ballots_per_bucket(
            election_event_id,
            election_id.as_deref(),
            &range,
            &VotingStatusChannel::ONLINE.to_string(),
        )
        .await?
        .into_iter()
        .map(|bucket| {
            Ok(CastVotesPerDay {
                day: bucket.day,
                bucket: bucket.bucket,
                channel: parse_voting_channel(&bucket.channel)?,
                day_count: bucket.count,
            })
        })
        .collect()
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
    let votes = get_event_ballot_box(hasura_transaction, tenant_id, election_event_id)
        .await?
        .store
        .votes_of_voters(
            election_event_id,
            &user_ids,
            election_uuid.map(|id| id.to_string()).as_deref(),
        )
        .await?
        .into_iter()
        .map(|votes| {
            Ok((
                votes.voter_id,
                VotesInfo {
                    election_id: votes.election_id,
                    num_votes: usize::try_from(votes.votes)?,
                    last_voted_at: ballot_box_time(votes.last_voted_at)?.to_string(),
                },
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut user_votes_map = HashMap::<String, Vec<VotesInfo>>::new();
    for (voter_id, votes_info) in votes {
        user_votes_map.entry(voter_id).or_default().push(votes_info);
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
    let ballot_box = get_event_ballot_box(hasura_transaction, tenant_id, election_event_id).await?;
    let election_id = election_id_pattern.map(|id| id.to_string());
    let filter = IpBallotsFilter {
        ip_pattern: ip_pattern.as_deref(),
        country_pattern: country_pattern.as_deref(),
        election_id: election_id.as_deref(),
        limit: query_limit,
        offset: query_offset,
    };
    get_top_count_votes_by_ip_from_ballot_box(
        hasura_transaction,
        &ballot_box.store,
        tenant_id,
        election_event_id,
        &filter,
    )
    .await
}

/// Ballots by IP address, numbered by rank.
async fn get_top_count_votes_by_ip_from_ballot_box(
    hasura_transaction: &Transaction<'_>,
    store: &PostgresStore,
    tenant_id: &str,
    election_event_id: &str,
    filter: &IpBallotsFilter<'_>,
) -> Result<(Vec<CastVoteCountByIp>, i32)> {
    let groups = store.ballots_by_ip(election_event_id, filter).await?;
    let election_ids: Vec<String> = groups
        .iter()
        .map(|group| group.election_id.clone())
        .collect();
    let presentations: HashMap<String, Option<Value>> = hasura_transaction
        .query(
            "SELECT id::text, presentation FROM sequent_backend.election \
             WHERE tenant_id = $1::text::uuid AND election_event_id = $2::text::uuid \
               AND id = ANY($3::text[]::uuid[])",
            &[&tenant_id, &election_event_id, &election_ids],
        )
        .await
        .context("Error reading the elections' presentation")?
        .into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect::<Result<_>>()?;
    let ranked: Vec<CastVoteCountByIp> = groups
        .into_iter()
        .enumerate()
        .map(|(index, group)| {
            let rank = filter.offset + i64::try_from(index)? + 1;
            Ok(CastVoteCountByIp {
                id: rank.to_string(),
                ip: Some(group.ip),
                country: Some(group.country),
                vote_count: Some(group.count),
                election_presentation: presentations.get(&group.election_id).cloned().flatten(),
                election_id: group.election_id,
                voters_id: group.voter_ids,
            })
        })
        .collect::<Result<_>>()?;
    let count = i32::try_from(ranked.len())?;
    Ok((ranked, count))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
