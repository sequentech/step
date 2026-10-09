// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending one logical message on the recipient's one channel: choosing the
//! account, recording the attempt before it leaves Step, and recording what
//! the provider said. Nothing here ever tries another channel: a message
//! that can't go out on its channel gets one FAILED row with its reason.

use super::accounts::runtime_account;
use super::audit::{AttemptOutcome, MessageKind, ENVIRONMENT_PROVIDER};
use super::config::get_event_messaging_config;
use super::keys::{tenant_key, TenantKey};
use crate::postgres::messaging::{
    get_message, get_messaging_account, insert_message, last_get_started_at, list_attempts,
    list_messaging_accounts, mark_dispatched, transition_message, MessageRecord, MessagingAccount,
    NewMessage, StateChange,
};
use crate::services::providers::email_sender::EmailSender;
use crate::services::providers::sms_sender::SmsSender;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Client as DbClient;
use messaging::attempts::{decide, next_retry_at, AttemptRecord, Dispatch};
use messaging::destination::Destination;
use messaging::providers::{self, Endpoints};
use messaging::rate_limit::RateLimiter;
use messaging::sender::{
    preflight, ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome,
};
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, BillingCategory, DeliveryFeedback, EventMessagingConfig,
    MessageAttemptState, MessageChannel, MessageContent, MessagePurpose, MessageReason,
    OutOfWindowPolicy, ProviderCapabilities, RecipientKind,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Who the message goes to: one channel and its destination.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipient {
    pub voter_id: Option<String>,
    /// The recipient's one channel. Never replaced by another one.
    pub channel: MessageChannel,
    /// Phone number (E.164), email address or Page-scoped ID.
    pub destination: String,
    /// The voter's elections, for per-election (Post) channel restrictions.
    pub election_ids: Vec<String>,
}

/// One logical message.
#[derive(Debug, Clone)]
pub struct Delivery {
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub purpose: MessagePurpose,
    pub recipient: Recipient,
    /// Rendered content for the recipient's channel. `None`: the send has
    /// none for it, and the message fails with
    /// [`MessageReason::NO_CONTENT_FOR_CHANNEL`].
    pub content: Option<MessageContent>,
    pub language: Option<String>,
    /// The stored template the content was rendered from. Also what lets a
    /// transient failure be retried later: without it the ledger has
    /// nothing to render again.
    pub template_alias: Option<String>,
    /// Stable across retries of the same message.
    pub logical_key: String,
    pub expires_at: Option<DateTime<Utc>>,
    /// Send through this account instead of the event's or the default,
    /// such as for an administrator's test message.
    pub account_id: Option<Uuid>,
    /// Which message this is, to pick its approved template among the
    /// event's bindings: a template alias or Keycloak's message key.
    pub template_key: Option<String>,
    /// The approved template to use instead of the event's binding.
    pub provider_template: Option<ProviderTemplate>,
}

/// An approved template at the provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderTemplate {
    pub name: String,
    /// The provider's code for the template's language.
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeliveryResult {
    pub message_id: Option<Uuid>,
    pub attempt: Option<i32>,
    pub channel: MessageChannel,
    pub state: MessageAttemptState,
    /// Short, non-sensitive reason when not accepted.
    pub reason: Option<String>,
    /// When the same-channel retry is due, if one is scheduled.
    pub next_retry_at: Option<DateTime<Utc>>,
    /// Shown in records instead of the address.
    pub masked_destination: Option<String>,
    /// The attempts whose outcome this call recorded, for the electoral log
    /// and statistics. Empty when the call recorded nothing new, such as a
    /// replayed job.
    pub recorded: Vec<AttemptOutcome>,
}

impl DeliveryResult {
    fn from_record(record: &MessageRecord) -> DeliveryResult {
        DeliveryResult {
            message_id: Some(record.id),
            attempt: Some(record.attempt),
            channel: record.channel,
            state: record.state,
            reason: record.error.clone(),
            next_retry_at: record.next_retry_at,
            masked_destination: Some(record.masked_destination.clone()),
            recorded: vec![],
        }
    }

