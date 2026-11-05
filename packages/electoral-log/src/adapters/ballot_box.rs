// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box: cast votes stored in an electoral-log database.
//!
//! A vote is accepted by one statement that updates the voter's state, stores the
//! ballot and queues it for the sequencer, which later appends the ballot's record
//! to the event's board. The tables are partitioned by election event.

use super::ballot_box_received::ReceivedStatus;
use super::ballot_box_seal::is_ballot_box_sealed;
use super::postgres::PostgresStore;
use super::privacy::ForensicNetworkRecord;
use crate::vitrine::{leaf_hash, ElectionEntry, Hash, Stage};
use anyhow::{ensure, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Object;
use std::str::FromStr;
use std::time::{Duration, Instant};
use strum_macros::{Display, EnumString};
use tokio_postgres::error::SqlState;
use tokio_postgres::types::ToSql;
use tracing::warn;

/// Status of a stored ballot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum BallotStatus {
    /// Counts as cast and goes to the tally.
    Valid,
    /// Counts as cast, waiting for an external confirmation such as Datafix's.
    Pending,
    /// Refused after acceptance; no longer counts as cast.
    Rejected,
}

/// A vote to accept.
#[derive(Debug, Clone)]
pub struct AcceptBallot<'a> {
    pub election_event_id: &'a str,
    pub election_id: &'a str,
    pub area_id: &'a str,
    pub voter_id: &'a str,
    pub ballot_id: &'a str,
    /// Name of the ballot's format, so that formats can coexist.
    pub format: &'a str,
    pub content: &'a str,
    pub voter_signature: Option<&'a [u8]>,
    /// Hash of the voter's ID, as in the cast-vote record.
    pub pseudonym_hash: &'a [u8],
    /// Hash of the ballot, as in the cast-vote record.
    pub ballot_hash: &'a [u8],
    pub voting_channel: &'a str,
    pub status: BallotStatus,
    pub voter_ip: Option<&'a str>,
    pub voter_country: Option<&'a str>,
    pub username: Option<&'a str>,
    /// Votes a voter may cast in the election; 0 means unlimited.
    pub allowed_votes: i32,
    /// The event's Vitrine `ballots` log, when the event has transparency on.
    pub vitrine: Option<VitrineQueue>,
}

/// Where an accepted ballot's `vitrine/election/ballots/v1` entry goes: the queue
/// partition of the event's `ballots` log, in the event's database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VitrineQueue {
    /// The sidecar's ID of the log.
    pub log_id: i64,
    /// When the ballot is accepted, in POSIX seconds truncated to the profile's
    /// granularity. The ballot row and the entry both hold it.
    pub accepted_at: i64,
    /// The ballot's Tracking ID digest: the `ballot_id` of every entry the
    /// ballot gets. The caller computes it from the ballot hash and the
    /// voter's key, as the voter's client did before sending the ballot.
    pub tracking_id: Hash,
}

/// The stages a vote that is not a received ballot logs together: with no
/// receipts there is no separate send, so the ballot box logs `received` and
/// `cast` in one statement.
const CAST_WITHOUT_RECEIPT: &[Stage] = &[Stage::Received, Stage::Cast];
/// The stage a vote that casts a received ballot logs: its `received` entry
/// was queued when the ballot box received it.
const CAST_RECEIVED: &[Stage] = &[Stage::Cast];

/// Whether an accepted ballot's entry went into its log's queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueState {
    /// Queued in the transaction that stored the ballot.
    Queued,
    /// The queue refused the insert, so the ballot was stored without it.
    NotQueued,
}

/// An accepted ballot's Vitrine entry and its leaf hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VitrineEntry {
    pub entry: Vec<u8>,
    pub leaf_hash: Hash,
    pub queue: QueueState,
}

/// Whether an acceptance is new.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[strum(serialize_all = "snake_case")]
pub enum Acceptance {
    /// This call stored the ballot.
    New,
    /// The voter had cast this ballot already: the stored acceptance, with the
    /// same entry, so a retried cast gets the same receipt. The entry is reported
    /// as queued; the log says whether it is.
    Replayed,
}

/// The received ballot a vote casts, with the cast the voter signed and the receipt
/// the ballot box signed for it.
#[derive(Debug, Clone)]
pub struct AcceptReceived<'a> {
    /// The receipt's short Ballot ID.
    pub ballot_id: &'a str,
    /// When the ballot was cast, as the receipt says: the stored ballot's
    /// `accepted_at`.
    pub cast_at: DateTime<Utc>,
    pub cast_signature: &'a str,
    pub cast_receipt_signature: &'a str,
}

/// What happened to a vote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptOutcome {
    /// Stored and queued for the sequencer under this sequence number, with
    /// this ID and status.
    Accepted {
        seq: i64,
        id: String,
        status: BallotStatus,
        acceptance: Acceptance,
        vitrine: Option<VitrineEntry>,
    },
    /// The voter already voted in this election in another area.
    VotedInOtherArea,
    /// The voter has cast all the votes the election allows.
    TooManyVotes,
    /// The ballot ID was already used in this election event. For a received
    /// ballot: it was already cast.
    DuplicateBallotId,
    /// The ballot box of the election's area is sealed and takes no more ballots.
    BallotBoxSealed,
    /// No ballot received for the voter under this Ballot ID and ballot hash waits
    /// to be cast: there is none, or it was audited.
    NotReceivable,
}

