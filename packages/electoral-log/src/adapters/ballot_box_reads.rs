// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Reads of the ballot box for voters, administrators and reports. Counts and
//! statistics cover valid ballots.

use super::ballot_box::{canonical_uuid, BallotStatus};
use super::postgres::PostgresStore;
use anyhow::{anyhow, Context, Result};
use tokio_postgres::Row;

/// A stored ballot, without its content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBallot {
    pub id: String,
    pub ballot_id: String,
    pub election_id: String,
    pub area_id: String,
    pub voter_id: String,
    pub status: BallotStatus,
    pub voting_channel: String,
    /// Microseconds since the epoch.
    pub accepted_at: i64,
}

/// A stored ballot with its content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BallotContent {
    pub ballot: StoredBallot,
    pub content: String,
}

/// Which ballot IDs a lookup matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BallotIdMatch<'a> {
    /// This ballot ID.
    Exact(&'a str),
    /// Ballot IDs that start with these characters.
    Prefix(&'a str),
}

/// Valid ballots in one time bucket and voting channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketCount {
    /// Start of the bucket, as `YYYY-MM-DDTHH:MM:SS` in the requested time zone.
    pub bucket: String,
    /// Day of the bucket, as `YYYY-MM-DD`.
    pub day: String,
    pub channel: String,
    pub count: i64,
}

/// Time buckets to count ballots in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketRange<'a> {
    /// `minute`, `hour` or `day`.
    pub resolution: &'a str,
    /// Time zone the buckets are in, as PostgreSQL names it.
    pub time_zone: &'a str,
    /// First and last time, as `YYYY-MM-DD HH:MM:SS[.ffffff]` in that time zone.
    /// Ignored when `last_buckets` is set.
    pub start: &'a str,
    pub end: &'a str,
    /// The latest this many buckets, up to now.
    pub last_buckets: Option<i32>,
}

/// Valid ballots cast from one IP address and country in one election.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpBallots {
    pub ip: String,
    pub country: String,
    pub election_id: String,
    /// The voter of each ballot.
    pub voter_ids: Vec<String>,
    pub count: i64,
}

/// Filters of `ballots_by_ip`; patterns are SQL `ILIKE` patterns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IpBallotsFilter<'a> {
    pub ip_pattern: Option<&'a str>,
    pub country_pattern: Option<&'a str>,
    pub election_id: Option<&'a str>,
    pub limit: i64,
    pub offset: i64,
}

/// A voter's ballots in one election that were not rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoterVotes {
    pub voter_id: String,
    pub election_id: String,
    pub votes: i64,
    /// Microseconds since the epoch.
    pub last_voted_at: i64,
}

/// Valid ballots and the voters who cast them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Participation {
    pub ballots: i64,
    pub voters: i64,
}

const STORED_BALLOT: &str = "id::text, ballot_id, election_id::text, area_id::text, voter_id, \
     status, voting_channel, (extract(epoch FROM accepted_at) * 1000000)::bigint";

const VALID: &str = "valid";
const PENDING: &str = "pending";
const REJECTED: &str = "rejected";

/// Query of the tally input of an election's area, as rows `(voter_id, content,
/// voting_channel)`: each voter's latest valid ballot among those the sequencer has
/// appended to the board, ordered by voter ID. The IDs are checked and written as
/// literals, so that the query can be streamed with `COPY`, which takes no
/// parameters.
pub fn tally_ballots_query(
    election_event_id: &str,
    election_id: &str,
    area_id: &str,
) -> Result<String> {
    let event = canonical_uuid(election_event_id)?;
    let election = canonical_uuid(election_id)?;
    let area = canonical_uuid(area_id)?;
    Ok(format!(
        "SELECT DISTINCT ON (b.voter_id) b.voter_id, b.content, b.voting_channel \
         FROM ballot_box_ballot b \
         WHERE b.election_event_id = '{event}' AND b.election_id = '{election}' \
           AND b.area_id = '{area}' AND b.status = '{VALID}' \
           AND NOT EXISTS (SELECT 1 FROM ballot_box_pending p \
                           WHERE p.election_event_id = b.election_event_id AND p.seq = b.seq) \
         ORDER BY b.voter_id, b.seq DESC"
    ))
}

