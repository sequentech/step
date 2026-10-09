// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Ballot box seals (VOTE-FREEZE), table `sequent_backend.ballot_box_seal`
//! (migration `1791000001600_ballot_box_seal`).
//!
//! A ballot box is the `cast_vote` rows of one tenant, event, election and
//! area. Closing an election whose event seals at close creates one
//! `pending` row per box; the sealer moves it to `sealed` (or `failed`) and
//! the publisher to `published`. Triggers keep the rows permanent and refuse
//! any write to the `cast_vote` rows of a box whose seal isn't `pending`.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use tokio_postgres::Row;
use tracing::instrument;
use uuid::Uuid;

/// The prefix of a ballot box's advisory-lock key; the migration's
/// `ballot_box_lock_key` builds the same text.
const LOCK_KEY_PREFIX: &str = "ballot-box";

/// The `RAISE EXCEPTION` message of the `cast_vote` guard when a write
/// reaches a sealed ballot box.
pub const BALLOT_BOX_SEALED_ERROR: &str = "ballot_box_sealed";

/// The `RAISE EXCEPTION` message of the `cast_vote` guard when a write to a
/// seal-at-close event runs above READ COMMITTED (its snapshot could miss a
/// seal committed while it waited for the box lock).
pub const BALLOT_BOX_SEAL_REQUIRES_READ_COMMITTED_ERROR: &str =
    "ballot_box_seal_requires_read_committed";

/// Where a seal is in its life: `pending → sealed → published`, or
/// `pending → failed`. A `failed` seal keeps its box locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BallotBoxSealStatus {
    Pending,
    Sealed,
    Published,
    Failed,
}

/// A signer of a signed Close voting request, as the seal row records it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosedBySigner {
    pub name: String,
    pub certificate_sha256: String,
}

/// Who or what closed the election (`closed_by` jsonb):
/// `{kind: "user"|"signed"|"scheduled", username?, signers?, signing_code?}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ClosedBy {
    /// A manual Stop Voting.
    User {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        username: Option<String>,
    },
    /// A signed Close voting request. The close hook inserts it bare; the
    /// signed-action sink completes the signers and signing code.
    Signed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signers: Option<Vec<ClosedBySigner>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signing_code: Option<String>,
    },
    /// A scheduled close.
    Scheduled,
}

/// One row of `sequent_backend.ballot_box_seal`.
#[derive(Debug, Clone, PartialEq)]
pub struct BallotBoxSeal {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub area_id: Uuid,
    pub status: BallotBoxSealStatus,
    pub closed_at: DateTime<Utc>,
    pub grace_deadline: DateTime<Utc>,
    pub close_request_id: Option<Uuid>,
    pub closed_by: ClosedBy,
    pub sealed_at: Option<DateTime<Utc>>,
    pub ballots_in_box: Option<i64>,
    pub ballots_counted: Option<i64>,
    /// Hex SHA-512 of the manifest.
    pub seal_hash: Option<String>,
    /// The manifest's exact Borsh bytes.
    pub manifest: Option<Vec<u8>>,
    /// The exact Borsh `Message`, as the electoral log stores it.
    pub signed_message: Option<Vec<u8>>,
    pub failure_reason: Option<String>,
    pub log_entry_id: Option<i64>,
    pub public_document_id: Option<Uuid>,
    pub public_path: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /// When the sealer last tried a pending seal, and why the box isn't
    /// sealed yet ([`WaitingReason`] as text).
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub waiting_reason: Option<String>,
    /// When a failed seal's `BallotBoxSealFailed` entry was posted.
    pub failure_posted_at: Option<DateTime<Utc>>,
    /// The election and area names the seal signed (unset on seals made
    /// before they were kept).
    pub election_name: Option<String>,
    pub area_name: Option<String>,
}

impl BallotBoxSeal {
    /// The advisory-lock key of this row's ballot box.
    pub fn lock_key(&self) -> String {
        ballot_box_lock_key(
            &self.tenant_id,
            &self.election_event_id,
            &self.election_id,
            &self.area_id,
        )
    }
}

