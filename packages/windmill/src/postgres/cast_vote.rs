// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(Serialize)]
struct CastVoteAnnotations<'a> {
    ip: Option<&'a str>,
    country: Option<&'a str>,
    voting_channel: VotingStatusChannel,
}

#[derive(Clone, Copy)]
pub(crate) enum CastVoteRelation {
    Production,
    #[cfg(test)]
    StatisticsTest,
}

impl CastVoteRelation {
    /// PostgreSQL identifiers cannot be query parameters. Keeping the relation
    /// selector closed prevents caller-controlled text from reaching the SQL.
    fn sql_identifier(self) -> &'static str {
        match self {
            Self::Production => "sequent_backend.cast_vote",
            #[cfg(test)]
            Self::StatisticsTest => "pg_temp.cast_vote_stats_test",
        }
    }
}

pub(crate) fn count_distinct_voters_by_channel_query(
    cast_vote_relation: CastVoteRelation,
    filter_by_election: bool,
) -> String {
    let election_filter = if filter_by_election {
        "AND election_id = $5"
    } else {
        ""
    };
    let cast_vote_relation = cast_vote_relation.sql_identifier();

    format!(
        r#"
            WITH latest_valid_votes AS (
                SELECT DISTINCT ON (voter_id_string)
                    voter_id_string,
                    COALESCE(annotations->>'voting_channel', $4) AS channel
                FROM {cast_vote_relation}
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    status = $3 AND
                    voter_id_string IS NOT NULL
                    {election_filter}
                ORDER BY
                    voter_id_string,
                    created_at DESC NULLS LAST,
                    id DESC
            )
            SELECT
                channel,
                COUNT(*) AS count
            FROM latest_valid_votes
            GROUP BY channel
            ORDER BY channel;
            "#
    )
}

pub(crate) fn count_votes_per_day_query(cast_vote_relation: CastVoteRelation) -> String {
    let cast_vote_relation = cast_vote_relation.sql_identifier();

    format!(
        r#"
            WITH resolution AS (
                SELECT
                    CASE $9
                        WHEN 'minute' THEN interval '1 minute'
                        WHEN 'hour' THEN interval '1 hour'
                        ELSE interval '1 day'
                    END AS bucket_interval
            ),
            range_bounds AS (
                SELECT
                    CASE
                        WHEN $10::integer IS NULL THEN date_trunc($9, $3::timestamp)
                        ELSE date_trunc($9, CURRENT_TIMESTAMP AT TIME ZONE $5)
                            - bucket_interval * ($10::integer - 1)
                    END AS start_bucket,
                    CASE
                        WHEN $10::integer IS NULL THEN date_trunc($9, $4::timestamp)
                        ELSE date_trunc($9, CURRENT_TIMESTAMP AT TIME ZONE $5)
                    END AS end_bucket,
                    bucket_interval
                FROM resolution
            )
            SELECT
                series.bucket,
                series.bucket::date AS day,
                COALESCE(v.annotations->>'voting_channel', $8) AS channel,
                COUNT(v.id) AS day_count
            FROM range_bounds bounds
            CROSS JOIN LATERAL generate_series(
                bounds.start_bucket,
                bounds.end_bucket,
                bounds.bucket_interval
            ) AS series(bucket)
            LEFT JOIN {cast_vote_relation} v
                ON date_trunc($9, v.created_at AT TIME ZONE $5) = series.bucket
                AND v.tenant_id = $1
                AND v.election_event_id = $2
                AND (v.election_id = $6 OR $6 IS NULL)
                AND v.status = $7
                AND (v.created_at AT TIME ZONE $5) >= bounds.start_bucket
                AND (v.created_at AT TIME ZONE $5)
                    < bounds.end_bucket + bounds.bucket_interval
            GROUP BY
                series.bucket,
                COALESCE(v.annotations->>'voting_channel', $8)
            ORDER BY
                series.bucket,
                channel;
            "#
    )
}

