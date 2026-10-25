// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot box seals in the event's database. A seal, sealed or failed, is stored with
//! its record on the event's board in one transaction, and from then on the box takes
//! no writes: the `ballot_box_seal_guard` trigger of `ballot_box_ballot` refuses them.
//!
//! The sealer works in one READ COMMITTED transaction of the event's database: it
//! takes the box's seal lock with [`try_lock_ballot_box`], which waits for no write
//! and makes new ones wait, reads the box with [`box_ballots`], appends the seal's
//! record with `append_in` and stores the seal with [`insert_seal`].

use super::ballot_box::{canonical_uuid, BallotStatus};
use super::postgres::{decode, qualified_columns, PostgresStore};
use crate::domain::{ElectoralLogMessage, StoredRecord};
use crate::messages::statement::StatementType;
use anyhow::{anyhow, Context, Result};
use std::fmt;
use strum_macros::{Display, EnumString};
use tokio_postgres::error::SqlState;
use tokio_postgres::{IsolationLevel, Row, Transaction};
use uuid::Uuid;

/// Message of the error the guard raises for a write to a sealed box.
const SEALED: &str = "ballot_box_sealed";
/// Rows of a box read at a time.
const BALLOTS_PAGE: i32 = 1000;

/// One election's area of an election event: the ballots of a ballot box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BallotBoxId<'a> {
    pub election_event_id: &'a str,
    pub election_id: &'a str,
    pub area_id: &'a str,
}

/// How a seal ended. A failed seal locks its box too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum SealStatus {
    Sealed,
    Failed,
}

/// What a seal stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealOutcome {
    Sealed {
        /// SHA-512 of the manifest.
        seal_hash: Vec<u8>,
        /// The manifest's Borsh bytes.
        manifest: Vec<u8>,
        ballots_in_box: i64,
        ballots_counted: i64,
    },
    Failed {
        reason: String,
    },
}

impl SealOutcome {
    pub fn status(&self) -> SealStatus {
        match self {
            SealOutcome::Sealed { .. } => SealStatus::Sealed,
            SealOutcome::Failed { .. } => SealStatus::Failed,
        }
    }
}

/// A seal to store, with the record that `append_in` stored for it in the same
/// transaction.
#[derive(Debug, Clone)]
pub struct NewSeal<'a> {
    pub ballot_box: BallotBoxId<'a>,
    pub outcome: &'a SealOutcome,
    /// The signed `BallotBoxSealed` or `BallotBoxSealFailed` message, as appended.
    pub signed_message: &'a [u8],
    pub record: &'a StoredRecord,
}

/// A stored seal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSeal {
    pub election_id: String,
    pub area_id: String,
    pub outcome: SealOutcome,
    pub signed_message: Vec<u8>,
    pub log_delivery_id: String,
    pub log_uid: Uuid,
    pub log_leaf_index: i64,
    /// Microseconds since the epoch.
    pub created_at: i64,
}

/// A ballot of a box, as the sealer and the tally read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxBallot {
    pub seq: i64,
    /// The stored ballot's id (`ballot_box_ballot.id`).
    pub id: String,
    pub voter_id: String,
    pub status: BallotStatus,
    pub format: String,
    pub content: String,
    pub ballot_id: String,
    pub voting_channel: String,
}

/// What the tally reads of a sealed box, in one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedBoxRead {
    pub seal: Option<StoredSeal>,
    /// The box's `BallotBoxSealed` records on the board, in log order.
    pub seal_records: Vec<ElectoralLogMessage>,
}

/// A write refused because its ballot box is sealed. Retrying it cannot succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BallotBoxSealedError {
    /// Which box, as the database says.
    pub detail: String,
}

impl fmt::Display for BallotBoxSealedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "The ballot box is sealed: {}", self.detail)
    }
}

impl std::error::Error for BallotBoxSealedError {}

/// Whether the database refused a write because its ballot box is sealed.
pub fn is_ballot_box_sealed(error: &tokio_postgres::Error) -> bool {
    error
        .as_db_error()
        .is_some_and(|db| db.code() == &SqlState::INSUFFICIENT_PRIVILEGE && db.message() == SEALED)
}

/// Whether an error, or one of its causes, is a write refused by a seal.
pub fn is_sealed_refusal(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.downcast_ref::<BallotBoxSealedError>().is_some())
}