fn stored_ballot(row: &Row) -> Result<StoredBallot> {
    let status: String = row.try_get(5)?;
    Ok(StoredBallot {
        id: row.try_get(0)?,
        ballot_id: row.try_get(1)?,
        election_id: row.try_get(2)?,
        area_id: row.try_get(3)?,
        voter_id: row.try_get(4)?,
        status: status
            .parse()
            .map_err(|_| anyhow!("Unknown ballot status {status:?}"))?,
        voting_channel: row.try_get(6)?,
        accepted_at: row.try_get(7)?,
    })
}

impl PostgresStore {
    /// A voter's ballots in an election event, in acceptance order.
    pub async fn voter_ballots(
        &self,
        election_event_id: &str,
        voter_id: &str,
    ) -> Result<Vec<StoredBallot>> {
        let rows = self
            .client()
            .await?
            .query(
                &format!(
                    "SELECT {STORED_BALLOT} FROM ballot_box_ballot \
                     WHERE election_event_id = $1::text::uuid AND voter_id = $2 \
                     ORDER BY seq"
                ),
                &[&election_event_id, &voter_id],
            )
            .await
            .context("Error reading the voter's ballots")?;
        rows.iter().map(stored_ballot).collect()
    }

    /// A voter's ballots in an election whose ID matches, with their content.
    pub async fn voter_ballot_contents(
        &self,
        election_event_id: &str,
        voter_id: &str,
        election_id: &str,
        ballot_id: BallotIdMatch<'_>,
    ) -> Result<Vec<BallotContent>> {
        let (condition, value) = match ballot_id {
            BallotIdMatch::Exact(value) => ("ballot_id = $4", value),
            BallotIdMatch::Prefix(value) => ("starts_with(ballot_id, $4)", value),
        };
        let rows = self
            .client()
            .await?
            .query(
                &format!(
                    "SELECT {STORED_BALLOT}, content FROM ballot_box_ballot \
                     WHERE election_event_id = $1::text::uuid AND voter_id = $2 \
                       AND election_id = $3::text::uuid AND {condition} \
                     ORDER BY seq"
                ),
                &[&election_event_id, &voter_id, &election_id, &value],
            )
            .await
            .context("Error looking up the voter's ballots")?;
        rows.iter()
            .map(|row| {
                Ok(BallotContent {
                    ballot: stored_ballot(row)?,
                    content: row.try_get(8)?,
                })
            })
            .collect()
    }

    /// Voters with a valid ballot, by the voting channel of their latest one.
    pub async fn voters_by_channel(
        &self,
        election_event_id: &str,
        election_id: Option<&str>,
    ) -> Result<Vec<(String, i64)>> {
        let rows = self
            .client()
            .await?
            .query(
                "WITH latest AS ( \
                     SELECT DISTINCT ON (voter_id) voting_channel \
                     FROM ballot_box_ballot \
                     WHERE election_event_id = $1::text::uuid AND status = $3 \
                       AND ($2::text IS NULL OR election_id = $2::text::uuid) \
                     ORDER BY voter_id, seq DESC \
                 ) \
                 SELECT voting_channel, count(*) FROM latest GROUP BY 1 ORDER BY 1",
                &[&election_event_id, &election_id, &VALID],
            )
            .await
            .context("Error counting voters by channel")?;
        rows.iter()
            .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
            .collect()
    }

