// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Private event identities. These tables are deliberately outside the schema
//! available to the electoral-log console. Callers authorize tenant/event access
//! before selecting an event's store; public knowledge of a code is not authority.

use super::ballot_box_received::{received_ballot, ReceivedBallot, RECEIVED_COLUMNS};
use super::postgres::PostgresStore;
use anyhow::{bail, ensure, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const ALLOCATION_BATCH: usize = 1_000;
const ALLOCATION_ATTEMPTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BallotIdentity {
    VoterId,
    Pseudonym,
}

impl BallotIdentity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VoterId => "voter-id",
            Self::Pseudonym => "pseudonym",
        }
    }
}

/// The stored form is `7QKM-2HXD`, the displayed form `7QKM:2HXD`; a typed code may have
/// either. Ambiguous alphabet characters are
/// refused rather than silently converted into a different code.
pub fn normalize_privacy_code(input: &str) -> Option<String> {
    let input = input.trim().to_ascii_uppercase();
    let compact = match input.len() {
        8 => input,
        9 if matches!(input.as_bytes().get(4), Some(b'-' | b':')) => input.replace(['-', ':'], ""),
        _ => return None,
    };
    if compact.len() != 8 || !compact.bytes().all(|byte| ALPHABET.contains(&byte)) {
        return None;
    }
    Some(format!("{}-{}", &compact[..4], &compact[4..]))
}