/// `error` as a [`BallotBoxSealedError`] when a seal refused the write, and with
/// `context` otherwise.
pub(crate) fn sealed_or(error: tokio_postgres::Error, context: &'static str) -> anyhow::Error {
    if is_ballot_box_sealed(&error) {
        let detail = error
            .as_db_error()
            .and_then(|db| db.detail())
            .unwrap_or_default()
            .to_string();
        anyhow::Error::new(BallotBoxSealedError { detail })
    } else {
        anyhow::Error::new(error).context(context)
    }
}

const SEAL_COLUMNS: &str = "election_id::text, area_id::text, status, seal_hash, manifest, \
     ballots_in_box, ballots_counted, failure_reason, signed_message, log_delivery_id, \
     log_uid, log_leaf_index, (extract(epoch FROM created_at) * 1000000)::bigint";

const BOX: &str =
    "election_event_id = $1::text::uuid AND election_id = $2::text::uuid AND area_id = $3::text::uuid";

fn stored_seal(row: &Row) -> Result<StoredSeal> {
    let status: String = row.try_get(2)?;
    let outcome = match status
        .parse::<SealStatus>()
        .map_err(|_| anyhow!("Unknown seal status {status:?}"))?
    {
        SealStatus::Sealed => SealOutcome::Sealed {
            seal_hash: row.try_get(3)?,
            manifest: row.try_get(4)?,
            ballots_in_box: row.try_get(5)?,
            ballots_counted: row.try_get(6)?,
        },
        SealStatus::Failed => SealOutcome::Failed {
            reason: row.try_get(7)?,
        },
    };
    Ok(StoredSeal {
        election_id: row.try_get(0)?,
        area_id: row.try_get(1)?,
        outcome,
        signed_message: row.try_get(8)?,
        log_delivery_id: row.try_get(9)?,
        log_uid: row.try_get(10)?,
        log_leaf_index: row.try_get(11)?,
        created_at: row.try_get(12)?,
    })
}

fn box_parameters<'a>(
    ballot_box: &'a BallotBoxId<'_>,
) -> [&'a (dyn tokio_postgres::types::ToSql + Sync); 3] {
    [
        &ballot_box.election_event_id,
        &ballot_box.election_id,
        &ballot_box.area_id,
    ]
}

fn box_ballot(row: &Row) -> Result<BoxBallot> {
    let status: String = row.try_get(2)?;
    Ok(BoxBallot {
        seq: row.try_get(0)?,
        voter_id: row.try_get(1)?,
        status: status
            .parse()
            .map_err(|_| anyhow!("Unknown ballot status {status:?}"))?,
        format: row.try_get(3)?,
        content: row.try_get(4)?,
        ballot_id: row.try_get(5)?,
        voting_channel: row.try_get(6)?,
        id: row.try_get(7)?,
    })
}

/// Take the seal lock of a ballot box exclusively, until the transaction ends, unless
/// a write of the box holds it. While it is held, writes of the box wait and then are
/// checked against the box's seal. Its key is `ballot-box-seal:<event>:<election>:<area>`.
pub async fn try_lock_ballot_box(
    tx: &Transaction<'_>,
    ballot_box: BallotBoxId<'_>,
) -> Result<bool> {
    Ok(tx
        .query_one(
            "SELECT pg_try_advisory_xact_lock(hashtextextended(ballot_box_seal_lock_key(\
             $1::text::uuid, $2::text::uuid, $3::text::uuid), 0))",
            &box_parameters(&ballot_box),
        )
        .await
        .context("Error locking the ballot box")?
        .try_get(0)?)
}

/// Every ballot of a box, in any status, in acceptance order, one at a time.
pub async fn box_ballots<F>(
    tx: &Transaction<'_>,
    ballot_box: BallotBoxId<'_>,
    mut each: F,
) -> Result<()>
where
    F: FnMut(BoxBallot) -> Result<()> + Send,
{
    let portal = tx
        .bind(
            &format!(
                "SELECT seq, voter_id, status, format, content, ballot_id, voting_channel, \
                 id::text \
                 FROM ballot_box_ballot WHERE {BOX} ORDER BY seq"
            ),
            &box_parameters(&ballot_box),
        )
        .await
        .context("Error reading the ballots of the box")?;
    loop {
        let rows = tx
            .query_portal(&portal, BALLOTS_PAGE)
            .await
            .context("Error reading the ballots of the box")?;
        for row in &rows {
            each(box_ballot(row)?)?;
        }
        if rows.len() < BALLOTS_PAGE as usize {
            return Ok(());
        }
    }
}

