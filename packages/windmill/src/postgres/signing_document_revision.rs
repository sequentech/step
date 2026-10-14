// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The PAdES revisions of a signing request's PDF (design §6): the base
//! document, a revision prepared for one signer, and the signed ones.
//!
//! Revisions of a request are numbered by one counter: the base is 0, and
//! every prepared or signed revision takes the next number. The caller
//! holds the request's row lock, so the numbers don't collide. The latest
//! revision is the highest base or signed one: the document as it stands.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use deadpool_postgres::Transaction;
use sequent_core::signing::DocumentRevisionState;
use serde_json::Value;
use std::str::FromStr;
use tokio_postgres::row::Row;
use tracing::instrument;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentRevisionRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub request_id: Uuid,
    pub revision: i32,
    /// The stored document; none for a prepared revision, which is rebuilt.
    pub document_id: Option<Uuid>,
    /// Lowercase hex SHA-256 of the revision's bytes.
    pub sha256: String,
    pub state: DocumentRevisionState,
    /// The signer of a prepared or signed revision.
    pub prepared_for_user: Option<String>,
    /// `[0, a, b, c]` of a prepared or signed revision.
    pub byte_range: Option<Value>,
    /// The signature field (0-based) a prepared or signed revision fills.
    pub field_index: Option<i32>,
    /// SHA-256 of the revision a prepared or signed one was built on.
    pub parent_sha256: Option<String>,
    /// The ByteRange digest a signer was given (the CMS messageDigest).
    pub digest_sha256: Option<String>,
    /// What a prepared revision prints and records, to rebuild it.
    pub appearance: Option<Value>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<Row> for DocumentRevisionRow {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self> {
        let state: String = row.try_get("state")?;
        Ok(DocumentRevisionRow {
            id: row.try_get("id")?,
            tenant_id: row.try_get("tenant_id")?,
            election_event_id: row.try_get("election_event_id")?,
            request_id: row.try_get("request_id")?,
            revision: row.try_get("revision")?,
            document_id: row.try_get("document_id")?,
            sha256: row.try_get("sha256")?,
            state: DocumentRevisionState::from_str(&state)
                .map_err(|err| anyhow!("state: unknown value {state:?}: {err}"))?,
            prepared_for_user: row.try_get("prepared_for_user")?,
            byte_range: row.try_get("byte_range")?,
            field_index: row.try_get("field_index")?,
            parent_sha256: row.try_get("parent_sha256")?,
            digest_sha256: row.try_get("digest_sha256")?,
            appearance: row.try_get("appearance")?,
            created_at: row.try_get("created_at")?,
        })
    }
}

/// A revision to insert; its number is the request's next.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDocumentRevision {
    pub tenant_id: Uuid,
    pub election_event_id: Uuid,
    pub request_id: Uuid,
    pub document_id: Option<Uuid>,
    pub sha256: String,
    pub state: DocumentRevisionState,
    pub prepared_for_user: Option<String>,
    pub byte_range: Option<Value>,
    pub field_index: Option<i32>,
    pub parent_sha256: Option<String>,
    pub digest_sha256: Option<String>,
    pub appearance: Option<Value>,
}

/// Whether a read locks the row it returns until the transaction ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionLock {
    Read,
    ForUpdate,
}

/// Inserts a revision with the request's next number (0 for the first).
/// The caller holds the request's row lock.
#[instrument(skip_all, fields(request_id = %revision.request_id, state = %revision.state), err)]
pub async fn insert_document_revision(
    hasura_transaction: &Transaction<'_>,
    revision: &NewDocumentRevision,
) -> Result<DocumentRevisionRow> {
    hasura_transaction
        .query_one(
            "INSERT INTO sequent_backend.signing_document_revision
                 (tenant_id, election_event_id, request_id, revision, document_id, sha256,
                  state, prepared_for_user, byte_range, field_index, parent_sha256,
                  digest_sha256, appearance)
             SELECT $1, $2, $3,
                    COALESCE((SELECT max(revision) + 1
                              FROM sequent_backend.signing_document_revision
                              WHERE tenant_id = $1 AND election_event_id = $2
                                AND request_id = $3), 0),
                    $4, $5, $6, $7, $8, $9, $10, $11, $12
             RETURNING *",
            &[
                &revision.tenant_id,
                &revision.election_event_id,
                &revision.request_id,
                &revision.document_id,
                &revision.sha256,
                &revision.state.to_string(),
                &revision.prepared_for_user,
                &revision.byte_range,
                &revision.field_index,
                &revision.parent_sha256,
                &revision.digest_sha256,
                &revision.appearance,
            ],
        )
        .await
        .context("Error inserting the document revision")?
        .try_into()
}