fn random_pseudonym() -> Result<String> {
    let mut randomness = [0u8; 8];
    openssl::rand::rand_bytes(&mut randomness).context("Generating a voter pseudonym")?;
    // Each of eight independent uniform five-bit values contributes 5 bits.
    let code: String = randomness
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(byte & 31)]))
        .collect();
    Ok(format!("{}-{}", &code[..4], &code[4..]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoterParticipation {
    pub voter_id: String,
    pub election_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceivedEvidenceKind {
    Received,
    Audited,
}

impl ReceivedEvidenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Received => "received",
            Self::Audited => "audited",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingReceivedBallot {
    pub id: i64,
    pub queued_at: DateTime<Utc>,
    pub kind: ReceivedEvidenceKind,
    pub ballot: ReceivedBallot,
}

#[derive(Debug, Clone)]
pub struct PendingPrivacyMessage {
    pub id: i64,
    pub delivery_id: String,
    pub message: Vec<u8>,
}

/// Network input accepted only after the service has applied its event policy.
#[derive(Debug, Clone)]
pub struct ForensicNetworkRecord<'a> {
    pub election_event_id: &'a str,
    pub election_id: Option<&'a str>,
    pub area_id: Option<&'a str>,
    pub event_kind: &'a str,
    pub delivery_id: &'a str,
    pub pseudonym: &'a str,
    pub ballot_id: Option<&'a str>,
    pub ip: &'a str,
    pub country: Option<&'a str>,
    pub user_agent: Option<&'a str>,
    pub recorded_at: DateTime<Utc>,
}

/// Retention dates are supplied by the operator's policy; no duration is guessed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind", content = "before")]
pub enum PrivacyPurge {
    Mappings,
    Network(chrono::DateTime<chrono::Utc>),
}

impl PostgresStore {
    pub async fn register_receipt_document(
        &self,
        event: &str,
        document: &str,
        identity: &str,
    ) -> Result<()> {
        self.client().await?.execute("INSERT INTO sequent_backend.private_receipt_document(election_event_id,document_id,pseudonym) VALUES($1::text::uuid,$2::text::uuid,$3)", &[&event,&document,&identity]).await?;
        Ok(())
    }
    pub async fn owns_receipt_document(
        &self,
        event: &str,
        document: &str,
        identity: &str,
    ) -> Result<bool> {
        Ok(self.client().await?.query_opt("SELECT 1 FROM sequent_backend.private_receipt_document WHERE election_event_id=$1::text::uuid AND document_id=$2::text::uuid AND pseudonym=$3", &[&event,&document,&identity]).await?.is_some())
    }

    /// Internal recipient selection reads first-participation flags, never a
    /// ballot history or a ballot timestamp next to a real voter identifier.
    pub async fn participating_voters(
        &self,
        event: &str,
        election: Option<&str>,
    ) -> Result<Vec<String>> {
        Ok(self.client().await?.query(
            "SELECT DISTINCT m.voter_id FROM sequent_backend.voter_pseudonym m JOIN sequent_backend.voter_participation p USING (election_event_id,pseudonym) WHERE m.election_event_id=$1::text::uuid AND ($2::text IS NULL OR p.election_id=$2::text::uuid)",
            &[&event,&election]).await?.iter().map(|row| row.get(0)).collect())
    }

    /// A batch lookup for the sequencer; one round trip, not one per cast.
    pub async fn received_ballots_for_hashes(
        &self,
        event: &str,
        hashes: &[String],
    ) -> Result<Vec<ReceivedBallot>> {
        let rows = self.client().await?.query(&format!(
            "SELECT {RECEIVED_COLUMNS} FROM ballot_box_received WHERE election_event_id=$1::text::uuid AND ballot_hash=ANY($2)"
        ), &[&event, &hashes]).await?;
        rows.iter().map(received_ballot).collect()
    }

    pub async fn purge_privacy_batch(
        &self,
        event: &str,
        scope: PrivacyPurge,
        limit: i64,
    ) -> Result<u64> {
        match scope {
            PrivacyPurge::Mappings => self.purge_voter_pseudonyms_batch(event, limit).await,
            PrivacyPurge::Network(before) => {
                self.purge_forensic_network(event, before, limit).await
            }
        }
    }

    /// Set once, before any ballots are received. Existing events with no row are
    /// legacy. A deleted private mapping is never implicitly regenerated.
    pub async fn set_ballot_identity(&self, event: &str, mode: BallotIdentity) -> Result<()> {
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        tx.execute(
            "SELECT pg_advisory_xact_lock(hashtextextended('ballot-identity:' || $1, 0))",
            &[&event],
        )
        .await?;
        let existing = tx.query_opt(
            "SELECT mode FROM sequent_backend.ballot_identity WHERE election_event_id = $1::text::uuid",
            &[&event],
        ).await?;
        if let Some(row) = existing {
            ensure!(
                row.get::<_, String>(0) == mode.as_str(),
                "Ballot identity is immutable"
            );
        } else {
            if mode == BallotIdentity::Pseudonym {
                tx.batch_execute(
                    "LOCK TABLE ballot_box_ballot, ballot_box_received IN SHARE ROW EXCLUSIVE MODE",
                )
                .await?;
                let row = tx.query_one(
                    "SELECT EXISTS (SELECT 1 FROM ballot_box_ballot WHERE election_event_id = $1::text::uuid)
                        OR EXISTS (SELECT 1 FROM ballot_box_received WHERE election_event_id = $1::text::uuid)",
                    &[&event],
                ).await?;
                ensure!(
                    !row.get::<_, bool>(0),
                    "An event with ballots cannot change identity"
                );
            }
            tx.execute(
                "INSERT INTO sequent_backend.ballot_identity (election_event_id, mode) VALUES ($1::text::uuid, $2)",
                &[&event, &mode.as_str()],
            ).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn ballot_identity(&self, event: &str) -> Result<BallotIdentity> {
        let row = self.client().await?.query_opt(
            "SELECT mode FROM sequent_backend.ballot_identity WHERE election_event_id = $1::text::uuid",
            &[&event],
        ).await?;
        match row.map(|row| row.get::<_, String>(0)).as_deref() {
            None | Some("voter-id") => Ok(BallotIdentity::VoterId),
            Some("pseudonym") => Ok(BallotIdentity::Pseudonym),
            _ => bail!("Unknown ballot identity"),
        }
    }

    /// Allocate missing identities in bounded batches. The allocation lock never
    /// participates in cast transactions. Shared reservations handle concurrent
    /// receipt codes; an existing voter always keeps their original pseudonym.
    pub async fn allocate_voter_pseudonyms(&self, event: &str, voters: &[String]) -> Result<usize> {
        ensure!(
            voters.iter().all(|voter| !voter.is_empty()),
            "A voter identity is empty"
        );
        let mut allocated = 0;
        for batch in voters.chunks(ALLOCATION_BATCH) {
            let mut client = self.client().await?;
            let tx = client.transaction().await?;
            tx.execute(
                "SELECT pg_advisory_xact_lock(hashtextextended('ballot-identity:' || $1, 0))",
                &[&event],
            )
            .await?;
            let row = tx.query_opt(
                "SELECT mode, mapping_purged_at IS NOT NULL FROM sequent_backend.ballot_identity
                 WHERE election_event_id = $1::text::uuid FOR SHARE",
                &[&event],
            ).await?.context("The event identity mode has not been set")?;
            ensure!(
                row.get::<_, String>(0) == "pseudonym",
                "The event uses voter identities"
            );
            ensure!(
                !row.get::<_, bool>(1),
                "The event's identity mapping was purged"
            );
            let existing: HashSet<String> = tx
                .query(
                    "SELECT voter_id FROM sequent_backend.voter_pseudonym
                 WHERE election_event_id = $1::text::uuid AND voter_id = ANY($2)",
                    &[&event, &batch],
                )
                .await?
                .into_iter()
                .map(|row| row.get(0))
                .collect();
            let mut pending: Vec<String> = batch
                .iter()
                .filter(|v| !existing.contains(*v))
                .cloned()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            for _ in 0..ALLOCATION_ATTEMPTS {
                if pending.is_empty() {
                    break;
                }
                let codes = pending
                    .iter()
                    .map(|_| random_pseudonym())
                    .collect::<Result<Vec<_>>>()?;
                let rows = tx.query(
                    "WITH candidates AS (SELECT DISTINCT ON (code) voter_id, code FROM UNNEST($2::text[], $3::text[]) AS c(voter_id, code) ORDER BY code, voter_id),
                     reserved AS (
                        INSERT INTO sequent_backend.ballot_code (election_event_id, code, kind)
                        SELECT $1::text::uuid, code, 'pseudonym' FROM candidates
                        ON CONFLICT DO NOTHING RETURNING code
                     )
                     INSERT INTO sequent_backend.voter_pseudonym (election_event_id, voter_id, pseudonym)
                     SELECT $1::text::uuid, c.voter_id, c.code FROM candidates c JOIN reserved r USING (code)
                     RETURNING voter_id",
                    &[&event, &pending, &codes],
                ).await?;
                let inserted: HashSet<String> = rows.into_iter().map(|row| row.get(0)).collect();
                allocated += inserted.len();
                pending.retain(|voter| !inserted.contains(voter));
            }
            ensure!(
                pending.is_empty(),
                "Unable to allocate unique voter pseudonyms"
            );
            tx.commit().await?;
        }
        Ok(allocated)
    }

    pub async fn voter_pseudonym(&self, event: &str, voter: &str) -> Result<Option<String>> {
        Ok(self
            .client()
            .await?
            .query_opt(
                "SELECT pseudonym FROM sequent_backend.voter_pseudonym
             WHERE election_event_id = $1::text::uuid AND voter_id = $2",
                &[&event, &voter],
            )
            .await?
            .map(|row| row.get(0)))
    }

    /// The service must require the resolver permission and durably audit the
    /// request before returning this private answer to its caller.
    pub async fn resolve_pseudonym(&self, event: &str, code: &str) -> Result<Option<String>> {
        let Some(code) = normalize_privacy_code(code) else {
            return Ok(None);
        };
        Ok(self
            .client()
            .await?
            .query_opt(
                "SELECT voter_id FROM sequent_backend.voter_pseudonym
             WHERE election_event_id = $1::text::uuid AND pseudonym = $2",
                &[&event, &code],
            )
            .await?
            .map(|row| row.get(0)))
    }

    pub async fn pseudonyms_for_voters(
        &self,
        event: &str,
        voters: &[String],
    ) -> Result<HashMap<String, String>> {
        let rows = self
            .client()
            .await?
            .query(
                "SELECT voter_id, pseudonym FROM sequent_backend.voter_pseudonym
             WHERE election_event_id = $1::text::uuid AND voter_id = ANY($2)",
                &[&event, &voters],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.get(0), row.get(1)))
            .collect())
    }

    /// Internal census/tally join only. Never return this batch mapping from a
    /// public action; human resolution uses the permissioned single-code route.
    pub async fn voters_for_pseudonyms(
        &self,
        event: &str,
        pseudonyms: &[String],
    ) -> Result<HashMap<String, String>> {
        let rows = self
            .client()
            .await?
            .query(
                "SELECT pseudonym, voter_id FROM sequent_backend.voter_pseudonym
             WHERE election_event_id = $1::text::uuid AND pseudonym = ANY($2)",
                &[&event, &pseudonyms],
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|row| (row.get(0), row.get(1)))
            .collect())
    }

    /// Counts immutable first-vote flags without reading ballots or timestamps.
    pub async fn privacy_participation_count(
        &self,
        event: &str,
        election: Option<&str>,
    ) -> Result<i64> {
        Ok(self
            .client()
            .await?
            .query_one(
                "SELECT COUNT(DISTINCT pseudonym) FROM sequent_backend.voter_participation
             WHERE election_event_id = $1::text::uuid
               AND ($2::text IS NULL OR election_id = $2::text::uuid)",
                &[&event, &election],
            )
            .await?
            .get(0))
    }

    pub async fn participation_for_voters(
        &self,
        event: &str,
        voters: &[String],
        election: Option<&str>,
    ) -> Result<Vec<VoterParticipation>> {
        Ok(self
            .client()
            .await?
            .query(
                "SELECT m.voter_id, p.election_id::text FROM sequent_backend.voter_pseudonym m
             JOIN sequent_backend.voter_participation p USING (election_event_id, pseudonym)
             WHERE m.election_event_id = $1::text::uuid AND m.voter_id = ANY($2)
               AND ($3::text IS NULL OR p.election_id = $3::text::uuid)",
                &[&event, &voters, &election],
            )
            .await?
            .into_iter()
            .map(|row| VoterParticipation {
                voter_id: row.get(0),
                election_id: row.get(1),
            })
            .collect())
    }

    pub async fn pending_received_ballots(
        &self,
        event: &str,
        limit: i64,
    ) -> Result<Vec<PendingReceivedBallot>> {
        ensure!((1..=10_000).contains(&limit), "Invalid evidence batch size");
        // A LATERAL projection keeps RECEIVED_COLUMNS unambiguous and makes the
        // shared row decoder reusable without changing receipt APIs.
        let rows = self
            .client()
            .await?
            .query(
                &format!(
                    "SELECT r.*, p.id, p.kind, p.created_at FROM sequent_backend.ballot_evidence_pending p
                      CROSS JOIN LATERAL (SELECT {RECEIVED_COLUMNS} FROM ballot_box_received
                        WHERE election_event_id = p.election_event_id AND id = p.received_id) r
                      WHERE p.election_event_id = $1::text::uuid ORDER BY p.id LIMIT $2"
                ),
                &[&event, &limit],
            )
            .await?;
        rows.iter()
            .map(|row| {
                Ok(PendingReceivedBallot {
                    id: row.try_get(17)?,
                    queued_at: row.try_get(19)?,
                    kind: match row.try_get::<_, String>(18)?.as_str() {
                        "received" => ReceivedEvidenceKind::Received,
                        "audited" => ReceivedEvidenceKind::Audited,
                        _ => bail!("Unknown pending ballot evidence kind"),
                    },
                    ballot: received_ballot(row)?,
                })
            })
            .collect()
    }

    pub async fn remove_pending_received(&self, event: &str, ids: &[i64]) -> Result<u64> {
        Ok(self
            .client()
            .await?
            .execute(
                "DELETE FROM sequent_backend.ballot_evidence_pending
             WHERE election_event_id = $1::text::uuid AND id = ANY($2)",
                &[&event, &ids],
            )
            .await?)
    }

    /// The caller supplies an already signed, privacy-safe message. Queueing
    /// completes before a privileged read returns its answer.
    pub async fn queue_privacy_message(
        &self,
        event: &str,
        delivery_id: &str,
        message: &[u8],
    ) -> Result<()> {
        ensure!(
            !delivery_id.is_empty() && !message.is_empty(),
            "Empty privacy audit message"
        );
        self.client().await?.execute(
            "INSERT INTO sequent_backend.privacy_message_pending (election_event_id, delivery_id, message)
             VALUES ($1::text::uuid, $2, $3) ON CONFLICT (election_event_id, delivery_id) DO NOTHING",
            &[&event, &delivery_id, &message],
        ).await?;
        Ok(())
    }

    pub async fn pending_privacy_messages(
        &self,
        event: &str,
        limit: i64,
    ) -> Result<Vec<PendingPrivacyMessage>> {
        ensure!(
            (1..=10_000).contains(&limit),
            "Invalid privacy message batch size"
        );
        Ok(self
            .client()
            .await?
            .query(
                "SELECT id, delivery_id, message FROM sequent_backend.privacy_message_pending
             WHERE election_event_id = $1::text::uuid ORDER BY id LIMIT $2",
                &[&event, &limit],
            )
            .await?
            .into_iter()
            .map(|row| PendingPrivacyMessage {
                id: row.get(0),
                delivery_id: row.get(1),
                message: row.get(2),
            })
            .collect())
    }

    pub async fn remove_pending_privacy_messages(&self, event: &str, ids: &[i64]) -> Result<u64> {
        Ok(self
            .client()
            .await?
            .execute(
                "DELETE FROM sequent_backend.privacy_message_pending
             WHERE election_event_id = $1::text::uuid AND id = ANY($2)",
                &[&event, &ids],
            )
            .await?)
    }

    /// Purge only the identity mapping and its participation children. Public
    /// ballot evidence and shared code reservations remain verifiable.
    pub async fn purge_voter_pseudonyms(&self, event: &str) -> Result<u64> {
        let mut total = 0;
        loop {
            let removed = self.purge_voter_pseudonyms_batch(event, 10_000).await?;
            total += removed;
            if removed == 0 {
                return Ok(total);
            }
        }
    }

    /// A retention worker may checkpoint between these bounded transactions.
    pub async fn purge_voter_pseudonyms_batch(&self, event: &str, limit: i64) -> Result<u64> {
        ensure!(
            (1..=10_000).contains(&limit),
            "Invalid mapping purge batch size"
        );
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        tx.execute(
            "UPDATE sequent_backend.ballot_identity SET mapping_purged_at = COALESCE(mapping_purged_at, now())
             WHERE election_event_id = $1::text::uuid AND mode = 'pseudonym'", &[&event],
        ).await?;
        let removed = tx.execute(
            "DELETE FROM sequent_backend.voter_pseudonym WHERE election_event_id = $1::text::uuid
             AND voter_id IN (SELECT voter_id FROM sequent_backend.voter_pseudonym
                 WHERE election_event_id = $1::text::uuid ORDER BY voter_id LIMIT $2)", &[&event, &limit],
        ).await?;
        tx.execute("INSERT INTO sequent_backend.privacy_purge_evidence (election_event_id, kind, removed) VALUES ($1::text::uuid, 'mappings', $2)", &[&event, &(removed as i64)]).await?;
        tx.commit().await?;
        Ok(removed)
    }

    /// Idempotent ingestion; only a newly inserted record is queued for the
    /// summary job, avoiding an address counter on the voting request path.
    pub async fn record_forensic_network(
        &self,
        record: &ForensicNetworkRecord<'_>,
    ) -> Result<bool> {
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        let inserted = record_forensic_in(&tx, record).await?;
        tx.commit().await?;
        Ok(inserted)
    }
}

/// Capture network metadata in the same transaction as successful acceptance.
pub(crate) async fn record_forensic_in(
    tx: &tokio_postgres::Transaction<'_>,
    record: &ForensicNetworkRecord<'_>,
) -> Result<bool> {
    ensure!(
        normalize_privacy_code(record.pseudonym).as_deref() == Some(record.pseudonym),
        "A forensic pseudonym must be canonical"
    );
    let row = tx.query_opt(
            "WITH inserted AS (
                INSERT INTO sequent_backend.forensic_network_record
                  (election_event_id, election_id, area_id, event_kind, delivery_id, pseudonym,
                   ballot_id, ip, country, user_agent, recorded_at)
                VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (election_event_id, delivery_id) DO NOTHING RETURNING id
             ), queued AS (
                INSERT INTO sequent_backend.forensic_summary_pending (record_id)
                SELECT id FROM inserted
             ) SELECT id FROM inserted",
            &[&record.election_event_id, &record.election_id, &record.area_id, &record.event_kind,
              &record.delivery_id, &record.pseudonym, &record.ballot_id, &record.ip, &record.country,
              &record.user_agent, &record.recorded_at],
    ).await?;
    Ok(row.is_some())
}

