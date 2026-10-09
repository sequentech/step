// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The messaging sweep, run by beat on its own queue:
//!
//! 1. Expires Messenger links.
//! 2. Attempts left queued by a worker that stopped: never handed to the
//!    provider → FAILED (transient, so retried); maybe handed → UNKNOWN.
//! 3. Unknown outcomes: asks providers that can tell, with a backoff per
//!    attempt. After [`UNRESOLVED_AFTER_HOURS`] an unknown outcome is shown
//!    as Unresolved and no longer asked about. It never becomes FAILED, and
//!    it is never sent again.
//! 4. Same-channel retries that are due: the next attempt, on the same
//!    channel, with the content rendered again from the stored template.
//!
//! Every list is read a page at a time by keyset, and only rows the step
//! can act on are read, so rows that can never be resolved can't starve
//! newer ones.

use super::accounts::runtime_account;
use super::audit::{AttemptOutcome, ENVIRONMENT_PROVIDER};
use super::dispatch::{attempt_outcome, deliver, Delivery, Dispatcher};
use crate::postgres::messaging::{
    cancel_retry, expire_messenger_links, get_messaging_account, list_due_retries,
    list_reconcilable_unknown, list_stale_queued, mark_reconciled, transition_message, LedgerKey,
    MessageRecord, StateChange,
};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use messaging::attempts::next_retry_at;
use messaging::providers;
use messaging::sender::{ChannelSender, FailureKind};
use sequent_core::types::messaging::{
    MessageAttemptState, MessagePurpose, MessageReason, MessagingProvider, UNRESOLVED_AFTER_HOURS,
};
use std::collections::HashMap;
use strum::IntoEnumIterator;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Queued attempts older than this were left by a worker that stopped
/// before recording the outcome.
pub const STALE_QUEUED_MINUTES: i64 = 2;
/// The least time between two questions to a provider about one attempt.
pub const RECONCILE_EVERY_MINUTES: i64 = 5;
const PAGE: i64 = 200;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SweepSummary {
    pub expired_links: u64,
    /// Queued attempts never handed to the provider, now FAILED.
    pub never_sent: usize,
    /// Queued attempts that may have reached the provider, now UNKNOWN.
    pub maybe_sent: usize,
    pub asked: usize,
    pub resolved: usize,
    pub retried: usize,
    pub not_retried: usize,
}

/// What the sweep sends again for a failed attempt.
pub enum RetryContent {
    /// The message, rendered again for the recipient.
    Message(Box<Delivery>),
    /// Not retried, with the reason recorded on the attempt.
    Refused(MessageReason),
}

/// Renders a failed attempt's message again from what the ledger keeps (its
/// stored template, language, voter and event). The ledger keeps no
/// content.
#[async_trait]
pub trait RetryRenderer: Send + Sync {
    async fn render(&self, record: &MessageRecord) -> Result<RetryContent>;
}

/// Providers that can say what happened to an attempt.
pub fn reconcilable_providers() -> Vec<MessagingProvider> {
    MessagingProvider::iter()
        .filter(|provider| {
            provider
                .channel()
                .and_then(|channel| provider.capabilities(channel))
                .is_some_and(|capabilities| capabilities.reconciliation)
        })
        .collect()
}

/// Whether an unknown outcome is old enough to be shown as Unresolved.
pub fn is_unresolved(record: &MessageRecord, now: DateTime<Utc>) -> bool {
    record.state == MessageAttemptState::UNKNOWN
        && record.created_at < now - Duration::hours(UNRESOLVED_AFTER_HOURS)
}

fn key_of(record: &MessageRecord) -> LedgerKey {
    (record.created_at, record.id)
}

