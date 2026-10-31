// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballots the ballot box received at review, under the signed-by-ballot-box receipts
//! policy. The ballot box signs a receipt with a short Ballot ID for each; the voter
//! casts one by that Ballot ID, with `accept_received_ballot`.

use super::ballot_box::{election_entry, queue_entry, QueueState, VitrineEntry, VitrineQueue};
use super::ballot_box_reads::{BallotContent, BallotIdMatch, StoredBallot};
use super::postgres::PostgresStore;
use super::privacy::BallotIdentity;
use anyhow::{anyhow, ensure, Context, Result};
use chrono::{DateTime, Duration, Utc};
use strum_macros::{Display, EnumString};
use tokio_postgres::Row;
use tracing::warn;

use crate::vitrine::{leaf_hash, Stage};

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

/// Network evidence retained only for the first successful receipt.
#[derive(Debug, Clone, Copy)]
pub struct ReceivedNetwork<'a> {
    pub ip: &'a str,
    pub country: Option<&'a str>,
    pub user_agent: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditOutcome {
    Audited,
    AlreadyCast,
    NotFound,
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
    /// The event's Vitrine `ballots` log, when the event has transparency on:
    /// the `received` entry is queued in the transaction that stores the ballot.
    pub vitrine: Option<VitrineQueue>,
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
    /// The `received` entry that went into the Vitrine log with the ballot, or,
    /// for a ballot sent again, the same entry rebuilt from the stored one. It
    /// is not a column: a ballot read from the store has none.
    pub vitrine: Option<VitrineEntry>,
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
        vitrine: None,
    })
}

/// The `received` entry of a stored ballot, as the first receipt queued it.
fn received_entry(ballot: &ReceiveBallot<'_>, stored: &ReceivedBallot) -> Option<VitrineEntry> {
    let queue = ballot.vitrine?;
    election_entry(
        ballot.election_event_id,
        ballot.election_id,
        Stage::Received,
        &queue,
        stored.received_at.timestamp(),
    )
    .map_err(|error| warn!("No Vitrine entry for received ballot {}: {error:#}", stored.ballot_id))
    .ok()
    .map(|entry| VitrineEntry {
        leaf_hash: leaf_hash(&entry),
        entry,
        queue: QueueState::Queued,
    })
}

