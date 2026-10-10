// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Resolving attempts whose outcome is unknown. Where the provider can tell,
//! it is asked; otherwise the attempt stays unknown for review. Nothing here
//! sends a message again.

use super::accounts::runtime_account;
use super::dispatch::Dispatcher;
use crate::postgres::messaging::{
    expire_messenger_links, get_messaging_account, list_unresolved_messages, transition_message,
    StateChange,
};
use anyhow::Result;
use chrono::{Duration, Utc};
use deadpool_postgres::Client as DbClient;
use messaging::providers;
use sequent_core::types::messaging::MessageAttemptState;
use tracing::{info, instrument, warn};

/// Queued attempts older than this were left by a worker that stopped
/// before recording the outcome.
pub const STALE_QUEUED_MINUTES: i64 = 10;
const BATCH: i64 = 500;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReconcileSummary {
    pub marked_unknown: usize,
    pub resolved: usize,
    pub still_unknown: usize,
    pub expired_links: u64,
}

#[instrument(skip_all, err)]
pub async fn reconcile_messages(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
) -> Result<ReconcileSummary> {
    let mut summary = ReconcileSummary::default();
    let tx = client.transaction().await?;
    summary.expired_links = expire_messenger_links(&tx, Utc::now()).await?;
    let unresolved = list_unresolved_messages(
        &tx,
        Utc::now() - Duration::minutes(STALE_QUEUED_MINUTES),
        BATCH,
    )
    .await?;
    tx.commit().await?;

    for message in unresolved {
        if message.state == MessageAttemptState::QUEUED {
            let tx = client.transaction().await?;
            transition_message(
                &tx,
                &message.id,
                MessageAttemptState::UNKNOWN,
                &StateChange {
                    error: Some("the worker stopped before recording the outcome".to_string()),
                    ..Default::default()
                },
            )
            .await?;
            tx.commit().await?;
            summary.marked_unknown += 1;
        }
        let Some(account_id) = message.account_id else {
            summary.still_unknown += 1;
            continue;
        };
        let tx = client.transaction().await?;
        let account = get_messaging_account(&tx, &message.tenant_id, &account_id).await?;
        let runtime = match &account {
            Some(account)
                if account
                    .provider
                    .capabilities(account.channel)
                    .is_some_and(|c| c.reconciliation) =>
            {
                Some(runtime_account(&tx, account).await?)
            }
            _ => None,
        };
        tx.commit().await?;
        let Some(runtime) = runtime else {
            summary.still_unknown += 1;
            continue;
        };
        // Providers that take Step's message ID know it by the idempotency
        // key even when the answer to the send was lost.
        let lookup_id = message.provider_message_id.clone().or_else(|| {
            message
                .logical_key
                .as_ref()
                .map(|key| format!("{key}:{}", message.attempt))
        });
        let Some(lookup_id) = lookup_id else {
            summary.still_unknown += 1;
            continue;
        };
        let state =
            match providers::build(&runtime, &providers::http_client()?, &dispatcher.endpoints)
                .await
            {
                Ok(sender) => sender.reconcile(&lookup_id).await,
                Err(error) => {
                    warn!(account = %account_id, "cannot reconcile: {error:#}");
                    None
                }
            };
        match state {
            Some(state) => {
                let tx = client.transaction().await?;
                transition_message(
                    &tx,
                    &message.id,
                    state,
                    &StateChange {
                        provider_message_id: Some(lookup_id),
                        ..Default::default()
                    },
                )
                .await?;
                tx.commit().await?;
                summary.resolved += 1;
            }
            None => summary.still_unknown += 1,
        }
    }
    info!(?summary, "reconciled messages");
    Ok(summary)
}
