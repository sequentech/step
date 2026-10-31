// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Sending one logical message on the recipient's one channel: choosing the
//! account, recording the attempt before it leaves Step, and recording what
//! the provider said. Nothing here ever tries another channel: a message
//! that can't go out on its channel gets one FAILED row with its reason.

use super::accounts::{not_ready_reason, runtime_account, send_readiness};
use super::audit::{AttemptOutcome, MessageKind, ENVIRONMENT_PROVIDER};
use super::config::get_event_messaging_config;
use super::keys::{tenant_key, TenantKey};
use crate::postgres::messaging::{
    cancel_retry, get_message, get_messaging_account, insert_message, last_get_started_at,
    list_attempts, list_messaging_accounts, mark_dispatched, transition_message, MessageRecord,
    MessagingAccount, NewMessage, StateChange,
};
use crate::postgres::rate_bucket::{self, Take};
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
use messaging::rate_limit::BucketShare;
use messaging::sender::{
    preflight, ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome,
};
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, BillingCategory, DeliveryFeedback, EventMessagingConfig,
    MessageAttemptState, MessageChannel, MessageContent, MessagePurpose, MessageReason,
    NoticeRecipient, OutOfWindowPolicy, ProviderCapabilities, RecipientKind, RETRY_PREPARE_HOURS,
};
use sequent_core::types::templates::SendTemplateBody;
use sequent_core::types::voter_channel::{VoterChannel, VoterChannelSource};
use tokio::sync::OnceCell;
use tokio::time::Instant;
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
    /// Messenger: the Page the Page-scoped ID belongs to, from the voter's
    /// record (or the Page whose conversation bound a code's link). A send
    /// through an account of another Page is refused
    /// ([`MessageReason::MESSENGER_PAGE_CHANGED`]). `None` when the sender
    /// doesn't know it (a request naming a destination with no voter
    /// record): not checked.
    pub messenger_page: Option<String>,
}

impl Recipient {
    /// The voter's one channel ([`VoterChannel`]), with their Messenger
    /// Page: what every send to a voter's record uses.
    pub fn of_voter(
        voter_id: Option<String>,
        voter: VoterChannel,
        election_ids: Vec<String>,
    ) -> Recipient {
        Recipient {
            voter_id,
            channel: voter.channel,
            destination: voter.destination,
            election_ids,
            messenger_page: voter.messenger_page,
        }
    }
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

/// Shared by every delivery of a process: provider endpoints and the HTTP
/// client. Sending rates are per account and shared by every process, in
/// Postgres ([`rate_bucket`]).
pub struct Dispatcher {
    pub endpoints: Endpoints,
    http: reqwest::Client,
}

/// The share of its account's bucket a delivery takes.
#[allow(non_camel_case_types)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ShareRule {
    /// By purpose ([`BucketShare::of`]): codes the whole bucket, notices
    /// only above the account's OTP reserve. Windmill's bulk sends and
    /// retries.
    #[default]
    BY_PURPOSE,
    /// The whole bucket, whatever the purpose: a voter is waiting.
    /// Everything Keycloak asks harvest to send, codes and notices.
    WHOLE_BUCKET,
}

/// The caller's place for the delivery among its account's messages in
/// flight (harvest's places, taken before any database connection).
#[allow(non_camel_case_types)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    /// The send isn't limited by places: windmill's sends.
    #[default]
    NOT_REQUIRED,
    /// The caller holds a place until this call returns.
    HELD,
    /// No place came free within the budget: the attempt isn't sent and
    /// takes no token ([`rate_bucket::CAPACITY_BUSY`]).
    NOT_GRANTED,
}

/// What a due retry does when it can't be sent yet: its account isn't
/// ready, or has no capacity before the deadline.
#[allow(non_camel_case_types)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum WhenNotSendable {
    /// The attempt is recorded FAILED with the reason, a transient
    /// failure: the retry rules then apply as for any other.
    #[default]
    RECORD_FAILED,
    /// The sweep's retries: no new attempt. The failed attempt stays due,
    /// [`RETRY_WAIT_MINUTES`] later, with a log line saying what it waits
    /// for; after [`RETRY_PREPARE_HOURS`] of waiting it isn't retried
    /// ([`MessageReason::RETRY_NOT_PREPARED`]). First attempts are always
    /// recorded.
    KEEP_RETRY_DUE,
}