/// A stored ballot waiting to be appended to the event's board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingBallot {
    pub seq: i64,
    pub ballot_id: String,
    pub election_id: String,
    pub area_id: String,
    pub voter_id: String,
    pub pseudonym_hash: Vec<u8>,
    pub ballot_hash: Vec<u8>,
    pub voting_channel: String,
    pub status: String,
    pub voter_ip: Option<String>,
    pub voter_country: Option<String>,
    pub username: Option<String>,
    /// Seconds since the epoch.
    pub accepted_at: i64,
}

/// A voter's state in one election.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoterState {
    pub area_id: String,
    pub votes: i32,
    pub last_ballot_id: String,
}

/// The partitioned tables that hold a partition per election event.
const BALLOTS: &str = "ballot_box_ballot";
const VOTERS: &str = "ballot_box_voter";
const RECEIVED: &str = "ballot_box_received";
/// The TRUNCATE guard of an event's ballots partition.
const TRUNCATE_GUARD: &str = "ballot_box_seal_guard_truncate";
/// How long dropping a ballot box waits for another event's partition of the same
/// table to finish detaching: PostgreSQL detaches one at a time per table.
const DETACH_WAIT: Duration = Duration::from_secs(120);
const DETACH_RETRY: Duration = Duration::from_millis(200);

const ACCEPT: &str = r#"
WITH voter AS (
    INSERT INTO ballot_box_voter AS v
        (election_event_id, election_id, voter_id, area_id, votes, last_ballot_id, updated_at)
    VALUES ($1::text::uuid, $2::text::uuid, $3, $4::text::uuid, 1, $5, now())
    ON CONFLICT (election_event_id, election_id, voter_id) DO UPDATE
        SET votes = v.votes + 1,
            area_id = EXCLUDED.area_id,
            last_ballot_id = EXCLUDED.last_ballot_id,
            updated_at = EXCLUDED.updated_at
        WHERE (v.votes = 0 OR v.area_id = EXCLUDED.area_id)
          AND ($6::integer = 0 OR v.votes < $6::integer)
    RETURNING election_event_id
), ballot AS (
    INSERT INTO ballot_box_ballot
        (election_event_id, ballot_id, election_id, area_id, voter_id, format, content,
         voter_signature, pseudonym_hash, ballot_hash, voting_channel, status, voter_ip,
         voter_country, username)
    SELECT election_event_id, $5, $2::text::uuid, $4::text::uuid, $3, $7, $8, $9, $10, $11,
           $12, $13, $14, $15, $16
    FROM voter
    RETURNING election_event_id, seq, id
), queued AS (
    INSERT INTO ballot_box_pending (election_event_id, seq)
    SELECT election_event_id, seq FROM ballot
)
SELECT seq, id::text FROM ballot
"#;

/// `ACCEPT` with the ballot row's `accepted_at` bound to `$17`, so that the row and
/// the ballot's Vitrine entry hold the same time.
fn accept_bound() -> String {
    ACCEPT
        .replace(
            "voter_country, username)",
            "voter_country, username, accepted_at)",
        )
        .replace(
            "$12, $13, $14, $15, $16\n",
            "$12, $13, $14, $15, $16, to_timestamp($17::bigint)\n",
        )
}

/// The stored ballot with a given ballot ID, for idempotent casting.
const STORED_BALLOT: &str = "SELECT seq, id::text, voter_id, election_id::text, status, \
     floor(extract(epoch FROM accepted_at))::bigint FROM ballot_box_ballot \
     WHERE election_event_id = $1::text::uuid AND ballot_id = $2";

/// `ACCEPT` for a received ballot: the voter's received ballot, locked while it waits
/// to be cast, gates the vote, and is marked cast with the ballot's insert.
const ACCEPT_RECEIVED: &str = r#"
WITH gate AS (
    SELECT election_event_id, id FROM ballot_box_received
    WHERE election_event_id = $1::text::uuid AND election_id = $2::text::uuid
      AND voter_id = $3 AND ballot_id = $17 AND ballot_hash = $5 AND status = 'received'
    FOR UPDATE
), voter AS (
    INSERT INTO ballot_box_voter AS v
        (election_event_id, election_id, voter_id, area_id, votes, last_ballot_id, updated_at)
    SELECT election_event_id, $2::text::uuid, $3, $4::text::uuid, 1, $5, now() FROM gate
    ON CONFLICT (election_event_id, election_id, voter_id) DO UPDATE
        SET votes = v.votes + 1,
            area_id = EXCLUDED.area_id,
            last_ballot_id = EXCLUDED.last_ballot_id,
            updated_at = EXCLUDED.updated_at
        WHERE (v.votes = 0 OR v.area_id = EXCLUDED.area_id)
          AND ($6::integer = 0 OR v.votes < $6::integer)
    RETURNING election_event_id
), ballot AS (
    INSERT INTO ballot_box_ballot
        (election_event_id, ballot_id, election_id, area_id, voter_id, format, content,
         voter_signature, pseudonym_hash, ballot_hash, voting_channel, status, voter_ip,
         voter_country, username, accepted_at)
    SELECT election_event_id, $5, $2::text::uuid, $4::text::uuid, $3, $7, $8, $9, $10, $11,
           $12, $13, $14, $15, $16, $18::timestamptz
    FROM voter
    RETURNING election_event_id, seq, id
), queued AS (
    INSERT INTO ballot_box_pending (election_event_id, seq)
    SELECT election_event_id, seq FROM ballot
), marked AS (
    UPDATE ballot_box_received r
    SET status = 'cast', cast_at = $18::timestamptz, cast_signature = $19,
        cast_receipt_signature = $20
    FROM gate, ballot
    WHERE r.election_event_id = gate.election_event_id AND r.id = gate.id
)
SELECT seq, id::text FROM ballot
"#;

