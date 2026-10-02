// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending one logical message: choosing the channel and account, recording
//! the attempt before it leaves Step, and recording what the provider said.

use super::accounts::runtime_account;
use super::config::get_event_messaging_config;
use super::keys::{tenant_key, TenantKey};
use crate::postgres::messaging::{
    get_messaging_account, insert_message, last_inbound_at, list_attempts, list_messaging_accounts,
    transition_message, MessageRecord, MessagingAccount, NewMessage, StateChange,
};
use crate::services::providers::email_sender::EmailSender;
use crate::services::providers::sms_sender::SmsSender;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Client as DbClient;
use messaging::attempts::{decide, AttemptRecord, Dispatch, NoticeRoute};
use messaging::destination::Destination;
use messaging::providers::{self, Endpoints};
use messaging::rate_limit::RateLimiter;
use messaging::sender::{
    preflight, ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome,
};
use sequent_core::types::messaging::{
    AccountCheck, EventMessagingConfig, MessageAttemptState, MessageChannel, MessageContent,
    MessageDirection, MessagePurpose, MessagingProvider, OutOfWindowPolicy, ProviderCapabilities,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;
use tracing::{info, instrument, warn};
use uuid::Uuid;

/// Whether a notice may move to another channel after a confirmed failure.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackPolicy {
    /// Only the first channel.
    NONE,
    /// The event's notice fallback order, among the voter's verified
    /// channels.
    EVENT_ORDER,
}

/// Who the message goes to.
#[derive(Debug, Clone, PartialEq)]
pub struct Recipient {
    pub voter_id: Option<String>,
    /// The address on each channel the recipient can be reached on.
    pub destinations: BTreeMap<MessageChannel, String>,
    /// Channels whose address the voter verified.
    pub verified: Vec<MessageChannel>,
    /// The voter's elections, for per-election channel restrictions.
    pub election_ids: Vec<String>,
}

/// One logical message.
#[derive(Debug, Clone)]
pub struct Delivery {
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub purpose: MessagePurpose,
    pub first_channel: MessageChannel,
    pub fallback: FallbackPolicy,
    pub recipient: Recipient,
    /// Rendered content per channel.
    pub contents: BTreeMap<MessageChannel, MessageContent>,
    pub language: Option<String>,
    pub template_alias: Option<String>,
    /// Stable across retries of the same message.
    pub logical_key: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeliveryResult {
    pub message_id: Option<Uuid>,
    pub channel: Option<MessageChannel>,
    pub state: MessageAttemptState,
    /// Short, non-sensitive reason when not accepted.
    pub reason: Option<String>,
    /// Retry the same channel after this long.
    pub retry_after: Option<Duration>,
}

impl DeliveryResult {
    fn from_record(record: &MessageRecord) -> DeliveryResult {
        DeliveryResult {
            message_id: Some(record.id),
            channel: Some(record.channel),
            state: record.state,
            reason: record.error.clone(),
            retry_after: None,
        }
    }
}

/// Shared by every delivery of a process: HTTP clients and per-account
/// rates.
pub struct Dispatcher {
    pub endpoints: Endpoints,
    http: reqwest::Client,
    limiters: Mutex<HashMap<Uuid, Arc<RateLimiter>>>,
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
            .get_or_try_init(|| async { Dispatcher::new(Endpoints::default()) })
            .await
    }