/// Whether the account's readiness is checked before sending.
#[allow(non_camel_case_types)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessGate {
    /// Every send through an account: an account that isn't ready for the
    /// purpose sends nothing ([`not_ready_reason`]).
    #[default]
    CHECKED,
    /// An administrator's test message through a chosen account: the test
    /// is how an account is tried before it is ready.
    NOT_CHECKED_FOR_TEST_MESSAGE,
}

/// How a delivery may wait and what it does when it can't be sent yet.
/// The default is windmill's bulk sends: readiness checked, the share by
/// purpose, waiting for capacity as long as needed, recorded FAILED when
/// the account isn't ready.
#[derive(Debug, Default, Clone, Copy)]
pub struct SendPolicy {
    /// When waiting for the account's capacity ends. The attempt is then
    /// not sent ([`rate_bucket::CAPACITY_BUSY`]). `None`: no limit.
    pub capacity_deadline: Option<Instant>,
    pub share: ShareRule,
    pub place: Place,
    pub when_not_sendable: WhenNotSendable,
    pub readiness: ReadinessGate,
    /// When the provider's answer is no longer awaited: the attempt is
    /// then UNKNOWN ([`PROVIDER_ANSWER_LATE`]), since it may have been
    /// sent. `None`: the provider client's own timeout.
    pub answer_by: Option<Instant>,
}

/// The reason of an attempt whose provider didn't answer by the caller's
/// deadline ([`SendPolicy::answer_by`]).
pub const PROVIDER_ANSWER_LATE: &str =
    "The provider didn't answer in time: the message may have been sent";

/// How much later a waiting retry is due again ([`WhenNotSendable`]).
pub const RETRY_WAIT_MINUTES: i64 = 1;

/// Takes one token of `account`'s bucket as `policy` allows: `false` when
/// no place was granted or the deadline passed first.
async fn take_capacity(
    client: &DbClient,
    account: &MessagingAccount,
    purpose: MessagePurpose,
    policy: &SendPolicy,
) -> Result<bool> {
    if policy.place == Place::NOT_GRANTED {
        return Ok(false);
    }
    let share = match policy.share {
        ShareRule::BY_PURPOSE => BucketShare::of(purpose),
        ShareRule::WHOLE_BUCKET => BucketShare::WHOLE,
    };
    let taken = rate_bucket::take(
        client,
        &account.id,
        &account.limits,
        share,
        policy.capacity_deadline,
    )
    .await?;
    Ok(match taken {
        Take::Unthrottled | Take::Taken => true,
        Take::DeadlinePassed => false,
    })
}

static DISPATCHER: OnceCell<Dispatcher> = OnceCell::const_new();