/// A row safe to send to any authorized viewer of this event/election/area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacyBallotRow {
    pub election_id: String,
    pub ballot_id: String,
    pub pseudonym: String,
    pub cast_at: String,
    pub voting_channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyBallotPage {
    pub ballots: Vec<PrivacyBallotRow>,
    /// Sequence cursor; the API must bind it to the authorized query scope.
    pub next_cursor: Option<i64>,
    pub match_count: Option<i64>,
    pub match_count_capped: bool,
}

/// The area is mandatory. Election visibility must be checked by the caller;
/// omit election only if every election in that area is authorized.
#[derive(Debug, Clone)]
pub struct PrivacyBallotQuery<'a> {
    pub event: &'a str,
    pub area: &'a str,
    pub election: Option<&'a str>,
    pub elections: Option<&'a [String]>,
    pub prefix: Option<&'a str>,
    pub before: Option<i64>,
    pub limit: i64,
}

fn search_prefix(input: &str) -> Result<String> {
    let compact = input.trim().to_ascii_uppercase().replace('-', "");
    ensure!(
        compact.len() <= 8 && compact.bytes().all(|b| ALPHABET.contains(&b)),
        "Invalid ballot code prefix"
    );
    Ok(if compact.len() > 4 {
        format!("{}-{}", &compact[..4], &compact[4..])
    } else {
        compact
    })
}