fn cast_vote_annotations(
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    voting_channel: VotingStatusChannel,
) -> Result<Value> {
    Ok(serde_json::to_value(CastVoteAnnotations {
        ip: voter_ip.as_deref(),
        country: voter_country.as_deref(),
        voting_channel,
    })?)
}

#[instrument(skip(hasura_transaction, content, cast_ballot_signature), err)]
pub async fn insert_cast_vote(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
    content: &str,
    voter_id_string: &str,
    ballot_id: &str,
    cast_ballot_signature: &[u8],
    voter_ip: &Option<String>,
    voter_country: &Option<String>,
    voting_channel: VotingStatusChannel,
    initial_status: CastVoteStatus,
) -> Result<CastVote> {
    let status = initial_status.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                INSERT INTO
                    sequent_backend.cast_vote
                (tenant_id, election_event_id, election_id, area_id, voter_id_string, ballot_id, content, cast_ballot_signature, annotations, status)
                VALUES(
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    $7,
                    $8,
                    COALESCE($9::jsonb, '{}'),
                    $10
                )
                RETURNING
                    id,
                    ballot_id,
                    election_id,
                    election_event_id,
                    tenant_id,
                    election_id,
                    area_id,
                    created_at,
                    last_updated_at,
                    labels,
                    annotations,
                    content,
                    cast_ballot_signature,
                    voter_id_string,
                    election_event_id,
                    status;
            "#,
        )
        .await?;

    let annotations = cast_vote_annotations(voter_ip, voter_country, voting_channel)?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &election_id,
                &area_id,
                &voter_id_string,
                &ballot_id,
                &content,
                &cast_ballot_signature,
                &annotations,
                &status,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error inserting cast vote: {}", err))?;

    let cast_votes: Vec<CastVote> = rows
        .into_iter()
        .map(|row| -> Result<CastVote> { row.try_into() })
        .collect::<Result<Vec<CastVote>>>()?;

    if 1 == cast_votes.len() {
        Ok(cast_votes[0].clone())
    } else {
        Err(anyhow!("Unexpected rows affected {}", cast_votes.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cast_vote_annotations_include_voting_channel() {
        let annotations = cast_vote_annotations(
            &Some("203.0.113.1".to_string()),
            &Some("CO".to_string()),
            VotingStatusChannel::TELEPHONE,
        )
        .unwrap();

        assert_eq!(annotations["ip"], "203.0.113.1");
        assert_eq!(annotations["country"], "CO");
        assert_eq!(annotations["voting_channel"], "TELEPHONE");
    }
}

/// Atomically moves a cast vote from `expected_status` to `new_status`, scoped
/// to its tenant and event. Returns `false` without any change when the row is
/// no longer in `expected_status`, so concurrent workers cannot apply the same
/// transition twice.
#[instrument(skip(hasura_transaction), err)]
pub async fn compare_and_set_cast_vote_status(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    cast_vote_id: &Uuid,
    expected_status: CastVoteStatus,
    new_status: CastVoteStatus,
) -> Result<bool> {
    let expected_status = expected_status.to_string();
    let new_status = new_status.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE sequent_backend.cast_vote
                SET
                    status = $1,
                    last_updated_at = NOW()
                WHERE
                    id = $2 AND
                    status = $3 AND
                    tenant_id = $4 AND
                    election_event_id = $5
            "#,
        )
        .await?;

    let updated = hasura_transaction
        .execute(
            &statement,
            &[
                &new_status,
                &cast_vote_id,
                &expected_status,
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error updating cast vote: {}", err))?;

    Ok(updated == 1)
}

/// Loads a single cast vote by id within the given tenant and event, or `None`
/// when no such row exists.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_cast_vote_by_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    cast_vote_id: &Uuid,
) -> Result<Option<CastVote>> {
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id,
                    ballot_id,
                    election_id,
                    election_event_id,
                    tenant_id,
                    area_id,
                    created_at,
                    last_updated_at,
                    content,
                    cast_ballot_signature,
                    voter_id_string,
                    status
                FROM sequent_backend.cast_vote
                WHERE id = $1 AND tenant_id = $2 AND election_event_id = $3
            "#,
        )
        .await?;

    hasura_transaction
        .query_opt(
            &statement,
            &[
                cast_vote_id,
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await?
        .map(TryInto::try_into)
        .transpose()
}

/// Used by the datafix flow to tell a VoterView
/// `HasVoted` response caused by our own earlier `SetVoted` (a legitimate
/// re-vote) apart from a genuine "already voted through another channel". <br/>
/// Returns whether the voter already has at least one `valid` cast vote in
/// the cast_vote table for this election event.
#[instrument(skip(hasura_transaction), err)]
pub async fn has_valid_cast_vote(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    voter_id_string: &str,
) -> Result<bool> {
    let status = CastVoteStatus::Valid.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM sequent_backend.cast_vote
                    WHERE
                        tenant_id = $1 AND
                        election_event_id = $2 AND
                        voter_id_string = $3 AND
                        status = $4
                ) AS found
            "#,
        )
        .await?;

    let row = hasura_transaction
        .query_one(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &voter_id_string,
                &status,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error checking for valid cast votes: {}", err))?;

    Ok(row.get("found"))
}

/// Counts votes in a contest area whose Datafix outcome is not resolved.
/// Tally extraction must wait because neither status is countable.
#[instrument(skip(hasura_transaction), err)]
pub async fn count_unresolved_cast_votes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
) -> Result<i64> {
    let unresolved_status = CastVoteStatus::InProgress.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT COUNT(*) AS count
                FROM sequent_backend.cast_vote
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    area_id = $4 AND
                    status = $5
            "#,
        )
        .await?;

    let row = hasura_transaction
        .query_one(
            &statement,
            &[
                tenant_id,
                election_event_id,
                election_id,
                area_id,
                &unresolved_status,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error counting cast votes by status: {}", err))?;

    Ok(row.get("count"))
}

/// Discards every `valid` or `in-progress` ballot of the voter for the event.
/// Used when an admin disables a Datafix voter: the ballots are discarded
/// unconditionally as part of the disable, regardless of whether the
/// `SetNotVoted` notification to VoterView succeeds — a divergence between the
/// platform and VoterView is caught by the separate manual reconciliation
/// process. Returns the number of rows discarded.
#[instrument(skip(hasura_transaction), err)]
pub async fn discard_voter_cast_votes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    voter_id_string: &str,
) -> Result<u64> {
    let discarded_status = CastVoteStatus::Discarded.to_string();
    let active_statuses = vec![
        CastVoteStatus::Valid.to_string(),
        CastVoteStatus::InProgress.to_string(),
    ];
    let statement = hasura_transaction
        .prepare(
            r#"
                UPDATE sequent_backend.cast_vote
                SET
                    status = $4,
                    last_updated_at = NOW()
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    voter_id_string = $3 AND
                    status = ANY($5)
            "#,
        )
        .await?;

    hasura_transaction
        .execute(
            &statement,
            &[
                tenant_id,
                election_event_id,
                &voter_id_string,
                &discarded_status,
                &active_statuses,
            ],
        )
        .await
        .map_err(Into::into)
}

/// Snapshot of a voter's cast-vote states within one event, gathered in a single
/// query to drive the disabled-voter release decisions.
#[derive(Debug, Clone, Copy)]
pub struct VoterCastVoteState {
    /// At least one ballot is `in-progress`.
    pub has_unresolved_vote: bool,
    /// At least one ballot is `valid`.
    pub has_valid_vote: bool,
}

/// Computes `VoterCastVoteState` for the voter in a single round-trip.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_voter_cast_vote_state(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    voter_id_string: &str,
) -> Result<VoterCastVoteState> {
    let unresolved_status = CastVoteStatus::InProgress.to_string();
    let valid_status = CastVoteStatus::Valid.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                WITH voter_votes AS MATERIALIZED (
                    SELECT status
                    FROM sequent_backend.cast_vote
                    WHERE
                        tenant_id = $1 AND
                        election_event_id = $2 AND
                        voter_id_string = $3
                )
                SELECT
                    EXISTS (
                        SELECT 1 FROM voter_votes
                        WHERE status = $4
                    ) AS has_unresolved_vote,
                    EXISTS (
                        SELECT 1 FROM voter_votes
                        WHERE status = $5
                    ) AS has_valid_vote
            "#,
        )
        .await?;

    let row = hasura_transaction
        .query_one(
            &statement,
            &[
                tenant_id,
                election_event_id,
                &voter_id_string,
                &unresolved_status,
                &valid_status,
            ],
        )
        .await?;
    Ok(VoterCastVoteState {
        has_unresolved_vote: row.get("has_unresolved_vote"),
        has_valid_vote: row.get("has_valid_vote"),
    })
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_cast_votes(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    voter_id_string: &str,
    statuses: &[CastVoteStatus],
) -> Result<Vec<CastVote>> {
    let statuses: Vec<String> = statuses.iter().map(|status| status.to_string()).collect();
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    id,
                    ballot_id,
                    election_id,
                    election_event_id,
                    tenant_id,
                    election_id,
                    area_id,
                    created_at,
                    last_updated_at,
                    labels,
                    annotations,
                    content,
                    cast_ballot_signature,
                    voter_id_string,
                    election_event_id,
                    status
                FROM
                    sequent_backend.cast_vote
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    voter_id_string = $4 AND
                    status = ANY($5)
                ;
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &tenant_id,
                &election_event_id,
                &election_id,
                &voter_id_string,
                &statuses,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error getting cast votes: {}", err))?;

    let cast_votes: Vec<CastVote> = rows
        .into_iter()
        .map(|row| -> Result<CastVote> { row.try_into() })
        .collect::<Result<Vec<CastVote>>>()?;

    Ok(cast_votes)
}