    /// Valid ballots per time bucket and voting channel. Every bucket of the range
    /// is listed; one without ballots has `default_channel` and a count of 0.
    pub async fn ballots_per_bucket(
        &self,
        election_event_id: &str,
        election_id: Option<&str>,
        range: &BucketRange<'_>,
        default_channel: &str,
    ) -> Result<Vec<BucketCount>> {
        let rows = self
            .client()
            .await?
            .query(
                r#"
                WITH resolution AS (
                    SELECT CASE $6::text
                        WHEN 'minute' THEN interval '1 minute'
                        WHEN 'hour' THEN interval '1 hour'
                        ELSE interval '1 day'
                    END AS bucket_interval
                ),
                range_bounds AS (
                    SELECT
                        CASE
                            WHEN $7::integer IS NULL THEN date_trunc($6::text, $3::text::timestamp)
                            ELSE date_trunc($6::text, CURRENT_TIMESTAMP AT TIME ZONE $5::text)
                                - bucket_interval * ($7::integer - 1)
                        END AS start_bucket,
                        CASE
                            WHEN $7::integer IS NULL THEN date_trunc($6::text, $4::text::timestamp)
                            ELSE date_trunc($6::text, CURRENT_TIMESTAMP AT TIME ZONE $5::text)
                        END AS end_bucket,
                        bucket_interval
                    FROM resolution
                )
                SELECT
                    to_char(series.bucket, 'YYYY-MM-DD"T"HH24:MI:SS'),
                    to_char(series.bucket, 'YYYY-MM-DD'),
                    COALESCE(b.voting_channel, $8::text) AS channel,
                    count(b.seq)
                FROM range_bounds bounds
                CROSS JOIN LATERAL generate_series(
                    bounds.start_bucket, bounds.end_bucket, bounds.bucket_interval
                ) AS series(bucket)
                LEFT JOIN ballot_box_ballot b
                    ON b.election_event_id = $1::text::uuid
                    AND ($2::text IS NULL OR b.election_id = $2::text::uuid)
                    AND b.status = $9
                    AND date_trunc($6::text, b.accepted_at AT TIME ZONE $5::text) = series.bucket
                    AND (b.accepted_at AT TIME ZONE $5::text) >= bounds.start_bucket
                    AND (b.accepted_at AT TIME ZONE $5::text)
                        < bounds.end_bucket + bounds.bucket_interval
                GROUP BY series.bucket, COALESCE(b.voting_channel, $8::text)
                ORDER BY series.bucket, channel
                "#,
                &[
                    &election_event_id,
                    &election_id,
                    &range.start,
                    &range.end,
                    &range.time_zone,
                    &range.resolution,
                    &range.last_buckets,
                    &default_channel,
                    &VALID,
                ],
            )
            .await
            .context("Error counting ballots per time bucket")?;
        rows.iter()
            .map(|row| {
                Ok(BucketCount {
                    bucket: row.try_get(0)?,
                    day: row.try_get(1)?,
                    channel: row.try_get(2)?,
                    count: row.try_get(3)?,
                })
            })
            .collect()
    }

    /// Valid ballots grouped by IP address, country and election, most first.
    /// Ballots without an IP address or country are left out.
    pub async fn ballots_by_ip(
        &self,
        election_event_id: &str,
        filter: &IpBallotsFilter<'_>,
    ) -> Result<Vec<IpBallots>> {
        let rows = self
            .client()
            .await?
            .query(
                "SELECT voter_ip, voter_country, election_id::text, array_agg(voter_id ORDER BY seq), \
                        count(*) AS ballots \
                 FROM ballot_box_ballot \
                 WHERE election_event_id = $1::text::uuid AND status = $7 \
                   AND voter_ip IS NOT NULL AND voter_country IS NOT NULL \
                   AND ($2::text IS NULL OR voter_ip ILIKE $2) \
                   AND ($3::text IS NULL OR voter_country ILIKE $3) \
                   AND ($4::text IS NULL OR election_id = $4::text::uuid) \
                 GROUP BY voter_ip, voter_country, election_id \
                 ORDER BY ballots DESC, voter_ip, voter_country, election_id \
                 LIMIT $5 OFFSET $6",
                &[
                    &election_event_id,
                    &filter.ip_pattern,
                    &filter.country_pattern,
                    &filter.election_id,
                    &filter.limit,
                    &filter.offset,
                    &VALID,
                ],
            )
            .await
            .context("Error counting ballots by IP address")?;
        rows.iter()
            .map(|row| {
                Ok(IpBallots {
                    ip: row.try_get(0)?,
                    country: row.try_get(1)?,
                    election_id: row.try_get(2)?,
                    voter_ids: row.try_get(3)?,
                    count: row.try_get(4)?,
                })
            })
            .collect()
    }

