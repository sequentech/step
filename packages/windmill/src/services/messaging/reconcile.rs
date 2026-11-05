// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The messaging sweep, run by beat on its own queue. One pass:
//!
//! 1. Expires Messenger links.
//! 2. Attempts left queued by a worker that stopped: never handed to the
//!    provider → FAILED (transient, so retried when the message's origin
//!    allows it); maybe handed → UNKNOWN.
//! 3. Same-channel retries that are due: the next attempt, on the same
//!    channel, with the content rendered again by the message's origin.
//!    Retries never wait on reconciliation.
//! 4. Unknown outcomes: asks providers that can tell, with a backoff per
//!    attempt, a cap and a time budget per pass. After
//!    [`UNRESOLVED_AFTER_HOURS`] an unknown outcome is shown as Unresolved
//!    and no longer asked about. It never becomes FAILED, and it is never
//!    sent again.
//! 5. Attempts whose electoral log entry was never handed to the log (the
//!    sender stopped first): returned with the pass's outcomes, to log.
//!
//! Every list is read a page at a time by keyset, and only rows the step
//! can act on are read, so rows that can never be resolved can't starve
//! newer ones. One unreadable row, one account that can't be used or one
//! failing step is logged and skipped: it never stops the pass.

use super::accounts::runtime_account;
use super::audit::{AttemptOutcome, ENVIRONMENT_PROVIDER};
use super::dispatch::{
    attempt_outcome, deliver_within, retry_refusal, Delivery, Dispatcher, SendPolicy,
    WhenNotSendable,
};
use crate::postgres::messaging::{
    cancel_retry, expire_messenger_links, get_messaging_account, list_due_retries,
    list_reconcilable_unknown, list_stale_queued, list_unlogged, mark_reconciled,
    transition_message, LedgerKey, MessageRecord, StateChange,
};
use crate::services::election_event_statistics::update_election_event_statistics;
use crate::services::election_statistics::update_election_statistics;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use messaging::attempts::next_retry_at;
use messaging::providers::{self, PROVIDER_TIMEOUT};
use messaging::sender::{ChannelSender, FailureKind};
use sequent_core::types::messaging::{
    MessageAttemptState, MessageChannel, MessagePurpose, MessageReason, MessagingProvider,
    RETRY_PREPARE_HOURS, UNRESOLVED_AFTER_HOURS,
};
use std::collections::{BTreeMap, HashMap};
use std::time::Instant;
use strum::IntoEnumIterator;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Queued attempts never handed to the provider and older than this were
/// left by a worker that stopped. One handed to the provider is stale this
/// long after it was handed over, plus the provider timeout.
pub const STALE_QUEUED_MINUTES: i64 = 2;
/// The least time between two questions to a provider about one attempt.
pub const RECONCILE_EVERY_MINUTES: i64 = 5;
/// An attempt's electoral log entry is written by its sender; the sweep
/// writes the ones still missing this long after the outcome.
pub const LOG_GRACE_MINUTES: i64 = 5;

/// How much one pass does at most, so a pass ends well before the next one
/// is due and a provider that doesn't answer can't hold everything up.
#[derive(Debug, Clone)]
pub struct SweepLimits {
    /// Rows read per keyset page.
    pub page: i64,
    /// Retries sent per pass, and the time they may take.
    pub max_retries: usize,
    pub retry_budget: std::time::Duration,
    /// Provider lookups per pass, and the time they may take.
    pub max_lookups: usize,
    pub lookup_budget: std::time::Duration,
    /// Unlogged attempts returned per pass.
    pub max_unlogged: usize,
}

impl Default for SweepLimits {
    fn default() -> Self {
        SweepLimits {
            page: 200,
            max_retries: 500,
            retry_budget: std::time::Duration::from_secs(25),
            max_lookups: 200,
            lookup_budget: std::time::Duration::from_secs(20),
            max_unlogged: 500,
        }
    }
}

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
    /// Attempts whose electoral log entry the sweep returns to be written.
    pub unlogged: usize,
    /// Rows, accounts or steps that failed and were skipped.
    pub skipped: usize,
}