/// Precomputes unresolved and valid ballot state for every voter with an
/// active cast vote in an event. Reconciliation needs both states: an
/// `in-progress` ballot is expected during a Datafix freeze and must never be
/// mistaken for "not voted" merely because it is not valid yet.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_voter_cast_vote_states_for_event(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<HashMap<String, VoterCastVoteState>> {
    let unresolved_status = CastVoteStatus::InProgress.to_string();
    let valid_status = CastVoteStatus::Valid.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT
                    voter_id_string,
                    bool_or(status = $3) AS has_unresolved_vote,
                    bool_or(status = $4) AS has_valid_vote
                FROM sequent_backend.cast_vote
                WHERE tenant_id = $1
                    AND election_event_id = $2
                    AND status IN ($3, $4)
                    AND voter_id_string IS NOT NULL
                GROUP BY voter_id_string
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &unresolved_status,
                &valid_status,
            ],
        )
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            row.get::<_, Option<String>>("voter_id_string")
                .map(|voter_id| {
                    (
                        voter_id,
                        VoterCastVoteState {
                            has_unresolved_vote: row.get("has_unresolved_vote"),
                            has_valid_vote: row.get("has_valid_vote"),
                        },
                    )
                })
        })
        .collect())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_cast_votes_by_election_id(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: &str,
) -> Result<Vec<CastVote>> {
    let status = CastVoteStatus::Valid.to_string();
    let statement = hasura_transaction
        .prepare(
            r#"
                SELECT 
                    *
                FROM
                    sequent_backend.cast_vote
                WHERE
                    tenant_id = $1 AND
                    election_event_id = $2 AND
                    election_id = $3 AND
                    status = $4
            "#,
        )
        .await?;

    let rows: Vec<Row> = hasura_transaction
        .query(
            &statement,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                &parse_uuid_v4(election_id)?,
                &status,
            ],
        )
        .await
        .map_err(|err| anyhow!("Error getting cast votes: {}", err))?;

    let cast_votes: Vec<CastVote> = rows
        .into_iter()
        .map(|row| -> Result<CastVote> { row.try_into() })
        .collect::<Result<Vec<CastVote>>>()?;

    Ok(cast_votes)
}