/// A ballot the voter sent before, with its `received` entry.
fn already_received(ballot: &ReceiveBallot<'_>, row: &Row) -> Result<ReceiveOutcome> {
    let mut stored = received_ballot(row)?;
    stored.vitrine = received_entry(ballot, &stored);
    Ok(ReceiveOutcome::AlreadyReceived(stored))
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
        sign: F,
    ) -> Result<ReceiveOutcome>
    where
        F: FnMut(DateTime<Utc>) -> Result<ReceivedSignature> + Send,
    {
        self.receive_ballot_with_network(ballot, first_received_at, None, sign)
            .await
    }

    /// Receipt, shared code reservation, evidence outbox and permitted network
    /// attribution commit together. A replay never changes the original network.
    pub async fn receive_ballot_with_network<F>(
        &self,
        ballot: &ReceiveBallot<'_>,
        first_received_at: DateTime<Utc>,
        network: Option<&ReceivedNetwork<'_>>,
        mut sign: F,
    ) -> Result<ReceiveOutcome>
    where
        F: FnMut(DateTime<Utc>) -> Result<ReceivedSignature> + Send,
    {
        let privacy =
            self.ballot_identity(ballot.election_event_id).await? == BallotIdentity::Pseudonym;
        ensure!(
            network.is_none() || privacy,
            "Forensic attribution requires pseudonyms"
        );
        let ip = network.map(|value| value.ip);
        let country = network.and_then(|value| value.country);
        let user_agent = network.and_then(|value| value.user_agent);
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
            return already_received(ballot, &row);
        }
        let insert = client
            .prepare_cached(&format!(
                "WITH received AS (
                 INSERT INTO ballot_box_received (election_event_id, election_id, area_id,
                 voter_id, ballot_id, ballot_hash, content, voter_signing_pk,
                 voter_ballot_signature, received_at, key_id, received_signature)
                 VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4, $5, $6, $7, $8,
                 $9, $10, $11, $12)
                 ON CONFLICT DO NOTHING RETURNING *
                 ), forensic AS (
                    INSERT INTO sequent_backend.forensic_network_record
                        (election_event_id,election_id,area_id,event_kind,delivery_id,pseudonym,
                         ballot_id,ip,country,user_agent,recorded_at)
                    SELECT election_event_id,election_id,area_id,'receive',
                        'receive:' || election_event_id::text || ':' || ballot_id,voter_id,
                        ballot_id,$13,$14,$15,received_at FROM received WHERE $13::text IS NOT NULL
                    ON CONFLICT (election_event_id,delivery_id) DO NOTHING RETURNING id
                 ), forensic_queued AS (
                    INSERT INTO sequent_backend.forensic_summary_pending(record_id) SELECT id FROM forensic
                 ) SELECT {RECEIVED_COLUMNS} FROM received"
            ))
            .await
            .context("Error preparing the statement that stores received ballots")?;
        for attempt in 0..RECEIVE_ATTEMPTS {
            let received_at = first_received_at + Duration::milliseconds(attempt);
            let signature = sign(received_at)?;
            let mut transaction = client.transaction().await?;
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
                        return already_received(ballot, &row);
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
                        &ip,
                        &country,
                        &user_agent,
                    ],
                )
                .await
                .context("Error storing the received ballot")?;
            if let Some(row) = inserted {
                let mut stored = received_ballot(&row)?;
                // The entry and the ballot commit together or not at all, but the
                // queue never refuses the ballot: its insert is under a savepoint.
                if let Some(queue) = ballot.vitrine {
                    stored.vitrine = match election_entry(
                        ballot.election_event_id,
                        ballot.election_id,
                        Stage::Received,
                        &queue,
                        received_at.timestamp(),
                    ) {
                        Ok(entry) => Some(
                            queue_entry(
                                &mut transaction,
                                &queue,
                                entry,
                                &format!("received ballot {}", signature.ballot_id),
                            )
                            .await,
                        ),
                        Err(error) => {
                            warn!(
                                "No Vitrine entry for received ballot {}: {error:#}",
                                signature.ballot_id
                            );
                            None
                        }
                    };
                }
                transaction.commit().await?;
                return Ok(ReceiveOutcome::Received(stored));
            }
            // Roll back an unused reservation as well as the unsuccessful insert.
            transaction.rollback().await?;
            // Either the voter's same ballot arrived on another request, or the
            // Ballot ID is taken in the election event.
            if let Some(row) = client.query_opt(&by_hash, &hash_parameters).await? {
                return already_received(ballot, &row);
            }
        }
        Ok(ReceiveOutcome::NoFreeBallotId)
    }

    /// Atomically closes the audit/cast choice for the authenticated owner.
    /// The same row lock used by cast makes the two outcomes mutually exclusive.
    /// Updating to audited also queues privacy evidence through the schema trigger.
    pub async fn audit_received_ballot(
        &self,
        event: &str,
        election: &str,
        area: &str,
        voter: &str,
        ballot_id: &str,
    ) -> Result<AuditOutcome> {
        Ok(self
            .audit_received_ballot_with_vitrine(event, election, area, voter, ballot_id, None)
            .await?
            .0)
    }

    /// As `audit_received_ballot`, and queues the ballot's `audited` entry, in
    /// the transaction that closes the choice, when the event has a Vitrine
    /// `ballots` log. Auditing a ballot again returns the entry rebuilt from the
    /// stored audit time.
    pub async fn audit_received_ballot_with_vitrine(
        &self,
        event: &str,
        election: &str,
        area: &str,
        voter: &str,
        ballot_id: &str,
        vitrine: Option<VitrineQueue>,
    ) -> Result<(AuditOutcome, Option<VitrineEntry>)> {
        let audited_at = Utc::now();
        let mut client = self.client().await?;
        let mut transaction = client.transaction().await?;
        let row = transaction
            .query_opt(
                "WITH selected AS (
                SELECT id,status,audited_at FROM ballot_box_received
                WHERE election_event_id=$1::text::uuid AND election_id=$2::text::uuid
                    AND area_id=$3::text::uuid AND voter_id=$4 AND ballot_id=$5 FOR UPDATE
             ), audited AS (
                UPDATE ballot_box_received r SET status='audited', audited_at=$6
                FROM selected s WHERE r.election_event_id=$1::text::uuid AND r.id=s.id
                    AND s.status='received' RETURNING r.id
             ) SELECT s.status, s.audited_at, EXISTS (SELECT 1 FROM audited) FROM selected s",
                &[&event, &election, &area, &voter, &ballot_id, &audited_at],
            )
            .await?;
        let Some(row) = row else {
            transaction.rollback().await?;
            return Ok((AuditOutcome::NotFound, None));
        };
        let status: String = row.get(0);
        let stored_audit: Option<DateTime<Utc>> = row.get(1);
        let just_audited: bool = row.get(2);
        let outcome = match status.as_str() {
            "received" | "audited" => AuditOutcome::Audited,
            "cast" => AuditOutcome::AlreadyCast,
            _ => AuditOutcome::NotFound,
        };
        let entry = match (outcome, vitrine) {
            (AuditOutcome::Audited, Some(queue)) => {
                let at = if just_audited { Some(audited_at) } else { stored_audit };
                match at.map(|at| election_entry(event, election, Stage::Audited, &queue, at.timestamp())) {
                    Some(Ok(entry)) if just_audited => Some(
                        queue_entry(
                            &mut transaction,
                            &queue,
                            entry,
                            &format!("audited ballot {ballot_id}"),
                        )
                        .await,
                    ),
                    Some(Ok(entry)) => Some(VitrineEntry {
                        leaf_hash: leaf_hash(&entry),
                        entry,
                        queue: QueueState::Queued,
                    }),
                    Some(Err(error)) => {
                        warn!("No Vitrine entry for audited ballot {ballot_id}: {error:#}");
                        None
                    }
                    None => None,
                }
            }
            _ => None,
        };
        transaction.commit().await?;
        Ok((outcome, entry))
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
    /// Resolve authenticated ownership in the receipt read itself. The private
    /// identity is a query parameter only; returned ballot rows keep the pseudonym.
    pub async fn received_ballot_for_owner(
        &self,
        event: &str,
        election: &str,
        area: &str,
        authenticated_voter: &str,
        ballot_id: &str,
        mode: BallotIdentity,
    ) -> Result<Option<ReceivedBallot>> {
        let owner = match mode {
            BallotIdentity::VoterId => "$3",
            BallotIdentity::Pseudonym => {
                "(SELECT p.pseudonym FROM sequent_backend.voter_pseudonym p
                JOIN sequent_backend.ballot_identity i USING(election_event_id)
                WHERE p.election_event_id=$1::text::uuid AND p.voter_id=$3
                    AND i.mode='pseudonym' AND i.mapping_purged_at IS NULL)"
            }
        };
        self.client()
            .await?
            .query_opt(
                &format!(
                    "SELECT {RECEIVED_COLUMNS} FROM ballot_box_received
                WHERE election_event_id=$1::text::uuid AND election_id=$2::text::uuid
                    AND voter_id={owner} AND ballot_id=$4 AND area_id=$5::text::uuid"
                ),
                &[&event, &election, &authenticated_voter, &ballot_id, &area],
            )
            .await
            .context("Error reading the authenticated owner's receipt")?
            .as_ref()
            .map(received_ballot)
            .transpose()
    }

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
            BallotIdMatch::Any => ("length($4::text) >= 0", ""),
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