/// Store a seal. Returns false, storing nothing, when the box already has one: the
/// caller then rolls its transaction back.
pub async fn insert_seal(tx: &Transaction<'_>, seal: &NewSeal<'_>) -> Result<bool> {
    let (seal_hash, manifest, ballots_in_box, ballots_counted, failure_reason) = match seal.outcome
    {
        SealOutcome::Sealed {
            seal_hash,
            manifest,
            ballots_in_box,
            ballots_counted,
        } => (
            Some(seal_hash.as_slice()),
            Some(manifest.as_slice()),
            Some(*ballots_in_box),
            Some(*ballots_counted),
            None,
        ),
        SealOutcome::Failed { reason } => (None, None, None, None, Some(reason.as_str())),
    };
    let status = seal.outcome.status().to_string();
    let inserted = tx
        .execute(
            "INSERT INTO ballot_box_seal (election_event_id, election_id, area_id, status, \
             seal_hash, manifest, ballots_in_box, ballots_counted, failure_reason, \
             signed_message, log_delivery_id, log_uid, log_leaf_index) \
             VALUES ($1::text::uuid, $2::text::uuid, $3::text::uuid, $4, $5, $6, $7, $8, $9, \
             $10, $11, $12, $13) \
             ON CONFLICT (election_event_id, election_id, area_id) DO NOTHING",
            &[
                &seal.ballot_box.election_event_id,
                &seal.ballot_box.election_id,
                &seal.ballot_box.area_id,
                &status,
                &seal_hash,
                &manifest,
                &ballots_in_box,
                &ballots_counted,
                &failure_reason,
                &seal.signed_message,
                &seal.record.delivery_id,
                &seal.record.log_uid,
                &seal.record.leaf_index,
            ],
        )
        .await
        .context("Error storing the ballot box seal")?;
    Ok(inserted == 1)
}

/// The seal of a box, read in the caller's transaction.
pub async fn ballot_box_seal_in(
    tx: &Transaction<'_>,
    ballot_box: BallotBoxId<'_>,
) -> Result<Option<StoredSeal>> {
    tx.query_opt(
        &format!("SELECT {SEAL_COLUMNS} FROM ballot_box_seal WHERE {BOX}"),
        &box_parameters(&ballot_box),
    )
    .await
    .context("Error reading the ballot box seal")?
    .as_ref()
    .map(stored_seal)
    .transpose()
}

impl PostgresStore {
    /// The seal of a box, if it has one.
    pub async fn ballot_box_seal(&self, ballot_box: BallotBoxId<'_>) -> Result<Option<StoredSeal>> {
        self.client()
            .await?
            .query_opt(
                &format!("SELECT {SEAL_COLUMNS} FROM ballot_box_seal WHERE {BOX}"),
                &box_parameters(&ballot_box),
            )
            .await
            .context("Error reading the ballot box seal")?
            .as_ref()
            .map(stored_seal)
            .transpose()
    }

    /// The seals of an election event's boxes, by election and area.
    pub async fn ballot_box_seals(&self, election_event_id: &str) -> Result<Vec<StoredSeal>> {
        self.client()
            .await?
            .query(
                &format!(
                    "SELECT {SEAL_COLUMNS} FROM ballot_box_seal \
                     WHERE election_event_id = $1::text::uuid ORDER BY election_id, area_id"
                ),
                &[&election_event_id],
            )
            .await
            .context("Error reading the ballot box seals")?
            .iter()
            .map(stored_seal)
            .collect()
    }

    /// Whether any box of an election event has a seal.
    pub async fn has_ballot_box_seals(&self, election_event_id: &str) -> Result<bool> {
        Ok(self
            .client()
            .await?
            .query_one(
                "SELECT EXISTS (SELECT 1 FROM ballot_box_seal \
                 WHERE election_event_id = $1::text::uuid)",
                &[&election_event_id],
            )
            .await
            .context("Error looking for ballot box seals")?
            .try_get(0)?)
    }