impl TryFrom<Row> for BallotBoxSeal {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        let status: String = row.try_get("status")?;
        let closed_by: serde_json::Value = row.try_get("closed_by")?;
        Ok(BallotBoxSeal {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            election_id: row.try_get("election_id")?,
            area_id: row.try_get("area_id")?,
            status: BallotBoxSealStatus::from_str(&status)
                .map_err(|_| anyhow!("Unknown ballot box seal status {status}"))?,
            closed_at: row.try_get("closed_at")?,
            grace_deadline: row.try_get("grace_deadline")?,
            close_request_id: row.try_get("close_request_id")?,
            closed_by: serde_json::from_value(closed_by)
                .context("Error reading the closed_by of a ballot box seal")?,
            sealed_at: row.try_get("sealed_at")?,
            ballots_in_box: row.try_get("ballots_in_box")?,
            ballots_counted: row.try_get("ballots_counted")?,
            seal_hash: row.try_get("seal_hash")?,
            manifest: optional_column(&row, "manifest")?,
            signed_message: optional_column(&row, "signed_message")?,
            failure_reason: row.try_get("failure_reason")?,
            log_entry_id: row.try_get("log_entry_id")?,
            public_document_id: row.try_get("public_document_id")?,
            public_path: row.try_get("public_path")?,
            published_at: row.try_get("published_at")?,
            created_at: row.try_get("created_at")?,
            last_attempt_at: row.try_get("last_attempt_at")?,
            waiting_reason: row.try_get("waiting_reason")?,
            failure_posted_at: row.try_get("failure_posted_at")?,
            election_name: row.try_get("election_name")?,
            area_name: row.try_get("area_name")?,
        })
    }
}

/// A `pending` seal the close hook creates for one ballot box.
#[derive(Debug, Clone, PartialEq)]
pub struct NewPendingSeal {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub election_id: Uuid,
    pub area_id: Uuid,
    pub closed_at: DateTime<Utc>,
    pub grace_deadline: DateTime<Utc>,
    pub close_request_id: Option<Uuid>,
    pub closed_by: ClosedBy,
}

/// What sealing a box stores.
#[derive(Debug, Clone, PartialEq)]
pub struct SealedFields {
    pub sealed_at: DateTime<Utc>,
    pub ballots_in_box: i64,
    pub ballots_counted: i64,
    pub seal_hash: String,
    pub manifest: Vec<u8>,
    pub signed_message: Vec<u8>,
}

/// What publishing a seal stores.
#[derive(Debug, Clone, PartialEq)]
pub struct PublishedFields {
    pub log_entry_id: i64,
    pub public_document_id: Uuid,
    pub public_path: String,
    pub published_at: DateTime<Utc>,
}

/// The advisory-lock key of a ballot box, the same text as the SQL
/// `ballot_box_lock_key` the `cast_vote` guard locks (shared):
/// `ballot-box:{tenant}:{event}:{election}:{area}`, lowercase hyphenated
/// UUIDs as PostgreSQL prints them.
pub fn ballot_box_lock_key(
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
) -> String {
    format!(
        "{LOCK_KEY_PREFIX}:{}:{}:{}:{}",
        tenant_id.hyphenated(),
        election_event_id.hyphenated(),
        election_id.hyphenated(),
        area_id.hyphenated()
    )
}

/// Every column but the two large ones (`manifest`, `signed_message`):
/// what the lists read. A row read this way has `None` for those two.
const STATE_COLUMNS: &str = "id, tenant_id, election_event_id, election_id, area_id, status, \
     closed_at, grace_deadline, close_request_id, closed_by, sealed_at, ballots_in_box, \
     ballots_counted, seal_hash, failure_reason, log_entry_id, public_document_id, \
     public_path, published_at, created_at, last_attempt_at, waiting_reason, \
     failure_posted_at, election_name, area_name";

/// Every column: what reading one seal returns.
const SELECT_COLUMNS: &str = "id, tenant_id, election_event_id, election_id, area_id, status, \
     closed_at, grace_deadline, close_request_id, closed_by, sealed_at, ballots_in_box, \
     ballots_counted, seal_hash, failure_reason, log_entry_id, public_document_id, \
     public_path, published_at, created_at, last_attempt_at, waiting_reason, \
     failure_posted_at, election_name, area_name, manifest, signed_message";