impl Dispatcher {
    pub fn new(endpoints: Endpoints) -> Result<Dispatcher> {
        Ok(Dispatcher {
            endpoints,
            http: providers::http_client()?,
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

/// The account the event's configuration names for a channel doesn't
/// exist (deleted since, or not an id): nothing goes out on the channel,
/// never through the deployment's own email or SMS sender. Shown as
/// [`MessageReason::NO_ACCOUNT_FOR_CHANNEL`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredAccountMissing {
    pub channel: MessageChannel,
    pub account_id: String,
}

impl std::fmt::Display for ConfiguredAccountMissing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(MessageReason::NO_ACCOUNT_FOR_CHANNEL.as_str())
    }
}

impl std::error::Error for ConfiguredAccountMissing {}

/// The account for `channel`: the event's choice, otherwise the tenant
/// default. `Ok(None)` only when neither exists: email and SMS then use the
/// deployment's sender. An account the configuration names that doesn't
/// exist is an error, [`ConfiguredAccountMissing`] (downcast it), never
/// `None`.
pub async fn account_for(
    tx: &deadpool_postgres::Transaction<'_>,
    tenant_id: &Uuid,
    config: Option<&EventMessagingConfig>,
    channel: MessageChannel,
) -> Result<Option<MessagingAccount>> {
    if let Some(channel_config) = config.and_then(|c| c.channel(channel)) {
        let missing = || ConfiguredAccountMissing {
            channel,
            account_id: channel_config.account_id.clone(),
        };
        let Ok(id) = Uuid::parse_str(&channel_config.account_id) else {
            return Err(missing().into());
        };
        return match get_messaging_account(tx, tenant_id, &id).await? {
            Some(account) => Ok(Some(account)),
            None => Err(missing().into()),
        };
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

/// Where a notice from Keycloak goes: the channel, the destination and,
/// for Messenger, the Page the destination belongs to when known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeTarget {
    pub channel: MessageChannel,
    pub destination: String,
    pub messenger_page: Option<String>,
}

/// The channel and destination a notice from Keycloak goes to, or why it
/// can't go anywhere, with the channel its FAILED row is recorded on.
///
/// - A notice about the previous channel goes where the request says.
/// - Otherwise, with the voter's record: the voter's channel and the
///   record's destination (and Messenger Page), whatever destination the
///   request names (R7-23: a different one is logged and not used). A
///   request naming another *recorded* channel is refused
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
) -> std::result::Result<NoticeTarget, (Option<MessageChannel>, MessageReason)> {
    let requested_destination = requested_destination
        .map(str::trim)
        .filter(|destination| !destination.is_empty())
        .map(str::to_string);
    let as_requested = || match (requested_channel, requested_destination.clone()) {
        (Some(channel), Some(destination)) => Ok(NoticeTarget {
            channel,
            destination,
            messenger_page: None,
        }),
        (channel, _) => Err((channel, MessageReason::NO_CONTACT_CHANNEL)),
    };
    let voter = match (policy, voter) {
        (NoticeRecipient::PREVIOUS_CHANNEL, _) | (_, None) => return as_requested(),
        (NoticeRecipient::VOTER_CHANNEL, Some(voter)) => voter,
    };
    let recorded = || {
        if requested_destination
            .as_deref()
            .is_some_and(|requested| requested != voter.destination.trim())
        {
            warn!(
                channel = %voter.channel,
                source = %voter.source,
                "{RECORDED_DESTINATION_USED}"
            );
        }
        Ok(NoticeTarget {
            channel: voter.channel,
            destination: voter.destination.clone(),
            messenger_page: voter.messenger_page.clone(),
        })
    };
    match requested_channel {
        None => recorded(),
        Some(channel) if channel == voter.channel => recorded(),
        Some(_) if voter.source == VoterChannelSource::LEGACY => as_requested(),
        Some(channel) => Err((Some(channel), MessageReason::NOT_THE_VOTERS_CHANNEL)),
    }
}

/// Logged when a notice on the voter's channel names another destination
/// than the voter's record: the record's is used (R7-23). Neither
/// destination is logged.
pub const RECORDED_DESTINATION_USED: &str =
    "The notice names another destination than the voter's record: the record's is used";

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
    match crate::services::participation_authorization::authorized_bulk_send(
        tx, tenant_id, &send_id,
    )
    .await
    {
        Ok(command) => Ok(command.map(|command| command.body)),
        Err(error) => {
            if error.downcast_ref::<tokio_postgres::Error>().is_some()
                || error.downcast_ref::<crate::services::participation_authorization::ParticipationReauthorization>().is_some() {
                return Err(error);
            }
            warn!(%send_id, "bulk send requires reauthorization or repair: {error}");
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
        MessageKind::BULK_NOTICE => match bulk_send_body(tx, tenant_id, logical_key).await {
            Err(error) if error.downcast_ref::<crate::services::participation_authorization::ParticipationReauthorization>().is_some() =>
                Some(MessageReason::PARTICIPATION_REAUTHORIZATION_REQUIRED),
            Err(error) => return Err(error),
            Ok(None) => Some(MessageReason::NO_STORED_TEMPLATE),
            Ok(Some(body)) if !body.secret_attribute_names.is_empty() => {
                Some(MessageReason::PROTECTED_FIELDS)
            }
            Ok(Some(_)) => None,
        },
        MessageKind::APPLICATION_RESULT if template_alias.is_some() => None,
        _ => Some(MessageReason::NO_STORED_TEMPLATE),
    })
}

/// Sends `delivery` on its recipient's channel, unless an earlier attempt
/// was accepted, may have reached the provider, or failed for good, or a
/// retry isn't due yet. A transient failure schedules the next attempt on
/// the same channel; the retry sweep sends it. Windmill's policy: see
/// [`SendPolicy`]'s default.
pub async fn deliver(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    delivery: &Delivery,
) -> Result<DeliveryResult> {
    deliver_within(client, dispatcher, delivery, SendPolicy::default()).await
}