const PRIVACY_BALLOT_SCOPE: &str = "b.election_event_id = $1::text::uuid AND b.area_id = $2::text::uuid
    AND ($3::text IS NULL OR b.election_id = $3::text::uuid)
    AND ($4::text[] IS NULL OR b.election_id::text = ANY($4)) AND b.status = 'valid' AND r.status = 'cast'";
const PRIVACY_BALLOT_JOIN: &str = "ballot_box_ballot b JOIN ballot_box_received r
    ON r.election_event_id = b.election_event_id AND r.ballot_hash = b.ballot_id
    AND r.election_id = b.election_id AND r.voter_id = b.voter_id";

impl PostgresStore {
    /// A bounded page of accepted ballots, independent of signed-log delivery.
    /// Search uses separate prefix branches so each can use its code index.
    pub async fn privacy_ballot_page(
        &self,
        query: &PrivacyBallotQuery<'_>,
    ) -> Result<PrivacyBallotPage> {
        ensure!((1..=200).contains(&query.limit), "Invalid ballot page size");
        ensure!(
            self.ballot_identity(query.event).await? == BallotIdentity::Pseudonym,
            "This event does not use pseudonyms"
        );
        let prefix = query
            .prefix
            .map(search_prefix)
            .transpose()?
            .filter(|p| !p.is_empty());
        let before = query.before.unwrap_or(i64::MAX);
        let limit = query.limit + 1;
        let client = self.client().await?;
        let pattern = prefix.as_ref().map(|prefix| format!("{prefix}%"));
        let (sql, rows) = if let Some(pattern) = pattern.as_ref() {
            // Bound each branch before deduplication. An outer LIMIT on UNION
            // alone would still build a set of every matching ballot.
            let matches = format!("(SELECT b.seq FROM {PRIVACY_BALLOT_JOIN} WHERE {PRIVACY_BALLOT_SCOPE} AND r.ballot_id LIKE $5 LIMIT 10001)
                UNION (SELECT b.seq FROM {PRIVACY_BALLOT_JOIN} WHERE {PRIVACY_BALLOT_SCOPE} AND b.voter_id LIKE $5 LIMIT 10001)");
            let page_matches = format!(
                "(SELECT b.seq FROM {PRIVACY_BALLOT_JOIN} WHERE {PRIVACY_BALLOT_SCOPE}
                AND r.ballot_id LIKE $5 AND b.seq < $6 ORDER BY b.seq DESC LIMIT $7)
                UNION (SELECT b.seq FROM {PRIVACY_BALLOT_JOIN} WHERE {PRIVACY_BALLOT_SCOPE}
                AND b.voter_id LIKE $5 AND b.seq < $6 ORDER BY b.seq DESC LIMIT $7)"
            );
            let sql = format!("WITH matched AS ({page_matches}) SELECT b.seq, b.election_id::text, r.ballot_id, b.voter_id, b.accepted_at, b.voting_channel
                 FROM {PRIVACY_BALLOT_JOIN} JOIN matched m ON m.seq = b.seq
                 WHERE {PRIVACY_BALLOT_SCOPE} AND b.seq < $6 ORDER BY b.seq DESC LIMIT $7");
            let rows = client
                .query(
                    &sql,
                    &[
                        &query.event,
                        &query.area,
                        &query.election,
                        &query.elections,
                        &pattern,
                        &before,
                        &limit,
                    ],
                )
                .await?;
            (Some(matches), rows)
        } else {
            let sql = format!("SELECT b.seq, b.election_id::text, r.ballot_id, b.voter_id, b.accepted_at, b.voting_channel
                 FROM {PRIVACY_BALLOT_JOIN} WHERE {PRIVACY_BALLOT_SCOPE} AND b.seq < $5 ORDER BY b.seq DESC LIMIT $6");
            let rows = client
                .query(
                    &sql,
                    &[
                        &query.event,
                        &query.area,
                        &query.election,
                        &query.elections,
                        &before,
                        &limit,
                    ],
                )
                .await?;
            (None, rows)
        };
        let match_count = match (sql, pattern) {
            (Some(matches), Some(pattern)) => Some(
                client
                    .query_one(
                        &format!("SELECT count(*) FROM ({matches} LIMIT 10001) matches"),
                        &[
                            &query.event,
                            &query.area,
                            &query.election,
                            &query.elections,
                            &pattern,
                        ],
                    )
                    .await?
                    .get::<_, i64>(0),
            ),
            _ => None,
        };
        let has_more = rows.len() > query.limit as usize;
        let visible = rows.iter().take(query.limit as usize);
        let next_cursor = if has_more {
            rows.get(query.limit as usize - 1).map(|r| r.get(0))
        } else {
            None
        };
        let ballots = visible
            .map(|row| {
                Ok(PrivacyBallotRow {
                    election_id: row.try_get(1)?,
                    ballot_id: row.try_get(2)?,
                    pseudonym: row.try_get(3)?,
                    cast_at: row.try_get::<_, DateTime<Utc>>(4)?.to_rfc3339(),
                    voting_channel: row.try_get(5)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(PrivacyBallotPage {
            ballots,
            next_cursor,
            match_count: match_count.map(|count| count.min(10_000)),
            match_count_capped: match_count.is_some_and(|count| count > 10_000),
        })
    }

    /// Refresh a cached total outside the request's hot path.
    pub async fn privacy_ballot_count(
        &self,
        event: &str,
        area: &str,
        election: Option<&str>,
        elections: Option<&[String]>,
    ) -> Result<i64> {
        Ok(self
            .client()
            .await?
            .query_one(
                "SELECT count(*) FROM ballot_box_ballot WHERE election_event_id = $1::text::uuid
             AND area_id = $2::text::uuid AND ($3::text IS NULL OR election_id = $3::text::uuid)
             AND ($4::text[] IS NULL OR election_id::text = ANY($4)) AND status = 'valid'",
                &[&event, &area, &election, &elections],
            )
            .await?
            .get(0))
    }

    /// Public details only for cast ballots in the authorized area. The returned
    /// `voter_id` is a pseudonym, enforced by the event mode and write guard.
    pub async fn privacy_ballot_details(
        &self,
        event: &str,
        area: &str,
        election: Option<&str>,
        code: &str,
    ) -> Result<Option<ReceivedBallot>> {
        ensure!(
            self.ballot_identity(event).await? == BallotIdentity::Pseudonym,
            "This event does not use pseudonyms"
        );
        let Some(code) = normalize_privacy_code(code) else {
            return Ok(None);
        };
        self.client().await?.query_opt(
            &format!("SELECT {RECEIVED_COLUMNS} FROM ballot_box_received
                WHERE election_event_id = $1::text::uuid AND area_id = $2::text::uuid
                AND ($3::text IS NULL OR election_id = $3::text::uuid) AND ballot_id = $4 AND status = 'cast'"),
            &[&event, &area, &election, &code],
        ).await?.as_ref().map(received_ballot).transpose()
    }

    /// A summary job drains only a bounded batch. Locking queue rows makes
    /// retries/multiple workers safe without updating address totals on receive.
    pub async fn refresh_forensic_summary(&self, event: &str, limit: i64) -> Result<u64> {
        ensure!(
            (1..=10_000).contains(&limit),
            "Invalid forensic summary batch size"
        );
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        tx.execute(
            "SELECT pg_advisory_xact_lock(hashtextextended('forensic-summary:' || $1, 0))",
            &[&event],
        )
        .await?;
        let ids: Vec<i64> = tx
            .query(
                "SELECT p.record_id FROM sequent_backend.forensic_summary_pending p
             JOIN sequent_backend.forensic_network_record r ON r.id = p.record_id
             WHERE r.election_event_id = $1::text::uuid ORDER BY p.record_id
             LIMIT $2 FOR UPDATE OF p SKIP LOCKED",
                &[&event, &limit],
            )
            .await?
            .into_iter()
            .map(|row| row.get(0))
            .collect();
        if ids.is_empty() {
            return Ok(0);
        }
        tx.execute(
            "INSERT INTO sequent_backend.forensic_network_summary AS s (election_event_id, election_id, area_id, ip, records, country, first_at, last_at)
             SELECT election_event_id, election_id, area_id, ip, count(*), min(country), min(recorded_at), max(recorded_at)
             FROM sequent_backend.forensic_network_record WHERE id = ANY($1)
               AND event_kind = 'cast' AND election_id IS NOT NULL AND area_id IS NOT NULL
             GROUP BY election_event_id, election_id, area_id, ip
             ON CONFLICT (election_event_id, election_id, area_id, ip) DO UPDATE
             SET records = s.records + EXCLUDED.records, country = COALESCE(s.country, EXCLUDED.country),
                 first_at = LEAST(s.first_at, EXCLUDED.first_at),
                 last_at = GREATEST(s.last_at, EXCLUDED.last_at)", &[&ids],
        ).await?;
        tx.execute(
            "INSERT INTO sequent_backend.forensic_network_pseudonym AS s (election_event_id, election_id, area_id, ip, pseudonym, records)
             SELECT election_event_id, election_id, area_id, ip, pseudonym, count(*) FROM sequent_backend.forensic_network_record
             WHERE id = ANY($1) AND event_kind = 'cast' AND election_id IS NOT NULL AND area_id IS NOT NULL
             GROUP BY election_event_id, election_id, area_id, ip, pseudonym
             ON CONFLICT (election_event_id, election_id, area_id, ip, pseudonym) DO UPDATE SET records = s.records + EXCLUDED.records", &[&ids],
        ).await?;
        tx.execute(
            "DELETE FROM sequent_backend.forensic_summary_pending WHERE record_id = ANY($1)",
            &[&ids],
        )
        .await?;
        tx.commit().await?;
        Ok(ids.len() as u64)
    }

    /// Delete a bounded retention batch. Rebuild only affected addresses after
    /// their records change; this runs in the retention worker, never a UI query.
    pub async fn purge_forensic_network(
        &self,
        event: &str,
        before: DateTime<Utc>,
        limit: i64,
    ) -> Result<u64> {
        ensure!(
            (1..=10_000).contains(&limit),
            "Invalid forensic purge batch size"
        );
        let mut client = self.client().await?;
        let tx = client.transaction().await?;
        // Serialize the retention rebuild against summary workers for this event.
        tx.execute(
            "SELECT pg_advisory_xact_lock(hashtextextended('forensic-summary:' || $1, 0))",
            &[&event],
        )
        .await?;
        let removed = tx.query(
            "DELETE FROM sequent_backend.forensic_network_record WHERE id IN
             (SELECT id FROM sequent_backend.forensic_network_record WHERE election_event_id = $1::text::uuid
              AND recorded_at < $2 ORDER BY recorded_at, id LIMIT $3) RETURNING ip", &[&event, &before, &limit],
        ).await?;
        let ips: Vec<String> = removed
            .iter()
            .map(|row| row.get(0))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if !ips.is_empty() {
            tx.execute("DELETE FROM sequent_backend.forensic_network_summary WHERE election_event_id = $1::text::uuid AND ip = ANY($2)", &[&event, &ips]).await?;
            tx.execute("DELETE FROM sequent_backend.forensic_network_pseudonym WHERE election_event_id = $1::text::uuid AND ip = ANY($2)", &[&event, &ips]).await?;
            tx.execute(
                "INSERT INTO sequent_backend.forensic_network_summary (election_event_id, election_id, area_id, ip, records, country, first_at, last_at)
                 SELECT election_event_id, election_id, area_id, ip, count(*), min(country), min(recorded_at), max(recorded_at)
                 FROM sequent_backend.forensic_network_record r WHERE election_event_id = $1::text::uuid AND ip = ANY($2)
                   AND NOT EXISTS (SELECT 1 FROM sequent_backend.forensic_summary_pending p WHERE p.record_id = r.id)
                 AND event_kind = 'cast' AND election_id IS NOT NULL AND area_id IS NOT NULL
                 GROUP BY election_event_id, election_id, area_id, ip", &[&event, &ips],
            ).await?;
            tx.execute(
                "INSERT INTO sequent_backend.forensic_network_pseudonym (election_event_id, election_id, area_id, ip, pseudonym, records)
                 SELECT election_event_id, election_id, area_id, ip, pseudonym, count(*) FROM sequent_backend.forensic_network_record r
                 WHERE election_event_id = $1::text::uuid AND ip = ANY($2)
                   AND NOT EXISTS (SELECT 1 FROM sequent_backend.forensic_summary_pending p WHERE p.record_id = r.id)
                 AND event_kind = 'cast' AND election_id IS NOT NULL AND area_id IS NOT NULL
                 GROUP BY election_event_id, election_id, area_id, ip, pseudonym", &[&event, &ips],
            ).await?;
        }
        tx.execute("INSERT INTO sequent_backend.privacy_purge_evidence (election_event_id, kind, cutoff, removed) VALUES ($1::text::uuid, 'network', $2, $3)", &[&event, &before, &(removed.len() as i64)]).await?;
        tx.commit().await?;
        Ok(removed.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_codes_reject_ambiguous_or_malformed_values() {
        assert_eq!(
            normalize_privacy_code(" 7qkm2hxd ").as_deref(),
            Some("7QKM-2HXD")
        );
        assert_eq!(
            normalize_privacy_code("7QKM-2HXD").as_deref(),
            Some("7QKM-2HXD")
        );
        for invalid in [
            "7QKI-2HXD",
            "7QKL-2HXD",
            "7QKO-2HXD",
            "7QKU-2HXD",
            "7QK-M2HXD",
            "7QKM--2HXD",
            "éééé",
        ] {
            assert_eq!(normalize_privacy_code(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn generated_codes_have_the_public_format() {
        for _ in 0..256 {
            let code = random_pseudonym().expect("random source");
            assert_eq!(normalize_privacy_code(&code).as_ref(), Some(&code));
        }
    }
}