/// Moves a queued attempt that was never handed to the provider to FAILED,
/// only if it still isn't being sent.
async fn fail_never_dispatched(
    tx: &Transaction<'_>,
    record: &MessageRecord,
    now: DateTime<Utc>,
) -> Result<Option<MessageRecord>> {
    let purpose = record.purpose.unwrap_or(MessagePurpose::NOTICE);
    let mut reason = MessageReason::WORKER_STOPPED_BEFORE_SENDING;
    let mut retry_at = next_retry_at(purpose, record.attempt, FailureKind::TRANSIENT, now);
    if retry_at.is_some() && record.template_alias.is_none() {
        retry_at = None;
        reason = MessageReason::NO_STORED_TEMPLATE;
    }
    tx.query_opt(
        r#"
        UPDATE sequent_backend.message
        SET state = 'FAILED', failure = 'TRANSIENT', error = $2, next_retry_at = $3,
            failed_at = now(), updated_at = now()
        WHERE id = $1 AND state = 'QUEUED' AND dispatched_at IS NULL
        RETURNING *
        "#,
        &[&record.id, &reason.as_str(), &retry_at],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

async fn provider_of(client: &mut DbClient, record: &MessageRecord) -> Result<String> {
    let Some(account_id) = record.account_id else {
        return Ok(ENVIRONMENT_PROVIDER.to_string());
    };
    let tx = client.transaction().await?;
    let account = get_messaging_account(&tx, &record.tenant_id, &account_id).await?;
    tx.commit().await?;
    Ok(account
        .map(|a| a.provider.to_string())
        .unwrap_or_else(|| ENVIRONMENT_PROVIDER.to_string()))
}

/// Step 2: attempts left queued by a stopped worker.
async fn sweep_stale_queued(
    client: &mut DbClient,
    summary: &mut SweepSummary,
    recorded: &mut Vec<AttemptOutcome>,
) -> Result<()> {
    let now = Utc::now();
    let stale_before = now - Duration::minutes(STALE_QUEUED_MINUTES);
    let mut after = None;
    loop {
        let tx = client.transaction().await?;
        let page = list_stale_queued(&tx, stale_before, after, PAGE).await?;
        tx.commit().await?;
        let Some(last) = page.last() else {
            return Ok(());
        };
        after = Some(key_of(last));
        for record in &page {
            let tx = client.transaction().await?;
            let updated = match record.dispatched_at {
                None => {
                    let failed = fail_never_dispatched(&tx, record, now).await?;
                    summary.never_sent += usize::from(failed.is_some());
                    failed
                }
                Some(_) => {
                    let unknown = transition_message(
                        &tx,
                        &record.id,
                        MessageAttemptState::UNKNOWN,
                        &StateChange {
                            error: Some(MessageReason::WORKER_STOPPED_AFTER_SENDING.to_string()),
                            ..Default::default()
                        },
                    )
                    .await?;
                    summary.maybe_sent += usize::from(unknown.is_some());
                    unknown
                }
            };
            tx.commit().await?;
            // The attempt's outcome is recorded here for the first time.
            if let Some(updated) = updated {
                let provider = provider_of(client, &updated).await?;
                recorded.push(attempt_outcome(&updated, &provider));
            }
        }
        if (page.len() as i64) < PAGE {
            return Ok(());
        }
    }
}

/// Step 3: unknown outcomes the provider may resolve.
async fn sweep_unknown(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    summary: &mut SweepSummary,
) -> Result<()> {
    let now = Utc::now();
    let providers: Vec<String> = reconcilable_providers()
        .iter()
        .map(ToString::to_string)
        .collect();
    if providers.is_empty() {
        return Ok(());
    }
    let newer_than = now - Duration::hours(UNRESOLVED_AFTER_HOURS);
    let asked_before = now - Duration::minutes(RECONCILE_EVERY_MINUTES);
    let mut senders: HashMap<Uuid, Option<Box<dyn ChannelSender>>> = HashMap::new();
    let mut after = None;
    loop {
        let tx = client.transaction().await?;
        let page =
            list_reconcilable_unknown(&tx, &providers, newer_than, asked_before, after, PAGE)
                .await?;
        tx.commit().await?;
        let Some(last) = page.last() else {
            return Ok(());
        };
        after = Some(key_of(last));
        for record in &page {
            let Some(account_id) = record.account_id else {
                continue;
            };
            if !senders.contains_key(&account_id) {
                let tx = client.transaction().await?;
                let account = get_messaging_account(&tx, &record.tenant_id, &account_id).await?;
                let runtime = match &account {
                    Some(account) => Some(runtime_account(&tx, account).await?),
                    None => None,
                };
                tx.commit().await?;
                let sender = match runtime {
                    Some(runtime) => {
                        match providers::build(&runtime, dispatcher.http(), &dispatcher.endpoints)
                            .await
                        {
                            Ok(sender) => Some(sender),
                            Err(error) => {
                                warn!(account = %account_id, "cannot reconcile: {error:#}");
                                None
                            }
                        }
                    }
                    None => None,
                };
                senders.insert(account_id, sender);
            }
            let Some(Some(sender)) = senders.get(&account_id) else {
                continue;
            };
            // Providers that take Step's message ID know it by the
            // idempotency key even when the answer to the send was lost.
            let lookup_id = record.provider_message_id.clone().or_else(|| {
                record
                    .logical_key
                    .as_ref()
                    .map(|key| format!("{key}:{}", record.attempt))
            });
            let Some(lookup_id) = lookup_id else {
                continue;
            };
            summary.asked += 1;
            let state = sender.reconcile(&lookup_id).await;
            let tx = client.transaction().await?;
            mark_reconciled(&tx, &record.id).await?;
            if let Some(state) = state {
                // The lookup ID may be Step's own reference: it never
                // replaces the provider's ID.
                if transition_message(&tx, &record.id, state, &StateChange::default())
                    .await?
                    .is_some()
                {
                    summary.resolved += 1;
                }
            }
            tx.commit().await?;
        }
        if (page.len() as i64) < PAGE {
            return Ok(());
        }
    }
}

/// Step 4: same-channel retries that are due.
async fn sweep_retries(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    renderer: &dyn RetryRenderer,
    summary: &mut SweepSummary,
    recorded: &mut Vec<AttemptOutcome>,
) -> Result<()> {
    let now = Utc::now();
    let mut after: Option<LedgerKey> = None;
    loop {
        let tx = client.transaction().await?;
        let page = list_due_retries(&tx, now, after, PAGE).await?;
        tx.commit().await?;
        let Some(last) = page.last() else {
            return Ok(());
        };
        after = last.next_retry_at.map(|at| (at, last.id));
        for record in &page {
            let content = match renderer.render(record).await {
                Ok(content) => content,
                Err(error) => {
                    // Left due: the next sweep tries again.
                    warn!(message_id = %record.id, "could not render a retry: {error:#}");
                    continue;
                }
            };
            let refusal = match &content {
                RetryContent::Refused(reason) => Some(*reason),
                // Never another channel: a voter who changed channel since
                // isn't sent the old message on the new one.
                RetryContent::Message(delivery) if delivery.recipient.channel != record.channel => {
                    Some(MessageReason::CHANNEL_CHANGED)
                }
                RetryContent::Message(_) => None,
            };
            if let Some(reason) = refusal {
                let tx = client.transaction().await?;
                cancel_retry(&tx, &record.id, reason.as_str()).await?;
                tx.commit().await?;
                info!(message_id = %record.id, %reason, "not retried");
                summary.not_retried += 1;
                continue;
            }
            let RetryContent::Message(delivery) = content else {
                continue;
            };
            match deliver(client, dispatcher, &delivery).await {
                Ok(result) => {
                    summary.retried += usize::from(!result.recorded.is_empty());
                    recorded.extend(result.recorded);
                }
                Err(error) => {
                    warn!(message_id = %record.id, "could not retry: {error:#}");
                }
            }
        }
        if (page.len() as i64) < PAGE {
            return Ok(());
        }
    }
}

/// One pass of the sweep. Returns what it did and the attempts whose
/// outcome it recorded, for the electoral log.
#[instrument(skip_all, err)]
pub async fn sweep_messages(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    renderer: &dyn RetryRenderer,
) -> Result<(SweepSummary, Vec<AttemptOutcome>)> {
    let mut summary = SweepSummary::default();
    let mut recorded = vec![];
    let tx = client.transaction().await?;
    summary.expired_links = expire_messenger_links(&tx, Utc::now()).await?;
    tx.commit().await?;
    sweep_stale_queued(client, &mut summary, &mut recorded).await?;
    sweep_unknown(client, dispatcher, &mut summary).await?;
    sweep_retries(client, dispatcher, renderer, &mut summary, &mut recorded).await?;
    info!(?summary, "messaging sweep");
    Ok((summary, recorded))
}

/// A renderer that renders nothing: every due retry is refused for lack of
/// a stored template. For callers that only reconcile.
pub struct NoRetries;

#[async_trait]
impl RetryRenderer for NoRetries {
    async fn render(&self, _record: &MessageRecord) -> Result<RetryContent> {
        Ok(RetryContent::Refused(MessageReason::NO_STORED_TEMPLATE))
    }
}

/// The sweep without retries: what the first release called
/// reconciliation.
pub async fn reconcile_messages(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
) -> Result<SweepSummary> {
    let mut summary = SweepSummary::default();
    let mut recorded = vec![];
    let tx = client.transaction().await?;
    summary.expired_links = expire_messenger_links(&tx, Utc::now()).await?;
    tx.commit().await?;
    sweep_stale_queued(client, &mut summary, &mut recorded).await?;
    sweep_unknown(client, dispatcher, &mut summary).await?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_providers_that_can_tell_are_asked() {
        assert_eq!(
            reconcilable_providers(),
            vec![MessagingProvider::VIBER_INFOBIP]
        );
    }
}