const RECEIVED_STATUS: &str = "SELECT status FROM ballot_box_received \
     WHERE election_event_id = $1::text::uuid AND election_id = $2::text::uuid \
       AND voter_id = $3 AND ballot_id = $4 AND ballot_hash = $5";

const VOTER_STATE: &str = "SELECT area_id::text, votes, last_ballot_id FROM ballot_box_voter \
     WHERE election_event_id = $1::text::uuid AND election_id = $2::text::uuid AND voter_id = $3";

/// Parameters `$1` to `$16` of `ACCEPT` and `ACCEPT_RECEIVED`.
fn accept_parameters<'a>(
    ballot: &'a AcceptBallot<'_>,
    status: &'a (dyn tokio_postgres::types::ToSql + Sync),
) -> Vec<&'a (dyn tokio_postgres::types::ToSql + Sync)> {
    vec![
        &ballot.election_event_id,
        &ballot.election_id,
        &ballot.voter_id,
        &ballot.area_id,
        &ballot.ballot_id,
        &ballot.allowed_votes,
        &ballot.format,
        &ballot.content,
        &ballot.voter_signature,
        &ballot.pseudonym_hash,
        &ballot.ballot_hash,
        &ballot.voting_channel,
        status,
        &ballot.voter_ip,
        &ballot.voter_country,
        &ballot.username,
    ]
}

fn accepted_outcome(
    row: &tokio_postgres::Row,
    ballot: &AcceptBallot<'_>,
    vitrine: Option<VitrineEntry>,
) -> AcceptOutcome {
    AcceptOutcome::Accepted {
        seq: row.get(0),
        id: row.get(1),
        status: ballot.status,
        acceptance: Acceptance::New,
        vitrine,
    }
}

/// The `vitrine/election/ballots/v1` entry of one stage of a ballot.
pub(crate) fn election_entry(
    election_event_id: &str,
    election_id: &str,
    stage: Stage,
    queue: &VitrineQueue,
    accepted_at: i64,
) -> Result<Vec<u8>> {
    ElectionEntry {
        event_id: canonical_uuid(election_event_id)?,
        election_id: canonical_uuid(election_id)?,
        stage,
        ballot_id: queue.tracking_id,
        accepted_at: u64::try_from(accepted_at).context("accepted_at is before 1970")?,
    }
    .encode()
}

/// Queue an encoded entry in the log's queue partition, in the caller's
/// transaction. The queue never refuses a ballot: the insert runs under a
/// savepoint, and if it fails the ballot stays accepted without its entry.
pub(crate) async fn queue_entry(
    transaction: &mut deadpool_postgres::Transaction<'_>,
    queue: &VitrineQueue,
    entry: Vec<u8>,
    what: &str,
) -> VitrineEntry {
    let insert = format!(
        "INSERT INTO vitrine.queue_{} (entry) VALUES ($1::bytea)",
        queue.log_id
    );
    let queued = async {
        let savepoint = transaction.savepoint("vitrine_queue").await?;
        savepoint.execute(insert.as_str(), &[&entry]).await?;
        savepoint.commit().await
    }
    .await;
    let state = match queued {
        Ok(()) => QueueState::Queued,
        Err(error) => {
            warn!(
                "The queue of Vitrine log {} refused {what}; accepting it without \
                 its entry: {error}",
                queue.log_id
            );
            QueueState::NotQueued
        }
    };
    VitrineEntry {
        leaf_hash: leaf_hash(&entry),
        entry,
        queue: state,
    }
}

/// Queue the ballot's Vitrine entries, one per stage, in the transaction that
/// stored the ballot. The last one is the entry the voter's answer is about.
async fn queue_vitrine_entry(
    transaction: &mut deadpool_postgres::Transaction<'_>,
    ballot: &AcceptBallot<'_>,
    stages: &[Stage],
) -> Option<VitrineEntry> {
    let queue = ballot.vitrine?;
    let mut last = None;
    for stage in stages {
        let entry = match election_entry(
            ballot.election_event_id,
            ballot.election_id,
            *stage,
            &queue,
            queue.accepted_at,
        ) {
            Ok(entry) => entry,
            Err(error) => {
                warn!(
                    "No Vitrine entry for ballot {}: {error:#}",
                    ballot.ballot_id
                );
                return None;
            }
        };
        last = Some(
            queue_entry(
                transaction,
                &queue,
                entry,
                &format!("ballot {} ({stage})", ballot.ballot_id),
            )
            .await,
        );
    }
    last
}

