// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ballot box: cast votes stored in an electoral-log database.
//!
//! A vote is accepted by one statement that updates the voter's state, stores the
//! ballot and queues it for the sequencer, which later appends the ballot's record
//! to the event's board. The tables are partitioned by election event.

use super::postgres::PostgresStore;
use anyhow::{ensure, Context, Result};
use std::time::{Duration, Instant};
use strum_macros::{Display, EnumString};
use tokio_postgres::error::SqlState;

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
}

/// What happened to a vote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptOutcome {
    /// Stored and queued for the sequencer under this sequence number, with
    /// this ID.
    Accepted { seq: i64, id: String },
    /// The voter already voted in this election in another area.
    VotedInOtherArea,
    /// The voter has cast all the votes the election allows.
    TooManyVotes,
    /// The ballot ID was already used in this election event.
    DuplicateBallotId,
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
            last_ballot_id = EXCLUDED.last_ballot_id,
            updated_at = EXCLUDED.updated_at
        WHERE v.area_id = EXCLUDED.area_id
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

const VOTER_STATE: &str = "SELECT area_id::text, votes, last_ballot_id FROM ballot_box_voter \
     WHERE election_event_id = $1::text::uuid AND election_id = $2::text::uuid AND voter_id = $3";

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
        for parent in [BALLOTS, VOTERS] {
            let partition = format!("{parent}_{suffix}");
            let storage = if parent == BALLOTS {
                format!("ALTER TABLE {partition} ALTER COLUMN content SET STORAGE EXTERNAL;")
            } else {
                String::new()
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
            transaction
                .commit()
                .await
                .with_context(|| format!("Error creating the ballot box of event {event}"))?;
        }
        Ok(())
    }

    /// Drop an election event's ballot box: its partitions, with every ballot and
    /// voter count, its queue and its lease. Idempotent.
    ///
    /// Each partition is detached with `DETACH PARTITION … CONCURRENTLY`, which waits
    /// for the transactions that may see it but holds back no vote of the tenant's
    /// other events, and then dropped. A detach that was interrupted is finished first.
    /// PostgreSQL detaches one partition of a table at a time, so drops of several
    /// events of a database take turns, for up to two minutes.
    pub async fn drop_ballot_box(&self, election_event_id: &str) -> Result<()> {
        let suffix = partition_suffix(election_event_id)?;
        let event = canonical_uuid(election_event_id)?;
        let client = self.client().await?;
        client
            .execute(
                "DELETE FROM ballot_box_pending WHERE election_event_id = $1::text::uuid",
                &[&event],
            )
            .await?;
        for parent in [BALLOTS, VOTERS] {
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
    /// queue it for the sequencer, unless the election's rules refuse it.
    pub async fn accept_ballot(&self, ballot: &AcceptBallot<'_>) -> Result<AcceptOutcome> {
        let client = self.client().await?;
        // Prepared once per connection, so that each vote does not parse and plan it.
        let statement = client
            .prepare_cached(ACCEPT)
            .await
            .context("Error preparing the statement that accepts ballots")?;
        let status = ballot.status.to_string();
        let accepted = client
            .query_opt(
                &statement,
                &[
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
                    &status,
                    &ballot.voter_ip,
                    &ballot.voter_country,
                    &ballot.username,
                ],
            )
            .await;
        match accepted {
            Ok(Some(row)) => Ok(AcceptOutcome::Accepted {
                seq: row.get(0),
                id: row.get(1),
            }),
            Ok(None) => {
                // On the same connection: taking a second one while holding this one
                // could exhaust the pool under load.
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
            Err(error) if error.code() == Some(&SqlState::UNIQUE_VIOLATION) => {
                Ok(AcceptOutcome::DuplicateBallotId)
            }
            Err(error) => Err(error).context("Error accepting the ballot"),
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
                "SELECT DISTINCT election_event_id::text FROM ballot_box_pending",
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
                "SELECT count(*) FROM ballot_box_pending WHERE election_event_id = $1::text::uuid",
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
