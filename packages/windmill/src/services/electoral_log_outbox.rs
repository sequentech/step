// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The transactional outbox of administrator-signed electoral-log entries.
//!
//! An action that must be recorded with its administrator's own signature
//! (a certificate-authority import or deletion) writes its entry, signed by
//! that administrator at that moment, to `sequent_backend.electoral_log_outbox`
//! in the action's own transaction. If the transaction rolls back, the entry
//! does not exist; if it commits, the entry does. Nothing depends on timing.
//!
//! The relay ([`relay_electoral_log_outbox`], run by the audit-log outbox's beat
//! task every few seconds and kicked after such an action) reads committed
//! rows in `seq` order, and for each:
//!
//! 1. makes sure the tenant's log announces the administrator's key: it checks
//!    that the vault holds that very key for that administrator (the key the
//!    action signed with), then posts the announcement (idempotent: one
//!    delivery ID per account and key). The announcement always precedes the
//!    entry, so the batch task never finds the key unannounced for a genuine
//!    entry;
//! 2. writes the entry's waiting row and its reference on the event queue
//!    (task-queue database; nothing is added if it is already there), keeping
//!    the action's time and `seq`, so it is checked and ordered as of the
//!    action;
//! 3. marks the row relayed.
//!
//! Crash points: before 2, the row is relayed again later; between 2 and 3,
//! the next run finds the waiting row and adds nothing (or, if the entry was
//! stored meanwhile, writes it again and the log stores nothing twice: delivery
//! IDs); a row that keeps failing stays, with its error and attempts, and holds
//! the later rows of its scope (per-scope order).
//!
//! Ordering against direct posts: the entry joins its log's order when the
//! relay writes its waiting row (under the scope's lock); from then on direct
//! posts of the scope are refused until it is stored. A direct post made
//! between the action's commit and the relay (a few seconds) is stored before
//! the entry, though the entry's statement carries the action's earlier time.
//! The batch task never reads a vault key: only the relay, on the action's side,
//! checks it, as the audit-log writer does.

use crate::services::electoral_log_pending::PendingStatement;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::{Client, Transaction};
use std::collections::HashSet;
use tracing::{error, warn};
use voting_crypto::signing::VerificationKey;

/// Rows one relay pass moves.
const RELAY_BATCH: i64 = 100;
/// Attempts after which a failing row is reported as an error.
const ALERT_ATTEMPTS: i32 = 5;
/// Alert class of an outbox row that keeps failing to be relayed.
pub const ALERT_OUTBOX_STUCK: &str = "electoral-log-outbox-stuck";

/// Writes an administrator-signed entry to the outbox, in the action's own
/// transaction. Returns its delivery ID.
pub async fn write_outbox_entry(
    hasura_transaction: &Transaction<'_>,
    statement: &PendingStatement,
) -> Result<String> {
    anyhow::ensure!(
        statement.statement.admin().is_some(),
        "Only administrator-signed entries go through the outbox"
    );
    let delivery_id = statement.origin.delivery_id();
    let payload = serde_json::to_value(statement).context("Error serializing an outbox entry")?;
    hasura_transaction
        .execute(
            "INSERT INTO sequent_backend.electoral_log_outbox \
             (delivery_id, tenant_id, election_event_id, payload) \
             VALUES ($1, $2::text::uuid, $3::text::uuid, $4)",
            &[
                &delivery_id,
                &statement.tenant_id,
                &statement.election_event_id,
                &payload,
            ],
        )
        .await
        .context("Error writing the electoral-log outbox entry")?;
    Ok(delivery_id)
}