/// Run an accepting statement in a transaction and, when it stored a ballot,
/// queue the ballot's Vitrine entry in the same transaction.
async fn query_and_queue(
    client: &mut Object,
    statement: &tokio_postgres::Statement,
    parameters: &[&(dyn ToSql + Sync)],
    ballot: &AcceptBallot<'_>,
    stages: &[Stage],
) -> Result<(Option<tokio_postgres::Row>, Option<VitrineEntry>), tokio_postgres::Error> {
    if ballot.vitrine.is_none() {
        return Ok((client.query_opt(statement, parameters).await?, None));
    }
    let mut transaction = client.transaction().await?;
    let row = transaction.query_opt(statement, parameters).await?;
    let vitrine = match row {
        Some(_) => queue_vitrine_entry(&mut transaction, ballot, stages).await,
        None => None,
    };
    transaction.commit().await?;
    Ok((row, vitrine))
}

/// The stored acceptance of a ballot the voter cast already, if there is one.
async fn replay(client: &Object, ballot: &AcceptBallot<'_>) -> Result<Option<AcceptOutcome>> {
    let Some(row) = client
        .query_opt(
            STORED_BALLOT,
            &[&ballot.election_event_id, &ballot.ballot_id],
        )
        .await
        .context("Error reading the stored ballot")?
    else {
        return Ok(None);
    };
    let voter_id: String = row.get(2);
    let election_id: String = row.get(3);
    let status: String = row.get(4);
    let status = BallotStatus::from_str(&status)
        .with_context(|| format!("Unknown ballot status {status:?}"))?;
    if voter_id != ballot.voter_id
        || !election_id.eq_ignore_ascii_case(ballot.election_id)
        || status == BallotStatus::Rejected
    {
        return Ok(None);
    }
    let accepted_at: i64 = row.get(5);
    let vitrine = ballot.vitrine.and_then(|queue| {
        election_entry(
            ballot.election_event_id,
            ballot.election_id,
            Stage::Cast,
            &queue,
            accepted_at,
        )
            .map_err(|error| {
                warn!(
                    "No Vitrine entry for ballot {}: {error:#}",
                    ballot.ballot_id
                )
            })
            .ok()
            .map(|entry| VitrineEntry {
                leaf_hash: leaf_hash(&entry),
                entry,
                queue: QueueState::Queued,
            })
    });
    Ok(Some(AcceptOutcome::Accepted {
        seq: row.get(0),
        id: row.get(1),
        status,
        acceptance: Acceptance::Replayed,
        vitrine,
    }))
}

/// Why the voter's rules refused a vote that stored nothing.
async fn refused_outcome(
    client: &deadpool_postgres::Object,
    ballot: &AcceptBallot<'_>,
) -> Result<AcceptOutcome> {
    // On the same connection: taking a second one while holding this one could
    // exhaust the pool under load.
    let state = client
        .query_opt(
            VOTER_STATE,
            &[
                &ballot.election_event_id,
                &ballot.election_id,
                &ballot.voter_id,
            ],
        )
        .await?
        .map(voter_state_from_row)
        .context("A refused vote left no voter state")?;
    Ok(if state.area_id != ballot.area_id.to_ascii_lowercase() {
        AcceptOutcome::VotedInOtherArea
    } else {
        AcceptOutcome::TooManyVotes
    })
}

fn error_outcome(error: tokio_postgres::Error) -> Result<AcceptOutcome> {
    if error.code() == Some(&SqlState::UNIQUE_VIOLATION) {
        Ok(AcceptOutcome::DuplicateBallotId)
    } else if is_ballot_box_sealed(&error) {
        Ok(AcceptOutcome::BallotBoxSealed)
    } else {
        Err(error).context("Error accepting the ballot")
    }
}

fn voter_state_from_row(row: tokio_postgres::Row) -> VoterState {
    VoterState {
        area_id: row.get(0),
        votes: row.get(1),
        last_ballot_id: row.get(2),
    }
}

/// A UUID in its canonical form, lowercase with dashes, for SQL that cannot take it
/// as a parameter.
pub(crate) fn canonical_uuid(id: &str) -> Result<String> {
    let hex = partition_suffix(id)?;
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ))
}

fn partition_suffix(election_event_id: &str) -> Result<String> {
    let suffix: String = election_event_id
        .chars()
        .filter(|c| *c != '-')
        .collect::<String>()
        .to_ascii_lowercase();
    ensure!(
        suffix.len() == 32 && suffix.chars().all(|c| c.is_ascii_hexdigit()),
        "Invalid election event ID {election_event_id:?}"
    );
    Ok(suffix)
}

async fn received_refused_outcome(
    client: &deadpool_postgres::Object,
    ballot: &AcceptBallot<'_>,
    received: &AcceptReceived<'_>,
) -> Result<AcceptOutcome> {
    let status = client
        .query_opt(
            RECEIVED_STATUS,
            &[
                &ballot.election_event_id,
                &ballot.election_id,
                &ballot.voter_id,
                &received.ballot_id,
                &ballot.ballot_id,
            ],
        )
        .await?
        .map(|row| row.try_get::<_, String>(0))
        .transpose()?;
    match status.as_deref().map(str::parse::<ReceivedStatus>) {
        Some(Ok(ReceivedStatus::Received)) => refused_outcome(client, ballot).await,
        Some(Ok(ReceivedStatus::Cast)) => Ok(AcceptOutcome::DuplicateBallotId),
        Some(Ok(ReceivedStatus::Audited)) | None => Ok(AcceptOutcome::NotReceivable),
        Some(Err(_)) => Err(anyhow::anyhow!("Unknown received ballot status {status:?}")),
    }
}