    /// Whether this call accepted a message that wasn't accepted before.
    pub fn newly_sent(&self) -> bool {
        self.recorded.iter().any(|outcome| {
            matches!(
                outcome.outcome,
                MessageAttemptState::ACCEPTED | MessageAttemptState::DELIVERED
            )
        })
    }
}

/// The outcome of `record` as the electoral log entry of its attempt.
pub fn attempt_outcome(record: &MessageRecord, provider: &str) -> AttemptOutcome {
    AttemptOutcome {
        tenant_id: record.tenant_id,
        election_event_id: record.election_event_id,
        voter_id: record.voter_id.clone(),
        message_id: record.id,
        attempt: record.attempt,
        channel: record.channel,
        account_id: record.account_id,
        provider: provider.to_string(),
        purpose: record.purpose.unwrap_or(MessagePurpose::NOTICE),
        kind: MessageKind::of(record.logical_key.as_deref().unwrap_or_default()),
        outcome: record.state,
        reason: record.error.clone(),
        receiver: record.masked_destination.clone(),
    }
}

/// Shared by every delivery of a process: HTTP clients and per-account
/// rates.
pub struct Dispatcher {
    pub endpoints: Endpoints,
    http: reqwest::Client,
    limiters: Mutex<
        HashMap<
            Uuid,
            (
                sequent_core::types::messaging::AccountLimits,
                Arc<RateLimiter>,
            ),
        >,
    >,
}

static DISPATCHER: OnceCell<Dispatcher> = OnceCell::const_new();

impl Dispatcher {
    pub fn new(endpoints: Endpoints) -> Result<Dispatcher> {
        Ok(Dispatcher {
            endpoints,
            http: providers::http_client()?,
            limiters: Mutex::new(HashMap::new()),
        })
    }

    pub async fn global() -> Result<&'static Dispatcher> {
        DISPATCHER
            .get_or_try_init(|| async { Dispatcher::new(Endpoints::from_env()) })
            .await
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    /// The account's limiter, made again when the account's limits changed.
    fn limiter(&self, account: &MessagingAccount) -> Arc<RateLimiter> {
        let Ok(mut limiters) = self.limiters.lock() else {
            return Arc::new(RateLimiter::new(&account.limits));
        };
        match limiters.get(&account.id) {
            Some((limits, limiter)) if *limits == account.limits => limiter.clone(),
            _ => {
                let limiter = Arc::new(RateLimiter::new(&account.limits));
                limiters.insert(account.id, (account.limits.clone(), limiter.clone()));
                limiter
            }
        }
    }
}

/// Email and SMS through the process environment, for tenants that have
/// not configured sending accounts.
struct EnvironmentSender {
    channel: MessageChannel,
}

#[async_trait]
impl ChannelSender for EnvironmentSender {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            channel: self.channel,
            recipient: match self.channel {
                MessageChannel::EMAIL => RecipientKind::EMAIL_ADDRESS,
                _ => RecipientKind::PHONE_NUMBER,
            },
            purposes: vec![MessagePurpose::OTP, MessagePurpose::NOTICE],
            template_required_for: vec![],
            delivery_feedback: DeliveryFeedback::UNAVAILABLE,
            reconciliation: false,
            conversation_window_hours: None,
            requires_provider_approval: false,
        }
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        let sent = match self.channel {
            MessageChannel::EMAIL => match EmailSender::new().await {
                Ok(sender) => sender
                    .send(
                        vec![message.destination.as_str().to_string()],
                        message.content.subject.clone().unwrap_or_default(),
                        message.content.text.clone(),
                        message.content.html.clone(),
                        vec![],
                    )
                    .await
                    .is_ok(),
                Err(_) => {
                    return refused("the email transport is not configured");
                }
            },
            MessageChannel::SMS => match SmsSender::new().await {
                Ok(sender) => sender
                    .send(
                        message.destination.as_str().to_string(),
                        message.content.text.clone(),
                    )
                    .await
                    .is_ok(),
                Err(_) => {
                    return refused("the SMS transport is not configured");
                }
            },
            _ => return refused(MessageReason::NO_ACCOUNT_FOR_CHANNEL.as_str()),
        };
        if sent {
            SendOutcome::Accepted {
                provider_message_id: None,
            }
        } else {
            // The environment transports do not say whether the provider
            // took the message.
            SendOutcome::Unknown {
                reason: "the environment transport reported an error".to_string(),
            }
        }
    }

    async fn check(&self) -> AccountCheck {
        AccountCheck {
            connected: true,
            production_access: true,
            ..Default::default()
        }
    }
}