    /// The sealed boxes, as `(election, area)`, that hold a pending or valid ballot of
    /// the voter: changing those ballots is refused.
    pub async fn voter_sealed_boxes(
        &self,
        election_event_id: &str,
        voter_id: &str,
    ) -> Result<Vec<(String, String)>> {
        let (pending, valid) = (
            BallotStatus::Pending.to_string(),
            BallotStatus::Valid.to_string(),
        );
        self.client()
            .await?
            .query(
                "SELECT DISTINCT b.election_id::text, b.area_id::text FROM ballot_box_ballot b \
                 JOIN ballot_box_seal s ON s.election_event_id = b.election_event_id \
                   AND s.election_id = b.election_id AND s.area_id = b.area_id \
                 WHERE b.election_event_id = $1::text::uuid AND b.voter_id = $2 \
                   AND b.status IN ($3, $4) \
                 ORDER BY 1, 2",
                &[&election_event_id, &voter_id, &pending, &valid],
            )
            .await
            .context("Error listing the voter's sealed ballot boxes")?
            .iter()
            .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
            .collect()
    }

    /// The areas with ballots, in any status, of an election.
    pub async fn ballot_box_areas(
        &self,
        election_event_id: &str,
        election_id: &str,
    ) -> Result<Vec<String>> {
        self.client()
            .await?
            .query(
                "SELECT DISTINCT area_id::text FROM ballot_box_ballot \
                 WHERE election_event_id = $1::text::uuid AND election_id = $2::text::uuid \
                 ORDER BY 1",
                &[&election_event_id, &election_id],
            )
            .await
            .context("Error listing the areas with ballots")?
            .iter()
            .map(|row| Ok(row.try_get(0)?))
            .collect()
    }

    /// Read, in one snapshot, a box's seal, its `BallotBoxSealed` records on `board`
    /// and, when it is sealed, each of its ballots, in any status.
    pub async fn read_sealed_box<F>(
        &self,
        board: &str,
        ballot_box: BallotBoxId<'_>,
        each: F,
    ) -> Result<SealedBoxRead>
    where
        F: FnMut(BoxBallot) -> Result<()> + Send,
    {
        let election = canonical_uuid(ballot_box.election_id)?;
        let area = canonical_uuid(ballot_box.area_id)?;
        let mut client = self.client().await?;
        let tx = client
            .build_transaction()
            .isolation_level(IsolationLevel::RepeatableRead)
            .read_only(true)
            .start()
            .await?;
        let seal = ballot_box_seal_in(&tx, ballot_box).await?;
        let seal_records = tx
            .query(
                &format!(
                    "SELECT {} FROM electoral_log_messages m \
                     WHERE m.board_name = $1 AND m.statement_kind = $2 \
                       AND m.election_id = $3 AND m.area_id = $4 \
                     ORDER BY m.id",
                    qualified_columns("m")
                ),
                &[
                    &board,
                    &StatementType::BallotBoxSealed.to_string(),
                    &election,
                    &area,
                ],
            )
            .await
            .context("Error reading the seal records of the box")?
            .into_iter()
            .map(decode)
            .collect::<Result<_>>()?;
        if matches!(
            seal,
            Some(StoredSeal {
                outcome: SealOutcome::Sealed { .. },
                ..
            })
        ) {
            box_ballots(&tx, ballot_box, each).await?;
        }
        tx.commit().await?;
        Ok(SealedBoxRead { seal, seal_records })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_have_stable_names() {
        for (status, name) in [
            (SealStatus::Sealed, "sealed"),
            (SealStatus::Failed, "failed"),
        ] {
            assert_eq!(status.to_string(), name);
            assert_eq!(name.parse::<SealStatus>().unwrap(), status);
        }
        assert!("pending".parse::<SealStatus>().is_err());
        assert_eq!(
            SealOutcome::Failed {
                reason: "reason".into()
            }
            .status(),
            SealStatus::Failed
        );
    }

    #[test]
    fn a_sealed_refusal_is_found_under_its_context() {
        let refusal = anyhow::Error::new(BallotBoxSealedError {
            detail: "The ballot box of election e, area a is sealed.".into(),
        })
        .context("Error changing a ballot's status");
        assert!(is_sealed_refusal(&refusal));
        assert!(!is_sealed_refusal(&anyhow!("Another error")));
        assert_eq!(
            BallotBoxSealedError {
                detail: "detail".into()
            }
            .to_string(),
            "The ballot box is sealed: detail"
        );
    }

    #[test]
    fn the_schema_names_the_errors_and_the_lock_key_as_the_code_does() {
        let schema = include_str!("../../schema.sql");
        assert!(schema.contains(&format!("RAISE EXCEPTION '{SEALED}'")));
        assert!(schema.contains(
            "SELECT 'ballot-box-seal:' || event::text || ':' || election::text || ':' || area::text;"
        ));
    }
}