fn check_forensic_scope(
    ballot: &AcceptBallot<'_>,
    record: &ForensicNetworkRecord<'_>,
) -> Result<()> {
    ensure!(
        super::privacy::normalize_privacy_code(record.pseudonym).as_deref()
            == Some(record.pseudonym),
        "A forensic pseudonym must be canonical"
    );
    ensure!(
        record.election_event_id == ballot.election_event_id
            && record.election_id == Some(ballot.election_id)
            && record.area_id == Some(ballot.area_id)
            && record.pseudonym == ballot.voter_id
            && record.event_kind == "cast",
        "Forensic metadata does not match the accepted ballot"
    );
    Ok(())
}

/// Extend the existing single acceptance statement: forensic data and its
/// summary outbox are inserted only from a successfully accepted ballot.
fn acceptance_with_forensics(sql: &str, first: usize) -> String {
    let delivery = first;
    let code = first + 1;
    let ip = first + 2;
    let country = first + 3;
    let agent = first + 4;
    let recorded = first + 5;
    sql.replace(
        "SELECT seq, id::text FROM ballot",
        &format!(
            ", forensic AS (
            INSERT INTO sequent_backend.forensic_network_record
                (election_event_id,election_id,area_id,event_kind,delivery_id,pseudonym,
                 ballot_id,ip,country,user_agent,recorded_at)
            SELECT $1::text::uuid,$2::text::uuid,$4::text::uuid,'cast',${delivery},$3,
                ${code},${ip},${country},${agent},${recorded} FROM ballot
            ON CONFLICT (election_event_id,delivery_id) DO NOTHING RETURNING id
        ), forensic_queued AS (
            INSERT INTO sequent_backend.forensic_summary_pending(record_id) SELECT id FROM forensic
        ) SELECT seq, id::text FROM ballot"
        ),
    )
}

impl PostgresStore {
    /// Create an election event's ballot box partitions. Idempotent.
    ///
    /// Each partition is created as a table of its own and then attached, which takes
    /// a `SHARE UPDATE EXCLUSIVE` lock on the partitioned table, so the votes of the
    /// tenant's other events go on meanwhile. `CREATE TABLE … PARTITION OF` would need
    /// an `ACCESS EXCLUSIVE` lock: it would wait for the votes in progress and hold
    /// back new ones until it got it.
    pub async fn create_ballot_box(&self, election_event_id: &str) -> Result<()> {
        let suffix = partition_suffix(election_event_id)?;
        let event = canonical_uuid(election_event_id)?;
        let mut client = self.client().await?;
        for parent in [BALLOTS, VOTERS, RECEIVED] {
            let partition = format!("{parent}_{suffix}");
            let storage = if parent == VOTERS {
                String::new()
            } else {
                format!("ALTER TABLE {partition} ALTER COLUMN content SET STORAGE EXTERNAL;")
            };
            let transaction = client.transaction().await?;
            // Concurrent creations of one event's ballot box take turns.
            transaction
                .execute(
                    "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                    &[&format!("ballot-box:{event}")],
                )
                .await?;
            transaction
                .batch_execute(&format!(
                    "CREATE TABLE IF NOT EXISTS {partition} \
                         (LIKE {parent} INCLUDING DEFAULTS INCLUDING CONSTRAINTS INCLUDING STORAGE);
                     {storage}"
                ))
                .await?;
            let attached: bool = transaction
                .query_one(
                    "SELECT relispartition FROM pg_class WHERE oid = $1::text::regclass",
                    &[&partition],
                )
                .await?
                .get(0);
            if !attached {
                transaction
                    .batch_execute(&format!(
                        "ALTER TABLE {parent} ATTACH PARTITION {partition} FOR VALUES IN ('{event}')"
                    ))
                    .await?;
            }
            // Statement triggers of the partitioned table are not cloned to partitions.
            if parent == BALLOTS {
                let guarded = transaction
                    .query_opt(
                        "SELECT 1 FROM pg_trigger WHERE tgrelid = $1::text::regclass AND tgname = $2",
                        &[&partition, &TRUNCATE_GUARD],
                    )
                    .await?
                    .is_some();
                if !guarded {
                    transaction
                        .batch_execute(&format!(
                            "CREATE TRIGGER {TRUNCATE_GUARD} BEFORE TRUNCATE ON {partition} \
                             FOR EACH STATEMENT EXECUTE FUNCTION {TRUNCATE_GUARD}('{event}')"
                        ))
                        .await?;
                }
            }
            transaction
                .commit()
                .await
                .with_context(|| format!("Error creating the ballot box of event {event}"))?;
        }
        Ok(())
    }

    /// Create what later versions of the schema add to each ballot box of the
    /// database, as applying the schema to an existing database does.
    pub async fn upgrade_ballot_boxes(&self) -> Result<()> {
        let rows = self
            .client()
            .await?
            .query(
                "SELECT c.relname::text FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid \
                 WHERE i.inhparent = $1::text::regclass AND NOT i.inhdetachpending",
                &[&BALLOTS],
            )
            .await
            .context("Error listing the ballot boxes")?;
        let prefix = format!("{BALLOTS}_");
        for row in rows {
            let partition: String = row.try_get(0)?;
            let event = partition
                .strip_prefix(&prefix)
                .with_context(|| format!("Unexpected ballot box partition {partition}"))?;
            self.create_ballot_box(event).await?;
        }
        Ok(())
    }