/// The account `delivery` goes through on `channel`: the chosen one (on
/// its channel only), else the event's choice, else the tenant's default.
/// `Err(reason)`: the chosen or configured account doesn't exist (or is
/// for another channel), so nothing is sent, never through the
/// deployment's sender.
async fn delivery_account(
    tx: &Transaction<'_>,
    delivery: &Delivery,
    config: Option<&EventMessagingConfig>,
    channel: MessageChannel,
) -> Result<std::result::Result<Option<MessagingAccount>, MessageReason>> {
    match delivery.account_id {
        Some(account_id) => Ok(get_messaging_account(tx, &delivery.tenant_id, &account_id)
            .await?
            .filter(|account| account.channel == channel)
            .map(Some)
            .ok_or(MessageReason::NO_ACCOUNT_FOR_CHANNEL)),
        None => match account_for(tx, &delivery.tenant_id, config, channel).await {
            Ok(account) => Ok(Ok(account)),
            Err(error) if error.is::<ConfiguredAccountMissing>() => {
                Ok(Err(MessageReason::NO_ACCOUNT_FOR_CHANNEL))
            }
            Err(error) => Err(error),
        },
    }
}

/// The account a first attempt of `delivery` would go through: what
/// harvest takes a place on before it takes a connection for the send.
/// `None` for the environment's email and SMS.
pub async fn account_for_delivery(
    client: &mut DbClient,
    delivery: &Delivery,
) -> Result<Option<MessagingAccount>> {
    let tx = client.transaction().await?;
    let config = match &delivery.election_event_id {
        Some(event) => get_event_messaging_config(&tx, &delivery.tenant_id, event).await?,
        None => None,
    };
    let account =
        delivery_account(&tx, delivery, config.as_ref(), delivery.recipient.channel).await?;
    tx.commit().await?;
    // A missing account is refused by the send itself.
    Ok(account.unwrap_or(None))
}

/// Keeps the due retry `latest` waiting for `waiting_for` (no new
/// attempt): due again [`RETRY_WAIT_MINUTES`] later, or, after
/// [`RETRY_PREPARE_HOURS`] since it failed, not retried
/// ([`MessageReason::RETRY_NOT_PREPARED`]).
async fn keep_retry_due(
    client: &mut DbClient,
    latest: &MessageRecord,
    waiting_for: &str,
) -> Result<DeliveryResult> {
    let tx = client.transaction().await?;
    let now = Utc::now();
    let current = if now - latest.created_at >= Duration::hours(RETRY_PREPARE_HOURS) {
        info!(message_id = %latest.id, %waiting_for, "retry waited too long: not retried");
        cancel_retry(&tx, &latest.id, MessageReason::RETRY_NOT_PREPARED).await?
    } else {
        let due = now + Duration::minutes(RETRY_WAIT_MINUTES);
        info!(message_id = %latest.id, %waiting_for, %due, "retry waiting for capacity or readiness: still due");
        tx.query_opt(
            r#"
            UPDATE sequent_backend.message
            SET next_retry_at = $2, updated_at = now()
            WHERE id = $1 AND state = 'FAILED' AND next_retry_at IS NOT NULL
            RETURNING *
            "#,
            &[&latest.id, &due],
        )
        .await?
        .map(MessageRecord::try_from)
        .transpose()?
    };
    tx.commit().await?;
    Ok(DeliveryResult::from_record(
        current.as_ref().unwrap_or(latest),
    ))
}