/// A column the query may have left out: `None` when it did.
fn optional_column<'a, T: tokio_postgres::types::FromSql<'a>>(
    row: &'a Row,
    name: &str,
) -> Result<Option<T>> {
    if row.columns().iter().any(|column| column.name() == name) {
        Ok(row.try_get::<_, Option<T>>(name)?)
    } else {
        Ok(None)
    }
}

fn rows_to_seals(rows: Vec<Row>) -> Result<Vec<BallotBoxSeal>> {
    rows.into_iter().map(BallotBoxSeal::try_from).collect()
}

/// Inserts one `pending` seal per row, skipping boxes that already have a
/// seal. Returns how many it inserted.
#[instrument(skip_all, err)]
pub async fn insert_pending(transaction: &Transaction<'_>, rows: &[NewPendingSeal]) -> Result<u64> {
    let statement = transaction
        .prepare(
            "INSERT INTO sequent_backend.ballot_box_seal
                 (tenant_id, election_event_id, election_id, area_id, status,
                  closed_at, grace_deadline, close_request_id, closed_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (tenant_id, election_event_id, election_id, area_id) DO NOTHING",
        )
        .await
        .context("Error preparing the pending ballot box seal insert")?;
    let pending = BallotBoxSealStatus::Pending.to_string();
    let mut inserted = 0;
    for row in rows {
        let closed_by = serde_json::to_value(&row.closed_by)?;
        inserted += transaction
            .execute(
                &statement,
                &[
                    &row.tenant_id,
                    &row.election_event_id,
                    &row.election_id,
                    &row.area_id,
                    &pending,
                    &row.closed_at,
                    &row.grace_deadline,
                    &row.close_request_id,
                    &closed_by,
                ],
            )
            .await
            .with_context(|| {
                format!(
                    "Error inserting the pending seal of election {} area {}",
                    row.election_id, row.area_id
                )
            })?;
    }
    Ok(inserted)
}

/// The `pending` seals whose grace deadline has passed at `now`, oldest
/// deadline first.
#[instrument(skip(transaction), err)]
pub async fn list_due_pending(
    transaction: &Transaction<'_>,
    now: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<BallotBoxSeal>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {STATE_COLUMNS} FROM sequent_backend.ballot_box_seal
                 WHERE status = $1 AND grace_deadline <= $2
                 ORDER BY grace_deadline, id
                 LIMIT $3"
            ),
            &[&BallotBoxSealStatus::Pending.to_string(), &now, &limit],
        )
        .await
        .context("Error listing the due pending ballot box seals")?;
    rows_to_seals(rows)
}

/// Takes the ballot box's advisory lock exclusively until the transaction
/// ends. Run it as the first statement of a READ COMMITTED transaction: a
/// cast already holding the shared lock commits first and is sealed; a cast
/// arriving later waits and then sees the seal.
#[instrument(skip(transaction), err)]
pub async fn lock_box(transaction: &Transaction<'_>, key: &str) -> Result<()> {
    transaction
        .execute(
            "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
            &[&key],
        )
        .await
        .with_context(|| format!("Error locking ballot box {key}"))?;
    Ok(())
}

/// The seal `id`, locked FOR UPDATE until the transaction ends.
#[instrument(skip(transaction), err)]
pub async fn get_for_update(
    transaction: &Transaction<'_>,
    id: &Uuid,
) -> Result<Option<BallotBoxSeal>> {
    let row = transaction
        .query_opt(
            &format!(
                "SELECT {SELECT_COLUMNS} FROM sequent_backend.ballot_box_seal
                 WHERE id = $1
                 FOR UPDATE"
            ),
            &[id],
        )
        .await
        .with_context(|| format!("Error reading ballot box seal {id}"))?;
    row.map(BallotBoxSeal::try_from).transpose()
}

/// A seal row locked without waiting.
#[derive(Debug, Clone, PartialEq)]
pub enum TryLocked {
    Locked(BallotBoxSeal),
    /// Another transaction holds the row.
    Busy,
    NotFound,
}