    /// Drop an election event's ballot box: its partitions, with every ballot, voter
    /// count and received ballot, its queue and its lease. Idempotent. Refuses a
    /// sealed ballot box: seals are kept until the event's database is dropped.
    ///
    /// Each partition is detached with `DETACH PARTITION … CONCURRENTLY`, which waits
    /// for the transactions that may see it but holds back no vote of the tenant's
    /// other events, and then dropped. A detach that was interrupted is finished first.
    /// PostgreSQL detaches one partition of a table at a time, so drops of several
    /// events of a database take turns, for up to two minutes.
    pub async fn drop_ballot_box(&self, election_event_id: &str) -> Result<()> {
        let suffix = partition_suffix(election_event_id)?;
        let event = canonical_uuid(election_event_id)?;
        ensure!(
            !self.has_ballot_box_seals(&event).await?,
            "The ballot box of election event {event} is sealed"
        );
        let client = self.client().await?;
        client
            .execute(
                "DELETE FROM ballot_box_pending WHERE election_event_id = $1::text::uuid",
                &[&event],
            )
            .await?;
        client.execute(
            "DELETE FROM sequent_backend.ballot_evidence_pending WHERE election_event_id = $1::text::uuid", &[&event],
        ).await?;
        for parent in [BALLOTS, VOTERS, RECEIVED] {
            let partition = format!("{parent}_{suffix}");
            let detach_pending: Option<bool> = client
                .query_opt(
                    "SELECT inhdetachpending FROM pg_inherits WHERE inhrelid = to_regclass($1)",
                    &[&partition],
                )
                .await?
                .map(|row| row.get(0));
            let detach = match detach_pending {
                Some(false) => Some("CONCURRENTLY"),
                Some(true) => Some("FINALIZE"),
                None => None,
            };
            if let Some(mode) = detach {
                let started = Instant::now();
                loop {
                    let detached = client
                        .batch_execute(&format!(
                            "ALTER TABLE {parent} DETACH PARTITION {partition} {mode}"
                        ))
                        .await;
                    match detached {
                        Ok(()) => break,
                        Err(error)
                            if error.code()
                                == Some(&SqlState::OBJECT_NOT_IN_PREREQUISITE_STATE)
                                && started.elapsed() < DETACH_WAIT =>
                        {
                            tokio::time::sleep(DETACH_RETRY).await;
                        }
                        Err(error) => {
                            return Err(error)
                                .with_context(|| format!("Error detaching {partition}"))
                        }
                    }
                }
            }
            client
                .batch_execute(&format!("DROP TABLE IF EXISTS {partition}"))
                .await
                .with_context(|| format!("Error dropping {partition}"))?;
        }
        client
            .execute(
                "DELETE FROM ballot_box_sequencer WHERE election_event_id = $1::text::uuid",
                &[&event],
            )
            .await?;
        Ok(())
    }

    /// Whether an election event has a ballot box.
    pub async fn has_ballot_box(&self, election_event_id: &str) -> Result<bool> {
        let suffix = partition_suffix(election_event_id)?;
        let row = self
            .client()
            .await?
            .query_one(
                "SELECT to_regclass($1) IS NOT NULL",
                &[&format!("ballot_box_ballot_{suffix}")],
            )
            .await?;
        Ok(row.get(0))
    }

    /// Accept a vote: in one statement, count it for the voter, store the ballot and
    /// queue it for the sequencer, unless the election's rules or the box's seal
    /// refuse it. With transparency on, the ballot's Vitrine entry is queued in the
    /// same transaction.
    ///
    /// Casting is idempotent: a ballot ID that the same voter already cast in the
    /// same election returns the stored acceptance, unless the ballot was rejected
    /// since. The same ballot ID from another voter is still refused.
    ///
    /// The Vitrine queue never refuses a ballot: if its insert fails, the ballot
    /// is accepted without it, with the same `accepted_at`.
    pub async fn accept_ballot(&self, ballot: &AcceptBallot<'_>) -> Result<AcceptOutcome> {
        let mut client = self.client().await?;
        // Prepared once per connection, so that each vote does not parse and plan it.
        let statement = if ballot.vitrine.is_some() {
            client.prepare_cached(&accept_bound()).await
        } else {
            client.prepare_cached(ACCEPT).await
        }
        .context("Error preparing the statement that accepts ballots")?;
        let status = ballot.status.to_string();
        let accepted_at = ballot.vitrine.map(|queue| queue.accepted_at);
        let mut parameters = accept_parameters(ballot, &status);
        if ballot.vitrine.is_some() {
            parameters.push(&accepted_at);
        }
        let accepted = query_and_queue(&mut client, &statement, &parameters, ballot, CAST_WITHOUT_RECEIPT).await;
        match accepted {
            Ok((Some(row), vitrine)) => Ok(accepted_outcome(&row, ballot, vitrine)),
            Ok((None, _)) => {
                if let Some(replayed) = replay(&client, ballot).await? {
                    return Ok(replayed);
                }
                refused_outcome(&client, ballot).await
            }
            Err(error) if error.code() == Some(&SqlState::UNIQUE_VIOLATION) => {
                match replay(&client, ballot).await? {
                    Some(replayed) => Ok(replayed),
                    None => error_outcome(error),
                }
            }
            Err(error) => error_outcome(error),
        }
    }

