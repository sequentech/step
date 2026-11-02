// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Messaging channels, sending accounts and the per-event messaging
//! configuration shared by harvest, windmill, Keycloak and the portals.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use strum_macros::{Display, EnumIter, EnumString};

/// Annotation key holding the event's [`EventMessagingConfig`].
pub const MESSAGING_CONFIG_ANNOTATION: &str = "messaging:config";
/// Realm attribute holding the [`PublicMessagingChannels`] projection.
pub const REALM_ATTR_MESSAGING: &str = "sequent.messaging";
pub const EVENT_MESSAGING_CONFIG_VERSION: u32 = 1;
/// Template key of verification codes, for binding their approved template.
pub const OTP_TEMPLATE_KEY: &str = "otp";

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    EnumIter,
    PartialOrd,
    Ord,
)]
pub enum MessageChannel {
    EMAIL,
    SMS,
    WHATSAPP,
    VIBER,
    MESSENGER,
}

impl MessageChannel {
    pub fn recipient_kind(&self) -> RecipientKind {
        match self {
            MessageChannel::EMAIL => RecipientKind::EMAIL_ADDRESS,
            MessageChannel::SMS
            | MessageChannel::WHATSAPP
            | MessageChannel::VIBER => RecipientKind::PHONE_NUMBER,
            MessageChannel::MESSENGER => RecipientKind::PAGE_SCOPED_ID,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    EnumIter,
    PartialOrd,
    Ord,
)]
pub enum MessagePurpose {
    /// One-time codes. Never retried by the server and never sent after
    /// expiry: the voter asks for another code on the same channel.
    OTP,
    /// Enrollment results, credentials, reminders and other notices.
    NOTICE,
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
)]
pub enum RecipientKind {
    EMAIL_ADDRESS,
    PHONE_NUMBER,
    PAGE_SCOPED_ID,
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    EnumIter,
)]
pub enum MessagingProvider {
    AWS_SES,
    SMTP,
    AWS_SNS,
    WHATSAPP_CLOUD_API,
    MESSENGER_SEND_API,
    /// Viber Business Messages through Infobip. This is not Viber's bot
    /// API.
    VIBER_INFOBIP,
    /// Removed: a provider described by configuration. Parses so stored
    /// accounts load; see [`MessagingProvider::support`].
    HTTP_API,
    /// Removed: printed messages instead of sending them. Parses so stored
    /// accounts load; see [`MessagingProvider::support`].
    CONSOLE,
}

/// Whether accounts of a provider may be created and send.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
)]
pub enum ProviderSupport {
    SUPPORTED,
    /// The provider was removed. Its accounts still load and are shown with
    /// [`MessageReason::UNSUPPORTED_PROVIDER`]; creating one, or sending,
    /// checking or testing through one, is refused with that reason.
    UNSUPPORTED,
}

impl MessagingProvider {
    pub fn support(&self) -> ProviderSupport {
        match self {
            MessagingProvider::HTTP_API | MessagingProvider::CONSOLE => {
                ProviderSupport::UNSUPPORTED
            }
            _ => ProviderSupport::SUPPORTED,
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
)]
pub enum DeliveryFeedback {
    /// The provider reports delivery; missing receipts are still not failure.
    PROVIDER_RECEIPTS,
    /// Only acceptance is known. Delivery is shown as unavailable.
    UNAVAILABLE,
}

/// What a provider adapter supports. Sending checks these before any
/// request leaves Step.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ProviderCapabilities {
    pub channel: MessageChannel,
    pub recipient: RecipientKind,
    pub purposes: Vec<MessagePurpose>,
    /// Purposes that may only be sent with a provider-approved template.
    pub template_required_for: Vec<MessagePurpose>,
    pub delivery_feedback: DeliveryFeedback,
    /// Whether an unknown outcome can be resolved by asking the provider.
    pub reconciliation: bool,
    /// Hours after the recipient's last interaction during which free-form
    /// messages may be sent.
    pub conversation_window_hours: Option<u32>,
    /// Whether the account owner must confirm a provider eligibility
    /// approval before any purpose can be enabled.
    pub requires_provider_approval: bool,
}

impl MessagingProvider {
    pub fn channel(&self) -> Option<MessageChannel> {
        match self {
            MessagingProvider::AWS_SES | MessagingProvider::SMTP => {
                Some(MessageChannel::EMAIL)
            }
            MessagingProvider::AWS_SNS => Some(MessageChannel::SMS),
            MessagingProvider::WHATSAPP_CLOUD_API => {
                Some(MessageChannel::WHATSAPP)
            }
            MessagingProvider::MESSENGER_SEND_API => {
                Some(MessageChannel::MESSENGER)
            }
            MessagingProvider::VIBER_INFOBIP => Some(MessageChannel::VIBER),
            MessagingProvider::HTTP_API | MessagingProvider::CONSOLE => None,
        }
    }

    /// Capabilities of this provider when used for `channel`.
    pub fn capabilities(
        &self,
        channel: MessageChannel,
    ) -> Option<ProviderCapabilities> {
        if let Some(own) = self.channel() {
            if own != channel {
                return None;
            }
        }
        let both = vec![MessagePurpose::OTP, MessagePurpose::NOTICE];
        let capabilities = match self {
            MessagingProvider::AWS_SES | MessagingProvider::SMTP => {
                ProviderCapabilities {
                    channel,
                    recipient: RecipientKind::EMAIL_ADDRESS,
                    purposes: both,
                    template_required_for: vec![],
                    delivery_feedback: match self {
                        MessagingProvider::AWS_SES => {
                            DeliveryFeedback::PROVIDER_RECEIPTS
                        }
                        _ => DeliveryFeedback::UNAVAILABLE,
                    },
                    reconciliation: false,
                    conversation_window_hours: None,
                    requires_provider_approval: false,
                }
            }
            // SNS reports SMS delivery to CloudWatch Logs, not to Step.
            MessagingProvider::AWS_SNS => ProviderCapabilities {
                channel,
                recipient: RecipientKind::PHONE_NUMBER,
                purposes: both,
                template_required_for: vec![],
                delivery_feedback: DeliveryFeedback::UNAVAILABLE,
                reconciliation: false,
                conversation_window_hours: None,
                requires_provider_approval: false,
            },
            MessagingProvider::WHATSAPP_CLOUD_API => ProviderCapabilities {
                channel,
                recipient: RecipientKind::PHONE_NUMBER,
                purposes: both.clone(),
                template_required_for: both,
                delivery_feedback: DeliveryFeedback::PROVIDER_RECEIPTS,
                reconciliation: false,
                conversation_window_hours: Some(24),
                requires_provider_approval: true,
            },
            MessagingProvider::MESSENGER_SEND_API => ProviderCapabilities {
                channel,
                recipient: RecipientKind::PAGE_SCOPED_ID,
                purposes: both,
                template_required_for: vec![],
                delivery_feedback: DeliveryFeedback::PROVIDER_RECEIPTS,
                reconciliation: false,
                conversation_window_hours: Some(24),
                requires_provider_approval: false,
            },
            MessagingProvider::VIBER_INFOBIP => ProviderCapabilities {
                channel,
                recipient: RecipientKind::PHONE_NUMBER,
                purposes: both.clone(),
                template_required_for: both,
                delivery_feedback: DeliveryFeedback::PROVIDER_RECEIPTS,
                reconciliation: true,
                conversation_window_hours: None,
                requires_provider_approval: false,
            },
            MessagingProvider::HTTP_API | MessagingProvider::CONSOLE => {
                ProviderCapabilities {
                    channel,
                    recipient: channel.recipient_kind(),
                    purposes: both,
                    template_required_for: vec![],
                    delivery_feedback: DeliveryFeedback::UNAVAILABLE,
                    reconciliation: false,
                    conversation_window_hours: None,
                    requires_provider_approval: false,
                }
            }
        };
        Some(capabilities)
    }
}

/// State of one delivery attempt. Each attempt is a separate record.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    EnumIter,
)]
pub enum MessageAttemptState {
    /// Persisted, not yet handed to the provider.
    QUEUED,
    /// The provider accepted the request. Not proof of receipt.
    ACCEPTED,
    DELIVERED,
    /// Confirmed failure, from the provider's response or a receipt.
    FAILED,
    /// The request may have reached the provider but its answer was lost.
    /// Never treated as failure: reconcile first.
    UNKNOWN,
}

impl MessageAttemptState {
    /// Whether a report or reconciliation may move an attempt from `self` to
    /// `next`. Late reports can never regress a delivered message.
    pub fn can_transition_to(&self, next: MessageAttemptState) -> bool {
        use MessageAttemptState::*;
        match (self, next) {
            // A fast report can arrive before the send's answer is recorded.
            (QUEUED, ACCEPTED | DELIVERED | FAILED | UNKNOWN) => true,
            (ACCEPTED, DELIVERED | FAILED) => true,
            (UNKNOWN, ACCEPTED | DELIVERED | FAILED) => true,
            (FAILED, DELIVERED) => true,
            _ => false,
        }
    }

    /// Whether this attempt still blocks starting another one for the same
    /// logical message.
    pub fn is_active(&self) -> bool {
        !matches!(self, MessageAttemptState::FAILED)
    }
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
)]
pub enum MessageDirection {
    OUTBOUND,
    /// Rows the first release wrote for messages voters sent. Read only:
    /// incoming messages are no longer kept, and the migration deleted
    /// them.
    INBOUND,
}

/// The reason of every conditional outcome of a message, as stored in the
/// ledger's `error`, written to the electoral log and shown to staff. One
/// string per outcome, shared by the sender, the log and the screens; the
/// screens translate it by matching [`MessageReason::as_str`].
///
/// Provider refusals keep the provider's sanitized reason instead.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    EnumIter,
)]
pub enum MessageReason {
    /// The voter's record has no channel with a destination.
    #[strum(serialize = "No contact channel")]
    NO_CONTACT_CHANNEL,
    /// The event, or the voter's Post (election), doesn't offer the
    /// voter's channel for notices. Nothing is sent on another channel.
    #[strum(serialize = "Channel not offered for notices")]
    CHANNEL_NOT_OFFERED_FOR_NOTICES,
    /// The same, for codes.
    #[strum(serialize = "Channel not offered for codes")]
    CHANNEL_NOT_OFFERED_FOR_CODES,
    /// The send has no content for the voter's channel.
    #[strum(serialize = "No content for the channel")]
    NO_CONTENT_FOR_CHANNEL,
    /// Neither the event nor the tenant has a sending account for the
    /// channel.
    #[strum(serialize = "No sending account for the channel")]
    NO_ACCOUNT_FOR_CHANNEL,
    /// The voter's destination isn't a valid address for the channel.
    #[strum(serialize = "Invalid destination for the channel")]
    INVALID_DESTINATION,
    /// A transient failure that can't be retried: the ledger keeps no
    /// content, and the message has no stored template to render again.
    #[strum(serialize = "Can't be retried: no stored template")]
    NO_STORED_TEMPLATE,
    /// The original queued audience has no trustworthy election scope.
    #[strum(
        serialize = "Create a new send: participation authorization is missing or invalid"
    )]
    PARTICIPATION_REAUTHORIZATION_REQUIRED,
    /// A retry is due, but the voter changed their channel since: the
    /// message isn't sent on the new one.
    #[strum(serialize = "The voter's channel changed: not retried")]
    CHANNEL_CHANGED,
    /// The worker stopped after recording the attempt and before handing it
    /// to the provider: never sent, so it may be retried.
    #[strum(serialize = "The worker stopped before sending")]
    WORKER_STOPPED_BEFORE_SENDING,
    /// The worker stopped after handing the attempt to the provider and
    /// before recording its answer: the outcome is unknown.
    #[strum(serialize = "The worker stopped before recording the outcome")]
    WORKER_STOPPED_AFTER_SENDING,
    /// A code that expired while waiting to be sent.
    #[strum(serialize = "The code expired before it could be sent")]
    CODE_EXPIRED_BEFORE_SENDING,
    /// An unknown outcome older than [`UNRESOLVED_AFTER_HOURS`]. Shown as
    /// Unresolved: delivery couldn't be confirmed. Never a failure.
    #[strum(serialize = "Delivery couldn't be confirmed")]
    UNRESOLVED,
    /// A send to selected voters that names none. Refused when scheduled.
    #[strum(serialize = "Select at least one voter")]
    SELECTED_WITHOUT_VOTERS,
    /// The account's provider was removed (`HTTP_API`, `CONSOLE`).
    #[strum(serialize = "Unsupported provider: replace this account")]
    UNSUPPORTED_PROVIDER,
    /// A notice whose send used encrypted voter attributes. A retry would
    /// need them decrypted again, which only the administrator's send
    /// approves, and their content is never stored: it isn't retried.
    #[strum(serialize = "Can't be retried: contains protected fields")]
    PROTECTED_FIELDS,
    /// A failure the provider reported more than
    /// [`LATE_FAILURE_RETRY_HOURS`] after the attempt: sending it again so
    /// late could reach the voter after it stopped being useful.
    #[strum(serialize = "Reported too late to retry")]
    REPORTED_TOO_LATE,
    /// A due retry whose message couldn't be prepared again for
    /// [`RETRY_PREPARE_HOURS`] (template, voter or event unreadable).
    #[strum(
        serialize = "Can't be retried: the message couldn't be prepared again"
    )]
    RETRY_NOT_PREPARED,
    /// A notice asked for on a channel other than the voter's recorded
    /// one. Nothing is sent on either channel.
    #[strum(serialize = "Not the voter's channel")]
    NOT_THE_VOTERS_CHANNEL,
    /// A Messenger message whose voter is connected to another Page than
    /// the sending account's: their Page-scoped ID belongs to that Page,
    /// so nothing is handed to the provider and it isn't retried. The
    /// voter connects Messenger again at their next sign-in.
    #[strum(
        serialize = "Not sent: the voter is connected to another Messenger Page"
    )]
    MESSENGER_PAGE_CHANGED,
}