/// Moves committed outbox rows to the electoral log's waiting table, in order;
/// see the module documentation. Returns how many were relayed.
pub async fn relay_electoral_log_outbox(client: &mut Client) -> Result<usize> {
    let transaction = client
        .transaction()
        .await
        .context("Error starting the outbox relay")?;
    let rows = transaction
        .query(
            "SELECT delivery_id, seq, payload, (extract(epoch FROM created_at) * 1000)::bigint, \
             attempts \
             FROM sequent_backend.electoral_log_outbox WHERE relayed_at IS NULL \
             ORDER BY seq LIMIT $1 FOR UPDATE SKIP LOCKED",
            &[&RELAY_BATCH],
        )
        .await
        .context("Error reading the electoral-log outbox")?;
    let mut relayed = 0;
    let mut held_scopes: HashSet<String> = HashSet::new();
    for row in rows {
        let delivery_id: String = row.get(0);
        let seq: i64 = row.get(1);
        let payload: serde_json::Value = row.get(2);
        let created_ms: i64 = row.get(3);
        let attempts: i32 = row.get(4);
        let scope = format!(
            "{}/{}",
            payload["tenant_id"].as_str().unwrap_or_default(),
            payload["election_event_id"].as_str().unwrap_or_default()
        );
        if held_scopes.contains(&scope) {
            continue;
        }
        let moved = async {
            let statement: PendingStatement =
                serde_json::from_value(payload).context("Not a pending statement")?;
            announce_admin_key(&transaction, &statement).await?;
            statement
                .write_waiting(&delivery_id, Some((created_ms, seq)))
                .await
        }
        .await;
        match moved {
            Ok(()) => {
                transaction
                    .execute(
                        "UPDATE sequent_backend.electoral_log_outbox \
                         SET relayed_at = clock_timestamp() WHERE delivery_id = $1",
                        &[&delivery_id],
                    )
                    .await
                    .context("Error marking an outbox entry relayed")?;
                relayed += 1;
            }
            Err(failure) => {
                // Its scope's later rows wait behind it.
                held_scopes.insert(scope.clone());
                let text = format!("{failure:#}");
                transaction
                    .execute(
                        "UPDATE sequent_backend.electoral_log_outbox \
                         SET attempts = attempts + 1, last_error = $2 WHERE delivery_id = $1",
                        &[&delivery_id, &text],
                    )
                    .await
                    .context("Error recording an outbox entry's failure")?;
                if attempts + 1 >= ALERT_ATTEMPTS {
                    error!(
                        alert = ALERT_OUTBOX_STUCK,
                        delivery_id = delivery_id.as_str(),
                        scope = scope.as_str(),
                        attempts = attempts + 1,
                        "An electoral-log outbox entry keeps failing to be relayed; its log's later entries wait: {text}"
                    );
                } else {
                    warn!(
                        delivery_id = delivery_id.as_str(),
                        "Error relaying an electoral-log outbox entry: {text}"
                    );
                }
            }
        }
    }
    transaction
        .commit()
        .await
        .context("Error committing the outbox relay")?;
    Ok(relayed)
}

/// Makes sure the tenant's log announces the key an administrator-signed entry
/// names, before the entry is relayed: the vault must hold that very key for
/// that administrator (the key the action signed with), and its announcement
/// is posted (idempotently) if it isn't yet.
async fn announce_admin_key(
    hasura_transaction: &Transaction<'_>,
    statement: &PendingStatement,
) -> Result<()> {
    let Some((user_id, public_key)) = statement.statement.admin() else {
        return Ok(());
    };
    let key = crate::services::vault::read_admin_user_signing_key(
        hasura_transaction,
        &statement.tenant_id,
        user_id,
    )
    .await?
    .ok_or_else(|| anyhow!("The vault has no key for administrator {user_id}"))?;
    anyhow::ensure!(
        VerificationKey::from_sk(&key)?.to_b64_string()? == public_key,
        "The entry's key is not administrator {user_id}'s key in the vault: it is not relayed"
    );
    let username = match &statement.statement {
        crate::services::electoral_log_pending::PendingStatementBody::AdminCertificateEvent {
            username,
            ..
        } => username.clone(),
        _ => None,
    };
    let mut client = crate::services::database::get_hasura_pool()
        .await
        .get()
        .await
        .context("Error getting a client to announce an admin key")?;
    crate::services::admin_public_key::NewAdminPublicKey {
        tenant_id: statement.tenant_id.clone(),
        user_id: user_id.to_string(),
        username,
        public_key: public_key.to_string(),
        elections_ids: None,
        user_area_id: None,
        transaction_id: 0,
    }
    .post(&mut client)
    .await
    .context("Error announcing the administrator's key in the tenant log")
}