/// The error and retry of an attempt that failed with `failure`: a
/// transient failure is retried on the same channel when its purpose and
/// origin allow it; otherwise why not is recorded next to the failure.
async fn failure_change(
    tx: &Transaction<'_>,
    delivery: &Delivery,
    attempt: i32,
    failure: &ProviderFailure,
) -> Result<StateChange> {
    let mut error = match &failure.code {
        Some(code) => format!("{} ({code})", failure.reason),
        None => failure.reason.clone(),
    };
    let mut retry_at = next_retry_at(delivery.purpose, attempt, failure.kind, Utc::now());
    if retry_at.is_some() {
        // The ledger keeps no content: only a message whose origin can
        // render it again is retried.
        if let Some(reason) = retry_refusal(
            tx,
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
    Ok(StateChange {
        error: Some(error),
        failure: Some(failure.kind),
        next_retry_at: retry_at,
        ..Default::default()
    })
}

/// R7-02: a Messenger send through an account whose Page isn't the one
/// the voter's Page-scoped ID belongs to. The ID can't reach the voter
/// through another Page, so nothing is handed to the provider:
/// [`MessageReason::MESSENGER_PAGE_CHANGED`]. A recipient whose Page isn't
/// known (`None`) isn't checked.
fn page_refusal(
    channel: MessageChannel,
    recipient: &Recipient,
    account: Option<&MessagingAccount>,
) -> Option<MessageReason> {
    if channel != MessageChannel::MESSENGER {
        return None;
    }
    let voter_page = recipient.messenger_page.as_deref()?;
    let account_page = account?.sender.messenger_page()?.page_id;
    (account_page.trim() != voter_page.trim()).then_some(MessageReason::MESSENGER_PAGE_CHANGED)
}

fn not_sent_yet(reason: impl ToString) -> ProviderFailure {
    ProviderFailure {
        kind: FailureKind::TRANSIENT,
        code: None,
        reason: reason.to_string(),
    }
}

/// [`deliver`] under `policy`. In order, before anything is written:
///
/// 1. refusals that never change (channel not offered, no account, invalid
///    destination, no content, preflight): one FAILED row;
/// 2. the readiness gate ([`ReadinessGate`]): an account that isn't ready
///    for the purpose sends nothing ([`not_ready_reason`]);
/// 3. the account's capacity: the caller's place, then a token of the
///    bucket ([`rate_bucket::CAPACITY_BUSY`] past the deadline).
///
/// A message stopped by 2 or 3 is a transient failure, FAILED and never
/// handed to the provider (a due retry under
/// [`WhenNotSendable::KEEP_RETRY_DUE`] stays due instead). Only then is the
/// attempt written, QUEUED and dispatched in one transaction, so nothing
/// waits with a QUEUED row; the provider's answer is awaited until
/// [`SendPolicy::answer_by`].
#[instrument(skip_all, fields(logical_key = %delivery.logical_key, purpose = %delivery.purpose), err)]
pub async fn deliver_within(
    client: &mut DbClient,
    dispatcher: &Dispatcher,
    delivery: &Delivery,
    policy: SendPolicy,
) -> Result<DeliveryResult> {
    // Only a concurrent worker recording the same attempt makes it loop.
    for _ in 0..4 {
        let tx = client.transaction().await?;
        let config = match &delivery.election_event_id {
            Some(event) => get_event_messaging_config(&tx, &delivery.tenant_id, event).await?,
            None => None,
        };
        let records = list_attempts(&tx, &delivery.tenant_id, &delivery.logical_key).await?;
        let latest = records.iter().max_by_key(|r| r.attempt).cloned();
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
                let current = latest.as_ref().map(DeliveryResult::from_record);
                tx.commit().await?;
                return current.ok_or_else(|| anyhow!("inconsistent attempts"));
            }
        };

        let (account, account_missing) =
            match delivery_account(&tx, delivery, config.as_ref(), channel).await? {
                Ok(account) => (account, None),
                Err(reason) => (None, Some(reason)),
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
                if let Some(reason) = account_missing {
                    return Err(reason.to_string());
                }
                if let Some(reason) = page_refusal(channel, &delivery.recipient, account.as_ref()) {
                    return Err(reason.to_string());
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
        // The send-time gate: the account must be ready for the purpose
        // now, for its current identity. The environment's email and SMS
        // (no account) are outside it.
        let blocker = match (&prepared, &account, policy.readiness) {
            (Ok(_), Some(account), ReadinessGate::CHECKED) => {
                send_readiness(account, delivery.purpose).err()
            }
            _ => None,
        };
        let refusal = match &prepared {
            Err(reason) => Some(reason.clone()),
            Ok(_) if blocker.is_some() => None,
            Ok((sender, message, _)) => {
                let limits = account
                    .as_ref()
                    .map(|a| a.limits.clone())
                    .unwrap_or_default();
                preflight(&sender.capabilities(), &limits, message, Utc::now())
                    .err()
                    .map(|error| error.to_string())
            }
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
        // Nothing is written until the attempt can go.
        tx.commit().await?;

        let not_sent = match (blocker, &account) {
            (Some(blocker), _) => {
                let reason = not_ready_reason(blocker);
                info!(%channel, %blocker, "the account isn't ready: not sent");
                Some(not_sent_yet(reason))
            }
            (None, Some(account)) => {
                if take_capacity(client, account, delivery.purpose, &policy).await? {
                    None
                } else {
                    info!(%channel, place = ?policy.place, "{}", rate_bucket::CAPACITY_BUSY);
                    Some(not_sent_yet(rate_bucket::CAPACITY_BUSY))
                }
            }
            (None, None) => None,
        };
        if let (Some(failure), WhenNotSendable::KEEP_RETRY_DUE, Some(latest)) =
            (&not_sent, policy.when_not_sendable, &latest)
        {
            if attempt > 1 {
                return keep_retry_due(client, latest, &failure.reason).await;
            }
        }
        let not_sent = not_sent.or_else(|| {
            message
                .expires_at
                .is_some_and(|expires_at| expires_at <= Utc::now())
                .then(|| ProviderFailure {
                    kind: FailureKind::PERMANENT,
                    code: None,
                    reason: MessageReason::CODE_EXPIRED_BEFORE_SENDING.to_string(),
                })
        });

        let mut tx = client.transaction().await?;
        let Some(queued) = insert_message(&tx, &record).await? else {
            // Another worker recorded this attempt first: decide again.
            tx.commit().await?;
            continue;
        };
        let outcome = match not_sent {
            // Never handed to the provider: `dispatched_at` stays empty.
            Some(failure) => SendOutcome::Rejected(failure),
            None => {
                mark_dispatched(&tx, &queued.id).await?;
                tx.commit().await?;
                let outcome = match policy.answer_by {
                    Some(answer_by) => {
                        match tokio::time::timeout_at(answer_by, sender.send(&message)).await {
                            Ok(outcome) => outcome,
                            Err(_) => {
                                warn!(message_id = %queued.id, %channel, "{PROVIDER_ANSWER_LATE}");
                                SendOutcome::Unknown {
                                    reason: PROVIDER_ANSWER_LATE.to_string(),
                                }
                            }
                        }
                    }
                    None => sender.send(&message).await,
                };
                tx = client.transaction().await?;
                outcome
            }
        };
        let change = match &outcome {
            SendOutcome::Accepted {
                provider_message_id,
            } => StateChange {
                provider_message_id: provider_message_id.clone(),
                ..Default::default()
            },
            SendOutcome::Rejected(failure) => {
                failure_change(&tx, delivery, attempt, failure).await?
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

    fn target(channel: MessageChannel, destination: &str) -> NoticeTarget {
        NoticeTarget {
            channel,
            destination: destination.to_string(),
            messenger_page: None,
        }
    }

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
            Ok(target(MessageChannel::WHATSAPP, "+639181234567"))
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
            Ok(target(MessageChannel::WHATSAPP, "+639181234567"))
        );
        // The notice that the channel changed goes to the previous one.
        assert_eq!(
            notice_recipient(
                NoticeRecipient::PREVIOUS_CHANNEL,
                Some(MessageChannel::SMS),
                Some("+639171234567"),
                Some(&whatsapp)
            ),
            Ok(target(MessageChannel::SMS, "+639171234567"))
        );
    }

    /// Regression (R7-23): on the voter's channel, the destination Keycloak
    /// named won over the voter's record, so a stale or wrong one went out
    /// unchecked. The record's destination is always used.
    #[test]
    fn a_notice_on_the_voters_channel_goes_to_the_recorded_destination() {
        let whatsapp = voter(
            MessageChannel::WHATSAPP,
            "+639181234567",
            VoterChannelSource::RECORDED,
        );
        assert_eq!(
            notice_recipient(
                NoticeRecipient::VOTER_CHANNEL,
                Some(MessageChannel::WHATSAPP),
                Some("+639990000000"),
                Some(&whatsapp)
            ),
            Ok(target(MessageChannel::WHATSAPP, "+639181234567"))
        );
    }

    /// R7-02: a notice to a Messenger voter carries their Page, so a send
    /// through an account of another Page is refused.
    #[test]
    fn a_messenger_notice_carries_the_voters_page() {
        let messenger = VoterChannel {
            channel: MessageChannel::MESSENGER,
            destination: "6543210".to_string(),
            messenger_page: Some("page-old".to_string()),
            source: VoterChannelSource::RECORDED,
        };
        assert_eq!(
            notice_recipient(NoticeRecipient::VOTER_CHANNEL, None, None, Some(&messenger)),
            Ok(NoticeTarget {
                channel: MessageChannel::MESSENGER,
                destination: "6543210".to_string(),
                messenger_page: Some("page-old".to_string()),
            })
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
            Ok(target(MessageChannel::SMS, "+639171234567"))
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