impl MessageReason {
    /// The refusal of a message whose channel the event or the voter's
    /// Post doesn't offer for `purpose`.
    pub fn channel_not_offered(purpose: MessagePurpose) -> MessageReason {
        match purpose {
            MessagePurpose::NOTICE => {
                MessageReason::CHANNEL_NOT_OFFERED_FOR_NOTICES
            }
            MessagePurpose::OTP => MessageReason::CHANNEL_NOT_OFFERED_FOR_CODES,
        }
    }

    pub fn as_str(&self) -> &'static str {
        use MessageReason::*;
        match self {
            NO_CONTACT_CHANNEL => "No contact channel",
            CHANNEL_NOT_OFFERED_FOR_NOTICES => {
                "Channel not offered for notices"
            }
            CHANNEL_NOT_OFFERED_FOR_CODES => "Channel not offered for codes",
            NO_CONTENT_FOR_CHANNEL => "No content for the channel",
            NO_ACCOUNT_FOR_CHANNEL => "No sending account for the channel",
            INVALID_DESTINATION => "Invalid destination for the channel",
            NO_STORED_TEMPLATE => "Can't be retried: no stored template",
            PARTICIPATION_REAUTHORIZATION_REQUIRED => "Create a new send: participation authorization is missing or invalid",
            CHANNEL_CHANGED => "The voter's channel changed: not retried",
            WORKER_STOPPED_BEFORE_SENDING => {
                "The worker stopped before sending"
            }
            WORKER_STOPPED_AFTER_SENDING => {
                "The worker stopped before recording the outcome"
            }
            CODE_EXPIRED_BEFORE_SENDING => {
                "The code expired before it could be sent"
            }
            UNRESOLVED => "Delivery couldn't be confirmed",
            SELECTED_WITHOUT_VOTERS => "Select at least one voter",
            UNSUPPORTED_PROVIDER => {
                "Unsupported provider: replace this account"
            }
            PROTECTED_FIELDS => "Can't be retried: contains protected fields",
            REPORTED_TOO_LATE => "Reported too late to retry",
            RETRY_NOT_PREPARED => {
                "Can't be retried: the message couldn't be prepared again"
            }
            NOT_THE_VOTERS_CHANNEL => "Not the voter's channel",
            MESSENGER_PAGE_CHANGED => {
                "Not sent: the voter is connected to another Messenger Page"
            }
        }
    }

    /// Why a failure isn't retried, recorded next to the failure's own
    /// reason (`"<failure>; <why not retried>"`), never in its place, so
    /// staff still see what went wrong.
    pub fn not_retried_after(&self, failure: Option<&str>) -> String {
        match failure.map(str::trim).filter(|f| !f.is_empty()) {
            Some(failure) => {
                format!("{failure}{REASON_SEPARATOR}{}", self.as_str())
            }
            None => self.as_str().to_string(),
        }
    }

    /// The parts of a stored reason: the failure, and why it wasn't
    /// retried when [`MessageReason::not_retried_after`] added that.
    pub fn split_stored(stored: &str) -> (&str, Option<MessageReason>) {
        if let Ok(reason) = stored.parse::<MessageReason>() {
            return ("", Some(reason));
        }
        match stored.rsplit_once(REASON_SEPARATOR) {
            Some((failure, reason)) => match reason.parse::<MessageReason>() {
                Ok(reason) => (failure, Some(reason)),
                Err(_) => (stored, None),
            },
            None => (stored, None),
        }
    }
}

/// Separates a failure's reason from why it isn't retried in a stored
/// reason (see [`MessageReason::not_retried_after`]).
pub const REASON_SEPARATOR: &str = "; ";

/// A failure reported this many hours or more after its attempt isn't
/// retried ([`MessageReason::REPORTED_TOO_LATE`]).
pub const LATE_FAILURE_RETRY_HOURS: i64 = 1;

/// A due retry that can't be prepared again for this many hours stops
/// waiting ([`MessageReason::RETRY_NOT_PREPARED`]).
pub const RETRY_PREPARE_HOURS: i64 = 1;

/// Whose channel a notice from Keycloak goes to. Harvest checks the
/// notice against the voter's record unless it is about the previous
/// channel.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    EnumIter,
)]
pub enum NoticeRecipient {
    /// The voter's recorded channel, resolved by harvest when the request
    /// names none. A request naming another recorded channel is refused
    /// ([`MessageReason::NOT_THE_VOTERS_CHANNEL`]).
    #[default]
    VOTER_CHANNEL,
    /// The channel the voter just replaced, for the notice that it was
    /// changed. Sent to the channel and destination the request names.
    PREVIOUS_CHANNEL,
}

/// An unknown outcome older than this is shown as Unresolved, and the sweep
/// stops asking the provider about it. It never becomes FAILED.
pub const UNRESOLVED_AFTER_HOURS: i64 = 24;

/// How a provider bills a message, for monthly reconciliation. Recorded
/// with each attempt; a provider report may replace it. No prices.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    EnumIter,
)]
pub enum BillingCategory {
    /// WhatsApp authentication templates (codes).
    AUTHENTICATION,
    /// WhatsApp and Messenger utility templates (notices).
    UTILITY,
    /// A Messenger reply within the 24-hour window.
    SERVICE,
    /// SMS and Viber business messages.
    TRANSACTIONAL,
    /// Email.
    EMAIL,
}

impl BillingCategory {
    /// The category expected when a message is sent. `in_window`: a
    /// Messenger message inside the 24-hour window.
    pub fn expected(
        channel: MessageChannel,
        purpose: MessagePurpose,
        in_window: bool,
    ) -> BillingCategory {
        match (channel, purpose) {
            (MessageChannel::WHATSAPP, MessagePurpose::OTP) => {
                BillingCategory::AUTHENTICATION
            }
            (MessageChannel::WHATSAPP, MessagePurpose::NOTICE) => {
                BillingCategory::UTILITY
            }
            (MessageChannel::MESSENGER, _) if in_window => {
                BillingCategory::SERVICE
            }
            (MessageChannel::MESSENGER, _) => BillingCategory::UTILITY,
            (MessageChannel::SMS | MessageChannel::VIBER, _) => {
                BillingCategory::TRANSACTIONAL
            }
            (MessageChannel::EMAIL, _) => BillingCategory::EMAIL,
        }
    }

    /// The category a provider reported, such as WhatsApp's
    /// `pricing.category`.
    pub fn from_report(category: &str) -> Option<BillingCategory> {
        match category.to_ascii_lowercase().as_str() {
            "authentication" | "authentication_international" => {
                Some(BillingCategory::AUTHENTICATION)
            }
            "utility" => Some(BillingCategory::UTILITY),
            "service" => Some(BillingCategory::SERVICE),
            _ => None,
        }
    }
}

/// Whether the provider has approved this account for its use case. Meta
/// only allows government WhatsApp messaging through an approved
/// arrangement, so a WhatsApp account starts as `PENDING`.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    Default,
)]
pub enum ProviderApproval {
    #[default]
    PENDING,
    CONFIRMED,
}

/// How Messenger notices may be sent outside the 24-hour window.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    Default,
)]
pub enum OutOfWindowPolicy {
    /// Only within the window. Outside it, codes need a fresh Get Started.
    /// Not allowed for Messenger notices, which use
    /// [`OutOfWindowPolicy::UTILITY_MESSAGES`]: there is no other channel.
    #[default]
    DISABLED,
    /// Meta's utility messages, once approved for the Page.
    UTILITY_MESSAGES,
}

/// Why a purpose cannot be enabled on an account, most fundamental first.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
)]
pub enum ReadinessBlocker {
    /// The account's provider was removed: replace the account.
    UNSUPPORTED_PROVIDER,
    /// The account's last check was of another identity (another WhatsApp
    /// number or business account, Messenger Page, Viber sender, SNS or SES
    /// identity): nothing it said counts until the account is checked as
    /// it is now ([`AccountCheck::is_for`]).
    UNCHECKED,
    NOT_CONNECTED,
    UNSUPPORTED_PURPOSE,
    NEEDS_PROVIDER_APPROVAL,
    NEEDS_PRODUCTION_ACCESS,
    NEEDS_APPROVED_TEMPLATE,
    /// Messenger codes need Get Started, which reaches Step only through
    /// the webhook component: without its public address
    /// (`MESSAGING_WEBHOOKS_PUBLIC_URL`) Messenger isn't ready for codes.
    /// Reason: "Callback address not configured".
    CALLBACK_NOT_CONFIGURED,
    /// A Viber (Infobip) account without generated report credentials: the
    /// subscription's reports are refused, so receipts are unavailable.
    /// Reason: "Generate the report credentials and create the Infobip
    /// subscription". Blocks no purpose.
    REPORT_CREDENTIALS_MISSING,
}

/// The blocker callbacks add to `purpose` on `channel`: Messenger codes
/// need the callback address, because the code goes out only after Get
/// Started, which arrives as a callback. Other purposes and channels work
/// without one (their receipts are then unavailable).
pub fn callback_blocker(
    channel: MessageChannel,
    purpose: MessagePurpose,
    callback_configured: bool,
) -> Option<ReadinessBlocker> {
    (channel == MessageChannel::MESSENGER
        && purpose == MessagePurpose::OTP
        && !callback_configured)
        .then_some(ReadinessBlocker::CALLBACK_NOT_CONFIGURED)
}

/// Result of the last connection check of an account.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct AccountCheck {
    pub connected: bool,
    pub production_access: bool,
    /// Languages with an approved template, per purpose.
    #[serde(default)]
    pub approved_templates: BTreeMap<MessagePurpose, Vec<String>>,
    pub checked_at: Option<String>,
    /// Short, non-sensitive reason when not connected.
    pub reason: Option<String>,
    /// What the provider reported beyond connection, for the account and
    /// the event tab. Checks stored before it existed read as empty.
    #[serde(default)]
    pub details: AccountCheckDetails,
}

impl AccountCheck {
    /// Whether this check describes an account whose identity is
    /// `current`. A check that doesn't name its identity was stored before
    /// identities were recorded; it describes the identity the account had
    /// then, which an identity change records, so it counts until then.
    pub fn is_for(&self, current: &AccountIdentity) -> bool {
        self.details
            .identity
            .as_ref()
            .map_or(true, |checked| checked == current)
    }

    /// The provider approval readiness uses: the one the check reported,
    /// where the provider has an API for it, else the one an administrator
    /// entered, with where it came from.
    pub fn approval(
        &self,
        entered: ProviderApproval,
    ) -> (ProviderApproval, ApprovalSource) {
        match self.details.provider_approval {
            Some(reported) => (reported, ApprovalSource::PROVIDER_CHECK),
            None => (entered, ApprovalSource::ADMIN_ENTERED),
        }
    }
}

/// Where an account's provider approval comes from.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
)]
pub enum ApprovalSource {
    /// The provider reports it, and the check read it.
    PROVIDER_CHECK,
    /// The provider has no API for it: an administrator entered it, and it
    /// is shown as such.
    ADMIN_ENTERED,
}

/// The provider's details from a check. Every part is optional: a provider
/// reports only some of them, and a part it couldn't read is absent, never
/// guessed.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct AccountCheckDetails {
    /// Meta's `debug_token`: the access token's validity and expiry.
    #[serde(default)]
    pub token: Option<TokenStatus>,
    /// WhatsApp: the number's status, quality rating and messaging limit.
    #[serde(default)]
    pub whatsapp_number: Option<WhatsAppNumberStatus>,
    /// Templates at the provider, with their category and status per
    /// language (WhatsApp, Messenger). Absent for providers without an API
    /// for them (Viber's partner templates are entered by an administrator).
    #[serde(default)]
    pub templates: Vec<TemplateStatus>,
    /// Why `templates` couldn't be read, when the provider has an API for
    /// them.
    #[serde(default)]
    pub templates_reason: Option<String>,
    /// Messenger: the `pages_messaging` permission, the app's subscription
    /// to the Page and its Get Started button.
    #[serde(default)]
    pub messenger: Option<MessengerPageStatus>,
    /// Viber (Infobip): whether the partner accepted the account's login.
    #[serde(default)]
    pub partner_login: Option<PartnerLogin>,
    /// The account's callback address at the check (`callback_readiness`).
    #[serde(default)]
    pub callback: Option<CallbackStatus>,
    /// The provider's approval, where its API reports it (the WhatsApp
    /// number's status and display name). Absent where there is no API:
    /// the administrator's entry is used then ([`AccountCheck::approval`]).
    #[serde(default)]
    pub provider_approval: Option<ProviderApproval>,
    /// Why `provider_approval` is `PENDING`, as the provider said it.
    #[serde(default)]
    pub provider_approval_reason: Option<String>,
    /// WhatsApp `user_changed_number` signals: voters to verify again.
    /// Logged and shown; nothing changes automatically. Kept across checks,
    /// newest first, at most [`NUMBER_CHANGES_KEPT`].
    #[serde(default)]
    pub number_changes: Vec<NumberChange>,
    /// The identity the check checked. Absent in checks stored before it
    /// existed: they describe the identity the account had then, and an
    /// identity change records it here, so they stop counting.
    #[serde(default)]
    pub identity: Option<AccountIdentity>,
    /// When the check started (RFC 3339). Its result is stored only if the
    /// account's identity and credentials are what the check started with.
    #[serde(default)]
    pub started_at: Option<String>,
    /// When a check of the account was last attempted, whether or not its
    /// result could be stored (RFC 3339). The daily pass orders by it, so
    /// an account whose check fails to run backs off.
    #[serde(default)]
    pub attempted_at: Option<String>,
    /// Meta: whether the app secret matched the app of the account's token
    /// (`appsecret_proof`). Absent where it wasn't checked.
    #[serde(default)]
    pub app_secret: Option<AppSecretStatus>,
}

