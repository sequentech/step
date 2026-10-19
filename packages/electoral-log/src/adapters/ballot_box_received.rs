// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballots the ballot box received at review, under the signed-by-ballot-box receipts
//! policy. The ballot box signs a receipt with a short Ballot ID for each; the voter
//! casts one by that Ballot ID, with `accept_received_ballot`.

use super::ballot_box_reads::{BallotContent, BallotIdMatch, StoredBallot};
use super::postgres::PostgresStore;
use super::privacy::BallotIdentity;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, Utc};
use strum_macros::{Display, EnumString};
use tokio_postgres::Row;

/// Receipt times a received ballot is signed for, one millisecond apart, before
/// giving up on finding a free Ballot ID. A taken Ballot ID is about one in a million
/// at 1.7 million ballots.
pub const RECEIVE_ATTEMPTS: i64 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum ReceivedStatus {
    /// Waiting to be cast.
    Received,
    Cast,
    /// Audited by the voter: it is never cast.
    Audited,
}

/// A ballot the ballot box received.
#[derive(Debug, Clone)]
pub struct ReceiveBallot<'a> {
    pub election_event_id: &'a str,
    pub election_id: &'a str,
    pub area_id: &'a str,
    pub voter_id: &'a str,
    /// The ballot's hash as the voter sent it.
    pub ballot_hash: &'a str,
    pub content: &'a str,
    pub voter_signing_pk: &'a str,
    pub voter_ballot_signature: &'a str,
}

/// The receipt the ballot box signed for a received ballot at one receipt time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedSignature {
    /// The short Ballot ID.
    pub ballot_id: String,
    pub key_id: String,
    pub received_signature: String,
}

/// A stored received ballot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedBallot {
    pub id: String,
    pub election_event_id: String,
    pub election_id: String,
    pub area_id: String,
    pub voter_id: String,
    /// The short Ballot ID.
    pub ballot_id: String,
    pub ballot_hash: String,
    pub content: String,
    pub voter_signing_pk: String,
    pub voter_ballot_signature: String,
    pub received_at: DateTime<Utc>,
    pub key_id: String,
    pub received_signature: String,
    pub status: ReceivedStatus,
    pub cast_at: Option<DateTime<Utc>>,
    pub cast_signature: Option<String>,
    pub cast_receipt_signature: Option<String>,
}

/// What receiving a ballot did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveOutcome {
    /// Stored now.
    Received(ReceivedBallot),
    /// The voter had sent this ballot before: the one stored then.
    AlreadyReceived(ReceivedBallot),
    /// Every attempt's Ballot ID was taken in the election event.
    NoFreeBallotId,
}

/// A cast received ballot, with the short Ballot ID of its receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedCastBallot {
    pub received_ballot_id: String,
    pub ballot: BallotContent,
}

pub(crate) const RECEIVED_COLUMNS: &str = "id::text, election_event_id::text, election_id::text, \
     area_id::text, voter_id, ballot_id, ballot_hash, content, voter_signing_pk, \
     voter_ballot_signature, received_at, key_id, received_signature, status, cast_at, \
     cast_signature, cast_receipt_signature";

const BY_HASH: &str = "election_event_id = $1::text::uuid AND election_id = $2::text::uuid \
     AND voter_id = $3 AND ballot_hash = $4";

pub(crate) fn received_ballot(row: &Row) -> Result<ReceivedBallot> {
    let status: String = row.try_get(13)?;
    Ok(ReceivedBallot {
        id: row.try_get(0)?,
        election_event_id: row.try_get(1)?,
        election_id: row.try_get(2)?,
        area_id: row.try_get(3)?,
        voter_id: row.try_get(4)?,
        ballot_id: row.try_get(5)?,
        ballot_hash: row.try_get(6)?,
        content: row.try_get(7)?,
        voter_signing_pk: row.try_get(8)?,
        voter_ballot_signature: row.try_get(9)?,
        received_at: row.try_get(10)?,
        key_id: row.try_get(11)?,
        received_signature: row.try_get(12)?,
        status: status
            .parse()
            .map_err(|_| anyhow!("Unknown received ballot status {status:?}"))?,
        cast_at: row.try_get(14)?,
        cast_signature: row.try_get(15)?,
        cast_receipt_signature: row.try_get(16)?,
    })
}