    /// Accept a vote that casts a received ballot, as `accept_ballot` does, and mark
    /// the received ballot cast in the same statement. Refused, leaving it received,
    /// unless the voter's received ballot with this Ballot ID and `ballot.ballot_id`
    /// as its ballot hash waits to be cast. A second cast of it, also a concurrent
    /// one, is `DuplicateBallotId`.
    pub async fn accept_received_ballot(
        &self,
        ballot: &AcceptBallot<'_>,
        received: &AcceptReceived<'_>,
    ) -> Result<AcceptOutcome> {
        let mut client = self.client().await?;
        let statement = client
            .prepare_cached(ACCEPT_RECEIVED)
            .await
            .context("Error preparing the statement that casts received ballots")?;
        let status = ballot.status.to_string();
        let mut parameters = accept_parameters(ballot, &status);
        parameters.extend_from_slice(&[
            &received.ballot_id,
            &received.cast_at,
            &received.cast_signature,
            &received.cast_receipt_signature,
        ]);
        let accepted =
            query_and_queue(&mut client, &statement, &parameters, ballot, CAST_RECEIVED).await;
        match accepted {
            Ok((Some(row), vitrine)) => Ok(accepted_outcome(&row, ballot, vitrine)),
            Ok((None, _)) => received_refused_outcome(&client, ballot, received).await,
            Err(error) => error_outcome(error),
        }
    }

    /// Cast and capture permitted network data atomically. A failed cast leaves
    /// neither a forensic record nor a summary-queue entry.
    pub async fn accept_received_ballot_with_forensics(
        &self,
        ballot: &AcceptBallot<'_>,
        received: &AcceptReceived<'_>,
        record: &ForensicNetworkRecord<'_>,
    ) -> Result<AcceptOutcome> {
        check_forensic_scope(ballot, record)?;
        ensure!(
            record.ballot_id == Some(received.ballot_id) && record.recorded_at == received.cast_at,
            "Forensic receipt metadata does not match the cast"
        );
        let mut client = self.client().await?;
        let statement = client
            .prepare_cached(&acceptance_with_forensics(ACCEPT_RECEIVED, 21))
            .await?;
        let status = ballot.status.to_string();
        let mut parameters = accept_parameters(ballot, &status);
        parameters.extend_from_slice(&[
            &received.ballot_id,
            &received.cast_at,
            &received.cast_signature,
            &received.cast_receipt_signature,
            &record.delivery_id,
            &record.ballot_id,
            &record.ip,
            &record.country,
            &record.user_agent,
            &record.recorded_at,
        ]);
        match query_and_queue(&mut client, &statement, &parameters, ballot, CAST_RECEIVED).await {
            Ok((Some(row), vitrine)) => Ok(accepted_outcome(&row, ballot, vitrine)),
            Ok((None, _)) => received_refused_outcome(&client, ballot, received).await,
            Err(error) => error_outcome(error),
        }
    }

    pub async fn accept_ballot_with_forensics(
        &self,
        ballot: &AcceptBallot<'_>,
        record: &ForensicNetworkRecord<'_>,
    ) -> Result<AcceptOutcome> {
        check_forensic_scope(ballot, record)?;
        ensure!(
            record.ballot_id == Some(ballot.ballot_id),
            "Forensic ballot metadata does not match the cast"
        );
        let mut client = self.client().await?;
        let bound = ballot.vitrine.is_some();
        let statement = client
            .prepare_cached(&acceptance_with_forensics(
                &if bound { accept_bound() } else { ACCEPT.to_string() },
                if bound { 18 } else { 17 },
            ))
            .await?;
        let status = ballot.status.to_string();
        let accepted_at = ballot.vitrine.map(|queue| queue.accepted_at);
        let mut parameters = accept_parameters(ballot, &status);
        if bound {
            parameters.push(&accepted_at);
        }
        parameters.extend_from_slice(&[
            &record.delivery_id,
            &record.ballot_id,
            &record.ip,
            &record.country,
            &record.user_agent,
            &record.recorded_at,
        ]);
        match query_and_queue(&mut client, &statement, &parameters, ballot, CAST_WITHOUT_RECEIPT).await {
            Ok((Some(row), vitrine)) => Ok(accepted_outcome(&row, ballot, vitrine)),
            Ok((None, _)) => refused_outcome(&client, ballot).await,
            Err(error) => error_outcome(error),
        }
    }

    /// A voter's state in one election, if the voter has voted in it.
    pub async fn voter_state(
        &self,
        election_event_id: &str,
        election_id: &str,
        voter_id: &str,
    ) -> Result<Option<VoterState>> {
        let row = self
            .client()
            .await?
            .query_opt(VOTER_STATE, &[&election_event_id, &election_id, &voter_id])
            .await?;
        Ok(row.map(voter_state_from_row))
    }