/// The seal `id`, locked FOR UPDATE until the transaction ends, without
/// waiting: `Busy` when another transaction holds it.
#[instrument(skip(transaction), err)]
pub async fn try_lock(transaction: &Transaction<'_>, id: &Uuid) -> Result<TryLocked> {
    let row = transaction
        .query_opt(
            &format!(
                "SELECT {SELECT_COLUMNS} FROM sequent_backend.ballot_box_seal
                 WHERE id = $1
                 FOR UPDATE SKIP LOCKED"
            ),
            &[id],
        )
        .await
        .with_context(|| format!("Error locking ballot box seal {id}"))?;
    if let Some(row) = row {
        return Ok(TryLocked::Locked(BallotBoxSeal::try_from(row)?));
    }
    let exists: bool = transaction
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM sequent_backend.ballot_box_seal WHERE id = $1)",
            &[id],
        )
        .await
        .with_context(|| format!("Error reading ballot box seal {id}"))?
        .try_get(0)?;
    Ok(if exists {
        TryLocked::Busy
    } else {
        TryLocked::NotFound
    })
}

/// The election and area names a seal signed.
#[derive(Debug, Clone, PartialEq)]
pub struct SealedNames {
    pub election_name: String,
    pub area_name: String,
}

/// Moves a `pending` seal to `sealed` with its seal data. Fails if the
/// seal isn't pending.
#[instrument(skip(transaction, fields), err)]
pub async fn mark_sealed(
    transaction: &Transaction<'_>,
    id: &Uuid,
    fields: &SealedFields,
) -> Result<()> {
    seal_row(transaction, id, fields, None).await
}

/// [`mark_sealed`], keeping the names the seal signed for its record.
#[instrument(skip(transaction, fields), err)]
pub async fn mark_sealed_with_names(
    transaction: &Transaction<'_>,
    id: &Uuid,
    fields: &SealedFields,
    names: &SealedNames,
) -> Result<()> {
    seal_row(transaction, id, fields, Some(names)).await
}

async fn seal_row(
    transaction: &Transaction<'_>,
    id: &Uuid,
    fields: &SealedFields,
    names: Option<&SealedNames>,
) -> Result<()> {
    let election_name = names.map(|names| names.election_name.clone());
    let area_name = names.map(|names| names.area_name.clone());
    let updated = transaction
        .execute(
            "UPDATE sequent_backend.ballot_box_seal
             SET status = $2, sealed_at = $3, ballots_in_box = $4, ballots_counted = $5,
                 seal_hash = $6, manifest = $7, signed_message = $8,
                 election_name = $10, area_name = $11
             WHERE id = $1 AND status = $9",
            &[
                id,
                &BallotBoxSealStatus::Sealed.to_string(),
                &fields.sealed_at,
                &fields.ballots_in_box,
                &fields.ballots_counted,
                &fields.seal_hash,
                &fields.manifest,
                &fields.signed_message,
                &BallotBoxSealStatus::Pending.to_string(),
                &election_name,
                &area_name,
            ],
        )
        .await
        .with_context(|| format!("Error sealing ballot box seal {id}"))?;
    if updated != 1 {
        return Err(anyhow!("Ballot box seal {id} is no longer pending"));
    }
    Ok(())
}

/// Moves a `pending` seal to `failed` with its reason. The box stays
/// locked. Fails if the seal isn't pending.
#[instrument(skip(transaction), err)]
pub async fn mark_failed(transaction: &Transaction<'_>, id: &Uuid, reason: &str) -> Result<()> {
    fail_row(transaction, id, reason, None).await
}

/// [`mark_failed`], keeping the election and area names for the incident.
#[instrument(skip(transaction, names), err)]
pub async fn mark_failed_with_names(
    transaction: &Transaction<'_>,
    id: &Uuid,
    reason: &str,
    names: &SealedNames,
) -> Result<()> {
    fail_row(transaction, id, reason, Some(names)).await
}

async fn fail_row(
    transaction: &Transaction<'_>,
    id: &Uuid,
    reason: &str,
    names: Option<&SealedNames>,
) -> Result<()> {
    let election_name = names.map(|names| names.election_name.clone());
    let area_name = names.map(|names| names.area_name.clone());
    let updated = transaction
        .execute(
            "UPDATE sequent_backend.ballot_box_seal
             SET status = $2, failure_reason = $3, election_name = $5, area_name = $6
             WHERE id = $1 AND status = $4",
            &[
                id,
                &BallotBoxSealStatus::Failed.to_string(),
                &reason,
                &BallotBoxSealStatus::Pending.to_string(),
                &election_name,
                &area_name,
            ],
        )
        .await
        .with_context(|| format!("Error failing ballot box seal {id}"))?;
    if updated != 1 {
        return Err(anyhow!("Ballot box seal {id} is no longer pending"));
    }
    Ok(())
}