/// What the sweep sends again for a failed attempt.
pub enum RetryContent {
    /// The message, rendered again for the recipient.
    Message(Box<Delivery>),
    /// Not retried, with the reason recorded on the attempt.
    Refused(MessageReason),
}

/// Renders a failed attempt's message again from what its origin keeps
/// (the stored send, the application's template reference), for the
/// voter's current record. The ledger keeps no content.
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

/// Moves a queued attempt that was never handed to the provider to FAILED,
/// only if it still isn't being sent. It is retried when its origin allows
/// it; otherwise why not is recorded next to the failure.
async fn fail_never_dispatched(
    tx: &Transaction<'_>,
    record: &MessageRecord,
    now: DateTime<Utc>,
) -> Result<Option<MessageRecord>> {
    let purpose = record.purpose.unwrap_or(MessagePurpose::NOTICE);
    let failure = MessageReason::WORKER_STOPPED_BEFORE_SENDING;
    let mut error = failure.to_string();
    let mut retry_at = next_retry_at(purpose, record.attempt, FailureKind::TRANSIENT, now);
    if retry_at.is_some() {
        if let Some(reason) = retry_refusal(
            tx,
            &record.tenant_id,
            record.logical_key.as_deref(),
            record.template_alias.as_deref(),
        )
        .await?
        {
            retry_at = None;
            error = reason.not_retried_after(Some(failure.as_str()));
        }
    }
    tx.query_opt(
        r#"
        UPDATE sequent_backend.message
        SET state = 'FAILED', failure = 'TRANSIENT', error = $2, next_retry_at = $3,
            failed_at = now(), updated_at = now()
        WHERE id = $1 AND state = 'QUEUED' AND dispatched_at IS NULL
        RETURNING *
        "#,
        &[&record.id, &error, &retry_at],
    )
    .await?
    .map(TryInto::try_into)
    .transpose()
}

/// The provider logged for an attempt: its account's, or the environment.
async fn provider_of(client: &mut DbClient, record: &MessageRecord) -> String {
    let Some(account_id) = record.account_id else {
        return ENVIRONMENT_PROVIDER.to_string();
    };
    let found = async {
        let tx = client.transaction().await?;
        let account = get_messaging_account(&tx, &record.tenant_id, &account_id).await?;
        tx.commit().await?;
        anyhow::Ok(account)
    }
    .await;
    match found {
        Ok(Some(account)) => account.provider.to_string(),
        Ok(None) => ENVIRONMENT_PROVIDER.to_string(),
        Err(error) => {
            warn!(account = %account_id, "could not read the account: {error:#}");
            ENVIRONMENT_PROVIDER.to_string()
        }
    }
}

/// Step 2: attempts left queued by a stopped worker.
async fn sweep_stale_queued(
    client: &mut DbClient,
    limits: &SweepLimits,
    summary: &mut SweepSummary,
    recorded: &mut Vec<AttemptOutcome>,
) -> Result<()> {
    let now = Utc::now();
    let queued_before = now - Duration::minutes(STALE_QUEUED_MINUTES);
    let dispatched_before = queued_before
        - Duration::from_std(PROVIDER_TIMEOUT).unwrap_or_else(|_| Duration::seconds(15));
    let mut after = None;
    loop {
        let tx = client.transaction().await?;
        let page =
            list_stale_queued(&tx, queued_before, dispatched_before, after, limits.page).await?;
        tx.commit().await?;
        let Some(last) = page.last else {
            return Ok(());
        };
        after = Some(last);
        summary.skipped += page.rows - page.records.len();
        for record in &page.records {
            let updated = async {
                let tx = client.transaction().await?;
                let updated = match record.dispatched_at {
                    None => fail_never_dispatched(&tx, record, now).await?,
                    Some(_) => {
                        transition_message(
                            &tx,
                            &record.id,
                            MessageAttemptState::UNKNOWN,
                            &StateChange {
                                error: Some(
                                    MessageReason::WORKER_STOPPED_AFTER_SENDING.to_string(),
                                ),
                                ..Default::default()
                            },
                        )
                        .await?
                    }
                };
                tx.commit().await?;
                anyhow::Ok(updated)
            }
            .await;
            match updated {
                // The attempt's outcome is recorded here for the first
                // time.
                Ok(Some(updated)) => {
                    match updated.state {
                        MessageAttemptState::FAILED => summary.never_sent += 1,
                        _ => summary.maybe_sent += 1,
                    }
                    let provider = provider_of(client, &updated).await;
                    recorded.push(attempt_outcome(&updated, &provider));
                }
                Ok(None) => {}
                Err(error) => {
                    summary.skipped += 1;
                    warn!(message_id = %record.id, "could not settle a stale attempt: {error:#}");
                }
            }
        }
        if (page.rows as i64) < limits.page {
            return Ok(());
        }
    }
}