/// The account for `channel`: the event's choice, otherwise the tenant
/// default. `None` with no account; email and SMS then use the environment.
async fn account_for(
    tx: &deadpool_postgres::Transaction<'_>,
    tenant_id: &Uuid,
    config: Option<&EventMessagingConfig>,
    channel: MessageChannel,
) -> Result<Option<MessagingAccount>> {
    if let Some(channel_config) = config.and_then(|c| c.channel(channel)) {
        let id = Uuid::parse_str(&channel_config.account_id)?;
        return get_messaging_account(tx, tenant_id, &id).await;
    }
    Ok(list_messaging_accounts(tx, tenant_id)
        .await?
        .into_iter()
        .find(|account| account.channel == channel && account.is_default))
}

/// Channels the event offers for `purpose` to a voter of `election_ids`.
/// Without an event configuration, email and SMS stay available as before.
pub fn eligible_channels(
    config: Option<&EventMessagingConfig>,
    purpose: MessagePurpose,
    election_ids: &[String],
) -> Vec<MessageChannel> {
    match config {
        Some(config) => {
            let mut eligible = config.enabled_channels(purpose, None);
            for election_id in election_ids {
                let allowed = config.enabled_channels(purpose, Some(election_id));
                eligible.retain(|channel| allowed.contains(channel));
            }
            eligible
        }
        None => vec![MessageChannel::EMAIL, MessageChannel::SMS],
    }
}

fn attempt_records(records: &[MessageRecord]) -> Vec<AttemptRecord> {
    records
        .iter()
        .map(|record| AttemptRecord {
            attempt: record.attempt,
            channel: record.channel,
            state: record.state,
            failure: record.failure,
            next_retry_at: record.next_retry_at,
        })
        .collect()
}

fn refused(reason: impl ToString) -> SendOutcome {
    SendOutcome::Rejected(ProviderFailure {
        kind: FailureKind::PERMANENT,
        code: None,
        reason: reason.to_string(),
    })
}

/// The time a voter's Messenger conversation stays open after Get Started.
const MESSENGER_WINDOW_HOURS: i64 = 24;

/// Records a message that has no channel at all: one FAILED row with
/// [`MessageReason::NO_CONTACT_CHANNEL`], so staff see it. The ledger needs
/// a channel, so the row is on email, the channel such a voter would have
/// had (`voter_channel`'s legacy rule). A replay records nothing new.
#[instrument(skip_all, fields(logical_key = %logical_key), err)]
pub async fn record_without_channel(
    client: &mut DbClient,
    tenant_id: Uuid,
    election_event_id: Option<Uuid>,
    voter_id: Option<String>,
    purpose: MessagePurpose,
    template_alias: Option<String>,
    language: Option<String>,
    logical_key: &str,
) -> Result<DeliveryResult> {
    let tx = client.transaction().await?;
    let inserted = insert_message(
        &tx,
        &NewMessage {
            tenant_id,
            election_event_id,
            voter_id,
            account_id: None,
            channel: MessageChannel::EMAIL,
            purpose: Some(purpose),
            template_alias,
            language,
            masked_destination: "-".to_string(),
            destination_digest: "-".to_string(),
            destination_country: None,
            logical_key: Some(logical_key.to_string()),
            attempt: 1,
            state: MessageAttemptState::FAILED,
            provider_message_id: None,
            error: Some(MessageReason::NO_CONTACT_CHANNEL.to_string()),
            failure: Some(FailureKind::PERMANENT),
            billing_category: None,
        },
    )
    .await?;
    let result = match inserted {
        Some(record) => {
            warn!(message_id = %record.id, "{}", MessageReason::NO_CONTACT_CHANNEL);
            let mut result = DeliveryResult::from_record(&record);
            result.recorded = vec![attempt_outcome(&record, ENVIRONMENT_PROVIDER)];
            result
        }
        None => list_attempts(&tx, &tenant_id, logical_key)
            .await?
            .last()
            .map(DeliveryResult::from_record)
            .ok_or_else(|| anyhow!("inconsistent attempts"))?,
    };
    tx.commit().await?;
    Ok(result)
}

