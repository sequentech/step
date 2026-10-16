// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Admin public keys on the tenant's log.
//!
//! An admin's signing key is made in the transaction of the change that first
//! needs it, and its public key is posted to the tenant's log only once that
//! transaction commits: a change that rolls back takes its key with it, and
//! the log never announces a key nobody has. The audit log outbox can't carry
//! the announcement, because it posts `Audit` statements only, and the
//! announcement is an `AdminPublicKey` statement.
//!
//! The post happens once, with a delivery id made from the account and the
//! key. A key whose announcement was lost, as when the process stopped first,
//! or that was made before keys were announced in the tenant's log, is
//! posted by [`backfill_admin_public_keys`], which the tenant log migration
//! runs whenever beat starts.

use crate::services::audit_log_writer::Usernames;
use crate::services::electoral_log::ElectoralLog;
use crate::services::electoral_log_board::{ensure_tenant_log, get_board_client};
use crate::services::vault;
use anyhow::{ensure, Context, Result};
use deadpool_postgres::{Client, Pool};
use electoral_log::messages::message::Message;
use electoral_log::messages::statement::{StatementBody, StatementType};
use electoral_log::{
    Filter, LogQuery, NumberColumn, NumberComparison, OrderColumn, SortDirection, SqlCompOperators,
    TextColumn,
};
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::Instant;
use tracing::{error, info, instrument};
use uuid::Uuid;
use voting_crypto::signing::VerificationKey;
use voting_crypto::RecordDeserialize;

/// How long a key's announcement waits for the transaction that saved the key
/// to end. A longer one leaves the key to the backfill.
const COMMIT_WAIT: Duration = Duration::from_secs(60 * 60);
/// The longest pause between two looks at that transaction.
const MAX_POLL: Duration = Duration::from_secs(5);
/// `txid_status` of a transaction that has not ended.
const IN_PROGRESS: &str = "in progress";
/// Announcements read from a log at a time.
const PAGE: i64 = 1000;

/// An admin's signing key made in a transaction that has not ended, whose
/// public key the tenant's log is to announce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAdminPublicKey {
    pub tenant_id: String,
    pub user_id: String,
    pub username: Option<String>,
    /// The public key, DER in base64.
    pub public_key: String,
    pub elections_ids: Option<String>,
    pub user_area_id: Option<String>,
    /// The transaction that saved the key (`txid_current()`).
    pub transaction_id: i64,
}

impl NewAdminPublicKey {
    /// Posts the public key to the tenant's log, creating the log the first
    /// time. The admin's key must be committed.
    #[instrument(skip(client), err)]
    pub async fn post(&self, client: &mut Client) -> Result<()> {
        let transaction = client
            .transaction()
            .await
            .context("Error starting to announce an admin key")?;
        let tenant_log = ensure_tenant_log(&transaction, &self.tenant_id).await?;
        transaction
            .commit()
            .await
            .context("Error keeping the tenant log's key")?;
        ElectoralLog::post_admin_pk(
            &tenant_log,
            &self.tenant_id,
            &self.user_id,
            self.username.clone(),
            &self.public_key,
            self.elections_ids.clone(),
            self.user_area_id.clone(),
        )
        .await
    }

    /// Whether the key was kept: waits for the transaction that saved it to
    /// end, then compares it with the key the vault has for the admin.
    #[instrument(skip(client), err)]
    pub async fn kept(&self, client: &mut Client) -> Result<bool> {
        let deadline = Instant::now() + COMMIT_WAIT;
        let mut pause = Duration::from_millis(50);
        loop {
            let status: Option<String> = client
                .query_one("SELECT txid_status($1)", &[&self.transaction_id])
                .await
                .context("Error looking at the transaction that saved an admin key")?
                .try_get(0)?;
            if status.as_deref() != Some(IN_PROGRESS) {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "The transaction that saved the key of admin {} is still open",
                self.user_id
            );
            tokio::time::sleep(pause).await;
            pause = (pause * 2).min(MAX_POLL);
        }
        let transaction = client
            .transaction()
            .await
            .context("Error starting to read an admin key")?;
        let kept = vault::read_admin_user_signing_key(&transaction, &self.tenant_id, &self.user_id)
            .await?;
        transaction.commit().await?;
        Ok(match kept {
            Some(key) => VerificationKey::from_sk(&key)?.to_b64_string()? == self.public_key,
            None => false,
        })
    }

