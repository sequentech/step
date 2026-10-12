// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
//! Signed electoral-log checkpoints, stored outside the electoral-log database.
use anyhow::{Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

/// A published checkpoint of an election event's electoral log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedCheckpoint {
    pub board_name: String,
    /// Identity of the Trellis log.
    pub log_uid: Uuid,
    pub tree_size: i64,
    /// Hex-encoded SHA-256 root.
    pub root: String,
    /// Stored name of an `ElectoralLogCheckpointReason`; parsed when audited.
    pub reason: String,
    /// DER/base64 public key of the signer.
    pub signer_pk: String,
    /// Base64 signature over the checkpoint signing bytes.
    pub signature: String,
}

/// The publication stored for a log size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCheckpoint {
    pub reason: String,
    /// Hex-encoded SHA-256 root.
    pub root: String,
}

/// Record a checkpoint unless one of the same log size was already published, and
/// return the publication stored for that size.
#[instrument(skip(hasura_transaction), err)]
pub async fn insert_electoral_log_checkpoint(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    checkpoint: &PublishedCheckpoint,
) -> Result<StoredCheckpoint> {
    let tenant = parse_uuid_v4(tenant_id)?;
    let event = parse_uuid_v4(election_event_id)?;
    let inserted = hasura_transaction
        .query_opt(
            r#"
            INSERT INTO sequent_backend.electoral_log_checkpoint
                (tenant_id, election_event_id, board_name, log_uid, tree_size, root, reason,
                 signer_pk, signature)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (tenant_id, election_event_id, log_uid, tree_size) DO NOTHING
            RETURNING reason, root
            "#,
            &[
                &tenant,
                &event,
                &checkpoint.board_name,
                &checkpoint.log_uid,
                &checkpoint.tree_size,
                &checkpoint.root,
                &checkpoint.reason,
                &checkpoint.signer_pk,
                &checkpoint.signature,
            ],
        )
        .await
        .context("Error inserting electoral-log checkpoint")?;
    // On a conflict, read the existing row in a new statement, which also sees a row
    // committed concurrently.
    let row = match inserted {
        Some(row) => row,
        None => hasura_transaction
            .query_one(
                r#"
                SELECT reason, root FROM sequent_backend.electoral_log_checkpoint
                WHERE tenant_id = $1 AND election_event_id = $2 AND log_uid = $3
                    AND tree_size = $4
                "#,
                &[&tenant, &event, &checkpoint.log_uid, &checkpoint.tree_size],
            )
            .await
            .context("Error reading the published electoral-log checkpoint")?,
    };
    Ok(StoredCheckpoint {
        reason: row.try_get("reason")?,
        root: row.try_get("root")?,
    })
}

/// All checkpoints published for an election event, oldest first.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_electoral_log_checkpoints(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Vec<PublishedCheckpoint>> {
    let rows = hasura_transaction
        .query(
            r#"
            SELECT board_name, log_uid, tree_size, root, reason, signer_pk, signature
            FROM sequent_backend.electoral_log_checkpoint
            WHERE tenant_id = $1 AND election_event_id = $2
            ORDER BY tree_size, created_at
            "#,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
            ],
        )
        .await
        .context("Error reading electoral-log checkpoints")?;
    rows.into_iter()
        .map(|row| {
            Ok(PublishedCheckpoint {
                board_name: row.try_get("board_name")?,
                log_uid: row.try_get("log_uid")?,
                tree_size: row.try_get("tree_size")?,
                root: row.try_get("root")?,
                reason: row.try_get("reason")?,
                signer_pk: row.try_get("signer_pk")?,
                signature: row.try_get("signature")?,
            })
        })
        .collect()
}

/// Checkpoints published of these logs by any election event of any tenant, with the
/// event that published each.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_checkpoints_of_logs(
    hasura_transaction: &Transaction<'_>,
    log_uids: &[Uuid],
) -> Result<Vec<(Uuid, PublishedCheckpoint)>> {
    let rows = hasura_transaction
        .query(
            r#"
            SELECT election_event_id, board_name, log_uid, tree_size, root, reason,
                signer_pk, signature
            FROM sequent_backend.electoral_log_checkpoint
            WHERE log_uid = ANY($1)
            ORDER BY tree_size, created_at
            "#,
            &[&log_uids],
        )
        .await
        .context("Error reading the checkpoints of electoral logs")?;
    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get("election_event_id")?,
                PublishedCheckpoint {
                    board_name: row.try_get("board_name")?,
                    log_uid: row.try_get("log_uid")?,
                    tree_size: row.try_get("tree_size")?,
                    root: row.try_get("root")?,
                    reason: row.try_get("reason")?,
                    signer_pk: row.try_get("signer_pk")?,
                    signature: row.try_get("signature")?,
                },
            ))
        })
        .collect()
}

/// Size of the largest checkpoint published of one of an election event's logs, if any.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_last_published_tree_size(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    log_uid: &Uuid,
) -> Result<Option<i64>> {
    let row = hasura_transaction
        .query_one(
            r#"
            SELECT max(tree_size) AS tree_size
            FROM sequent_backend.electoral_log_checkpoint
            WHERE tenant_id = $1 AND election_event_id = $2 AND log_uid = $3
            "#,
            &[
                &parse_uuid_v4(tenant_id)?,
                &parse_uuid_v4(election_event_id)?,
                log_uid,
            ],
        )
        .await
        .context("Error reading the last published electoral-log checkpoint")?;
    Ok(row.try_get("tree_size")?)
}

/// `(tenant_id, election_event_id)` of every election event with voting open on any
/// channel, for the whole event or for one of its elections.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_election_events_with_open_voting(
    hasura_transaction: &Transaction<'_>,
) -> Result<Vec<(String, String)>> {
    let rows = hasura_transaction
        .query(
            r#"
            SELECT tenant_id::text AS tenant_id, id::text AS election_event_id
            FROM sequent_backend.election_event
            WHERE 'OPEN' IN (
                status->>'voting_status', status->>'kiosk_voting_status',
                status->>'early_voting_status', status->>'telephone_voting_status'
            )
            UNION
            SELECT tenant_id::text, election_event_id::text
            FROM sequent_backend.election
            WHERE 'OPEN' IN (
                status->>'voting_status', status->>'kiosk_voting_status',
                status->>'early_voting_status', status->>'telephone_voting_status'
            )
            ORDER BY 1, 2
            "#,
            &[],
        )
        .await
        .context("Error reading the election events with open voting")?;
    rows.into_iter()
        .map(|row| Ok((row.try_get("tenant_id")?, row.try_get("election_event_id")?)))
        .collect()
}