    /// The oldest ballots of an event waiting for the sequencer, in acceptance order.
    pub async fn pending_ballots(
        &self,
        election_event_id: &str,
        limit: i64,
    ) -> Result<Vec<PendingBallot>> {
        let rows = self
            .client()
            .await?
            .query(
                "SELECT b.seq, b.ballot_id, b.election_id::text, b.area_id::text, b.voter_id, \
                        b.pseudonym_hash, b.ballot_hash, b.voting_channel, b.status, b.voter_ip, \
                        b.voter_country, b.username, extract(epoch FROM b.accepted_at)::bigint \
                 FROM ballot_box_pending p \
                 JOIN ballot_box_ballot b \
                   ON b.election_event_id = p.election_event_id AND b.seq = p.seq \
                 WHERE p.election_event_id = $1::text::uuid \
                 ORDER BY p.seq \
                 LIMIT $2",
                &[&election_event_id, &limit],
            )
            .await
            .context("Error reading the ballots waiting for the sequencer")?;
        Ok(rows
            .into_iter()
            .map(|row| PendingBallot {
                seq: row.get(0),
                ballot_id: row.get(1),
                election_id: row.get(2),
                area_id: row.get(3),
                voter_id: row.get(4),
                pseudonym_hash: row.get(5),
                ballot_hash: row.get(6),
                voting_channel: row.get(7),
                status: row.get(8),
                voter_ip: row.get(9),
                voter_country: row.get(10),
                username: row.get(11),
                accepted_at: row.get(12),
            })
            .collect())
    }

    /// Remove ballots the sequencer appended from its queue.
    pub async fn remove_pending(&self, election_event_id: &str, seqs: &[i64]) -> Result<u64> {
        self.client()
            .await?
            .execute(
                "DELETE FROM ballot_box_pending \
                 WHERE election_event_id = $1::text::uuid AND seq = ANY($2)",
                &[&election_event_id, &seqs],
            )
            .await
            .context("Error removing sequenced ballots from the queue")
    }

    /// Take the sequencer lease of an event for `seconds`, unless another holder's
    /// lease is still running. A lease, rather than a lock held on a connection,
    /// leaves the connection pool to the sequencer's work.
    pub async fn take_sequencer_lease(
        &self,
        election_event_id: &str,
        holder: &str,
        seconds: i32,
    ) -> Result<bool> {
        let taken = self
            .client()
            .await?
            .query_opt(
                "INSERT INTO ballot_box_sequencer AS s (election_event_id, holder, lease_until) \
                 VALUES ($1::text::uuid, $2, now() + make_interval(secs => $3)) \
                 ON CONFLICT (election_event_id) DO UPDATE \
                     SET holder = EXCLUDED.holder, lease_until = EXCLUDED.lease_until \
                     WHERE s.lease_until < now() OR s.holder = EXCLUDED.holder \
                 RETURNING holder",
                &[&election_event_id, &holder, &f64::from(seconds)],
            )
            .await
            .context("Error taking the sequencer lease")?;
        Ok(taken.is_some())
    }

    /// End a sequencer lease early.
    pub async fn release_sequencer_lease(
        &self,
        election_event_id: &str,
        holder: &str,
    ) -> Result<()> {
        self.client()
            .await?
            .execute(
                "UPDATE ballot_box_sequencer SET lease_until = now() \
                 WHERE election_event_id = $1::text::uuid AND holder = $2",
                &[&election_event_id, &holder],
            )
            .await
            .context("Error releasing the sequencer lease")?;
        Ok(())
    }

    /// Election events with ballots waiting for the sequencer.
    pub async fn events_with_pending_ballots(&self) -> Result<Vec<String>> {
        let rows = self
            .client()
            .await?
            .query(
                "SELECT election_event_id::text FROM ballot_box_pending
                 UNION SELECT election_event_id::text FROM sequent_backend.ballot_evidence_pending
                 UNION SELECT election_event_id::text FROM sequent_backend.privacy_message_pending",
                &[],
            )
            .await
            .context("Error listing the events with ballots waiting for the sequencer")?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }

    /// Ballots of an event waiting for the sequencer.
    pub async fn pending_count(&self, election_event_id: &str) -> Result<i64> {
        let row = self
            .client()
            .await?
            .query_one(
                "SELECT (SELECT count(*) FROM ballot_box_pending WHERE election_event_id = $1::text::uuid)
                     + (SELECT count(*) FROM sequent_backend.ballot_evidence_pending WHERE election_event_id = $1::text::uuid)
                     + (SELECT count(*) FROM sequent_backend.privacy_message_pending WHERE election_event_id = $1::text::uuid)",
                &[&election_event_id],
            )
            .await?;
        Ok(row.get(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partitions_are_named_after_the_event() {
        assert_eq!(
            partition_suffix("FDD21DB2-dd68-4974-90eb-7f2750b2b5df").unwrap(),
            "fdd21db2dd68497490eb7f2750b2b5df"
        );
        assert!(partition_suffix("fdd21db2").is_err());
        assert!(partition_suffix("fdd21db2-dd68-4974-90eb-7f2750b2b5dz").is_err());
        assert!(partition_suffix("x'); DROP TABLE ballot_box_ballot; --").is_err());
    }

    #[test]
    fn uuids_are_written_in_canonical_form() {
        assert_eq!(
            canonical_uuid("FDD21DB2DD68497490EB7F2750B2B5DF").unwrap(),
            "fdd21db2-dd68-4974-90eb-7f2750b2b5df"
        );
        assert!(canonical_uuid("fdd21db2-dd68-4974-90eb-7f2750b2b5d'").is_err());
    }

    #[test]
    fn statuses_have_stable_names() {
        for (status, name) in [
            (BallotStatus::Valid, "valid"),
            (BallotStatus::Pending, "pending"),
            (BallotStatus::Rejected, "rejected"),
        ] {
            assert_eq!(status.to_string(), name);
            assert_eq!(name.parse::<BallotStatus>().unwrap(), status);
        }
    }
}