    /// Posts the public key once the transaction that saved the key commits,
    /// and nothing when it rolls back. A failure is logged: the backfill posts
    /// the key later.
    pub async fn post_after_commit(self, pool: &Pool) {
        let posted = async {
            let mut client = pool
                .get()
                .await
                .context("Error getting a client to announce an admin key")?;
            if !self.kept(&mut client).await? {
                return Ok(false);
            }
            self.post(&mut client).await?;
            anyhow::Ok(true)
        };
        match posted.await {
            Ok(true) => info!(user_id = self.user_id, "Admin public key posted"),
            Ok(false) => info!(
                user_id = self.user_id,
                "Admin key not kept: its transaction rolled back"
            ),
            Err(error) => error!(
                tenant_id = self.tenant_id,
                user_id = self.user_id,
                "Error posting an admin public key: {error:#}"
            ),
        }
    }
}

/// The accounts and public keys a log announces.
async fn announced_admin_public_keys(board: &str) -> Result<HashSet<(String, String)>> {
    let client = get_board_client().await?;
    let mut announced = HashSet::new();
    let mut last_id = 0;
    loop {
        let query = LogQuery {
            filters: vec![
                Filter::Number(NumberColumn::Id, NumberComparison::GreaterThan, last_id),
                Filter::Text(
                    TextColumn::StatementKind,
                    SqlCompOperators::Equal,
                    StatementType::AdminPublicKey.to_string(),
                ),
            ],
            order: vec![(OrderColumn::Id, SortDirection::Asc)],
            limit: PAGE,
            ..LogQuery::default()
        };
        let entries = client
            .query(board, &query)
            .await
            .with_context(|| format!("Error reading the admin keys of log {board}"))?;
        for entry in &entries {
            last_id = entry.id;
            let Ok(message) = Message::record_deserialize(&entry.message) else {
                continue;
            };
            if let StatementBody::AdminPublicKey(_, user_id, key) = message.statement.body {
                announced.insert((user_id.unwrap_or_default(), key.0));
            }
        }
        if i64::try_from(entries.len())? < PAGE {
            return Ok(announced);
        }
    }
}

/// Posts to the tenant's log the public key of every admin key of the tenant
/// that the log does not announce, as for keys made before admin keys were
/// announced there. Running it again posts nothing. Returns the keys posted.
#[instrument(skip(client, usernames), err)]
pub async fn backfill_admin_public_keys<U: Usernames>(
    client: &mut Client,
    tenant_id: &str,
    usernames: &U,
) -> Result<usize> {
    let tenant = Uuid::parse_str(tenant_id).context("Invalid tenant id")?;
    let transaction = client
        .transaction()
        .await
        .context("Error starting to read the admin keys")?;
    let keys = vault::list_admin_user_signing_keys(&transaction, tenant_id).await?;
    let tenant_log = ensure_tenant_log(&transaction, tenant_id).await?;
    transaction
        .commit()
        .await
        .context("Error ending the read of the admin keys")?;
    if keys.is_empty() {
        return Ok(0);
    }
    let announced = announced_admin_public_keys(&tenant_log.board).await?;
    let mut posted = 0;
    for (user_id, key) in keys {
        let public_key = VerificationKey::from_sk(&key)?.to_b64_string()?;
        if announced.contains(&(user_id.clone(), public_key.clone())) {
            continue;
        }
        let username = usernames.username(tenant, &user_id).await;
        ElectoralLog::post_admin_pk(
            &tenant_log,
            tenant_id,
            &user_id,
            username,
            &public_key,
            None,
            None,
        )
        .await?;
        posted += 1;
    }
    Ok(posted)
}