/// Whether an account's Meta app secret matched its app.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct AppSecretStatus {
    /// `Some(false)`: Meta refused the secret's proof. `None`: the proof
    /// couldn't be checked; `reason` says why.
    pub valid: Option<bool>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// What a check is a check of: the provider and the sender's identifying
/// ids. Two checks describe the same sender only if their identities are
/// equal; names and labels are not part of it.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct AccountIdentity {
    pub provider: MessagingProvider,
    /// The identifying ids by name, trimmed; absent ids are left out.
    #[serde(default)]
    pub ids: BTreeMap<String, String>,
}

/// How many [`NumberChange`]s an account keeps.
pub const NUMBER_CHANGES_KEPT: usize = 50;

/// An access token as Meta's `debug_token` describes it.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct TokenStatus {
    pub valid: bool,
    /// RFC 3339; absent when the token doesn't expire.
    #[serde(default)]
    pub expires_at: Option<String>,
    /// Permissions the account needs that the token lacks.
    #[serde(default)]
    pub missing_scopes: Vec<String>,
}

/// A WhatsApp number as the Cloud API describes it. Values are Meta's.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct WhatsAppNumberStatus {
    /// `CONNECTED`, `PENDING`, `FLAGGED`, `RESTRICTED`, ...
    #[serde(default)]
    pub status: Option<String>,
    /// The display name's review: `APPROVED`, `PENDING_REVIEW`, ...
    #[serde(default)]
    pub name_status: Option<String>,
    /// `GREEN`, `YELLOW`, `RED` or `UNKNOWN`.
    #[serde(default)]
    pub quality_rating: Option<String>,
    /// Business-initiated conversations per day, such as `TIER_1K`.
    #[serde(default)]
    pub messaging_limit: Option<String>,
}

/// A template at the provider, for one language.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct TemplateStatus {
    pub name: String,
    pub language: String,
    /// `AUTHENTICATION`, `UTILITY`, `MARKETING`.
    #[serde(default)]
    pub category: Option<String>,
    /// `APPROVED`, `PENDING`, `REJECTED`, `PAUSED`, ...
    pub status: String,
}

/// A Messenger Page as the Graph API describes it. `None`: not reported.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct MessengerPageStatus {
    #[serde(default)]
    pub pages_messaging: Option<bool>,
    pub subscribed: bool,
    #[serde(default)]
    pub get_started: Option<bool>,
}

/// A partner login check.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct PartnerLogin {
    pub accepted: bool,
    #[serde(default)]
    pub reason: Option<String>,
}

/// The callback address at the check: its URL, or why there is none.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct CallbackStatus {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

/// A WhatsApp `user_changed_number` signal. Only masked numbers are kept.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct NumberChange {
    pub masked_old_number: String,
    #[serde(default)]
    pub masked_new_number: Option<String>,
    /// Voters the account sent to at the old number (from the ledger's
    /// keyed digest), to verify again.
    #[serde(default)]
    pub voter_ids: Vec<String>,
    /// RFC 3339.
    pub reported_at: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct PurposeReadiness {
    pub purpose: MessagePurpose,
    pub blockers: Vec<ReadinessBlocker>,
}

impl PurposeReadiness {
    pub fn is_ready(&self) -> bool {
        self.blockers.is_empty()
    }
}

/// How an account's readiness is established.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    Default,
)]
pub enum ReadinessPolicy {
    /// From the provider's answers to the connection check. Rows a
    /// development build stored as `UNCHECKED` read as this: "needs a
    /// check" is the check's identity ([`AccountCheck::is_for`]), never a
    /// stored policy, so older binaries parse every row.
    #[default]
    #[serde(alias = "UNCHECKED")]
    #[strum(to_string = "PROVIDER_CHECK", serialize = "UNCHECKED")]
    PROVIDER_CHECK,
    /// Removed: an administrator's statement replaced the provider check.
    /// It no longer bypasses anything. Stored accounts still parse, and
    /// are checked like [`ReadinessPolicy::PROVIDER_CHECK`].
    ADMIN_CONFIRMED,
}

impl ReadinessPolicy {
    /// The policy readiness is computed with: always the provider check.
    pub fn effective(self) -> ReadinessPolicy {
        ReadinessPolicy::PROVIDER_CHECK
    }
}

/// Readiness of `purpose` for `language` on an account whose identity is
/// `identity` (`None`: not known, so the check is taken as the account's).
/// The same function decides at config save and at send time.
pub fn purpose_readiness(
    capabilities: &ProviderCapabilities,
    approval: ProviderApproval,
    identity: Option<&AccountIdentity>,
    check: &AccountCheck,
    purpose: MessagePurpose,
    language: Option<&str>,
) -> PurposeReadiness {
    let mut blockers = vec![];
    // A check of another identity (an earlier number or Page) says nothing
    // about this one: none of it counts until the account is checked as it
    // is now.
    let unchecked = check.checked_at.is_some()
        && identity.is_some_and(|current| !check.is_for(current));
    if unchecked {
        blockers.push(ReadinessBlocker::UNCHECKED);
    }
    let checked = !unchecked;
    if checked && !check.connected {
        blockers.push(ReadinessBlocker::NOT_CONNECTED);
    }
    if !capabilities.purposes.contains(&purpose) {
        blockers.push(ReadinessBlocker::UNSUPPORTED_PURPOSE);
    }
    let (approval, _) = check.approval(approval);
    if capabilities.requires_provider_approval
        && approval != ProviderApproval::CONFIRMED
    {
        blockers.push(ReadinessBlocker::NEEDS_PROVIDER_APPROVAL);
    }
    if checked && !check.production_access {
        blockers.push(ReadinessBlocker::NEEDS_PRODUCTION_ACCESS);
    }
    if checked && capabilities.template_required_for.contains(&purpose) {
        let approved = check
            .approved_templates
            .get(&purpose)
            .map(|languages| match language {
                Some(language) => languages.iter().any(|l| l == language),
                None => !languages.is_empty(),
            })
            .unwrap_or(false);
        if !approved {
            blockers.push(ReadinessBlocker::NEEDS_APPROVED_TEMPLATE);
        }
    }
    PurposeReadiness { purpose, blockers }
}

/// What the event configuration needs to know about an account.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct AccountSummary {
    pub id: String,
    pub tenant_id: String,
    pub channel: MessageChannel,
    pub provider: MessagingProvider,
    /// What the account can do; for configurable providers this comes from
    /// the account, not the provider.
    pub capabilities: ProviderCapabilities,
    pub provider_approval: ProviderApproval,
    pub readiness: ReadinessPolicy,
    pub check: AccountCheck,
    /// The account's identity now; `check` counts only if it is of this
    /// identity ([`AccountCheck::is_for`]).
    #[serde(default)]
    pub identity: Option<AccountIdentity>,
    /// Public, non-secret label shown to voters (sender name, Page name).
    pub public_label: Option<String>,
    /// Messenger only: the Page voters open to connect.
    pub messenger_page: Option<MessengerPage>,
    /// What stops the account's callbacks (delivery reports, Messenger's
    /// Get Started) from reaching Step: `CALLBACK_NOT_CONFIGURED`,
    /// `REPORT_CREDENTIALS_MISSING`. Receipts are unavailable while any is
    /// set, and Messenger isn't ready for codes without the callback
    /// address ([`callback_blocker`]).
    #[serde(default)]
    pub callback_blockers: Vec<ReadinessBlocker>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MessengerPage {
    pub page_id: String,
    pub username: Option<String>,
    pub name: Option<String>,
}

/// Binds a purpose and language to a provider-approved template.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct TemplateBinding {
    pub purpose: MessagePurpose,
    /// Which message this template is for: a template alias for bulk sends,
    /// or Keycloak's message key. `None` is the default for the purpose.
    #[serde(default)]
    pub key: Option<String>,
    /// The voter's language.
    pub language: String,
    /// Template name or ID as the provider knows it.
    pub provider_template: String,
    /// The provider's code for the language when it differs, such as
    /// `en_US` for `en`.
    #[serde(default)]
    pub provider_language: Option<String>,
}

impl TemplateBinding {
    /// The language code to send to the provider.
    pub fn provider_language(&self) -> &str {
        self.provider_language.as_deref().unwrap_or(&self.language)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct EventChannelConfig {
    pub channel: MessageChannel,
    pub account_id: String,
    /// Purposes this event enables on the channel.
    #[serde(default)]
    pub purposes: Vec<MessagePurpose>,
    #[serde(default)]
    pub templates: Vec<TemplateBinding>,
    #[serde(default)]
    pub out_of_window: OutOfWindowPolicy,
}

/// Stored as JSON in `messaging_event_config` (first copied from the event
/// annotation [`MESSAGING_CONFIG_ANNOTATION`]). Holds references only: no
/// credentials or recipient data.
///
/// The first release's `notice_fallback` and `reply_text` still parse and
/// are dropped: there is no fallback and no automatic reply.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct EventMessagingConfig {
    pub version: u32,
    #[serde(default)]
    pub channels: Vec<EventChannelConfig>,
    /// Per election, the channels it offers. Elections not listed offer
    /// every enabled channel.
    #[serde(default)]
    pub election_channels: BTreeMap<String, Vec<MessageChannel>>,
}

impl Default for EventMessagingConfig {
    fn default() -> Self {
        EventMessagingConfig {
            version: EVENT_MESSAGING_CONFIG_VERSION,
            channels: vec![],
            election_channels: BTreeMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "kind")]
#[allow(non_camel_case_types)]
pub enum MessagingConfigError {
    UNSUPPORTED_VERSION {
        version: u32,
    },
    DUPLICATE_CHANNEL {
        channel: MessageChannel,
    },
    UNKNOWN_ACCOUNT {
        channel: MessageChannel,
        account_id: String,
    },
    ACCOUNT_OF_ANOTHER_TENANT {
        account_id: String,
    },
    ACCOUNT_CHANNEL_MISMATCH {
        channel: MessageChannel,
        account_id: String,
    },
    PURPOSE_NOT_READY {
        channel: MessageChannel,
        purpose: MessagePurpose,
        blockers: Vec<ReadinessBlocker>,
    },
    TEMPLATE_NOT_APPROVED {
        channel: MessageChannel,
        purpose: MessagePurpose,
        language: String,
    },
    OUT_OF_WINDOW_NOT_SUPPORTED {
        channel: MessageChannel,
    },
    /// Messenger notices reach voters outside the 24-hour window only as
    /// an approved utility template: bind one for notices.
    UTILITY_TEMPLATE_MISSING {
        channel: MessageChannel,
    },
    ELECTION_CHANNEL_NOT_ENABLED {
        election_id: String,
        channel: MessageChannel,
    },
    UNKNOWN_ELECTION {
        election_id: String,
    },
}

impl EventChannelConfig {
    /// The out-of-window policy in force. Messenger always uses the
    /// approved utility template outside the 24-hour window: "do not send"
    /// is gone, since nothing else would reach the voter. Stored values of
    /// the first release still parse.
    pub fn effective_out_of_window(&self) -> OutOfWindowPolicy {
        match self.channel {
            MessageChannel::MESSENGER => OutOfWindowPolicy::UTILITY_MESSAGES,
            _ => self.out_of_window,
        }
    }

    /// Why this channel can't carry notices as configured, if it can't: a
    /// Messenger channel with notices needs a bound utility template.
    pub fn notice_blocker(&self) -> Option<MessagingConfigError> {
        let template_bound = self
            .templates
            .iter()
            .any(|binding| binding.purpose == MessagePurpose::NOTICE);
        (self.channel == MessageChannel::MESSENGER
            && self.purposes.contains(&MessagePurpose::NOTICE)
            && !template_bound)
            .then_some(MessagingConfigError::UTILITY_TEMPLATE_MISSING {
                channel: self.channel,
            })
    }
}

impl EventMessagingConfig {
    pub fn channel(
        &self,
        channel: MessageChannel,
    ) -> Option<&EventChannelConfig> {
        self.channels.iter().find(|c| c.channel == channel)
    }

