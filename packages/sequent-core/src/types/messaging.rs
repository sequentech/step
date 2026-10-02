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
    /// One-time codes. Never retried after expiry, never fall back on their
    /// own: the voter explicitly chooses another method.
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
    /// Viber Business Messages through Infobip. Each Viber partner has its
    /// own adapter; this is not Viber's bot API.
    VIBER_INFOBIP,
    /// Prints messages instead of sending them. Development only.
    CONSOLE,
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
            MessagingProvider::CONSOLE => None,
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
            MessagingProvider::AWS_SNS => ProviderCapabilities {
                channel,
                recipient: RecipientKind::PHONE_NUMBER,
                purposes: both,
                template_required_for: vec![],
                delivery_feedback: DeliveryFeedback::PROVIDER_RECEIPTS,
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
            MessagingProvider::CONSOLE => ProviderCapabilities {
                channel,
                recipient: channel.recipient_kind(),
                purposes: both,
                template_required_for: vec![],
                delivery_feedback: DeliveryFeedback::UNAVAILABLE,
                reconciliation: false,
                conversation_window_hours: None,
                requires_provider_approval: false,
            },
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
            (QUEUED, ACCEPTED | FAILED | UNKNOWN) => true,
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
    INBOUND,
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
    /// Only within the window. Outside it, notices use another eligible
    /// channel and codes need a fresh interaction.
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
    NOT_CONNECTED,
    UNSUPPORTED_PURPOSE,
    NEEDS_PROVIDER_APPROVAL,
    NEEDS_PRODUCTION_ACCESS,
    NEEDS_APPROVED_TEMPLATE,
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

/// Readiness of `purpose` for `language` on an account.
pub fn purpose_readiness(
    provider: MessagingProvider,
    channel: MessageChannel,
    approval: ProviderApproval,
    check: &AccountCheck,
    purpose: MessagePurpose,
    language: Option<&str>,
) -> PurposeReadiness {
    let mut blockers = vec![];
    let Some(capabilities) = provider.capabilities(channel) else {
        return PurposeReadiness {
            purpose,
            blockers: vec![ReadinessBlocker::UNSUPPORTED_PURPOSE],
        };
    };
    if !check.connected {
        blockers.push(ReadinessBlocker::NOT_CONNECTED);
    }
    if !capabilities.purposes.contains(&purpose) {
        blockers.push(ReadinessBlocker::UNSUPPORTED_PURPOSE);
    }
    if capabilities.requires_provider_approval
        && approval != ProviderApproval::CONFIRMED
    {
        blockers.push(ReadinessBlocker::NEEDS_PROVIDER_APPROVAL);
    }
    if !check.production_access {
        blockers.push(ReadinessBlocker::NEEDS_PRODUCTION_ACCESS);
    }
    if capabilities.template_required_for.contains(&purpose) {
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
    pub provider_approval: ProviderApproval,
    pub check: AccountCheck,
    /// Public, non-secret label shown to voters (sender name, Page name).
    pub public_label: Option<String>,
    /// Messenger only: the Page voters open to connect.
    pub messenger_page: Option<MessengerPage>,
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
    pub language: String,
    /// Template name or ID as the provider knows it.
    pub provider_template: String,
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

/// Stored as JSON in the event annotation [`MESSAGING_CONFIG_ANNOTATION`].
/// Holds references only: no credentials or recipient data.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct EventMessagingConfig {
    pub version: u32,
    #[serde(default)]
    pub channels: Vec<EventChannelConfig>,
    /// Channels tried, in order, when a notice fails on the voter's channel.
    /// Never used for codes.
    #[serde(default)]
    pub notice_fallback: Vec<MessageChannel>,
    /// Per election, the channels it offers. Elections not listed offer
    /// every enabled channel.
    #[serde(default)]
    pub election_channels: BTreeMap<String, Vec<MessageChannel>>,
    /// Automatic reply to incoming messages, per language.
    #[serde(default)]
    pub reply_text: BTreeMap<String, String>,
}

impl Default for EventMessagingConfig {
    fn default() -> Self {
        EventMessagingConfig {
            version: EVENT_MESSAGING_CONFIG_VERSION,
            channels: vec![],
            notice_fallback: vec![],
            election_channels: BTreeMap::new(),
            reply_text: BTreeMap::new(),
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
    FALLBACK_CHANNEL_NOT_ENABLED {
        channel: MessageChannel,
    },
    DUPLICATE_FALLBACK_CHANNEL {
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

impl EventMessagingConfig {
    pub fn channel(
        &self,
        channel: MessageChannel,
    ) -> Option<&EventChannelConfig> {
        self.channels.iter().find(|c| c.channel == channel)
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
    /// template approvals, fallback entries and election restrictions.
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
            for purpose in &channel_config.purposes {
                let readiness = purpose_readiness(
                    account.provider,
                    channel,
                    account.provider_approval,
                    &account.check,
                    *purpose,
                    None,
                );
                if !readiness.is_ready() {
                    errors.push(MessagingConfigError::PURPOSE_NOT_READY {
                        channel,
                        purpose: *purpose,
                        blockers: readiness.blockers,
                    });
                }
            }
            let template_purposes = account
                .provider
                .capabilities(channel)
                .map(|c| c.template_required_for)
                .unwrap_or_default();
            for binding in &channel_config.templates {
                if !template_purposes.contains(&binding.purpose) {
                    continue;
                }
                let approved = account
                    .check
                    .approved_templates
                    .get(&binding.purpose)
                    .map(|langs| langs.contains(&binding.language))
                    .unwrap_or(false);
                if !approved {
                    errors.push(MessagingConfigError::TEMPLATE_NOT_APPROVED {
                        channel,
                        purpose: binding.purpose,
                        language: binding.language.clone(),
                    });
                }
            }
            if channel_config.out_of_window != OutOfWindowPolicy::DISABLED
                && channel != MessageChannel::MESSENGER
            {
                errors.push(
                    MessagingConfigError::OUT_OF_WINDOW_NOT_SUPPORTED {
                        channel,
                    },
                );
            }
        }
        let notice_channels: HashSet<MessageChannel> = self
            .enabled_channels(MessagePurpose::NOTICE, None)
            .into_iter()
            .collect();
        let mut seen_fallback = HashSet::new();
        for channel in &self.notice_fallback {
            if !seen_fallback.insert(*channel) {
                errors.push(MessagingConfigError::DUPLICATE_FALLBACK_CHANNEL {
                    channel: *channel,
                });
            } else if !notice_channels.contains(channel) {
                errors.push(
                    MessagingConfigError::FALLBACK_CHANNEL_NOT_ENABLED {
                        channel: *channel,
                    },
                );
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
    pub fn public_projection(
        &self,
        accounts: &[AccountSummary],
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
    },
    MESSENGER_SEND_API {
        page_id: String,
        page_name: Option<String>,
        page_username: Option<String>,
        api_version: String,
    },
    VIBER_INFOBIP {
        /// The account's Infobip API base URL.
        base_url: String,
        /// The Viber sender (service) name registered with Infobip.
        sender: String,
    },
    CONSOLE {},
}

impl AccountSender {
    pub fn provider(&self) -> MessagingProvider {
        match self {
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
}

impl MessagingProvider {
    /// Credentials this provider needs before it can connect.
    pub fn required_credentials(&self) -> Vec<CredentialName> {
        match self {
            MessagingProvider::WHATSAPP_CLOUD_API
            | MessagingProvider::MESSENGER_SEND_API => vec![
                CredentialName::ACCESS_TOKEN,
                CredentialName::APP_SECRET,
                CredentialName::VERIFY_TOKEN,
            ],
            MessagingProvider::VIBER_INFOBIP => vec![CredentialName::API_KEY],
            MessagingProvider::SMTP => vec![CredentialName::SMTP_PASSWORD],
            MessagingProvider::AWS_SES
            | MessagingProvider::AWS_SNS
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
    /// ISO 3166-1 alpha-2 codes this account may send to. Empty: any.
    #[serde(default)]
    pub allowed_countries: Vec<String>,
}

/// Internal request from Keycloak or windmill to send one message.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct SendMessageRequest {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
    pub voter_id: Option<String>,
    pub channel: MessageChannel,
    pub purpose: MessagePurpose,
    /// Phone number (E.164), email address or Page-scoped ID.
    pub destination: String,
    pub language: Option<String>,
    pub content: MessageContent,
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
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CreateMessengerLinkResponse {
    pub reference: String,
    /// `m.me` link with the reference.
    pub link: String,
    /// Word the voter can type in the chat instead of using the link.
    pub link_word: String,
    pub expires_at: String,
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
            provider_approval: ProviderApproval::PENDING,
            check,
            public_label: Some(format!("{id} label")),
            messenger_page: None,
        }
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
            MessagingProvider::WHATSAPP_CLOUD_API,
            MessageChannel::WHATSAPP,
            ProviderApproval::PENDING,
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
                MessagingProvider::VIBER_INFOBIP,
                MessageChannel::VIBER,
                ProviderApproval::PENDING,
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
            MessagingProvider::AWS_SNS,
            MessageChannel::SMS,
            ProviderApproval::PENDING,
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
                        language: "en".to_string(),
                        provider_template: "otp_en".to_string(),
                    }],
                    ..channel_config(
                        MessageChannel::VIBER,
                        "viber-1",
                        &[MessagePurpose::OTP, MessagePurpose::NOTICE],
                    )
                },
            ],
            notice_fallback: vec![MessageChannel::VIBER, MessageChannel::SMS],
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
            notice_fallback: vec![MessageChannel::SMS],
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
                MessagingConfigError::FALLBACK_CHANNEL_NOT_ENABLED {
                    channel: MessageChannel::SMS
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
                    language: "tl".to_string(),
                    provider_template: "otp_tl".to_string(),
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
            notice_fallback: vec![MessageChannel::SMS, MessageChannel::SMS],
            ..Default::default()
        };
        let errors = config
            .validate(TENANT, &accounts, &["election-1".to_string()])
            .expect_err("x");
        assert_eq!(
            errors,
            vec![
                MessagingConfigError::DUPLICATE_FALLBACK_CHANNEL {
                    channel: MessageChannel::SMS
                },
                MessagingConfigError::ELECTION_CHANNEL_NOT_ENABLED {
                    election_id: "election-1".to_string(),
                    channel: MessageChannel::VIBER
                },
            ]
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
            ..Default::default()
        };
        let projection = config.public_projection(&[messenger]);
        let json = serde_json::to_string(&projection).expect("json");
        assert!(!json.contains("messenger-account-secret-id"));
        assert_eq!(projection.channels.len(), 1);
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
        assert!(config.notice_fallback.is_empty());
    }
}
