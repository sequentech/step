// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One-time secrets for tasks. A password a task needs is not put in the task
//! message, which the task-queue database and the task's records keep: the
//! sender stores it encrypted with the master secret and the message carries
//! only a random reference, which the task redeems once.

use crate::postgres::secret::get_secret_by_key;
use crate::services::database::get_hasura_pool;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Local};
use deadpool_postgres::Transaction;
use tracing::instrument;
use uuid::Uuid;

const SECRET_KEY_PREFIX: &str = "task-secret";

/// An unredeemed secret is useless after this and deleted with the next one
/// stored for the tenant.
const UNREDEEMED_LIFETIME_HOURS: i64 = 24;

pub fn secret_key(reference: &str) -> String {
    format!("{SECRET_KEY_PREFIX}-{reference}")
}

/// Stores `secret` for one task of the tenant and returns its reference.
#[instrument(skip_all, err)]
pub async fn stash(tenant_id: &str, secret: &str) -> Result<String> {
    let mut client = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    delete_expired(&transaction, tenant_id).await?;
    let reference = Uuid::new_v4().to_string();
    vault::save_secret(
        &transaction,
        tenant_id,
        None,
        &secret_key(&reference),
        secret,
    )
    .await
    .context("Failed to store the task secret")?;
    transaction.commit().await?;
    Ok(reference)
}

/// Returns the secret and deletes it, so that it can be redeemed only once.
/// An unknown or expired reference is refused.
#[instrument(skip_all, err)]
pub async fn redeem(tenant_id: &str, reference: &str) -> Result<String> {
    read(tenant_id, reference, true).await
}

/// Returns the secret and keeps it, for a task that is retried after a
/// failure; it deletes the secret with `discard` once it succeeds. An unknown
/// or expired reference is refused, and an expired secret deleted.
#[instrument(skip_all, err)]
pub async fn reveal(tenant_id: &str, reference: &str) -> Result<String> {
    read(tenant_id, reference, false).await
}

async fn read(tenant_id: &str, reference: &str, once: bool) -> Result<String> {
    let key = secret_key(reference);
    let mut client = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    let stored = get_secret_by_key(&transaction, tenant_id, None, &key).await?;
    let fresh = stored
        .as_ref()
        .and_then(|secret| secret.created_at)
        .is_some_and(|created_at| !is_expired(created_at, Local::now()));
    let value = if fresh {
        vault::read_secret(&transaction, tenant_id, None, &key).await?
    } else {
        None
    };
    if stored.is_some() && (once || value.is_none()) {
        vault::delete_secret(&transaction, tenant_id, &key).await?;
    }
    transaction.commit().await?;
    value.ok_or_else(|| anyhow!("The task's secret is missing or has expired"))
}

/// Deletes a secret: its task was not queued, or no longer needs it.
#[instrument(skip_all, err)]
pub async fn discard(tenant_id: &str, reference: &str) -> Result<()> {
    let mut client = get_hasura_pool().await.get().await?;
    let transaction = client.transaction().await?;
    vault::delete_secret(&transaction, tenant_id, &secret_key(reference)).await?;
    transaction.commit().await?;
    Ok(())
}

fn is_expired(created_at: chrono::DateTime<Local>, now: chrono::DateTime<Local>) -> bool {
    now - created_at > Duration::hours(UNREDEEMED_LIFETIME_HOURS)
}

async fn delete_expired(transaction: &Transaction<'_>, tenant_id: &str) -> Result<()> {
    let tenant_uuid = Uuid::parse_str(tenant_id).context("Invalid tenant id")?;
    transaction
        .execute(
            r#"
                DELETE FROM "sequent_backend".secret
                WHERE tenant_id = $1
                  AND key LIKE $2
                  AND created_at < now() - make_interval(hours => $3::int);
            "#,
            &[
                &tenant_uuid,
                &format!("{SECRET_KEY_PREFIX}-%"),
                &(UNREDEEMED_LIFETIME_HOURS as i32),
            ],
        )
        .await
        .context("Failed to delete expired task secrets")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_is_scoped_to_task_secrets() {
        assert_eq!("task-secret-ref", secret_key("ref"));
    }

    #[test]
    fn a_secret_expires_after_its_lifetime() {
        let now = Local::now();
        assert!(!is_expired(now - Duration::hours(23), now));
        assert!(is_expired(now - Duration::hours(25), now));
    }
}