    /// The approved template for a message on `channel`. The most specific
    /// binding wins: the message's key and language, then the key in any
    /// language, then the purpose's default in the language, then any
    /// default for the purpose.
    pub fn template_for(
        &self,
        channel: MessageChannel,
        purpose: MessagePurpose,
        key: Option<&str>,
        language: Option<&str>,
    ) -> Option<&TemplateBinding> {
        let bindings: Vec<&TemplateBinding> = self
            .channel(channel)?
            .templates
            .iter()
            .filter(|binding| binding.purpose == purpose)
            .collect();
        let keyed = |binding: &&&TemplateBinding| {
            key.is_some() && binding.key.as_deref() == key
        };
        let default = |binding: &&&TemplateBinding| binding.key.is_none();
        let in_language = |binding: &&&TemplateBinding| {
            language.is_some_and(|language| {
                binding.language == language
                    || binding.provider_language() == language
            })
        };
        bindings
            .iter()
            .find(|b| keyed(b) && in_language(b))
            .or_else(|| bindings.iter().find(|b| keyed(b)))
            .or_else(|| bindings.iter().find(|b| default(b) && in_language(b)))
            .or_else(|| bindings.iter().find(|b| default(b)))
            .copied()
    }

    /// Channels enabled for `purpose`, optionally restricted to an election.
    pub fn enabled_channels(
        &self,
        purpose: MessagePurpose,
        election_id: Option<&str>,
    ) -> Vec<MessageChannel> {
        let restriction = election_id
            .and_then(|id| self.election_channels.get(id))
            .map(|channels| channels.iter().copied().collect::<HashSet<_>>());
        self.channels
            .iter()
            .filter(|c| c.purposes.contains(&purpose))
            .map(|c| c.channel)
            .filter(|channel| {
                restriction
                    .as_ref()
                    .map(|allowed| allowed.contains(channel))
                    .unwrap_or(true)
            })
            .collect()
    }

    /// Checks references, tenant ownership, readiness of enabled purposes,
    /// template approvals, Messenger notices and election restrictions.
    pub fn validate(
        &self,
        tenant_id: &str,
        accounts: &[AccountSummary],
        election_ids: &[String],
    ) -> Result<(), Vec<MessagingConfigError>> {
        let mut errors = vec![];
        if self.version != EVENT_MESSAGING_CONFIG_VERSION {
            errors.push(MessagingConfigError::UNSUPPORTED_VERSION {
                version: self.version,
            });
        }
        let accounts_by_id: HashMap<&str, &AccountSummary> =
            accounts.iter().map(|a| (a.id.as_str(), a)).collect();
        let mut seen = HashSet::new();
        for channel_config in &self.channels {
            let channel = channel_config.channel;
            if !seen.insert(channel) {
                errors
                    .push(MessagingConfigError::DUPLICATE_CHANNEL { channel });
                continue;
            }
            let Some(account) =
                accounts_by_id.get(channel_config.account_id.as_str())
            else {
                errors.push(MessagingConfigError::UNKNOWN_ACCOUNT {
                    channel,
                    account_id: channel_config.account_id.clone(),
                });
                continue;
            };
            if account.tenant_id != tenant_id {
                errors.push(MessagingConfigError::ACCOUNT_OF_ANOTHER_TENANT {
                    account_id: account.id.clone(),
                });
                continue;
            }
            if account.channel != channel {
                errors.push(MessagingConfigError::ACCOUNT_CHANNEL_MISMATCH {
                    channel,
                    account_id: account.id.clone(),
                });
                continue;
            }
            if account.provider.support() == ProviderSupport::UNSUPPORTED {
                for purpose in &channel_config.purposes {
                    errors.push(MessagingConfigError::PURPOSE_NOT_READY {
                        channel,
                        purpose: *purpose,
                        blockers: vec![ReadinessBlocker::UNSUPPORTED_PROVIDER],
                    });
                }
                continue;
            }
            for purpose in &channel_config.purposes {
                let mut readiness = purpose_readiness(
                    &account.capabilities,
                    account.provider_approval,
                    account.identity.as_ref(),
                    &account.check,
                    *purpose,
                    None,
                );
                readiness.blockers.extend(callback_blocker(
                    channel,
                    *purpose,
                    !account
                        .callback_blockers
                        .contains(&ReadinessBlocker::CALLBACK_NOT_CONFIGURED),
                ));
                if !readiness.is_ready() {
                    errors.push(MessagingConfigError::PURPOSE_NOT_READY {
                        channel,
                        purpose: *purpose,
                        blockers: readiness.blockers,
                    });
                }
            }
            let template_purposes = &account.capabilities.template_required_for;
            for binding in &channel_config.templates {
                if !template_purposes.contains(&binding.purpose) {
                    continue;
                }
                let approved = account
                    .check
                    .approved_templates
                    .get(&binding.purpose)
                    .map(|langs| {
                        langs.iter().any(|l| {
                            l == binding.provider_language()
                                || *l == binding.language
                        })
                    })
                    .unwrap_or(false);
                if !approved {
                    errors.push(MessagingConfigError::TEMPLATE_NOT_APPROVED {
                        channel,
                        purpose: binding.purpose,
                        language: binding.language.clone(),
                    });
                }
            }
            // Out-of-window templates only make sense on a channel with a
            // conversation window whose notices are otherwise free text.
            let free_text_in_a_window =
                account.capabilities.conversation_window_hours.is_some()
                    && !template_purposes.contains(&MessagePurpose::NOTICE);
            if channel_config.effective_out_of_window()
                != OutOfWindowPolicy::DISABLED
                && !free_text_in_a_window
            {
                errors.push(
                    MessagingConfigError::OUT_OF_WINDOW_NOT_SUPPORTED {
                        channel,
                    },
                );
            }
        }
        for channel_config in &self.channels {
            if let Some(reason) = channel_config.notice_blocker() {
                errors.push(reason);
            }
        }
        let enabled: HashSet<MessageChannel> =
            self.channels.iter().map(|c| c.channel).collect();
        for (election_id, channels) in &self.election_channels {
            if !election_ids.contains(election_id) {
                errors.push(MessagingConfigError::UNKNOWN_ELECTION {
                    election_id: election_id.clone(),
                });
                continue;
            }
            for channel in channels {
                if !enabled.contains(channel) {
                    errors.push(
                        MessagingConfigError::ELECTION_CHANNEL_NOT_ENABLED {
                            election_id: election_id.clone(),
                            channel: *channel,
                        },
                    );
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// What Keycloak pages may see: channel labels and purposes, never
    /// account identifiers or credentials.
    ///
    /// `election_labels` are the names a voter record may hold for each
    /// election (its name, alias and external ID), so a restricted election
    /// can be recognised by, for instance, the voter's Post.
    pub fn public_projection(
        &self,
        accounts: &[AccountSummary],
        election_labels: &BTreeMap<String, Vec<String>>,
    ) -> PublicMessagingChannels {
        let accounts_by_id: HashMap<&str, &AccountSummary> =
            accounts.iter().map(|a| (a.id.as_str(), a)).collect();
        let channels = self
            .channels
            .iter()
            .filter(|c| !c.purposes.is_empty())
            .filter_map(|c| {
                let account = accounts_by_id.get(c.account_id.as_str())?;
                Some(PublicChannel {
                    channel: c.channel,
                    purposes: c.purposes.clone(),
                    sender_label: account.public_label.clone(),
                    messenger_page: if c.channel == MessageChannel::MESSENGER {
                        account.messenger_page.as_ref().map(|page| {
                            PublicMessengerPage {
                                page_id: page.page_id.clone(),
                                username: page.username.clone(),
                                name: page.name.clone(),
                            }
                        })
                    } else {
                        None
                    },
                })
            })
            .collect();
        PublicMessagingChannels {
            version: EVENT_MESSAGING_CONFIG_VERSION,
            channels,
            election_channels: self.election_channels.clone(),
            // Every election once any is restricted, so that an
            // unrestricted election is told apart from an unknown value.
            election_labels: if self.election_channels.is_empty() {
                BTreeMap::new()
            } else {
                election_labels.clone()
            },
        }
    }
}

/// Stored in the realm attribute [`REALM_ATTR_MESSAGING`].
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct PublicMessagingChannels {
    pub version: u32,
    pub channels: Vec<PublicChannel>,
    #[serde(default)]
    pub election_channels: BTreeMap<String, Vec<MessageChannel>>,
    /// For each restricted election, the names that identify it besides
    /// its ID.
    #[serde(default)]
    pub election_labels: BTreeMap<String, Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct PublicChannel {
    pub channel: MessageChannel,
    pub purposes: Vec<MessagePurpose>,
    pub sender_label: Option<String>,
    pub messenger_page: Option<PublicMessengerPage>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct PublicMessengerPage {
    /// Public Page ID, used for the `m.me` link when there is no username.
    pub page_id: String,
    pub username: Option<String>,
    pub name: Option<String>,
}

impl MessageChannel {
    /// Counter in the event and election `statistics` JSON.
    pub fn statistics_key(&self) -> &'static str {
        match self {
            MessageChannel::EMAIL => "num_emails_sent",
            MessageChannel::SMS => "num_sms_sent",
            MessageChannel::WHATSAPP => "num_whatsapp_sent",
            MessageChannel::VIBER => "num_viber_sent",
            MessageChannel::MESSENGER => "num_messenger_sent",
        }
    }
}

/// Non-secret identifiers of a sending account, per provider. Stored in
/// `messaging_account.sender`.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "provider")]
#[allow(non_camel_case_types)]
pub enum AccountSender {
    AWS_SES {
        from_address: String,
        from_name: Option<String>,
        region: Option<String>,
        /// SNS topic SES publishes delivery and bounce events to.
        /// Notifications from any other topic are refused.
        #[serde(default)]
        notification_topic_arn: Option<String>,
    },
    SMTP {
        from_address: String,
        from_name: Option<String>,
        server_url: String,
    },
    AWS_SNS {
        sender_id: Option<String>,
        origination_number: Option<String>,
        region: Option<String>,
    },
    WHATSAPP_CLOUD_API {
        business_account_id: String,
        phone_number_id: String,
        display_phone_number: String,
        display_name: Option<String>,
        /// Graph API version, such as `v23.0`.
        api_version: String,
        /// Graph API base URL when it is not Meta's own, such as a
        /// Solution Provider's Cloud API endpoint.
        #[serde(default)]
        api_base_url: Option<String>,
    },
    MESSENGER_SEND_API {
        page_id: String,
        page_name: Option<String>,
        page_username: Option<String>,
        api_version: String,
        #[serde(default)]
        api_base_url: Option<String>,
    },
    VIBER_INFOBIP {
        /// The account's Infobip API base URL.
        base_url: String,
        /// The Viber sender (service) name registered with Infobip.
        sender: String,
        /// Templates the partner approved, per purpose, as language to
        /// template ID. Entered by an administrator: Infobip's template
        /// API is not generally available.
        #[serde(default)]
        approved_templates: BTreeMap<MessagePurpose, BTreeMap<String, String>>,
    },
    /// Any provider with a JSON HTTP API, described by configuration.
    HTTP_API(HttpApiSender),
    CONSOLE {},
}

/// How a phone number is written in a request.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    Default,
)]
pub enum PhoneFormat {
    /// `+639171234567`
    #[default]
    E164,
    /// `639171234567`
    DIGITS,
}

/// An HTTP request whose URL, headers and JSON body may hold placeholders.
///
/// Placeholders: `{{to}}`, `{{text}}`, `{{subject}}`, `{{html}}`,
/// `{{code}}`, `{{template}}`, `{{language}}`, `{{message_id}}`,
/// `{{callback_url}}`, `{{param.1}}`…, `{{credential.NAME}}`,
/// `{{basic_auth}}` (USERNAME and PASSWORD), `{{token}}` (from the
/// account's token request) and `{{jwt}}`. A body string that is exactly
/// `{{parameters}}` becomes the array of template parameters, and
/// `{{named_parameters}}` the object of those written as `@name=value`.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HttpRequestTemplate {
    #[serde(default = "default_http_method")]
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

fn default_http_method() -> String {
    "POST".to_string()
}

/// How a provider's callbacks are authenticated.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
#[serde(tag = "kind")]
#[allow(non_camel_case_types)]
pub enum HttpWebhookAuth {
    /// Only the unguessable webhook URL.
    #[default]
    URL_KEY,
    /// A header must equal the WEBHOOK_SECRET credential.
    HEADER_SECRET { header: String },
    /// A header carries an HMAC-SHA256 under WEBHOOK_SECRET.
    HMAC_SHA256 {
        header: String,
        /// Text before the digest, such as `sha256=`.
        #[serde(default)]
        prefix: Option<String>,
        #[serde(default)]
        encoding: DigestEncoding,
        /// What is signed when it is not the body alone, with `{{body}}`
        /// and `{{header.NAME}}`, such as
        /// `{{body}}.{{header.x-nonce}}.{{header.x-timestamp}}`.
        #[serde(default)]
        signed: Option<String>,
    },
    /// A header carries `Bearer <JWT>` signed with HS256 under
    /// WEBHOOK_SECRET. A `payload_hash` claim, when present, must be the
    /// SHA-256 of the body.
    JWT_HS256 { header: String },
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    Default,
)]
pub enum JwtAlgorithm {
    /// Signed with the PEM private key in API_SECRET.
    #[default]
    RS256,
    /// Signed with the shared secret in API_SECRET.
    HS256,
}

/// A JWT minted for each request, available as `{{jwt}}`.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HttpJwt {
    #[serde(default)]
    pub algorithm: JwtAlgorithm,
    /// Claims besides `iat`, `exp` and `jti`, which are added. Strings may
    /// hold placeholders.
    #[serde(default)]
    pub claims: serde_json::Value,
    #[serde(default = "default_token_lifetime")]
    pub lifetime_seconds: u64,
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
    Default,
)]
pub enum DigestEncoding {
    #[default]
    HEX,
    BASE64,
}

/// Reading a provider's status values. Pointers are JSON pointers
/// (RFC 6901) into the response or report.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HttpStatusMapping {
    /// Where the provider's message ID is.
    pub message_id_pointer: String,
    /// Where the provider's status value is.
    pub state_pointer: String,
    /// Provider status values and the attempt state each means. Values not
    /// listed are ignored.
    pub states: BTreeMap<String, MessageAttemptState>,
    #[serde(default)]
    pub error_pointer: Option<String>,
}

/// Delivery reports and replies the provider posts to Step.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HttpReports {
    #[serde(default)]
    pub auth: HttpWebhookAuth,
    /// Where the list of reports is in the payload; the payload itself when
    /// absent.
    #[serde(default)]
    pub items_pointer: Option<String>,
    pub status: HttpStatusMapping,
    /// In an item that is an incoming message, where the sender is.
    #[serde(default)]
    pub inbound_from_pointer: Option<String>,
}