/// The document as it stands: the highest base or signed revision.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_latest_document_revision(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    lock: RevisionLock,
) -> Result<Option<DocumentRevisionRow>> {
    let lock = match lock {
        RevisionLock::Read => "",
        RevisionLock::ForUpdate => " FOR UPDATE",
    };
    hasura_transaction
        .query_opt(
            &format!(
                "SELECT * FROM sequent_backend.signing_document_revision
                 WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3
                   AND state IN ('base', 'signed')
                 ORDER BY revision DESC LIMIT 1{lock}"
            ),
            &[&tenant_id, &election_event_id, &request_id],
        )
        .await
        .context("Error reading the latest document revision")?
        .map(DocumentRevisionRow::try_from)
        .transpose()
}

/// The revision `revision` prepared for `user_id`.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_prepared_document_revision(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    revision: i32,
    user_id: &str,
) -> Result<Option<DocumentRevisionRow>> {
    hasura_transaction
        .query_opt(
            "SELECT * FROM sequent_backend.signing_document_revision
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3
               AND revision = $4 AND state = 'prepared' AND prepared_for_user = $5",
            &[
                &tenant_id,
                &election_event_id,
                &request_id,
                &revision,
                &user_id,
            ],
        )
        .await
        .context("Error reading the prepared document revision")?
        .map(DocumentRevisionRow::try_from)
        .transpose()
}

/// Every revision of a request, in order.
#[instrument(skip(hasura_transaction), err)]
pub async fn list_document_revisions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
) -> Result<Vec<DocumentRevisionRow>> {
    hasura_transaction
        .query(
            "SELECT * FROM sequent_backend.signing_document_revision
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3
             ORDER BY revision",
            &[&tenant_id, &election_event_id, &request_id],
        )
        .await
        .context("Error listing the document revisions")?
        .into_iter()
        .map(DocumentRevisionRow::try_from)
        .collect()
}

/// A revision `user_id` prepared on the revision `parent_sha256` for the
/// certificate `certificate_sha256`: the same signer may sign it again.
#[instrument(skip(hasura_transaction), err)]
pub async fn find_reusable_prepared_revision(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    user_id: &str,
    certificate_sha256: &str,
    parent_sha256: &str,
) -> Result<Option<DocumentRevisionRow>> {
    hasura_transaction
        .query_opt(
            "SELECT * FROM sequent_backend.signing_document_revision
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3
               AND state = 'prepared' AND prepared_for_user = $4
               AND appearance->>'certificate_sha256' = $5 AND parent_sha256 = $6
             ORDER BY revision DESC LIMIT 1",
            &[
                &tenant_id,
                &election_event_id,
                &request_id,
                &user_id,
                &certificate_sha256,
                &parent_sha256,
            ],
        )
        .await
        .context("Error reading a reusable prepared revision")?
        .map(DocumentRevisionRow::try_from)
        .transpose()
}

/// How many revisions `user_id` prepared on `parent_sha256` in the last
/// `seconds`, by the database's clock.
#[instrument(skip(hasura_transaction), err)]
pub async fn count_recent_prepared_revisions(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    request_id: Uuid,
    user_id: &str,
    parent_sha256: &str,
    seconds: i64,
) -> Result<i64> {
    Ok(hasura_transaction
        .query_one(
            "SELECT count(*) FROM sequent_backend.signing_document_revision
             WHERE tenant_id = $1 AND election_event_id = $2 AND request_id = $3
               AND state = 'prepared' AND prepared_for_user = $4 AND parent_sha256 = $5
               AND created_at > clock_timestamp() - make_interval(secs => $6::bigint)",
            &[
                &tenant_id,
                &election_event_id,
                &request_id,
                &user_id,
                &parent_sha256,
                &seconds,
            ],
        )
        .await
        .context("Error counting the recent prepared revisions")?
        .try_get(0)?)
}

/// Whether reading the document needs more than reading the request: a
/// password-protected document or a voter-secret export
/// (`document.annotations.access`). Such a document is never a signing
/// document. `false` for an unknown document.
#[instrument(skip(hasura_transaction), err)]
pub async fn signing_document_access_restricted(
    hasura_transaction: &Transaction<'_>,
    tenant_id: Uuid,
    election_event_id: Uuid,
    document_id: Uuid,
) -> Result<bool> {
    Ok(hasura_transaction
        .query_opt(
            "SELECT coalesce((annotations->'access'->>'voter_secret_attributes')::boolean, false)
                    OR (annotations->'access'->>'password_secret_id') IS NOT NULL
             FROM sequent_backend.document
             WHERE tenant_id = $1 AND election_event_id = $2 AND id = $3",
            &[&tenant_id, &election_event_id, &document_id],
        )
        .await
        .context("Error reading the document's access")?
        .map(|row| row.try_get::<_, Option<bool>>(0))
        .transpose()?
        .flatten()
        .unwrap_or(false))
}