/// The `sealed` seals still to publish, oldest seal first.
#[instrument(skip(transaction), err)]
pub async fn list_sealed_unpublished(
    transaction: &Transaction<'_>,
    limit: i64,
) -> Result<Vec<BallotBoxSeal>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {STATE_COLUMNS} FROM sequent_backend.ballot_box_seal
                 WHERE status = $1
                 ORDER BY sealed_at, id
                 LIMIT $2"
            ),
            &[&BallotBoxSealStatus::Sealed.to_string(), &limit],
        )
        .await
        .context("Error listing the unpublished ballot box seals")?;
    rows_to_seals(rows)
}

/// Moves a `sealed` seal to `published` with its log entry and public
/// record. Fails if the seal isn't sealed.
#[instrument(skip(transaction), err)]
pub async fn mark_published(
    transaction: &Transaction<'_>,
    id: &Uuid,
    fields: &PublishedFields,
) -> Result<()> {
    let updated = transaction
        .execute(
            "UPDATE sequent_backend.ballot_box_seal
             SET status = $2, log_entry_id = $3, public_document_id = $4,
                 public_path = $5, published_at = $6
             WHERE id = $1 AND status = $7",
            &[
                id,
                &BallotBoxSealStatus::Published.to_string(),
                &fields.log_entry_id,
                &fields.public_document_id,
                &fields.public_path,
                &fields.published_at,
                &BallotBoxSealStatus::Sealed.to_string(),
            ],
        )
        .await
        .with_context(|| format!("Error publishing ballot box seal {id}"))?;
    if updated != 1 {
        return Err(anyhow!("Ballot box seal {id} is not sealed"));
    }
    Ok(())
}

/// Records the signed close on the election's `pending` seals: the Close
/// voting request and who signed it. Returns how many seals it updated.
#[instrument(skip(transaction, closed_by), err)]
pub async fn set_close_provenance(
    transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    close_request_id: Option<Uuid>,
    closed_by: &ClosedBy,
) -> Result<u64> {
    let closed_by = serde_json::to_value(closed_by)?;
    transaction
        .execute(
            "UPDATE sequent_backend.ballot_box_seal
             SET close_request_id = $4, closed_by = $5
             WHERE tenant_id = $1 AND election_event_id = $2 AND election_id = $3
               AND status = $6",
            &[
                tenant_id,
                election_event_id,
                election_id,
                &close_request_id,
                &closed_by,
                &BallotBoxSealStatus::Pending.to_string(),
            ],
        )
        .await
        .with_context(|| {
            format!("Error recording the close of election {election_id} on its seals")
        })
}

/// Why the sealer left a pending seal pending on its last attempt; stored
/// as text in `waiting_reason`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitingReason {
    /// The grace period hasn't ended.
    Deadline,
    /// An enabled voting channel isn't finished.
    ChannelOpen(String),
    /// Datafix votes of the box are still in progress.
    DatafixVotes(i64),
    /// Another run holds the box.
    Busy,
    /// The attempt failed; the next one retries.
    Error(String),
}

/// The longest error text a waiting reason keeps.
const WAITING_ERROR_CHARS: usize = 120;

impl std::fmt::Display for WaitingReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WaitingReason::Deadline => write!(f, "deadline"),
            WaitingReason::ChannelOpen(channel) => write!(f, "channel_open:{channel}"),
            WaitingReason::DatafixVotes(count) => write!(f, "datafix_votes:{count}"),
            WaitingReason::Busy => write!(f, "busy"),
            WaitingReason::Error(error) => {
                let short: String = error.chars().take(WAITING_ERROR_CHARS).collect();
                write!(f, "error:{short}")
            }
        }
    }
}