/// Asking the provider about one message.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HttpReconcile {
    pub request: HttpRequestTemplate,
    pub status: HttpStatusMapping,
}

/// Obtaining a short-lived token before sending, such as OAuth client
/// credentials.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HttpTokenRequest {
    pub request: HttpRequestTemplate,
    pub token_pointer: String,
    /// Seconds a token is reused for.
    #[serde(default = "default_token_lifetime")]
    pub lifetime_seconds: u64,
}

fn default_token_lifetime() -> u64 {
    300
}

/// A provider described by configuration: the request that sends a
/// message, and optionally how to check the account, read delivery
/// reports and look a message up.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HttpApiSender {
    /// Shown to voters as the sender.
    #[serde(default)]
    pub label: Option<String>,
    pub send: HttpRequestTemplate,
    /// Where the provider's message ID is in the send response.
    #[serde(default)]
    pub message_id_pointer: Option<String>,
    #[serde(default)]
    pub phone_format: PhoneFormat,
    /// Purposes that need a provider-approved template.
    #[serde(default)]
    pub template_required_for: Vec<MessagePurpose>,
    /// Hours after the recipient's last message during which free-form
    /// messages may be sent.
    #[serde(default)]
    pub conversation_window_hours: Option<u32>,
    /// A request that succeeds when the credentials work.
    #[serde(default)]
    pub check: Option<HttpRequestTemplate>,
    #[serde(default)]
    pub token: Option<HttpTokenRequest>,
    #[serde(default)]
    pub jwt: Option<HttpJwt>,
    #[serde(default)]
    pub reports: Option<HttpReports>,
    #[serde(default)]
    pub reconcile: Option<HttpReconcile>,
    /// Languages with an approved template, per purpose, as confirmed with
    /// the provider.
    #[serde(default)]
    pub approved_templates: BTreeMap<MessagePurpose, Vec<String>>,
}

impl AccountSender {
    /// What identifies the sender to its provider: a check of another
    /// identity says nothing about this one. Names, labels, API versions
    /// and templates are not part of it.
    pub fn identity(&self) -> AccountIdentity {
        let ids: Vec<(&str, Option<&str>)> = match self {
            AccountSender::AWS_SES {
                from_address,
                region,
                ..
            } => vec![
                ("from_address", Some(from_address.as_str())),
                ("region", region.as_deref()),
            ],
            AccountSender::SMTP {
                from_address,
                server_url,
                ..
            } => vec![
                ("from_address", Some(from_address.as_str())),
                ("server_url", Some(server_url.as_str())),
            ],
            AccountSender::AWS_SNS {
                sender_id,
                origination_number,
                region,
            } => vec![
                ("sender_id", sender_id.as_deref()),
                ("origination_number", origination_number.as_deref()),
                ("region", region.as_deref()),
            ],
            AccountSender::WHATSAPP_CLOUD_API {
                business_account_id,
                phone_number_id,
                ..
            } => vec![
                ("business_account_id", Some(business_account_id.as_str())),
                ("phone_number_id", Some(phone_number_id.as_str())),
            ],
            AccountSender::MESSENGER_SEND_API { page_id, .. } => {
                vec![("page_id", Some(page_id.as_str()))]
            }
            AccountSender::VIBER_INFOBIP {
                base_url, sender, ..
            } => vec![
                ("base_url", Some(base_url.trim_end_matches('/'))),
                ("sender", Some(sender.as_str())),
            ],
            AccountSender::HTTP_API(_) | AccountSender::CONSOLE {} => vec![],
        };
        AccountIdentity {
            provider: self.provider(),
            ids: ids
                .into_iter()
                .filter_map(|(name, value)| {
                    let value = value?.trim();
                    (!value.is_empty())
                        .then(|| (name.to_string(), value.to_string()))
                })
                .collect(),
        }
    }

    /// What this account can do on `channel`.
    pub fn capabilities(
        &self,
        channel: MessageChannel,
    ) -> Option<ProviderCapabilities> {
        let mut capabilities = self.provider().capabilities(channel)?;
        if let AccountSender::HTTP_API(api) = self {
            capabilities.template_required_for =
                api.template_required_for.clone();
            capabilities.conversation_window_hours =
                api.conversation_window_hours;
            capabilities.delivery_feedback = if api.reports.is_some() {
                DeliveryFeedback::PROVIDER_RECEIPTS
            } else {
                DeliveryFeedback::UNAVAILABLE
            };
            capabilities.reconciliation = api.reconcile.is_some();
        }
        // SES reports deliveries only through its SNS notification topic.
        if let AccountSender::AWS_SES {
            notification_topic_arn,
            ..
        } = self
        {
            if notification_topic_arn
                .as_deref()
                .map_or(true, |topic| topic.trim().is_empty())
            {
                capabilities.delivery_feedback = DeliveryFeedback::UNAVAILABLE;
            }
        }
        Some(capabilities)
    }

    pub fn provider(&self) -> MessagingProvider {
        match self {
            AccountSender::HTTP_API(_) => MessagingProvider::HTTP_API,
            AccountSender::AWS_SES { .. } => MessagingProvider::AWS_SES,
            AccountSender::SMTP { .. } => MessagingProvider::SMTP,
            AccountSender::AWS_SNS { .. } => MessagingProvider::AWS_SNS,
            AccountSender::WHATSAPP_CLOUD_API { .. } => {
                MessagingProvider::WHATSAPP_CLOUD_API
            }
            AccountSender::MESSENGER_SEND_API { .. } => {
                MessagingProvider::MESSENGER_SEND_API
            }
            AccountSender::VIBER_INFOBIP { .. } => {
                MessagingProvider::VIBER_INFOBIP
            }
            AccountSender::CONSOLE {} => MessagingProvider::CONSOLE,
        }
    }

    /// Public label shown to voters: sender name, number or Page name.
    pub fn public_label(&self) -> Option<String> {
        match self {
            AccountSender::AWS_SES {
                from_address,
                from_name,
                ..
            }
            | AccountSender::SMTP {
                from_address,
                from_name,
                ..
            } => Some(from_name.clone().unwrap_or(from_address.clone())),
            AccountSender::AWS_SNS {
                sender_id,
                origination_number,
                ..
            } => sender_id.clone().or(origination_number.clone()),
            AccountSender::WHATSAPP_CLOUD_API {
                display_name,
                display_phone_number,
                ..
            } => Some(
                display_name.clone().unwrap_or(display_phone_number.clone()),
            ),
            AccountSender::MESSENGER_SEND_API {
                page_name, page_id, ..
            } => Some(page_name.clone().unwrap_or(page_id.clone())),
            AccountSender::VIBER_INFOBIP { sender, .. } => Some(sender.clone()),
            AccountSender::HTTP_API(api) => api.label.clone(),
            AccountSender::CONSOLE {} => None,
        }
    }

    pub fn messenger_page(&self) -> Option<MessengerPage> {
        match self {
            AccountSender::MESSENGER_SEND_API {
                page_id,
                page_name,
                page_username,
                ..
            } => Some(MessengerPage {
                page_id: page_id.clone(),
                username: page_username.clone(),
                name: page_name.clone(),
            }),
            _ => None,
        }
    }
}

/// Write-only credentials of an account. Values live in the secret store;
/// the account row only records when each was last replaced.
#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Clone,
    Copy,
    EnumString,
    EnumIter,
    PartialOrd,
    Ord,
)]
pub enum CredentialName {
    /// WhatsApp system-user token or Messenger Page access token.
    ACCESS_TOKEN,
    /// Meta app secret, used to verify webhook signatures.
    APP_SECRET,
    /// Token Meta echoes during webhook subscription. Generated by Step.
    VERIFY_TOKEN,
    /// Infobip API key.
    API_KEY,
    SMTP_PASSWORD,
    AWS_ACCESS_KEY_ID,
    AWS_SECRET_ACCESS_KEY,
    /// Configurable providers: a second API secret.
    API_SECRET,
    USERNAME,
    PASSWORD,
    /// Shared secret a configurable provider's callbacks are checked with.
    WEBHOOK_SECRET,
    /// Basic auth user name Infobip's report subscription sends. Generated
    /// by Step and shown once.
    REPORT_USERNAME,
    /// Basic auth password Infobip's report subscription sends. Generated
    /// by Step and shown once.
    REPORT_PASSWORD,
}

impl CredentialName {
    /// Credentials Step generates; an administrator can't enter them.
    pub fn is_generated(&self) -> bool {
        matches!(
            self,
            CredentialName::VERIFY_TOKEN
                | CredentialName::REPORT_USERNAME
                | CredentialName::REPORT_PASSWORD
        )
    }
}

impl MessagingProvider {
    /// Credentials this provider needs before it can send. Webhook
    /// credentials are not among them: sending works before callbacks are
    /// set up.
    pub fn required_credentials(&self) -> Vec<CredentialName> {
        match self {
            MessagingProvider::WHATSAPP_CLOUD_API
            | MessagingProvider::MESSENGER_SEND_API => {
                vec![CredentialName::ACCESS_TOKEN]
            }
            MessagingProvider::VIBER_INFOBIP => vec![CredentialName::API_KEY],
            MessagingProvider::SMTP => vec![CredentialName::SMTP_PASSWORD],
            MessagingProvider::AWS_SES
            | MessagingProvider::AWS_SNS
            | MessagingProvider::HTTP_API
            | MessagingProvider::CONSOLE => vec![],
        }
    }

    /// Credentials an administrator may set. AWS keys are optional: without
    /// them the service's own role is used.
    pub fn accepted_credentials(&self) -> Vec<CredentialName> {
        match self {
            MessagingProvider::AWS_SES | MessagingProvider::AWS_SNS => vec![
                CredentialName::AWS_ACCESS_KEY_ID,
                CredentialName::AWS_SECRET_ACCESS_KEY,
            ],
            MessagingProvider::WHATSAPP_CLOUD_API
            | MessagingProvider::MESSENGER_SEND_API => vec![
                CredentialName::ACCESS_TOKEN,
                CredentialName::APP_SECRET,
                CredentialName::VERIFY_TOKEN,
            ],
            MessagingProvider::VIBER_INFOBIP => vec![
                CredentialName::API_KEY,
                CredentialName::REPORT_USERNAME,
                CredentialName::REPORT_PASSWORD,
            ],
            MessagingProvider::HTTP_API => vec![
                CredentialName::API_KEY,
                CredentialName::API_SECRET,
                CredentialName::ACCESS_TOKEN,
                CredentialName::USERNAME,
                CredentialName::PASSWORD,
                CredentialName::WEBHOOK_SECRET,
            ],
            _ => self.required_credentials(),
        }
    }
}

/// When a credential was last replaced. Never its value.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CredentialRecord {
    pub replaced_at: String,
}

/// Per-account sending limits. Codes keep `otp_reserved_per_second` even
/// when a bulk send saturates the account.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct AccountLimits {
    pub messages_per_second: Option<u32>,
    pub otp_reserved_per_second: Option<u32>,
    /// Country calling codes (digits, such as `63`) this account may send
    /// phone messages to. Empty: any.
    #[serde(default)]
    pub allowed_calling_codes: Vec<String>,
    /// Sends Keycloak asks for (codes and notices) this account may have in
    /// flight at once per harvest process; empty (or 0) means 8. The
    /// account's bucket is the limit across processes.
    #[serde(default)]
    pub otp_concurrency: Option<u32>,
}

/// Internal request from Keycloak or windmill to send one message.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct SendMessageRequest {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub voter_id: Option<String>,
    /// Codes: required. Notices to a voter may leave it out: harvest then
    /// sends on the voter's recorded channel, or records "No contact
    /// channel".
    #[serde(default)]
    pub channel: Option<MessageChannel>,
    pub purpose: MessagePurpose,
    /// Phone number (E.164), email address or Page-scoped ID. Optional
    /// like `channel`.
    #[serde(default)]
    pub destination: Option<String>,
    /// Whose channel a notice goes to.
    #[serde(default)]
    pub recipient: NoticeRecipient,
    pub language: Option<String>,
    pub content: MessageContent,
    /// Which message this is, such as Keycloak's message key, to pick its
    /// approved template among the event's bindings.
    #[serde(default)]
    pub template_key: Option<String>,
    /// The voter's elections, so per-election (Post) channel restrictions
    /// apply to this message too.
    #[serde(default)]
    pub election_ids: Vec<String>,
    /// Identifies the logical message across retries. A second request with
    /// the same key does not send again once the first was accepted.
    pub logical_key: String,
    /// Codes only: when the challenge expires (RFC 3339). Expired codes are
    /// never sent.
    pub expires_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MessageContent {
    /// Email subject.
    pub subject: Option<String>,
    /// Plain text body, also the in-conversation text for Messenger.
    pub text: String,
    pub html: Option<String>,
    /// Approved-template parameters, in order (WhatsApp, Viber).
    #[serde(default)]
    pub template_parameters: Vec<String>,
    /// Codes only: the code, so a WhatsApp authentication template can carry
    /// it in its copy-code button. Never stored.
    pub code: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub state: MessageAttemptState,
    /// Short reason when not accepted. Never contains message content.
    pub reason: Option<String>,
}