impl PostgresStore {
    /// Store a received ballot with the receipt `sign` signs for its receipt time,
    /// `first_received_at` first. While the receipt's Ballot ID is taken in the event,
    /// each further attempt signs one millisecond later, up to `RECEIVE_ATTEMPTS`. A
    /// ballot the voter already sent is not stored again.
    pub async fn receive_ballot<F>(
        &self,
        ballot: &ReceiveBallot<'_>,
        first_received_at: DateTime<Utc>,
        mut sign: F,
    ) -> Result<ReceiveOutcome>
    where
        F: FnMut(DateTime<Utc>) -> Result<ReceivedSignature> + Send,
    {
        let privacy =
            self.ballot_identity(ballot.election_event_id).await? == BallotIdentity::Pseudonym;
        let mut client = self.client().await?;
        let by_hash = client
            .prepare_cached(&format!(
                "SELECT {RECEIVED_COLUMNS} FROM ballot_box_received WHERE {BY_HASH}"
            ))
            .await
            .context("Error preparing the statement that reads received ballots")?;
        let hash_parameters: [&(dyn tokio_postgres::types::ToSql + Sync); 4] = [
            &ballot.election_event_id,
            &ballot.election_id,
            &ballot.voter_id,
            &ballot.ballot_hash,
        ];
        // On the same connection: taking a second one while holding this one could
        // exhaust the pool under load.
        if let Some(row) = client.query_opt(&by_hash, &hash_parameters).await? {
            return Ok(ReceiveOutcome::AlreadyReceived(received_ballot(&row)?));
        }
        let insert = client
            .prepare_cached(&format!(
                "INSERT INTO ballot_box_received (election_event_id, election_id, area_id, \
                 voter_id, ballot_id, ballot_hash, content, voter_signing_pk, \
                 voter_ballot_signature, received_at, key_id, received_signature) \
                 VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4, $5, $6, $7, $8, \
                 $9, $10, $11, $12) \
                 ON CONFLICT DO NOTHING RETURNING {RECEIVED_COLUMNS}"
            ))
            .await
            .context("Error preparing the statement that stores received ballots")?;
        for attempt in 0..RECEIVE_ATTEMPTS {
            let received_at = first_received_at + Duration::milliseconds(attempt);
            let signature = sign(received_at)?;
            let transaction = client.transaction().await?;
            if privacy {
                let reserved = transaction
                    .query_opt(
                        "INSERT INTO sequent_backend.ballot_code (election_event_id, code, kind)
                     VALUES ($1::text::uuid, $2, 'ballot') ON CONFLICT DO NOTHING RETURNING code",
                        &[&ballot.election_event_id, &signature.ballot_id],
                    )
                    .await?;
                if reserved.is_none() {
                    transaction.rollback().await?;
                    // A simultaneous retry may have reserved this same receipt.
                    if let Some(row) = client.query_opt(&by_hash, &hash_parameters).await? {
                        return Ok(ReceiveOutcome::AlreadyReceived(received_ballot(&row)?));
                    }
                    continue;
                }
            }
            let inserted = transaction
                .query_opt(
                    &insert,
                    &[
                        &ballot.election_event_id,
                        &ballot.election_id,
                        &ballot.area_id,
                        &ballot.voter_id,
                        &signature.ballot_id,
                        &ballot.ballot_hash,
                        &ballot.content,
                        &ballot.voter_signing_pk,
                        &ballot.voter_ballot_signature,
                        &received_at,
                        &signature.key_id,
                        &signature.received_signature,
                    ],
                )
                .await
                .context("Error storing the received ballot")?;
            if let Some(row) = inserted {
                let stored = received_ballot(&row)?;
                transaction.commit().await?;
                return Ok(ReceiveOutcome::Received(stored));
            }
            // Roll back an unused reservation as well as the unsuccessful insert.
            transaction.rollback().await?;
            // Either the voter's same ballot arrived on another request, or the
            // Ballot ID is taken in the election event.
            if let Some(row) = client.query_opt(&by_hash, &hash_parameters).await? {
                return Ok(ReceiveOutcome::AlreadyReceived(received_ballot(&row)?));
            }
        }
        Ok(ReceiveOutcome::NoFreeBallotId)
    }