/// Adds a retry accepted by the sweep to the event's and its elections'
/// message statistics, as the bulk send counts a first attempt. A crash
/// between the ledger and this count loses it: the ledger
/// (`count_event_messages`) stays the exact source.
async fn count_retry(
    client: &mut DbClient,
    delivery: &Delivery,
    channel: MessageChannel,
) -> Result<()> {
    let Some(event_id) = delivery.election_event_id else {
        return Ok(());
    };
    let increments = BTreeMap::from([(channel.statistics_key().to_string(), 1i64)]);
    let tenant_id = delivery.tenant_id.to_string();
    let event_id = event_id.to_string();
    let tx = client.transaction().await?;
    update_election_event_statistics(&tx, &tenant_id, &event_id, &increments).await?;
    for election_id in &delivery.recipient.election_ids {
        update_election_statistics(&tx, &tenant_id, &event_id, election_id, &increments).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Gives up a due retry with `reason`, next to the failure's own reason.
async fn refuse_retry(
    client: &mut DbClient,
    record: &MessageRecord,
    reason: MessageReason,
    summary: &mut SweepSummary,
) -> Result<()> {
    let tx = client.transaction().await?;
    cancel_retry(&tx, &record.id, reason).await?;
    tx.commit().await?;
    info!(message_id = %record.id, %reason, "not retried");
    summary.not_retried += 1;
    Ok(())
}

/// Step 3: same-channel retries that are due.
async fn sweep_retries(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    renderer: &dyn RetryRenderer,
    limits: &SweepLimits,
    summary: &mut SweepSummary,
    recorded: &mut Vec<AttemptOutcome>,
) -> Result<()> {
    let started = Instant::now();
    let now = Utc::now();
    let mut handled = 0;
    let mut after: Option<LedgerKey> = None;
    loop {
        let tx = client.transaction().await?;
        let page = list_due_retries(&tx, now, after, limits.page).await?;
        tx.commit().await?;
        let Some(last) = page.last else {
            return Ok(());
        };
        after = Some(last);
        summary.skipped += page.rows - page.records.len();
        for record in &page.records {
            if handled >= limits.max_retries || started.elapsed() >= limits.retry_budget {
                // Still due: the next pass goes on.
                return Ok(());
            }
            handled += 1;
            let content = match renderer.render(record).await {
                Ok(content) => content,
                Err(error) => {
                    // Left due for the next pass, unless it has failed to
                    // render for too long.
                    warn!(message_id = %record.id, "could not render a retry: {error:#}");
                    let overdue = record
                        .next_retry_at
                        .is_some_and(|at| now - at >= Duration::hours(RETRY_PREPARE_HOURS));
                    if overdue {
                        if let Err(error) =
                            refuse_retry(client, record, MessageReason::RETRY_NOT_PREPARED, summary)
                                .await
                        {
                            summary.skipped += 1;
                            warn!(message_id = %record.id, "could not stop a retry: {error:#}");
                        }
                    }
                    continue;
                }
            };
            let refusal = match &content {
                RetryContent::Refused(reason) => Some(*reason),
                // Never another channel: a voter who changed channel since
                // isn't sent the old message on the new one.
                RetryContent::Message(delivery)
                    if Some(delivery.recipient.channel) != record.channel =>
                {
                    Some(MessageReason::CHANNEL_CHANGED)
                }
                RetryContent::Message(_) => None,
            };
            if let Some(reason) = refusal {
                if let Err(error) = refuse_retry(client, record, reason, summary).await {
                    summary.skipped += 1;
                    warn!(message_id = %record.id, "could not stop a retry: {error:#}");
                }
                continue;
            }
            let RetryContent::Message(delivery) = content else {
                continue;
            };
            // A retry waits for capacity only within the pass's budget,
            // and one that can't be sent yet (no capacity, the account not
            // ready) stays due for a later pass instead of failing.
            let policy = SendPolicy {
                capacity_deadline: Some(tokio::time::Instant::from_std(
                    started + limits.retry_budget,
                )),
                when_not_sendable: WhenNotSendable::KEEP_RETRY_DUE,
                ..SendPolicy::default()
            };
            match deliver_within(client, dispatcher, &delivery, policy).await {
                Ok(result) => {
                    summary.retried += usize::from(!result.recorded.is_empty());
                    if result.newly_sent() {
                        if let Err(error) =
                            count_retry(client, &delivery, delivery.recipient.channel).await
                        {
                            warn!(message_id = %record.id, "could not count a retry: {error:#}");
                        }
                    }
                    recorded.extend(result.recorded);
                }
                Err(error) => {
                    summary.skipped += 1;
                    warn!(message_id = %record.id, "could not retry: {error:#}");
                }
            }
        }
        if (page.rows as i64) < limits.page {
            return Ok(());
        }
    }
}

/// The sender that can look up `account_id`'s attempts, or `None` when the
/// account can't be used (it is skipped, with a warning).
async fn lookup_sender(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    record: &MessageRecord,
    account_id: Uuid,
) -> Option<Box<dyn ChannelSender>> {
    let runtime = async {
        let tx = client.transaction().await?;
        let account = get_messaging_account(&tx, &record.tenant_id, &account_id).await?;
        let runtime = match &account {
            Some(account) => Some(runtime_account(&tx, account).await?),
            None => None,
        };
        tx.commit().await?;
        anyhow::Ok(runtime)
    }
    .await;
    let runtime = match runtime {
        Ok(Some(runtime)) => runtime,
        Ok(None) => return None,
        Err(error) => {
            warn!(account = %account_id, "cannot reconcile: {error:#}");
            return None;
        }
    };
    match providers::build(&runtime, dispatcher.http(), &dispatcher.endpoints).await {
        Ok(sender) => Some(sender),
        Err(error) => {
            warn!(account = %account_id, "cannot reconcile: {error:#}");
            None
        }
    }
}

/// Step 4: unknown outcomes the provider may resolve.
async fn sweep_unknown(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    limits: &SweepLimits,
    summary: &mut SweepSummary,
) -> Result<()> {
    let started = Instant::now();
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
    let mut asked = 0;
    let mut after = None;
    loop {
        let tx = client.transaction().await?;
        let page = list_reconcilable_unknown(
            &tx,
            &providers,
            newer_than,
            asked_before,
            after,
            limits.page,
        )
        .await?;
        tx.commit().await?;
        let Some(last) = page.last else {
            return Ok(());
        };
        after = Some(last);
        summary.skipped += page.rows - page.records.len();
        for record in &page.records {
            if asked >= limits.max_lookups || started.elapsed() >= limits.lookup_budget {
                // The rows not asked yet come first in the next pass: the
                // ones asked now wait for their backoff.
                return Ok(());
            }
            let Some(account_id) = record.account_id else {
                continue;
            };
            if !senders.contains_key(&account_id) {
                let sender = lookup_sender(client, dispatcher, record, account_id).await;
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
            asked += 1;
            summary.asked += 1;
            let state = sender.reconcile(&lookup_id).await;
            let applied = async {
                let tx = client.transaction().await?;
                mark_reconciled(&tx, &record.id).await?;
                let mut resolved = false;
                if let Some(state) = state {
                    // The lookup ID may be Step's own reference: it never
                    // replaces the provider's ID.
                    resolved = transition_message(&tx, &record.id, state, &StateChange::default())
                        .await?
                        .is_some();
                }
                tx.commit().await?;
                anyhow::Ok(resolved)
            }
            .await;
            match applied {
                Ok(resolved) => summary.resolved += usize::from(resolved),
                Err(error) => {
                    summary.skipped += 1;
                    warn!(message_id = %record.id, "could not record a lookup: {error:#}");
                }
            }
        }
        if (page.rows as i64) < limits.page {
            return Ok(());
        }
    }
}

/// Step 5: attempts whose electoral log entry is still missing.
async fn sweep_unlogged(
    client: &mut DbClient,
    limits: &SweepLimits,
    summary: &mut SweepSummary,
    recorded: &mut Vec<AttemptOutcome>,
) -> Result<()> {
    let recorded_before = Utc::now() - Duration::minutes(LOG_GRACE_MINUTES);
    let mut after = None;
    loop {
        let tx = client.transaction().await?;
        let page = list_unlogged(&tx, recorded_before, after, limits.page).await?;
        tx.commit().await?;
        let Some(last) = page.last else {
            return Ok(());
        };
        after = Some(last);
        summary.skipped += page.rows - page.records.len();
        for record in &page.records {
            if summary.unlogged >= limits.max_unlogged {
                return Ok(());
            }
            let provider = provider_of(client, record).await;
            recorded.push(attempt_outcome(record, &provider));
            summary.unlogged += 1;
        }
        if (page.rows as i64) < limits.page {
            return Ok(());
        }
    }
}

/// Logs a step that failed as a whole; the pass goes on.
fn step(name: &str, outcome: Result<()>, summary: &mut SweepSummary) {
    if let Err(error) = outcome {
        summary.skipped += 1;
        warn!("messaging sweep step {name} failed: {error:#}");
    }
}

/// One pass of the sweep with the default limits. Returns what it did and
/// the attempts whose electoral log entry is to be written now.
pub async fn sweep_messages(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    renderer: &dyn RetryRenderer,
) -> Result<(SweepSummary, Vec<AttemptOutcome>)> {
    sweep_messages_with(client, dispatcher, renderer, &SweepLimits::default()).await
}

/// One pass of the sweep within `limits`. A failing step is logged and the
/// next one still runs.
#[instrument(skip_all, err)]
pub async fn sweep_messages_with(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    renderer: &dyn RetryRenderer,
    limits: &SweepLimits,
) -> Result<(SweepSummary, Vec<AttemptOutcome>)> {
    let mut summary = SweepSummary::default();
    let mut recorded = vec![];
    let expired = async {
        let tx = client.transaction().await?;
        let expired = expire_messenger_links(&tx, Utc::now()).await?;
        tx.commit().await?;
        anyhow::Ok(expired)
    }
    .await;
    match expired {
        Ok(expired) => summary.expired_links = expired,
        Err(error) => step("links", Err(error), &mut summary),
    }
    let outcome = sweep_stale_queued(client, limits, &mut summary, &mut recorded).await;
    step("stale queued", outcome, &mut summary);
    let outcome = sweep_retries(
        client,
        dispatcher,
        renderer,
        limits,
        &mut summary,
        &mut recorded,
    )
    .await;
    step("retries", outcome, &mut summary);
    let outcome = sweep_unknown(client, dispatcher, limits, &mut summary).await;
    step("reconciliation", outcome, &mut summary);
    let outcome = sweep_unlogged(client, limits, &mut summary, &mut recorded).await;
    step("electoral log", outcome, &mut summary);
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

/// The sweep without retries or the log outbox: what the first release
/// called reconciliation.
pub async fn reconcile_messages(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
) -> Result<SweepSummary> {
    let limits = SweepLimits::default();
    let mut summary = SweepSummary::default();
    let mut recorded = vec![];
    let tx = client.transaction().await?;
    summary.expired_links = expire_messenger_links(&tx, Utc::now()).await?;
    tx.commit().await?;
    let outcome = sweep_stale_queued(client, &limits, &mut summary, &mut recorded).await;
    step("stale queued", outcome, &mut summary);
    let outcome = sweep_unknown(client, dispatcher, &limits, &mut summary).await;
    step("reconciliation", outcome, &mut summary);
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

    #[test]
    fn a_pass_is_bounded_well_inside_the_beat_interval() {
        let limits = SweepLimits::default();
        assert!(limits.retry_budget + limits.lookup_budget < std::time::Duration::from_secs(60));
        assert!(limits.max_lookups as i64 <= limits.page);
    }
}