/// Sends `delivery` on its recipient's channel, unless an earlier attempt
/// was accepted, may have reached the provider, or failed for good, or a
/// retry isn't due yet. A transient failure schedules the next attempt on
/// the same channel; the retry sweep sends it.
#[instrument(skip_all, fields(logical_key = %delivery.logical_key, purpose = %delivery.purpose), err)]
pub async fn deliver(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    delivery: &Delivery,
) -> Result<DeliveryResult> {
    // Only a concurrent worker recording the same attempt makes it loop.
    for _ in 0..4 {
        let tx = client.transaction().await?;
        let config = match &delivery.election_event_id {
            Some(event) => get_event_messaging_config(&tx, &delivery.tenant_id, event).await?,
            None => None,
        };
        let records = list_attempts(&tx, &delivery.tenant_id, &delivery.logical_key).await?;
        let latest = records.iter().max_by_key(|r| r.attempt);
        let (attempt, channel) = match decide(
            delivery.purpose,
            delivery.recipient.channel,
            &attempt_records(&records),
            Utc::now(),
        ) {
            Dispatch::Send { attempt, channel } => (attempt, channel),
            Dispatch::AlreadySent | Dispatch::AwaitReconciliation => {
                let current = records
                    .iter()
                    .find(|r| r.state != MessageAttemptState::FAILED)
                    .map(DeliveryResult::from_record);
                tx.commit().await?;
                return current.ok_or_else(|| anyhow!("inconsistent attempts"));
            }
            Dispatch::RetryLater { .. } | Dispatch::GiveUp => {
                let current = latest.map(DeliveryResult::from_record);
                tx.commit().await?;
                return current.ok_or_else(|| anyhow!("inconsistent attempts"));
            }
        };

        let account = match delivery.account_id {
            Some(account_id) => get_messaging_account(&tx, &delivery.tenant_id, &account_id)
                .await?
                .filter(|account| account.channel == channel),
            None => account_for(&tx, &delivery.tenant_id, config.as_ref(), channel).await?,
        };
        let provider = account
            .as_ref()
            .map(|a| a.provider.to_string())
            .unwrap_or_else(|| ENVIRONMENT_PROVIDER.to_string());
        let destination_key = tenant_key(&tx, &delivery.tenant_id, TenantKey::Destination).await?;
        let destination = Destination::parse(channel, &delivery.recipient.destination);
        let (masked, digest, country) = match &destination {
            Ok(destination) => (
                destination.masked(),
                destination.digest(&destination_key)?,
                destination.country().map(str::to_string),
            ),
            Err(_) => ("-".to_string(), "-".to_string(), None),
        };
        let mut record = NewMessage {
            tenant_id: delivery.tenant_id,
            election_event_id: delivery.election_event_id,
            voter_id: delivery.recipient.voter_id.clone(),
            account_id: account.as_ref().map(|a| a.id),
            channel,
            purpose: Some(delivery.purpose),
            template_alias: delivery.template_alias.clone(),
            language: delivery.language.clone(),
            masked_destination: masked,
            destination_digest: digest.clone(),
            destination_country: country,
            logical_key: Some(delivery.logical_key.clone()),
            attempt,
            state: MessageAttemptState::QUEUED,
            provider_message_id: None,
            error: None,
            failure: None,
            billing_category: None,
        };

        let prepared: std::result::Result<(Box<dyn ChannelSender>, OutboundMessage, bool), String> =
            async {
                // A send through a chosen account (a test message) is not
                // subject to the event's channels.
                if delivery.account_id.is_none()
                    && !eligible_channels(
                        config.as_ref(),
                        delivery.purpose,
                        &delivery.recipient.election_ids,
                    )
                    .contains(&channel)
                {
                    return Err(MessageReason::channel_not_offered(delivery.purpose).to_string());
                }
                let destination =
                    destination.map_err(|_| MessageReason::INVALID_DESTINATION.to_string())?;
                let content = delivery
                    .content
                    .clone()
                    .ok_or_else(|| MessageReason::NO_CONTENT_FOR_CHANNEL.to_string())?;
                let channel_config = config.as_ref().and_then(|c| c.channel(channel));
                // The approved template and the provider's code for its
                // language: the message's own, then the event's most
                // specific binding, then the account's declared approvals.
                let template: Option<ProviderTemplate> = delivery
                    .provider_template
                    .clone()
                    .or_else(|| {
                        config
                            .as_ref()
                            .and_then(|c| {
                                c.template_for(
                                    channel,
                                    delivery.purpose,
                                    delivery.template_key.as_deref(),
                                    delivery.language.as_deref(),
                                )
                            })
                            .map(|binding| ProviderTemplate {
                                name: binding.provider_template.clone(),
                                language: Some(binding.provider_language().to_string()),
                            })
                    })
                    .or_else(|| match account.as_ref().map(|a| &a.sender) {
                        Some(AccountSender::VIBER_INFOBIP {
                            approved_templates, ..
                        }) => approved_templates
                            .get(&delivery.purpose)
                            .and_then(|by_language| {
                                delivery
                                    .language
                                    .as_ref()
                                    .and_then(|language| by_language.get_key_value(language))
                                    .or_else(|| by_language.iter().next())
                                    .map(|(language, name)| ProviderTemplate {
                                        name: name.clone(),
                                        language: Some(language.clone()),
                                    })
                            }),
                        _ => None,
                    });
                let language = template
                    .as_ref()
                    .and_then(|t| t.language.clone())
                    .or_else(|| delivery.language.clone());
                let provider_template = template.map(|t| t.name);
                let (sender, last_get_started): (Box<dyn ChannelSender>, _) = match &account {
                    Some(account) => {
                        let runtime = runtime_account(&tx, account)
                            .await
                            .map_err(|e| e.to_string())?;
                        let sender =
                            providers::build(&runtime, &dispatcher.http, &dispatcher.endpoints)
                                .await
                                .map_err(|e| e.to_string())?;
                        let last = match channel {
                            MessageChannel::MESSENGER => {
                                last_get_started_at(&tx, &account.id, &digest)
                                    .await
                                    .map_err(|e| e.to_string())?
                            }
                            _ => None,
                        };
                        (sender, last)
                    }
                    None if matches!(channel, MessageChannel::EMAIL | MessageChannel::SMS) => {
                        (Box::new(EnvironmentSender { channel }), None)
                    }
                    None => return Err(MessageReason::NO_ACCOUNT_FOR_CHANNEL.to_string()),
                };
                let in_window = last_get_started
                    .is_some_and(|at| Utc::now() - at < Duration::hours(MESSENGER_WINDOW_HOURS));
                Ok((
                    sender,
                    OutboundMessage {
                        channel,
                        purpose: delivery.purpose,
                        destination,
                        language,
                        content,
                        provider_template,
                        idempotency_key: format!("{}:{attempt}", delivery.logical_key),
                        expires_at: delivery.expires_at,
                        last_inbound_at: last_get_started,
                        out_of_window: channel_config
                            .map(|c| c.effective_out_of_window())
                            .unwrap_or(OutOfWindowPolicy::DISABLED),
                    },
                    in_window,
                ))
            }
            .await;
        let refusal = match &prepared {
            Ok((sender, message, _)) => {
                let limits = account
                    .as_ref()
                    .map(|a| a.limits.clone())
                    .unwrap_or_default();
                preflight(&sender.capabilities(), &limits, message, Utc::now())
                    .err()
                    .map(|error| error.to_string())
            }
            Err(reason) => Some(reason.clone()),
        };
        if let Some(reason) = refusal {
            // Refused before the provider: one FAILED row, never another
            // channel.
            record.state = MessageAttemptState::FAILED;
            record.error = Some(reason);
            record.failure = Some(FailureKind::PERMANENT);
            let Some(failed) = insert_message(&tx, &record).await? else {
                tx.commit().await?;
                continue;
            };
            tx.commit().await?;
            info!(message_id = %failed.id, %channel, reason = ?failed.error, "message refused");
            let mut result = DeliveryResult::from_record(&failed);
            result.recorded = vec![attempt_outcome(&failed, &provider)];
            return Ok(result);
        }
        let Ok((sender, message, in_window)) = prepared else {
            return Err(anyhow!("unreachable: refusal handled above"));
        };
        record.billing_category = Some(BillingCategory::expected(
            channel,
            delivery.purpose,
            in_window,
        ));
        let Some(queued) = insert_message(&tx, &record).await? else {
            // Another worker recorded this attempt first: decide again.
            tx.commit().await?;
            continue;
        };
        tx.commit().await?;

        if let Some(account) = &account {
            dispatcher.limiter(account).acquire(delivery.purpose).await;
        }
        let outcome = if message
            .expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
        {
            // Never handed to the provider: `dispatched_at` stays empty.
            refused(MessageReason::CODE_EXPIRED_BEFORE_SENDING.as_str())
        } else {
            let tx = client.transaction().await?;
            let dispatching = mark_dispatched(&tx, &queued.id).await?;
            tx.commit().await?;
            if !dispatching {
                // The sweep found the attempt stale while it waited and
                // recorded it: it is not sent.
                let tx = client.transaction().await?;
                let current = get_message(&tx, &queued.id).await?;
                tx.commit().await?;
                return current
                    .map(|r| DeliveryResult::from_record(&r))
                    .ok_or_else(|| anyhow!("attempt disappeared"));
            }
            sender.send(&message).await
        };
        let change = match &outcome {
            SendOutcome::Accepted {
                provider_message_id,
            } => StateChange {
                provider_message_id: provider_message_id.clone(),
                ..Default::default()
            },
            SendOutcome::Rejected(failure) => {
                let mut error = match &failure.code {
                    Some(code) => format!("{} ({code})", failure.reason),
                    None => failure.reason.clone(),
                };
                let mut retry_at =
                    next_retry_at(delivery.purpose, attempt, failure.kind, Utc::now());
                if retry_at.is_some() && delivery.template_alias.is_none() {
                    // The ledger keeps no content: without a stored template
                    // there is nothing to send again.
                    retry_at = None;
                    error = MessageReason::NO_STORED_TEMPLATE.to_string();
                }
                StateChange {
                    error: Some(error),
                    failure: Some(failure.kind),
                    next_retry_at: retry_at,
                    ..Default::default()
                }
            }
            SendOutcome::Unknown { reason } => StateChange {
                error: Some(reason.clone()),
                ..Default::default()
            },
        };
        let tx = client.transaction().await?;
        let updated = transition_message(&tx, &queued.id, outcome.state(), &change).await?;
        let current = match updated {
            Some(updated) => updated,
            None => {
                warn!(message_id = %queued.id, "a report updated the attempt before its send finished");
                get_message(&tx, &queued.id)
                    .await?
                    .ok_or_else(|| anyhow!("attempt disappeared"))?
            }
        };
        tx.commit().await?;
        info!(message_id = %queued.id, %channel, state = %outcome.state(), "message attempt");
        let mut result = DeliveryResult::from_record(&current);
        // The attempt's own outcome, even when a fast report already moved
        // it on.
        let mut logged = attempt_outcome(&current, &provider);
        logged.outcome = outcome.state();
        logged.reason = change.error.clone();
        result.recorded = vec![logged];
        return Ok(result);
    }
    Err(anyhow!("too many attempts for {}", delivery.logical_key))
}