/// Records the sealer's attempt on a pending seal: now, and why the box
/// isn't sealed yet. Does nothing once the seal left `pending`, and doesn't
/// wait when another run holds the row (it skips the bookkeeping).
#[instrument(skip(transaction), err)]
pub async fn record_attempt(
    transaction: &Transaction<'_>,
    id: &Uuid,
    reason: &WaitingReason,
) -> Result<()> {
    transaction
        .execute(
            "UPDATE sequent_backend.ballot_box_seal
             SET last_attempt_at = now(), waiting_reason = $2
             WHERE id IN (
                 SELECT id FROM sequent_backend.ballot_box_seal
                 WHERE id = $1 AND status = 'pending'
                 FOR UPDATE SKIP LOCKED
             )",
            &[id, &reason.to_string()],
        )
        .await
        .with_context(|| format!("Error recording the attempt on ballot box seal {id}"))?;
    Ok(())
}

/// Records that a failed seal's `BallotBoxSealFailed` entry is on the log.
/// Does nothing if it was recorded already.
#[instrument(skip(transaction), err)]
pub async fn mark_failure_posted(transaction: &Transaction<'_>, id: &Uuid) -> Result<()> {
    transaction
        .execute(
            "UPDATE sequent_backend.ballot_box_seal
             SET failure_posted_at = now()
             WHERE id = $1 AND status = 'failed' AND failure_posted_at IS NULL",
            &[id],
        )
        .await
        .with_context(|| format!("Error recording the failure entry of ballot box seal {id}"))?;
    Ok(())
}

/// The full seal (with its manifest and signed message) of one ballot box,
/// if it has one.
#[instrument(skip(transaction), err)]
pub async fn get_for_box(
    transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
    area_id: &Uuid,
) -> Result<Option<BallotBoxSeal>> {
    let row = transaction
        .query_opt(
            &format!(
                "SELECT {SELECT_COLUMNS} FROM sequent_backend.ballot_box_seal
                 WHERE tenant_id = $1 AND election_event_id = $2
                   AND election_id = $3 AND area_id = $4"
            ),
            &[tenant_id, election_event_id, election_id, area_id],
        )
        .await
        .with_context(|| {
            format!("Error reading the seal of election {election_id} area {area_id}")
        })?;
    row.map(BallotBoxSeal::try_from).transpose()
}

/// The ids of the `pending` seals due at `now`, of the `sealed` ones to
/// publish, and of the `failed` ones whose failure entry may not be posted
/// yet: what the dispatcher enqueues. At most `limit` of each: pending ones
/// least recently tried first, the others in random order, so rows that
/// keep failing don't starve the rest.
#[instrument(skip(transaction), err)]
pub async fn list_open_work_ids(
    transaction: &Transaction<'_>,
    now: DateTime<Utc>,
    limit: i64,
) -> Result<Vec<Uuid>> {
    let rows = transaction
        .query(
            "(SELECT id FROM sequent_backend.ballot_box_seal
               WHERE status = 'pending' AND grace_deadline <= $1
               ORDER BY last_attempt_at NULLS FIRST, grace_deadline, id LIMIT $2)
             UNION ALL
             (SELECT id FROM sequent_backend.ballot_box_seal
               WHERE status = 'sealed' ORDER BY random() LIMIT $2)
             UNION ALL
             (SELECT id FROM sequent_backend.ballot_box_seal
               WHERE status = 'failed' AND failure_posted_at IS NULL
               ORDER BY random() LIMIT $2)",
            &[&now, &limit],
        )
        .await
        .context("Error listing the ballot box seals to work on")?;
    rows.into_iter().map(|row| Ok(row.try_get(0)?)).collect()
}

/// The `pending` seals created since `since` that the sealer hasn't tried
/// yet, with their deadlines: what a close schedules at their deadline.
#[instrument(skip(transaction), err)]
pub async fn list_new_pending(
    transaction: &Transaction<'_>,
    since: DateTime<Utc>,
) -> Result<Vec<(Uuid, DateTime<Utc>)>> {
    let rows = transaction
        .query(
            "SELECT id, grace_deadline FROM sequent_backend.ballot_box_seal
             WHERE status = 'pending' AND last_attempt_at IS NULL AND created_at >= $1
             ORDER BY grace_deadline, id",
            &[&since],
        )
        .await
        .context("Error listing the new pending ballot box seals")?;
    rows.into_iter()
        .map(|row| Ok((row.try_get(0)?, row.try_get(1)?)))
        .collect()
}

