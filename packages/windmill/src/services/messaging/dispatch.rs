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
    cancel_retry, get_bulk_send_payload, get_message, get_messaging_account, insert_message,
    last_get_started_at, list_attempts, list_messaging_accounts, mark_dispatched,
    transition_message, MessageRecord, MessagingAccount, NewMessage, StateChange,
};
use crate::services::providers::email_sender::{EmailSender, TransportFailure};
use crate::services::providers::sms_sender::SmsSender;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use deadpool_postgres::Client as DbClient;
use deadpool_postgres::Transaction;
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
    NoticeRecipient, OutOfWindowPolicy, ProviderCapabilities, RecipientKind,
};
use sequent_core::types::templates::SendTemplateBody;
use sequent_core::types::voter_channel::{VoterChannel, VoterChannelSource};
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
    /// `None` for a message recorded without a channel.
    pub channel: Option<MessageChannel>,
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

    /// Whether this call recorded a message as sent that wasn't before:
    /// what statistics count. A replay records nothing, so counts nothing.
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
                Ok(sender) => {
                    sender
                        .send_message(
                            vec![message.destination.as_str().to_string()],
                            message.content.subject.clone().unwrap_or_default(),
                            message.content.text.clone(),
                            message.content.html.clone(),
                            vec![],
                        )
                        .await
                }
                Err(_) => {
                    return refused("the email transport is not configured");
                }
            },
            MessageChannel::SMS => match SmsSender::new().await {
                Ok(sender) => {
                    sender
                        .send_message(
                            message.destination.as_str().to_string(),
                            message.content.text.clone(),
                        )
                        .await
                }
                Err(_) => {
                    return refused("the SMS transport is not configured");
                }
            },
            _ => return refused(MessageReason::NO_ACCOUNT_FOR_CHANNEL.as_str()),
        };
        transport_outcome(sent)
    }

    async fn check(&self) -> AccountCheck {
        AccountCheck {
            connected: true,
            production_access: true,
            ..Default::default()
        }
    }
}