    /// For each of these voters and each election they voted in, their ballots that
    /// were not rejected and when they cast the latest.
    pub async fn votes_of_voters(
        &self,
        election_event_id: &str,
        voter_ids: &[String],
        election_id: Option<&str>,
    ) -> Result<Vec<VoterVotes>> {
        let rows = self
            .client()
            .await?
            .query(
                "SELECT voter_id, election_id::text, count(*), \
                        (extract(epoch FROM max(accepted_at)) * 1000000)::bigint \
                 FROM ballot_box_ballot \
                 WHERE election_event_id = $1::text::uuid AND voter_id = ANY($2) \
                   AND ($3::text IS NULL OR election_id = $3::text::uuid) AND status <> $4 \
                 GROUP BY voter_id, election_id",
                &[&election_event_id, &voter_ids, &election_id, &REJECTED],
            )
            .await
            .context("Error counting the voters' ballots")?;
        rows.iter()
            .map(|row| {
                Ok(VoterVotes {
                    voter_id: row.try_get(0)?,
                    election_id: row.try_get(1)?,
                    votes: row.try_get(2)?,
                    last_voted_at: row.try_get(3)?,
                })
            })
            .collect()
    }

    /// Valid ballots of an event, or of one of its elections, and their voters.
    pub async fn participation(
        &self,
        election_event_id: &str,
        election_id: Option<&str>,
    ) -> Result<Participation> {
        let row = self
            .client()
            .await?
            .query_one(
                "SELECT count(*), count(DISTINCT voter_id) FROM ballot_box_ballot \
                 WHERE election_event_id = $1::text::uuid AND status = $3 \
                   AND ($2::text IS NULL OR election_id = $2::text::uuid)",
                &[&election_event_id, &election_id, &VALID],
            )
            .await
            .context("Error counting participation")?;
        Ok(Participation {
            ballots: row.try_get(0)?,
            voters: row.try_get(1)?,
        })
    }
}

/// Checks before an election's area is tallied.
impl PostgresStore {
    /// Accepted ballots of an election's area that the sequencer has not appended
    /// to the board yet.
    pub async fn unsequenced_count(
        &self,
        election_event_id: &str,
        election_id: &str,
        area_id: &str,
    ) -> Result<i64> {
        let row = self
            .client()
            .await?
            .query_one(
                "SELECT count(*) FROM ballot_box_pending p \
                 JOIN ballot_box_ballot b \
                   ON b.election_event_id = p.election_event_id AND b.seq = p.seq \
                 WHERE p.election_event_id = $1::text::uuid \
                   AND b.election_id = $2::text::uuid AND b.area_id = $3::text::uuid",
                &[&election_event_id, &election_id, &area_id],
            )
            .await
            .context("Error counting the ballots waiting for the sequencer")?;
        Ok(row.try_get(0)?)
    }

    /// Ballots of an election's area whose outcome is still pending.
    pub async fn pending_status_count(
        &self,
        election_event_id: &str,
        election_id: &str,
        area_id: &str,
    ) -> Result<i64> {
        let row = self
            .client()
            .await?
            .query_one(
                "SELECT count(*) FROM ballot_box_ballot \
                 WHERE election_event_id = $1::text::uuid AND election_id = $2::text::uuid \
                   AND area_id = $3::text::uuid AND status = $4",
                &[&election_event_id, &election_id, &area_id, &PENDING],
            )
            .await
            .context("Error counting the ballots with a pending outcome")?;
        Ok(row.try_get(0)?)
    }

    /// Valid ballots of an election's area that the sequencer appended, by its queue,
    /// but whose cast-vote record is not on the board: deleted or never written.
    pub async fn unrecorded_count(
        &self,
        board: &str,
        election_event_id: &str,
        election_id: &str,
        area_id: &str,
    ) -> Result<i64> {
        let row = self
            .client()
            .await?
            .query_one(
                "SELECT count(*) FROM ballot_box_ballot b \
                 WHERE b.election_event_id = $2::text::uuid AND b.election_id = $3::text::uuid \
                   AND b.area_id = $4::text::uuid AND b.status = $5 \
                   AND NOT EXISTS (SELECT 1 FROM ballot_box_pending p \
                                   WHERE p.election_event_id = b.election_event_id AND p.seq = b.seq) \
                   AND NOT EXISTS (SELECT 1 FROM electoral_log_messages m \
                                   WHERE m.board_name = $1 \
                                     AND m.delivery_id = 'ballot-box:' || b.election_event_id::text \
                                                         || ':' || b.seq::text)",
                &[&board, &election_event_id, &election_id, &area_id, &VALID],
            )
            .await
            .context("Error checking the ballots' cast-vote records")?;
        Ok(row.try_get(0)?)
    }
}