/// Whether the election has any ballot box seal, in any status.
#[instrument(skip(transaction), err)]
pub async fn any_for_election(
    transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_id: &Uuid,
) -> Result<bool> {
    let row = transaction
        .query_one(
            "SELECT EXISTS (
                 SELECT 1 FROM sequent_backend.ballot_box_seal
                 WHERE tenant_id = $1 AND election_event_id = $2 AND election_id = $3
             )",
            &[tenant_id, election_event_id, election_id],
        )
        .await
        .context("Error checking the ballot box seals of the election")?;
    Ok(row.try_get(0)?)
}

/// Every seal of the given elections of an event, in any status, ordered by
/// election and area, without the large columns (`manifest` and
/// `signed_message` are `None`; [`get_for_box`] reads them).
#[instrument(skip(transaction), err)]
pub async fn list_for_elections(
    transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
    election_ids: &[Uuid],
) -> Result<Vec<BallotBoxSeal>> {
    let rows = transaction
        .query(
            &format!(
                "SELECT {STATE_COLUMNS} FROM sequent_backend.ballot_box_seal
                 WHERE tenant_id = $1 AND election_event_id = $2
                   AND election_id = ANY($3)
                 ORDER BY election_id, area_id"
            ),
            &[tenant_id, election_event_id, &election_ids],
        )
        .await
        .context("Error listing the ballot box seals of the elections")?;
    rows_to_seals(rows)
}

/// Whether the event has any ballot box seal, in any status. Seals are
/// permanent, so an event with one can't be deleted.
#[instrument(skip(transaction), err)]
pub async fn any_for_event(
    transaction: &Transaction<'_>,
    tenant_id: &Uuid,
    election_event_id: &Uuid,
) -> Result<bool> {
    let row = transaction
        .query_one(
            "SELECT EXISTS (
                 SELECT 1 FROM sequent_backend.ballot_box_seal
                 WHERE tenant_id = $1 AND election_event_id = $2
             )",
            &[tenant_id, election_event_id],
        )
        .await
        .context("Error checking the ballot box seals of the event")?;
    Ok(row.try_get(0)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lock_key_matches_the_sql_text() {
        let tenant = Uuid::parse_str("90505c8a-23a9-4cdf-a26b-4e19f6a097d5").unwrap();
        let event = Uuid::parse_str("A0000000-0000-0000-0000-00000000000B").unwrap();
        let election = Uuid::nil();
        let area = Uuid::max();
        assert_eq!(
            ballot_box_lock_key(&tenant, &event, &election, &area),
            "ballot-box:90505c8a-23a9-4cdf-a26b-4e19f6a097d5:a0000000-0000-0000-0000-00000000000b:\
             00000000-0000-0000-0000-000000000000:ffffffff-ffff-ffff-ffff-ffffffffffff"
        );
    }

    #[test]
    fn closed_by_matches_the_column_json() {
        assert_eq!(
            serde_json::to_value(ClosedBy::User {
                username: Some("admin".into())
            })
            .unwrap(),
            json!({"kind": "user", "username": "admin"})
        );
        assert_eq!(
            serde_json::to_value(ClosedBy::Scheduled).unwrap(),
            json!({"kind": "scheduled"})
        );
        assert_eq!(
            serde_json::from_value::<ClosedBy>(json!({"kind": "signed"})).unwrap(),
            ClosedBy::Signed {
                signers: None,
                signing_code: None
            }
        );
        let signed = ClosedBy::Signed {
            signers: Some(vec![ClosedBySigner {
                name: "Chair".into(),
                certificate_sha256: "ab".into(),
            }]),
            signing_code: Some("CODE".into()),
        };
        assert_eq!(
            serde_json::to_value(&signed).unwrap(),
            json!({"kind": "signed", "signers": [{"name": "Chair", "certificate_sha256": "ab"}], "signing_code": "CODE"})
        );
    }

    #[test]
    fn status_text_matches_the_check_constraint() {
        for (status, text) in [
            (BallotBoxSealStatus::Pending, "pending"),
            (BallotBoxSealStatus::Sealed, "sealed"),
            (BallotBoxSealStatus::Published, "published"),
            (BallotBoxSealStatus::Failed, "failed"),
        ] {
            assert_eq!(status.to_string(), text);
            assert_eq!(BallotBoxSealStatus::from_str(text).unwrap(), status);
        }
    }
}