    /// The voter's received ballot with this ballot hash in an election.
    pub async fn received_ballot(
        &self,
        election_event_id: &str,
        election_id: &str,
        voter_id: &str,
        ballot_hash: &str,
    ) -> Result<Option<ReceivedBallot>> {
        self.client()
            .await?
            .query_opt(
                &format!("SELECT {RECEIVED_COLUMNS} FROM ballot_box_received WHERE {BY_HASH}"),
                &[&election_event_id, &election_id, &voter_id, &ballot_hash],
            )
            .await
            .context("Error reading the received ballot")?
            .as_ref()
            .map(received_ballot)
            .transpose()
    }

    /// The voter's received ballot with this short Ballot ID in an election, whoever
    /// else's ballot has it.
    pub async fn received_ballot_by_id(
        &self,
        election_event_id: &str,
        election_id: &str,
        voter_id: &str,
        ballot_id: &str,
    ) -> Result<Option<ReceivedBallot>> {
        self.client()
            .await?
            .query_opt(
                &format!(
                    "SELECT {RECEIVED_COLUMNS} FROM ballot_box_received \
                     WHERE election_event_id = $1::text::uuid AND ballot_id = $4 \
                       AND election_id = $2::text::uuid AND voter_id = $3"
                ),
                &[&election_event_id, &election_id, &voter_id, &ballot_id],
            )
            .await
            .context("Error reading the received ballot")?
            .as_ref()
            .map(received_ballot)
            .transpose()
    }

    /// The voter's cast received ballots in an election whose short Ballot ID
    /// matches, with their stored ballots, in acceptance order.
    pub async fn received_cast_ballots(
        &self,
        election_event_id: &str,
        voter_id: &str,
        election_id: &str,
        ballot_id: BallotIdMatch<'_>,
    ) -> Result<Vec<ReceivedCastBallot>> {
        let (condition, value) = match ballot_id {
            BallotIdMatch::Exact(value) => ("r.ballot_id = $4", value),
            BallotIdMatch::Prefix(value) => ("starts_with(r.ballot_id, $4)", value),
        };
        let rows = self
            .client()
            .await?
            .query(
                &format!(
                    "SELECT r.ballot_id, b.id::text, b.ballot_id, b.election_id::text, \
                            b.area_id::text, b.voter_id, b.status, b.voting_channel, \
                            (extract(epoch FROM b.accepted_at) * 1000000)::bigint, b.content \
                     FROM ballot_box_received r \
                     JOIN ballot_box_ballot b \
                       ON b.election_event_id = r.election_event_id AND b.ballot_id = r.ballot_hash \
                     WHERE r.election_event_id = $1::text::uuid AND r.voter_id = $2 \
                       AND r.election_id = $3::text::uuid AND r.status = $5 AND {condition} \
                     ORDER BY b.seq"
                ),
                &[
                    &election_event_id,
                    &voter_id,
                    &election_id,
                    &value,
                    &ReceivedStatus::Cast.to_string(),
                ],
            )
            .await
            .context("Error looking up the voter's cast received ballots")?;
        rows.iter()
            .map(|row| {
                let status: String = row.try_get(6)?;
                Ok(ReceivedCastBallot {
                    received_ballot_id: row.try_get(0)?,
                    ballot: BallotContent {
                        ballot: StoredBallot {
                            id: row.try_get(1)?,
                            ballot_id: row.try_get(2)?,
                            election_id: row.try_get(3)?,
                            area_id: row.try_get(4)?,
                            voter_id: row.try_get(5)?,
                            status: status
                                .parse()
                                .map_err(|_| anyhow!("Unknown ballot status {status:?}"))?,
                            voting_channel: row.try_get(7)?,
                            accepted_at: row.try_get(8)?,
                        },
                        content: row.try_get(9)?,
                    },
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_have_the_names_the_schema_checks() {
        let schema = include_str!("../../schema.sql");
        for (status, name) in [
            (ReceivedStatus::Received, "received"),
            (ReceivedStatus::Cast, "cast"),
            (ReceivedStatus::Audited, "audited"),
        ] {
            assert_eq!(status.to_string(), name);
            assert_eq!(name.parse::<ReceivedStatus>().unwrap(), status);
        }
        assert!(schema.contains("CHECK (status IN ('received', 'cast', 'audited'))"));
        assert!(schema.contains("AND (cast_signature IS NULL OR status = 'cast')"));
    }
}