/// What the ledger records for the environment transport's answer: a
/// definite refusal is FAILED (retried when transient, if the message's
/// origin allows it); only a lost answer is UNKNOWN, never sent again.
fn transport_outcome(sent: std::result::Result<(), TransportFailure>) -> SendOutcome {
    match sent {
        Ok(()) => SendOutcome::Accepted {
            provider_message_id: None,
        },
        Err(TransportFailure::Refused { transient, reason }) => {
            SendOutcome::Rejected(ProviderFailure {
                kind: if transient {
                    FailureKind::TRANSIENT
                } else {
                    FailureKind::PERMANENT
                },
                code: None,
                reason,
            })
        }
        Err(TransportFailure::Unknown { reason }) => SendOutcome::Unknown { reason },
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

/// A message refused before any attempt could be made, recorded as one
/// FAILED row with its reason so staff see it.
#[derive(Debug, Clone)]
pub struct Refusal {
    pub tenant_id: Uuid,
    pub election_event_id: Option<Uuid>,
    pub voter_id: Option<String>,
    /// `None` when the voter has no channel at all.
    pub channel: Option<MessageChannel>,
    pub destination: Option<String>,
    pub purpose: MessagePurpose,
    pub template_alias: Option<String>,
    pub language: Option<String>,
    pub logical_key: String,
    pub reason: MessageReason,
}

/// Records `refusal` as one FAILED, never retried row (masked
/// destination, or "-"). A replay records nothing new.
#[instrument(skip_all, fields(logical_key = %refusal.logical_key, reason = %refusal.reason), err)]
pub async fn record_refusal(client: &mut DbClient, refusal: &Refusal) -> Result<DeliveryResult> {
    let tx = client.transaction().await?;
    let masked = match (refusal.channel, refusal.destination.as_deref()) {
        (Some(channel), Some(destination)) => Destination::parse(channel, destination)
            .map(|d| d.masked())
            .unwrap_or_else(|_| "-".to_string()),
        _ => "-".to_string(),
    };
    let inserted = insert_message(
        &tx,
        &NewMessage {
            tenant_id: refusal.tenant_id,
            election_event_id: refusal.election_event_id,
            voter_id: refusal.voter_id.clone(),
            account_id: None,
            channel: refusal.channel,
            purpose: Some(refusal.purpose),
            template_alias: refusal.template_alias.clone(),
            language: refusal.language.clone(),
            masked_destination: masked,
            destination_digest: "-".to_string(),
            destination_country: None,
            logical_key: Some(refusal.logical_key.clone()),
            attempt: 1,
            state: MessageAttemptState::FAILED,
            provider_message_id: None,
            error: Some(refusal.reason.to_string()),
            failure: Some(FailureKind::PERMANENT),
            billing_category: None,
        },
    )
    .await?;
    let result = match inserted {
        Some(record) => {
            warn!(message_id = %record.id, "{}", refusal.reason);
            let mut result = DeliveryResult::from_record(&record);
            result.recorded = vec![attempt_outcome(&record, ENVIRONMENT_PROVIDER)];
            result
        }
        None => list_attempts(&tx, &refusal.tenant_id, &refusal.logical_key)
            .await?
            .last()
            .map(DeliveryResult::from_record)
            .ok_or_else(|| anyhow!("inconsistent attempts"))?,
    };
    tx.commit().await?;
    Ok(result)
}

/// Records a message that has no channel at all: one FAILED row with no
/// channel and [`MessageReason::NO_CONTACT_CHANNEL`], never counted as any
/// channel's failure. A replay records nothing new.
#[allow(clippy::too_many_arguments)]
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
    record_refusal(
        client,
        &Refusal {
            tenant_id,
            election_event_id,
            voter_id,
            channel: None,
            destination: None,
            purpose,
            template_alias,
            language,
            logical_key: logical_key.to_string(),
            reason: MessageReason::NO_CONTACT_CHANNEL,
        },
    )
    .await
}

/// The channel and destination a notice from Keycloak goes to, or why it
/// can't go anywhere, with the channel its FAILED row is recorded on.
///
/// - A notice about the previous channel goes where the request says.
/// - Otherwise, with the voter's record: the voter's channel. A request
///   naming another *recorded* channel is refused
///   ([`MessageReason::NOT_THE_VOTERS_CHANNEL`]); a legacy voter (email,
///   else SMS, never recorded) may still be sent on the channel the realm's
///   courier names.
/// - Without a record (no voter, or a voter with no channel): the channel
///   and destination the request names, or
///   [`MessageReason::NO_CONTACT_CHANNEL`].
pub fn notice_recipient(
    policy: NoticeRecipient,
    requested_channel: Option<MessageChannel>,
    requested_destination: Option<&str>,
    voter: Option<&VoterChannel>,
) -> std::result::Result<(MessageChannel, String), (Option<MessageChannel>, MessageReason)> {
    let requested_destination = requested_destination
        .map(str::trim)
        .filter(|destination| !destination.is_empty())
        .map(str::to_string);
    let as_requested = || match (requested_channel, requested_destination.clone()) {
        (Some(channel), Some(destination)) => Ok((channel, destination)),
        (channel, _) => Err((channel, MessageReason::NO_CONTACT_CHANNEL)),
    };
    let voter = match (policy, voter) {
        (NoticeRecipient::PREVIOUS_CHANNEL, _) | (_, None) => return as_requested(),
        (NoticeRecipient::VOTER_CHANNEL, Some(voter)) => voter,
    };
    match requested_channel {
        None => Ok((voter.channel, voter.destination.clone())),
        Some(channel) if channel == voter.channel => Ok((
            channel,
            requested_destination
                .clone()
                .unwrap_or_else(|| voter.destination.clone()),
        )),
        Some(_) if voter.source == VoterChannelSource::LEGACY => as_requested(),
        Some(channel) => Err((Some(channel), MessageReason::NOT_THE_VOTERS_CHANNEL)),
    }
}

/// The send ID of a bulk notice's logical key (`send-template:<id>:<voter>`).
pub fn bulk_send_id(logical_key: &str) -> Option<Uuid> {
    let rest = logical_key.strip_prefix("send-template:")?;
    let (send_id, _) = rest.split_once(':')?;
    Uuid::parse_str(send_id).ok()
}

/// What the administrator sent in the bulk send of `logical_key`, if it
/// was stored and can still be read.
pub async fn bulk_send_body(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    logical_key: &str,
) -> Result<Option<SendTemplateBody>> {
    let Some(send_id) = bulk_send_id(logical_key) else {
        return Ok(None);
    };
    let Some(payload) = get_bulk_send_payload(tx, tenant_id, &send_id).await? else {
        return Ok(None);
    };
    match serde_json::from_value::<SendTemplateBody>(payload) {
        Ok(body) => Ok(Some(body)),
        Err(error) => {
            warn!(%send_id, "unreadable bulk send payload: {error}");
            Ok(None)
        }
    }
}

/// Why a failed attempt of a message can't be sent again, by where the
/// message comes from; `None` when it can. The one rule for every place
/// that schedules or sends a retry:
///
/// - Bulk notices are rendered again from what the administrator sent (the
///   stored send), unless it used encrypted voter attributes
///   ([`MessageReason::PROTECTED_FIELDS`]) or is gone.
/// - Application results are rendered again from their template reference.
/// - Notices Keycloak sends (reset links, temporary credentials, other
///   notices), codes and test messages are never retried by the server:
///   their content isn't stored. The voter asks again.
pub async fn retry_refusal(
    tx: &Transaction<'_>,
    tenant_id: &Uuid,
    logical_key: Option<&str>,
    template_alias: Option<&str>,
) -> Result<Option<MessageReason>> {
    let Some(logical_key) = logical_key else {
        return Ok(Some(MessageReason::NO_STORED_TEMPLATE));
    };
    Ok(match MessageKind::of(logical_key) {
        MessageKind::BULK_NOTICE => match bulk_send_body(tx, tenant_id, logical_key).await? {
            None => Some(MessageReason::NO_STORED_TEMPLATE),
            Some(body) if !body.secret_attribute_names.is_empty() => {
                Some(MessageReason::PROTECTED_FIELDS)
            }
            Some(_) => None,
        },
        MessageKind::APPLICATION_RESULT if template_alias.is_some() => None,
        _ => Some(MessageReason::NO_STORED_TEMPLATE),
    })
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
            Dispatch::Send { attempt, channel }
                if attempt > 1 && channel != delivery.recipient.channel =>
            {
                // A replay found the retry due, but the voter changed
                // channel since: never the old channel to the new
                // destination, nor the message on another channel.
                let latest = latest.ok_or_else(|| anyhow!("inconsistent attempts"))?;
                let current = cancel_retry(&tx, &latest.id, MessageReason::CHANNEL_CHANGED)
                    .await?
                    .unwrap_or_else(|| latest.clone());
                tx.commit().await?;
                info!(message_id = %current.id, "{}", MessageReason::CHANNEL_CHANGED);
                return Ok(DeliveryResult::from_record(&current));
            }
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
            channel: Some(channel),
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
        let tx = client.transaction().await?;
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
                if retry_at.is_some() {
                    // The ledger keeps no content: only a message whose
                    // origin can render it again is retried.
                    if let Some(reason) = retry_refusal(
                        &tx,
                        &delivery.tenant_id,
                        Some(&delivery.logical_key),
                        delivery.template_alias.as_deref(),
                    )
                    .await?
                    {
                        retry_at = None;
                        error = reason.not_retried_after(Some(&error));
                    }
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
        // The attempt's entry says what the ledger says, also when a fast
        // report moved it on before the send's answer was recorded; and
        // statistics count it only when the ledger says it was sent.
        result.recorded = vec![attempt_outcome(&current, &provider)];
        return Ok(result);
    }
    Err(anyhow!("too many attempts for {}", delivery.logical_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voter(
        channel: MessageChannel,
        destination: &str,
        source: VoterChannelSource,
    ) -> VoterChannel {
        VoterChannel {
            channel,
            destination: destination.to_string(),
            messenger_page: None,
            source,
        }
    }

    #[test]
    fn a_notice_without_a_channel_goes_to_the_voters_one_channel() {
        let whatsapp = voter(
            MessageChannel::WHATSAPP,
            "+639181234567",
            VoterChannelSource::RECORDED,
        );
        assert_eq!(
            notice_recipient(NoticeRecipient::VOTER_CHANNEL, None, None, Some(&whatsapp)),
            Ok((MessageChannel::WHATSAPP, "+639181234567".to_string()))
        );
        // No record, and nothing named: no contact channel, no channel.
        assert_eq!(
            notice_recipient(NoticeRecipient::VOTER_CHANNEL, None, None, None),
            Err((None, MessageReason::NO_CONTACT_CHANNEL))
        );
    }

    #[test]
    fn a_notice_on_another_recorded_channel_is_refused() {
        let whatsapp = voter(
            MessageChannel::WHATSAPP,
            "+639181234567",
            VoterChannelSource::RECORDED,
        );
        assert_eq!(
            notice_recipient(
                NoticeRecipient::VOTER_CHANNEL,
                Some(MessageChannel::SMS),
                Some("+639171234567"),
                Some(&whatsapp)
            ),
            Err((
                Some(MessageChannel::SMS),
                MessageReason::NOT_THE_VOTERS_CHANNEL
            ))
        );
        // The same channel keeps the destination the request names.
        assert_eq!(
            notice_recipient(
                NoticeRecipient::VOTER_CHANNEL,
                Some(MessageChannel::WHATSAPP),
                Some("+639181234567"),
                Some(&whatsapp)
            ),
            Ok((MessageChannel::WHATSAPP, "+639181234567".to_string()))
        );
        // The notice that the channel changed goes to the previous one.
        assert_eq!(
            notice_recipient(
                NoticeRecipient::PREVIOUS_CHANNEL,
                Some(MessageChannel::SMS),
                Some("+639171234567"),
                Some(&whatsapp)
            ),
            Ok((MessageChannel::SMS, "+639171234567".to_string()))
        );
    }

    #[test]
    fn a_legacy_voter_keeps_the_realms_courier() {
        let legacy = voter(
            MessageChannel::EMAIL,
            "voter@example.org",
            VoterChannelSource::LEGACY,
        );
        assert_eq!(
            notice_recipient(
                NoticeRecipient::VOTER_CHANNEL,
                Some(MessageChannel::SMS),
                Some("+639171234567"),
                Some(&legacy)
            ),
            Ok((MessageChannel::SMS, "+639171234567".to_string()))
        );
    }

    #[test]
    fn the_environment_transport_fails_definite_refusals_and_only_loses_lost_answers() {
        let refusal = |transient| {
            transport_outcome(Err(TransportFailure::Refused {
                transient,
                reason: "refused by the mail server (550)".to_string(),
            }))
        };
        assert!(matches!(
            refusal(true),
            SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::TRANSIENT,
                ..
            })
        ));
        assert!(matches!(
            refusal(false),
            SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::PERMANENT,
                ..
            })
        ));
        assert!(matches!(
            transport_outcome(Err(TransportFailure::Unknown {
                reason: "the provider's answer was lost".to_string()
            })),
            SendOutcome::Unknown { .. }
        ));
        assert!(matches!(
            transport_outcome(Ok(())),
            SendOutcome::Accepted { .. }
        ));
    }

    #[test]
    fn bulk_send_ids_come_from_the_logical_key() {
        let id = Uuid::new_v4();
        assert_eq!(
            bulk_send_id(&format!("send-template:{id}:voter-1")),
            Some(id)
        );
        assert_eq!(bulk_send_id("send-template:celery-task:voter-1"), None);
        assert_eq!(bulk_send_id("keycloak:x"), None);
    }
}