/// Internal request from Keycloak to start linking Messenger for a code.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CreateMessengerLinkRequest {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    /// Keycloak authentication session, as an opaque digest.
    pub auth_session: String,
    /// The live challenge, as an opaque digest.
    pub challenge: String,
    /// The code to send once the voter interacts. Held encrypted until used,
    /// replaced or expired.
    pub code: String,
    pub language: Option<String>,
    pub content: MessageContent,
    /// When the code expires (RFC 3339). The reference never outlives it.
    pub expires_at: String,
    /// The saved voter's Page-scoped ID (sign-in, reset). The code then
    /// goes only to that conversation: Get Started from any other one
    /// consumes the link without sending, answered as an unknown reference
    /// is. Absent while enrolling. Held encrypted with the code, never
    /// logged.
    #[serde(default)]
    pub expected_recipient: Option<String>,
    /// The Page `expected_recipient` belongs to (the voter's
    /// `messenger-page`). When it isn't the Page of the event's Messenger
    /// account any more, the saved conversation can't be reached and the
    /// link is refused with [`MessageReason::MESSENGER_PAGE_CHANGED`]: the
    /// caller may then start a reconnect, a link without
    /// `expected_recipient` whose confirmation names the new Page-scoped ID
    /// and Page. Absent: not checked (callers before this field).
    #[serde(default)]
    pub expected_page: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CreateMessengerLinkResponse {
    pub reference: String,
    /// `m.me` link with the reference. The only way to complete the link:
    /// there is no word to type in the chat.
    pub link: String,
    pub expires_at: String,
    /// The Page the link opens: the event's Messenger account's Page now.
    #[serde(default)]
    pub page_id: Option<String>,
}

#[allow(non_camel_case_types)]
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    EnumString,
)]
pub enum MessengerLinkState {
    /// Waiting for the voter to interact with the Page.
    PENDING,
    /// The voter interacted and the code was handed to Messenger.
    CODE_SENT,
    /// The code was verified in the original session.
    CONFIRMED,
    EXPIRED,
    REPLACED,
}