    fn limiter(&self, account: &MessagingAccount) -> Arc<RateLimiter> {
        let Ok(mut limiters) = self.limiters.lock() else {
            return Arc::new(RateLimiter::new(&account.limits));
        };
        limiters
            .entry(account.id)
            .or_insert_with(|| Arc::new(RateLimiter::new(&account.limits)))
            .clone()
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
        MessagingProvider::CONSOLE
            .capabilities(self.channel)
            .unwrap_or_else(|| unreachable!("the console accepts every channel"))
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
            _ => return refused(format!("no environment transport for {}", self.channel)),
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

/// Channels the event offers for `purpose` to this recipient. Without an
/// event configuration, email and SMS stay available as before.
fn eligible_channels(
    config: Option<&EventMessagingConfig>,
    purpose: MessagePurpose,
    recipient: &Recipient,
) -> Vec<MessageChannel> {
    match config {
        Some(config) => {
            let mut eligible = config.enabled_channels(purpose, None);
            for election_id in &recipient.election_ids {
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

/// Sends `delivery`, following the attempt decisions until the message is
/// accepted, its outcome is unknown, or there is nothing more to try.
#[instrument(skip_all, fields(logical_key = %delivery.logical_key, purpose = %delivery.purpose), err)]
pub async fn deliver(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    delivery: &Delivery,
) -> Result<DeliveryResult> {
    // Every pass records an attempt, so this only bounds a misbehaving loop.
    for _ in 0..16 {
        let tx = client.transaction().await?;
        let config = match &delivery.election_event_id {
            Some(event) => get_event_messaging_config(&tx, &delivery.tenant_id, event).await?,
            None => None,
        };
        let records = list_attempts(&tx, &delivery.tenant_id, &delivery.logical_key).await?;
        let eligible = eligible_channels(config.as_ref(), delivery.purpose, &delivery.recipient);
        let fallback = match (delivery.fallback, &config) {
            (FallbackPolicy::EVENT_ORDER, Some(config)) => config.notice_fallback.clone(),
            _ => vec![],
        };
        let route = NoticeRoute {
            first: delivery.first_channel,
            fallback: &fallback,
            verified: &delivery.recipient.verified,
            eligible: &eligible,
        };
        let latest = records.iter().max_by_key(|r| r.attempt);
        let (attempt, channel) = match decide(delivery.purpose, &route, &attempt_records(&records))
        {
            Dispatch::Send { attempt, channel } => (attempt, channel),
            Dispatch::AlreadySent | Dispatch::AwaitReconciliation => {
                let current = records
                    .iter()
                    .find(|r| {
                        matches!(
                            r.state,
                            MessageAttemptState::ACCEPTED
                                | MessageAttemptState::DELIVERED
                                | MessageAttemptState::UNKNOWN
                                | MessageAttemptState::QUEUED
                        )
                    })
                    .map(DeliveryResult::from_record);
                tx.commit().await?;
                return current.ok_or_else(|| anyhow!("inconsistent attempts"));
            }
            Dispatch::RetryLater { after } => {
                tx.commit().await?;
                let mut result = latest
                    .map(DeliveryResult::from_record)
                    .ok_or_else(|| anyhow!("retry without attempts"))?;
                result.retry_after = Some(after);
                return Ok(result);
            }
            Dispatch::GiveUp => {
                tx.commit().await?;
                return Ok(latest
                    .map(DeliveryResult::from_record)
                    .unwrap_or(DeliveryResult {
                        message_id: None,
                        channel: None,
                        state: MessageAttemptState::FAILED,
                        reason: Some("no eligible channel".to_string()),
                        retry_after: None,
                    }));
            }
        };

        let account = account_for(&tx, &delivery.tenant_id, config.as_ref(), channel).await?;
        let destination_key = tenant_key(&tx, &delivery.tenant_id, TenantKey::Destination).await?;
        let raw_destination = delivery.recipient.destinations.get(&channel);
        let destination = raw_destination.map(|raw| Destination::parse(channel, raw));
        let (masked, digest, country) = match &destination {
            Some(Ok(destination)) => (
                destination.masked(),
                destination.digest(&destination_key)?,
                destination.calling_code().map(str::to_string),
            ),
            _ => ("-".to_string(), "-".to_string(), None),
        };
        let mut record = NewMessage {
            tenant_id: delivery.tenant_id,
            election_event_id: delivery.election_event_id,
            voter_id: delivery.recipient.voter_id.clone(),
            account_id: account.as_ref().map(|a| a.id),
            channel,
            direction: MessageDirection::OUTBOUND,
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
        };

        let prepared: std::result::Result<(Box<dyn ChannelSender>, OutboundMessage), String> =
            async {
                let destination = match destination {
                    Some(Ok(destination)) => destination,
                    Some(Err(error)) => return Err(error.to_string()),
                    None => return Err(format!("no {channel} address")),
                };
                let content = delivery
                    .contents
                    .get(&channel)
                    .cloned()
                    .ok_or_else(|| format!("no {channel} content"))?;
                let channel_config = config.as_ref().and_then(|c| c.channel(channel));
                let provider_template = channel_config.and_then(|c| {
                    c.templates
                        .iter()
                        .find(|t| {
                            t.purpose == delivery.purpose
                                && Some(&t.language) == delivery.language.as_ref()
                        })
                        .map(|t| t.provider_template.clone())
                });
                let (sender, last_inbound): (Box<dyn ChannelSender>, _) = match &account {
                    Some(account) => {
                        let runtime = runtime_account(&tx, account)
                            .await
                            .map_err(|e| e.to_string())?;
                        let sender =
                            providers::build(&runtime, &dispatcher.http, &dispatcher.endpoints)
                                .await
                                .map_err(|e| e.to_string())?;
                        let last = last_inbound_at(&tx, &account.id, &digest)
                            .await
                            .map_err(|e| e.to_string())?;
                        (sender, last)
                    }
                    None if matches!(channel, MessageChannel::EMAIL | MessageChannel::SMS) => {
                        (Box::new(EnvironmentSender { channel }), None)
                    }
                    None => return Err(format!("no {channel} account")),
                };
                Ok((
                    sender,
                    OutboundMessage {
                        channel,
                        purpose: delivery.purpose,
                        destination,
                        language: delivery.language.clone(),
                        content,
                        provider_template,
                        idempotency_key: format!("{}:{attempt}", delivery.logical_key),
                        expires_at: delivery.expires_at,
                        last_inbound_at: last_inbound,
                        out_of_window: channel_config
                            .map(|c| c.out_of_window)
                            .unwrap_or(OutOfWindowPolicy::DISABLED),
                    },
                ))
            }
            .await;
        let (sender, message) = match prepared {
            Ok(prepared) => prepared,
            Err(reason) => {
                record.state = MessageAttemptState::FAILED;
                record.error = Some(reason);
                record.failure = Some(FailureKind::PERMANENT);
                insert_message(&tx, &record).await?;
                tx.commit().await?;
                continue;
            }
        };
        let limits = account
            .as_ref()
            .map(|a| a.limits.clone())
            .unwrap_or_default();
        if let Err(error) = preflight(&sender.capabilities(), &limits, &message, Utc::now()) {
            record.state = MessageAttemptState::FAILED;
            record.error = Some(error.to_string());
            record.failure = Some(FailureKind::PERMANENT);
            insert_message(&tx, &record).await?;
            tx.commit().await?;
            continue;
        }
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
            refused("the code expired before it could be sent")
        } else {
            sender.send(&message).await
        };
        let change = match &outcome {
            SendOutcome::Accepted {
                provider_message_id,
            } => StateChange {
                provider_message_id: provider_message_id.clone(),
                ..Default::default()
            },
            SendOutcome::Rejected(failure) => StateChange {
                error: Some(match &failure.code {
                    Some(code) => format!("{} ({code})", failure.reason),
                    None => failure.reason.clone(),
                }),
                failure: Some(failure.kind),
                ..Default::default()
            },
            SendOutcome::Unknown { reason } => StateChange {
                error: Some(reason.clone()),
                ..Default::default()
            },
        };
        let tx = client.transaction().await?;
        let updated = transition_message(&tx, &queued.id, outcome.state(), &change).await?;
        tx.commit().await?;
        if updated.is_none() {
            warn!(message_id = %queued.id, "a report updated the attempt before its send finished");
        }
        info!(message_id = %queued.id, %channel, state = %outcome.state(), "message attempt");
        match outcome {
            SendOutcome::Accepted { .. } | SendOutcome::Unknown { .. } => {
                let tx = client.transaction().await?;
                let latest = list_attempts(&tx, &delivery.tenant_id, &delivery.logical_key)
                    .await?
                    .into_iter()
                    .find(|r| r.id == queued.id)
                    .map(|r| DeliveryResult::from_record(&r));
                tx.commit().await?;
                return latest.ok_or_else(|| anyhow!("attempt disappeared"));
            }
            SendOutcome::Rejected(_) => continue,
        }
    }
    Err(anyhow!("too many attempts for {}", delivery.logical_key))
}