/// Internal request from Keycloak once the code was verified, or to check
/// progress.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MessengerLinkRequest {
    pub tenant_id: String,
    pub reference: String,
    pub auth_session: String,
    pub challenge: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct MessengerLinkStatus {
    pub state: MessengerLinkState,
    /// Only once `CONFIRMED`: the Page-scoped ID to store on the voter.
    pub page_scoped_id: Option<String>,
    pub page_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator;

    const TENANT: &str = "tenant-1";

    fn ready_check(templates: &[(MessagePurpose, &str)]) -> AccountCheck {
        let mut approved_templates: BTreeMap<MessagePurpose, Vec<String>> =
            BTreeMap::new();
        for (purpose, language) in templates {
            approved_templates
                .entry(*purpose)
                .or_default()
                .push(language.to_string());
        }
        AccountCheck {
            connected: true,
            production_access: true,
            approved_templates,
            checked_at: Some("2026-10-02T00:00:00Z".to_string()),
            reason: None,
            details: AccountCheckDetails::default(),
        }
    }

    fn account(
        id: &str,
        channel: MessageChannel,
        provider: MessagingProvider,
        check: AccountCheck,
    ) -> AccountSummary {
        AccountSummary {
            id: id.to_string(),
            tenant_id: TENANT.to_string(),
            channel,
            provider,
            capabilities: caps(provider, channel),
            provider_approval: ProviderApproval::PENDING,
            readiness: ReadinessPolicy::PROVIDER_CHECK,
            check,
            identity: None,
            public_label: Some(format!("{id} label")),
            messenger_page: None,
            callback_blockers: vec![],
        }
    }

    fn caps(
        provider: MessagingProvider,
        channel: MessageChannel,
    ) -> ProviderCapabilities {
        provider.capabilities(channel).expect("capabilities")
    }

    fn binding(
        key: Option<&str>,
        language: &str,
        template: &str,
    ) -> TemplateBinding {
        TemplateBinding {
            purpose: MessagePurpose::NOTICE,
            key: key.map(str::to_string),
            language: language.to_string(),
            provider_template: template.to_string(),
            provider_language: None,
        }
    }

    #[test]
    fn an_admin_confirmation_no_longer_bypasses_the_provider_check() {
        let readiness = purpose_readiness(
            &caps(
                MessagingProvider::WHATSAPP_CLOUD_API,
                MessageChannel::WHATSAPP,
            ),
            ProviderApproval::CONFIRMED,
            None,
            &AccountCheck::default(),
            MessagePurpose::OTP,
            Some("en"),
        );
        assert_eq!(
            readiness.blockers,
            vec![
                ReadinessBlocker::NOT_CONNECTED,
                ReadinessBlocker::NEEDS_PRODUCTION_ACCESS,
                ReadinessBlocker::NEEDS_APPROVED_TEMPLATE,
            ]
        );
        // Stored accounts with the removed policy still parse, and count
        // as the provider check.
        assert_eq!(
            "ADMIN_CONFIRMED".parse::<ReadinessPolicy>(),
            Ok(ReadinessPolicy::ADMIN_CONFIRMED)
        );
        assert_eq!(
            ReadinessPolicy::ADMIN_CONFIRMED.effective(),
            ReadinessPolicy::PROVIDER_CHECK
        );
    }

    fn whatsapp_sender(phone_number_id: &str) -> AccountSender {
        AccountSender::WHATSAPP_CLOUD_API {
            business_account_id: " 100 ".to_string(),
            phone_number_id: phone_number_id.to_string(),
            display_phone_number: "+1 555 0100".to_string(),
            display_name: Some("Elections".to_string()),
            api_version: "v23.0".to_string(),
            api_base_url: None,
        }
    }

    /// AP-B3, R5-09: a check of another identity (an earlier number or
    /// Page) makes the account ready for nothing until it is checked as it
    /// is now. "Needs a check" is the check's identity, not a stored
    /// policy.
    #[test]
    fn a_check_of_another_identity_is_ready_for_nothing() {
        let old = whatsapp_sender("200");
        let new = whatsapp_sender("300");
        let mut check = ready_check(&[(MessagePurpose::OTP, "en")]);
        check.details.identity = Some(old.identity());
        let readiness = |identity: Option<&AccountIdentity>,
                         check: &AccountCheck| {
            purpose_readiness(
                &caps(
                    MessagingProvider::WHATSAPP_CLOUD_API,
                    MessageChannel::WHATSAPP,
                ),
                ProviderApproval::CONFIRMED,
                identity,
                check,
                MessagePurpose::OTP,
                Some("en"),
            )
        };
        assert!(readiness(Some(&old.identity()), &check).is_ready());
        assert!(readiness(None, &check).is_ready());
        assert_eq!(
            readiness(Some(&new.identity()), &check).blockers,
            vec![ReadinessBlocker::UNCHECKED]
        );
        // A check stored before identities were recorded counts for the
        // identity the account has.
        check.details.identity = None;
        assert!(readiness(Some(&new.identity()), &check).is_ready());
    }

    /// R5-09: a development row stored as `UNCHECKED` reads as the provider
    /// check, and nothing writes the value again.
    #[test]
    fn stored_unchecked_rows_read_as_the_provider_check() {
        for (text, policy) in [
            ("PROVIDER_CHECK", ReadinessPolicy::PROVIDER_CHECK),
            ("ADMIN_CONFIRMED", ReadinessPolicy::ADMIN_CONFIRMED),
            ("UNCHECKED", ReadinessPolicy::PROVIDER_CHECK),
        ] {
            assert_eq!(text.parse::<ReadinessPolicy>(), Ok(policy));
            assert_eq!(
                serde_json::from_value::<ReadinessPolicy>(serde_json::json!(
                    text
                ))
                .unwrap(),
                policy
            );
        }
        assert_eq!(
            ReadinessPolicy::PROVIDER_CHECK.to_string(),
            "PROVIDER_CHECK"
        );
        assert_eq!(
            serde_json::to_value(ReadinessPolicy::PROVIDER_CHECK).unwrap(),
            serde_json::json!("PROVIDER_CHECK")
        );
    }

    /// The identity is the provider and the identifying ids, trimmed;
    /// names, labels and API versions are not part of it.
    #[test]
    fn an_identity_is_the_identifying_ids() {
        let mut renamed = whatsapp_sender("200");
        if let AccountSender::WHATSAPP_CLOUD_API {
            display_name,
            api_version,
            business_account_id,
            ..
        } = &mut renamed
        {
            *display_name = Some("Other".to_string());
            *api_version = "v24.0".to_string();
            *business_account_id = "100".to_string();
        }
        assert_eq!(whatsapp_sender("200").identity(), renamed.identity());
        assert_ne!(
            whatsapp_sender("200").identity(),
            whatsapp_sender("300").identity()
        );
        assert_eq!(
            serde_json::to_value(whatsapp_sender("200").identity()).unwrap(),
            serde_json::json!({
                "provider": "WHATSAPP_CLOUD_API",
                "ids": {"business_account_id": "100", "phone_number_id": "200"}
            })
        );
        let sns = AccountSender::AWS_SNS {
            sender_id: Some("Elections".to_string()),
            origination_number: None,
            region: Some(" ".to_string()),
        };
        assert_eq!(
            sns.identity().ids,
            BTreeMap::from([(
                "sender_id".to_string(),
                "Elections".to_string()
            )])
        );
    }

    /// A check stored before `details` existed still parses.
    #[test]
    fn checks_stored_without_details_parse() {
        let check: AccountCheck = serde_json::from_value(serde_json::json!({
            "connected": true,
            "production_access": true,
            "checked_at": "2026-10-01T00:00:00Z",
            "reason": null
        }))
        .unwrap();
        assert_eq!(check.details, AccountCheckDetails::default());
    }

    /// The provider's approval comes from the check where it reports one;
    /// the administrator's entry only where it doesn't.
    #[test]
    fn a_reported_approval_wins_over_the_entered_one() {
        let mut check = ready_check(&[(MessagePurpose::OTP, "en")]);
        let readiness = |check: &AccountCheck, entered| {
            purpose_readiness(
                &caps(
                    MessagingProvider::WHATSAPP_CLOUD_API,
                    MessageChannel::WHATSAPP,
                ),
                entered,
                None,
                check,
                MessagePurpose::OTP,
                Some("en"),
            )
        };
        assert_eq!(
            check.approval(ProviderApproval::CONFIRMED),
            (ProviderApproval::CONFIRMED, ApprovalSource::ADMIN_ENTERED)
        );
        assert!(readiness(&check, ProviderApproval::CONFIRMED).is_ready());
        check.details.provider_approval = Some(ProviderApproval::PENDING);
        assert_eq!(
            check.approval(ProviderApproval::CONFIRMED),
            (ProviderApproval::PENDING, ApprovalSource::PROVIDER_CHECK)
        );
        assert_eq!(
            readiness(&check, ProviderApproval::CONFIRMED).blockers,
            vec![ReadinessBlocker::NEEDS_PROVIDER_APPROVAL]
        );
        check.details.provider_approval = Some(ProviderApproval::CONFIRMED);
        assert!(readiness(&check, ProviderApproval::PENDING).is_ready());
    }

    #[test]
    fn removed_providers_parse_and_are_unsupported() {
        for (sender, provider) in [
            (
                serde_json::json!({"provider": "CONSOLE"}),
                MessagingProvider::CONSOLE,
            ),
            (
                serde_json::json!({"provider": "HTTP_API", "send": {"url": "https://gateway.example.org/send"}}),
                MessagingProvider::HTTP_API,
            ),
        ] {
            let sender: AccountSender = serde_json::from_value(sender).unwrap();
            assert_eq!(sender.provider(), provider);
            assert_eq!(provider.support(), ProviderSupport::UNSUPPORTED);
        }
        for provider in MessagingProvider::iter().filter(|p| {
            !matches!(
                p,
                MessagingProvider::CONSOLE | MessagingProvider::HTTP_API
            )
        }) {
            assert_eq!(
                provider.support(),
                ProviderSupport::SUPPORTED,
                "{provider}"
            );
        }
    }

    #[test]
    fn an_event_cannot_enable_purposes_on_an_unsupported_account() {
        let config = EventMessagingConfig {
            channels: vec![EventChannelConfig {
                channel: MessageChannel::SMS,
                account_id: "console".to_string(),
                purposes: vec![MessagePurpose::NOTICE],
                ..serde_json::from_value(serde_json::json!({
                    "channel": "SMS", "account_id": "console",
                }))
                .unwrap()
            }],
            ..Default::default()
        };
        let account = account(
            "console",
            MessageChannel::SMS,
            MessagingProvider::CONSOLE,
            ready_check(&[]),
        );
        assert_eq!(
            config.validate(TENANT, &[account], &[]),
            Err(vec![MessagingConfigError::PURPOSE_NOT_READY {
                channel: MessageChannel::SMS,
                purpose: MessagePurpose::NOTICE,
                blockers: vec![ReadinessBlocker::UNSUPPORTED_PROVIDER],
            }])
        );
    }

    #[test]
    fn report_credentials_are_generated_never_entered() {
        let accepted = MessagingProvider::VIBER_INFOBIP.accepted_credentials();
        for name in [
            CredentialName::REPORT_USERNAME,
            CredentialName::REPORT_PASSWORD,
        ] {
            assert!(accepted.contains(&name));
            assert!(name.is_generated());
        }
        assert!(CredentialName::VERIFY_TOKEN.is_generated());
        assert!(!CredentialName::API_KEY.is_generated());
    }

    #[test]
    fn the_most_specific_template_binding_wins() {
        let config = EventMessagingConfig {
            channels: vec![EventChannelConfig {
                templates: vec![
                    binding(None, "en", "default_en"),
                    binding(None, "tl", "default_tl"),
                    binding(Some("reminder"), "en", "reminder_en"),
                    TemplateBinding {
                        provider_language: Some("fil".to_string()),
                        ..binding(Some("reminder"), "tl", "reminder_tl")
                    },
                ],
                ..channel_config(MessageChannel::WHATSAPP, "wa", &[])
            }],
            ..Default::default()
        };
        let template = |key, language| {
            config
                .template_for(
                    MessageChannel::WHATSAPP,
                    MessagePurpose::NOTICE,
                    key,
                    language,
                )
                .map(|b| b.provider_template.as_str())
        };
        assert_eq!(template(Some("reminder"), Some("tl")), Some("reminder_tl"));
        assert_eq!(
            template(Some("reminder"), Some("fil")),
            Some("reminder_tl")
        );
        assert_eq!(template(Some("reminder"), Some("fr")), Some("reminder_en"));
        assert_eq!(template(Some("reminder"), None), Some("reminder_en"));
        assert_eq!(template(Some("approved"), Some("tl")), Some("default_tl"));
        assert_eq!(template(None, None), Some("default_en"));
        assert_eq!(
            config
                .template_for(
                    MessageChannel::WHATSAPP,
                    MessagePurpose::OTP,
                    None,
                    Some("en")
                )
                .map(|b| b.provider_template.as_str()),
            None
        );
        assert_eq!(
            config
                .template_for(
                    MessageChannel::WHATSAPP,
                    MessagePurpose::NOTICE,
                    Some("reminder"),
                    Some("tl")
                )
                .map(TemplateBinding::provider_language),
            Some("fil")
        );
    }

    #[test]
    fn a_configured_http_provider_declares_its_own_capabilities() {
        let sender: AccountSender = serde_json::from_value(serde_json::json!({
            "provider": "HTTP_API",
            "label": "COMELEC",
            "send": {
                "url": "https://partner.example/v1/viber",
                "headers": {"Authorization": "Bearer {{credential.API_KEY}}"},
                "body": {"to": "{{to}}", "template": "{{template}}", "params": "{{parameters}}"}
            },
            "message_id_pointer": "/id",
            "phone_format": "DIGITS",
            "template_required_for": ["OTP"],
            "reports": {
                "auth": {"kind": "HEADER_SECRET", "header": "X-Secret"},
                "status": {
                    "message_id_pointer": "/id",
                    "state_pointer": "/status",
                    "states": {"delivered": "DELIVERED", "failed": "FAILED"}
                }
            }
        }))
        .expect("sender");
        assert_eq!(sender.provider(), MessagingProvider::HTTP_API);
        assert_eq!(sender.public_label(), Some("COMELEC".to_string()));
        let capabilities = sender
            .capabilities(MessageChannel::VIBER)
            .expect("capabilities");
        assert_eq!(capabilities.recipient, RecipientKind::PHONE_NUMBER);
        assert_eq!(
            capabilities.template_required_for,
            vec![MessagePurpose::OTP]
        );
        assert_eq!(
            capabilities.delivery_feedback,
            DeliveryFeedback::PROVIDER_RECEIPTS
        );
        assert!(!capabilities.reconciliation);
        let AccountSender::HTTP_API(api) = &sender else {
            panic!("HTTP_API");
        };
        assert_eq!(api.send.method, "POST");
        assert_eq!(
            serde_json::from_value::<AccountSender>(
                serde_json::to_value(&sender).expect("json")
            )
            .expect("round trip"),
            sender
        );
    }

    fn channel_config(
        channel: MessageChannel,
        account_id: &str,
        purposes: &[MessagePurpose],
    ) -> EventChannelConfig {
        EventChannelConfig {
            channel,
            account_id: account_id.to_string(),
            purposes: purposes.to_vec(),
            templates: vec![],
            out_of_window: OutOfWindowPolicy::DISABLED,
        }
    }

    #[test]
    fn delivered_and_failed_cannot_regress() {
        use MessageAttemptState::*;
        assert!(!DELIVERED.can_transition_to(FAILED));
        assert!(!DELIVERED.can_transition_to(ACCEPTED));
        assert!(!DELIVERED.can_transition_to(UNKNOWN));
        assert!(!ACCEPTED.can_transition_to(UNKNOWN));
        assert!(!ACCEPTED.can_transition_to(QUEUED));
        // A fast report may arrive before the send's answer is recorded.
        assert!(QUEUED.can_transition_to(DELIVERED));
        assert!(FAILED.can_transition_to(DELIVERED));
        assert!(!FAILED.can_transition_to(ACCEPTED));
    }

    #[test]
    fn unknown_outcomes_can_only_be_resolved_not_restarted() {
        use MessageAttemptState::*;
        assert!(UNKNOWN.can_transition_to(ACCEPTED));
        assert!(UNKNOWN.can_transition_to(DELIVERED));
        assert!(UNKNOWN.can_transition_to(FAILED));
        assert!(!UNKNOWN.can_transition_to(QUEUED));
        assert!(UNKNOWN.is_active());
        assert!(ACCEPTED.is_active());
        assert!(!FAILED.is_active());
    }

    #[test]
    fn every_provider_declares_capabilities_for_its_own_channel() {
        for provider in MessagingProvider::iter() {
            for channel in MessageChannel::iter() {
                let capabilities = provider.capabilities(channel);
                match provider.channel() {
                    Some(own) if own != channel => {
                        assert!(capabilities.is_none())
                    }
                    _ => {
                        let capabilities = capabilities.expect("capabilities");
                        assert_eq!(capabilities.channel, channel);
                        assert_eq!(
                            capabilities.recipient,
                            channel.recipient_kind()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn connected_whatsapp_is_not_ready_without_approval_and_templates() {
        let readiness = purpose_readiness(
            &caps(
                MessagingProvider::WHATSAPP_CLOUD_API,
                MessageChannel::WHATSAPP,
            ),
            ProviderApproval::PENDING,
            None,
            &ready_check(&[]),
            MessagePurpose::OTP,
            Some("en"),
        );
        assert_eq!(
            readiness.blockers,
            vec![
                ReadinessBlocker::NEEDS_PROVIDER_APPROVAL,
                ReadinessBlocker::NEEDS_APPROVED_TEMPLATE
            ]
        );
    }

    #[test]
    fn templates_are_checked_per_language() {
        let check = ready_check(&[(MessagePurpose::OTP, "en")]);
        let readiness = |language| {
            purpose_readiness(
                &caps(MessagingProvider::VIBER_INFOBIP, MessageChannel::VIBER),
                ProviderApproval::PENDING,
                None,
                &check,
                MessagePurpose::OTP,
                Some(language),
            )
        };
        assert!(readiness("en").is_ready());
        assert_eq!(
            readiness("tl").blockers,
            vec![ReadinessBlocker::NEEDS_APPROVED_TEMPLATE]
        );
    }

    #[test]
    fn not_connected_blocks_every_purpose() {
        let check = AccountCheck::default();
        let readiness = purpose_readiness(
            &caps(MessagingProvider::AWS_SNS, MessageChannel::SMS),
            ProviderApproval::PENDING,
            None,
            &check,
            MessagePurpose::NOTICE,
            None,
        );
        assert_eq!(
            readiness.blockers,
            vec![
                ReadinessBlocker::NOT_CONNECTED,
                ReadinessBlocker::NEEDS_PRODUCTION_ACCESS
            ]
        );
    }

    #[test]
    fn valid_configuration_passes() {
        let accounts = vec![
            account(
                "sms-1",
                MessageChannel::SMS,
                MessagingProvider::AWS_SNS,
                ready_check(&[]),
            ),
            account(
                "viber-1",
                MessageChannel::VIBER,
                MessagingProvider::VIBER_INFOBIP,
                ready_check(&[
                    (MessagePurpose::OTP, "en"),
                    (MessagePurpose::NOTICE, "en"),
                ]),
            ),
        ];
        let config = EventMessagingConfig {
            channels: vec![
                channel_config(
                    MessageChannel::SMS,
                    "sms-1",
                    &[MessagePurpose::OTP, MessagePurpose::NOTICE],
                ),
                EventChannelConfig {
                    templates: vec![TemplateBinding {
                        purpose: MessagePurpose::OTP,
                        ..binding(None, "en", "otp_en")
                    }],
                    ..channel_config(
                        MessageChannel::VIBER,
                        "viber-1",
                        &[MessagePurpose::OTP, MessagePurpose::NOTICE],
                    )
                },
            ],
            election_channels: BTreeMap::from([(
                "election-1".to_string(),
                vec![MessageChannel::SMS],
            )]),
            ..Default::default()
        };
        assert_eq!(
            config.validate(TENANT, &accounts, &["election-1".to_string()]),
            Ok(())
        );
        assert_eq!(
            config.enabled_channels(MessagePurpose::OTP, Some("election-1")),
            vec![MessageChannel::SMS]
        );
        assert_eq!(
            config.enabled_channels(MessagePurpose::OTP, Some("election-2")),
            vec![MessageChannel::SMS, MessageChannel::VIBER]
        );
    }

    #[test]
    fn invalid_references_are_reported() {
        let mut foreign = account(
            "sms-foreign",
            MessageChannel::SMS,
            MessagingProvider::AWS_SNS,
            ready_check(&[]),
        );
        foreign.tenant_id = "tenant-2".to_string();
        let accounts = vec![
            foreign,
            account(
                "email-1",
                MessageChannel::EMAIL,
                MessagingProvider::AWS_SES,
                ready_check(&[]),
            ),
        ];
        let config = EventMessagingConfig {
            version: 2,
            channels: vec![
                channel_config(
                    MessageChannel::SMS,
                    "sms-foreign",
                    &[MessagePurpose::OTP],
                ),
                channel_config(
                    MessageChannel::WHATSAPP,
                    "missing",
                    &[MessagePurpose::OTP],
                ),
                channel_config(
                    MessageChannel::VIBER,
                    "email-1",
                    &[MessagePurpose::OTP],
                ),
                channel_config(
                    MessageChannel::VIBER,
                    "email-1",
                    &[MessagePurpose::OTP],
                ),
            ],
            election_channels: BTreeMap::from([(
                "unknown-election".to_string(),
                vec![MessageChannel::SMS],
            )]),
            ..Default::default()
        };
        let errors = config
            .validate(TENANT, &accounts, &["election-1".to_string()])
            .expect_err("invalid");
        assert_eq!(
            errors,
            vec![
                MessagingConfigError::UNSUPPORTED_VERSION { version: 2 },
                MessagingConfigError::ACCOUNT_OF_ANOTHER_TENANT {
                    account_id: "sms-foreign".to_string()
                },
                MessagingConfigError::UNKNOWN_ACCOUNT {
                    channel: MessageChannel::WHATSAPP,
                    account_id: "missing".to_string()
                },
                MessagingConfigError::ACCOUNT_CHANNEL_MISMATCH {
                    channel: MessageChannel::VIBER,
                    account_id: "email-1".to_string()
                },
                MessagingConfigError::DUPLICATE_CHANNEL {
                    channel: MessageChannel::VIBER
                },
                MessagingConfigError::UNKNOWN_ELECTION {
                    election_id: "unknown-election".to_string()
                },
            ]
        );
    }

    #[test]
    fn enabling_an_unready_purpose_names_the_missing_prerequisites() {
        let accounts = vec![account(
            "wa-1",
            MessageChannel::WHATSAPP,
            MessagingProvider::WHATSAPP_CLOUD_API,
            ready_check(&[(MessagePurpose::NOTICE, "en")]),
        )];
        let config = EventMessagingConfig {
            channels: vec![EventChannelConfig {
                templates: vec![TemplateBinding {
                    purpose: MessagePurpose::OTP,
                    ..binding(None, "tl", "otp_tl")
                }],
                out_of_window: OutOfWindowPolicy::UTILITY_MESSAGES,
                ..channel_config(
                    MessageChannel::WHATSAPP,
                    "wa-1",
                    &[MessagePurpose::OTP],
                )
            }],
            ..Default::default()
        };
        let errors = config.validate(TENANT, &accounts, &[]).expect_err("x");
        assert_eq!(
            errors,
            vec![
                MessagingConfigError::PURPOSE_NOT_READY {
                    channel: MessageChannel::WHATSAPP,
                    purpose: MessagePurpose::OTP,
                    blockers: vec![
                        ReadinessBlocker::NEEDS_PROVIDER_APPROVAL,
                        ReadinessBlocker::NEEDS_APPROVED_TEMPLATE
                    ],
                },
                MessagingConfigError::TEMPLATE_NOT_APPROVED {
                    channel: MessageChannel::WHATSAPP,
                    purpose: MessagePurpose::OTP,
                    language: "tl".to_string(),
                },
                MessagingConfigError::OUT_OF_WINDOW_NOT_SUPPORTED {
                    channel: MessageChannel::WHATSAPP
                },
            ]
        );
    }

    #[test]
    fn election_restrictions_must_use_enabled_channels() {
        let accounts = vec![account(
            "sms-1",
            MessageChannel::SMS,
            MessagingProvider::AWS_SNS,
            ready_check(&[]),
        )];
        let config = EventMessagingConfig {
            channels: vec![channel_config(
                MessageChannel::SMS,
                "sms-1",
                &[MessagePurpose::NOTICE],
            )],
            election_channels: BTreeMap::from([(
                "election-1".to_string(),
                vec![MessageChannel::VIBER],
            )]),
            ..Default::default()
        };
        let errors = config
            .validate(TENANT, &accounts, &["election-1".to_string()])
            .expect_err("x");
        assert_eq!(
            errors,
            vec![MessagingConfigError::ELECTION_CHANNEL_NOT_ENABLED {
                election_id: "election-1".to_string(),
                channel: MessageChannel::VIBER
            },]
        );
    }

    #[test]
    fn public_projection_contains_no_account_references() {
        let mut messenger = account(
            "messenger-account-secret-id",
            MessageChannel::MESSENGER,
            MessagingProvider::MESSENGER_SEND_API,
            ready_check(&[]),
        );
        messenger.public_label = Some("COMELEC".to_string());
        messenger.messenger_page = Some(MessengerPage {
            page_id: "1234567890".to_string(),
            username: Some("comelec".to_string()),
            name: Some("COMELEC".to_string()),
        });
        let config = EventMessagingConfig {
            channels: vec![
                channel_config(
                    MessageChannel::MESSENGER,
                    "messenger-account-secret-id",
                    &[MessagePurpose::OTP],
                ),
                channel_config(MessageChannel::SMS, "sms-1", &[]),
            ],
            election_channels: BTreeMap::from([(
                "election-1".to_string(),
                vec![MessageChannel::MESSENGER],
            )]),
            ..Default::default()
        };
        let labels = BTreeMap::from([
            (
                "election-1".to_string(),
                vec!["Manila".to_string(), "PH-MNL".to_string()],
            ),
            ("election-2".to_string(), vec!["Madrid".to_string()]),
        ]);
        let projection = config.public_projection(&[messenger], &labels);
        let json = serde_json::to_string(&projection).expect("json");
        assert!(!json.contains("messenger-account-secret-id"));
        assert_eq!(projection.channels.len(), 1);
        // Every election is labelled once any is restricted, so that an
        // unrestricted Post is told apart from an unknown one.
        assert_eq!(projection.election_labels, labels);
        let unrestricted = EventMessagingConfig {
            election_channels: BTreeMap::new(),
            ..config.clone()
        };
        assert!(unrestricted
            .public_projection(&[], &labels)
            .election_labels
            .is_empty());
        assert_eq!(
            projection.channels[0].messenger_page,
            Some(PublicMessengerPage {
                page_id: "1234567890".to_string(),
                username: Some("comelec".to_string()),
                name: Some("COMELEC".to_string()),
            })
        );
    }

    #[test]
    fn required_credentials_are_always_accepted() {
        for provider in MessagingProvider::iter() {
            let accepted = provider.accepted_credentials();
            for credential in provider.required_credentials() {
                assert!(accepted.contains(&credential), "{provider}");
            }
        }
    }

    #[test]
    fn statistics_keys_are_distinct_and_keep_the_existing_names() {
        let keys: HashSet<&str> =
            MessageChannel::iter().map(|c| c.statistics_key()).collect();
        assert_eq!(keys.len(), MessageChannel::iter().count());
        assert_eq!(MessageChannel::EMAIL.statistics_key(), "num_emails_sent");
        assert_eq!(MessageChannel::SMS.statistics_key(), "num_sms_sent");
    }

    #[test]
    fn account_sender_is_tagged_by_provider() {
        let sender: AccountSender = serde_json::from_value(serde_json::json!({
            "provider": "MESSENGER_SEND_API",
            "page_id": "1234",
            "page_name": "COMELEC",
            "page_username": null,
            "api_version": "v23.0"
        }))
        .expect("sender");
        assert_eq!(sender.provider(), MessagingProvider::MESSENGER_SEND_API);
        assert_eq!(sender.public_label(), Some("COMELEC".to_string()));
        assert_eq!(
            sender.messenger_page().map(|page| page.page_id),
            Some("1234".to_string())
        );
        assert!(serde_json::from_value::<AccountSender>(serde_json::json!({
            "provider": "WHATSAPP_CLOUD_API",
            "page_id": "1234"
        }))
        .is_err());
    }

    #[test]
    fn configuration_round_trips_with_defaults() {
        let config: EventMessagingConfig = serde_json::from_str(
            r#"{"version":1,"channels":[{"channel":"SMS","account_id":"a"}]}"#,
        )
        .expect("parse");
        assert_eq!(config.channels[0].purposes, vec![]);
        assert_eq!(
            config.channels[0].out_of_window,
            OutOfWindowPolicy::DISABLED
        );
    }

    #[test]
    fn first_release_fallback_and_reply_fields_parse_and_are_dropped() {
        let config: EventMessagingConfig = serde_json::from_str(
            r#"{"version":1,"channels":[],"notice_fallback":["SMS","EMAIL"],
                "reply_text":{"en":"Thanks"}}"#,
        )
        .expect("parse");
        let written = serde_json::to_value(&config).expect("serialize");
        assert!(written.get("notice_fallback").is_none());
        assert!(written.get("reply_text").is_none());
    }

    #[test]
    fn messenger_notices_need_the_utility_template() {
        let accounts = vec![account(
            "messenger-1",
            MessageChannel::MESSENGER,
            MessagingProvider::MESSENGER_SEND_API,
            ready_check(&[]),
        )];
        let notices = channel_config(
            MessageChannel::MESSENGER,
            "messenger-1",
            &[MessagePurpose::NOTICE],
        );
        let blocked = MessagingConfigError::UTILITY_TEMPLATE_MISSING {
            channel: MessageChannel::MESSENGER,
        };
        let validate = |channel: EventChannelConfig| {
            EventMessagingConfig {
                channels: vec![channel],
                ..Default::default()
            }
            .validate(TENANT, &accounts, &[])
        };
        // "Do not send" outside the window is gone: Messenger always uses
        // the utility template, which must be bound.
        assert_eq!(
            notices.effective_out_of_window(),
            OutOfWindowPolicy::UTILITY_MESSAGES
        );
        assert_eq!(validate(notices.clone()), Err(vec![blocked]));
        assert_eq!(
            validate(EventChannelConfig {
                templates: vec![TemplateBinding {
                    purpose: MessagePurpose::NOTICE,
                    ..binding(None, "en", "voting_reminder")
                }],
                ..notices.clone()
            }),
            Ok(())
        );
        // Codes only: nothing to send outside the window.
        assert_eq!(
            validate(channel_config(
                MessageChannel::MESSENGER,
                "messenger-1",
                &[MessagePurpose::OTP],
            )),
            Ok(())
        );
    }

    #[test]
    fn ses_receipts_need_the_notification_topic() {
        let ses = |topic: Option<&str>| AccountSender::AWS_SES {
            from_address: "noreply@example.org".to_string(),
            from_name: None,
            region: None,
            notification_topic_arn: topic.map(str::to_string),
        };
        let feedback = |sender: AccountSender| {
            sender
                .capabilities(MessageChannel::EMAIL)
                .unwrap()
                .delivery_feedback
        };
        assert_eq!(feedback(ses(None)), DeliveryFeedback::UNAVAILABLE);
        assert_eq!(feedback(ses(Some(" "))), DeliveryFeedback::UNAVAILABLE);
        assert_eq!(
            feedback(ses(Some("arn:aws:sns:eu-west-1:1:ses"))),
            DeliveryFeedback::PROVIDER_RECEIPTS
        );
    }

    /// Regression (R1-06): the callback blocker reached no readiness, so a
    /// save enabled Messenger codes no voter could receive.
    #[test]
    fn a_save_refuses_messenger_codes_without_the_callback_address() {
        let ready = AccountCheck {
            connected: true,
            production_access: true,
            ..Default::default()
        };
        let mut page = account(
            "page",
            MessageChannel::MESSENGER,
            MessagingProvider::MESSENGER_SEND_API,
            ready,
        );
        page.provider_approval = ProviderApproval::CONFIRMED;
        page.callback_blockers =
            vec![ReadinessBlocker::CALLBACK_NOT_CONFIGURED];
        let config: EventMessagingConfig = serde_json::from_value(serde_json::json!({
            "version": 1,
            "channels": [{"channel": "MESSENGER", "account_id": "page", "purposes": ["OTP"]}],
        }))
        .unwrap();
        let errors = config
            .validate(TENANT, std::slice::from_ref(&page), &[])
            .unwrap_err();
        assert!(errors.iter().any(|error| matches!(
            error,
            MessagingConfigError::PURPOSE_NOT_READY { blockers, .. }
                if blockers.contains(&ReadinessBlocker::CALLBACK_NOT_CONFIGURED)
        )), "{errors:?}");
        page.callback_blockers = vec![];
        assert!(config
            .validate(TENANT, std::slice::from_ref(&page), &[])
            .is_ok());
    }

    #[test]
    fn messenger_codes_need_the_callback_address() {
        assert_eq!(
            callback_blocker(
                MessageChannel::MESSENGER,
                MessagePurpose::OTP,
                false
            ),
            Some(ReadinessBlocker::CALLBACK_NOT_CONFIGURED)
        );
        assert_eq!(
            callback_blocker(
                MessageChannel::MESSENGER,
                MessagePurpose::OTP,
                true
            ),
            None
        );
        assert_eq!(
            callback_blocker(
                MessageChannel::MESSENGER,
                MessagePurpose::NOTICE,
                false
            ),
            None
        );
        assert_eq!(
            callback_blocker(
                MessageChannel::WHATSAPP,
                MessagePurpose::OTP,
                false
            ),
            None
        );
    }

    #[test]
    fn a_reason_not_to_retry_is_kept_next_to_the_failure() {
        let reason = MessageReason::NO_STORED_TEMPLATE;
        let stored = reason.not_retried_after(Some("rate limited (429)"));
        assert_eq!(
            stored,
            "rate limited (429); Can't be retried: no stored template"
        );
        assert_eq!(
            MessageReason::split_stored(&stored),
            ("rate limited (429)", Some(reason))
        );
        assert_eq!(reason.not_retried_after(None), reason.as_str());
        assert_eq!(reason.not_retried_after(Some(" ")), reason.as_str());
        assert_eq!(
            MessageReason::split_stored(reason.as_str()),
            ("", Some(reason))
        );
        assert_eq!(
            MessageReason::split_stored("provider code 7; other"),
            ("provider code 7; other", None)
        );
    }

    #[test]
    fn notices_without_a_channel_parse_and_go_to_the_voter() {
        let request: SendMessageRequest =
            serde_json::from_value(serde_json::json!({
                "tenant_id": "t", "election_event_id": null,
                "voter_id": "v", "purpose": "NOTICE",
                "language": null, "logical_key": "k", "expires_at": null,
                "content": {"subject": null, "text": "x", "html": null,
                            "code": null}
            }))
            .unwrap();
        assert_eq!(request.channel, None);
        assert_eq!(request.destination, None);
        assert_eq!(request.recipient, NoticeRecipient::VOTER_CHANNEL);
    }

    #[test]
    fn every_reason_has_one_string() {
        use strum::IntoEnumIterator;
        let mut seen = HashSet::new();
        for reason in MessageReason::iter() {
            assert_eq!(reason.to_string(), reason.as_str());
            assert_eq!(reason.as_str().parse::<MessageReason>(), Ok(reason));
            assert!(seen.insert(reason.as_str()));
        }
    }

    #[test]
    fn billing_categories_follow_channel_purpose_and_window() {
        use BillingCategory::*;
        let expected = BillingCategory::expected;
        assert_eq!(
            expected(MessageChannel::WHATSAPP, MessagePurpose::OTP, false),
            AUTHENTICATION
        );
        assert_eq!(
            expected(MessageChannel::WHATSAPP, MessagePurpose::NOTICE, true),
            UTILITY
        );
        assert_eq!(
            expected(MessageChannel::MESSENGER, MessagePurpose::NOTICE, true),
            SERVICE
        );
        assert_eq!(
            expected(MessageChannel::MESSENGER, MessagePurpose::NOTICE, false),
            UTILITY
        );
        assert_eq!(
            expected(MessageChannel::SMS, MessagePurpose::OTP, false),
            TRANSACTIONAL
        );
        assert_eq!(
            expected(MessageChannel::EMAIL, MessagePurpose::NOTICE, false),
            EMAIL
        );
        assert_eq!(
            BillingCategory::from_report("authentication"),
            Some(AUTHENTICATION)
        );
        assert_eq!(BillingCategory::from_report("marketing"), None);
    }
}
