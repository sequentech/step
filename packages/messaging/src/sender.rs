// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The interface every provider adapter implements, and the checks made
//! before a message is handed to one.

use crate::destination::Destination;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sequent_core::types::messaging::{
    AccountCheck, AccountLimits, MessageAttemptState, MessageChannel, MessageContent,
    MessagePurpose, OutOfWindowPolicy, ProviderCapabilities,
};
use strum_macros::Display;

/// One message, ready for a provider.
#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub channel: MessageChannel,
    pub purpose: MessagePurpose,
    pub destination: Destination,
    pub language: Option<String>,
    pub content: MessageContent,
    /// Provider-approved template for this purpose and language.
    pub provider_template: Option<String>,
    /// Sent to providers that deduplicate requests.
    pub idempotency_key: String,
    /// Codes only: never sent after this instant.
    pub expires_at: Option<DateTime<Utc>>,
    /// The recipient's last message to the account, for conversation windows.
    pub last_inbound_at: Option<DateTime<Utc>>,
    pub out_of_window: OutOfWindowPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[allow(non_camel_case_types)]
pub enum FailureKind {
    /// Retrying will not help: invalid recipient, rejected template...
    PERMANENT,
    /// Throttling or a temporary outage, reported before the provider
    /// processed the request.
    TRANSIENT,
}

/// A provider's refusal, without message content or credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderFailure {
    pub kind: FailureKind,
    pub code: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    /// The provider took the message. Not proof of delivery.
    Accepted { provider_message_id: Option<String> },
    /// The provider refused it, so it was not sent.
    Rejected(ProviderFailure),
    /// The request may have been processed but its answer was lost.
    Unknown { reason: String },
}

impl SendOutcome {
    pub fn state(&self) -> MessageAttemptState {
        match self {
            SendOutcome::Accepted { .. } => MessageAttemptState::ACCEPTED,
            SendOutcome::Rejected(_) => MessageAttemptState::FAILED,
            SendOutcome::Unknown { .. } => MessageAttemptState::UNKNOWN,
        }
    }
}

#[async_trait]
pub trait ChannelSender: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;

    async fn send(&self, message: &OutboundMessage) -> SendOutcome;

    /// Checks credentials and account readiness without sending anything.
    async fn check(&self) -> AccountCheck;

    /// Asks the provider what happened to a message whose outcome is
    /// unknown. `None` when the provider cannot tell.
    async fn reconcile(&self, _provider_message_id: &str) -> Option<MessageAttemptState> {
        None
    }
}

/// Why a message was not handed to the provider.
#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[allow(non_camel_case_types)]
pub enum PreflightError {
    WRONG_RECIPIENT_KIND,
    UNSUPPORTED_PURPOSE,
    MISSING_APPROVED_TEMPLATE,
    OUTSIDE_CONVERSATION_WINDOW,
    DESTINATION_NOT_ALLOWED,
    CODE_EXPIRED,
    MISSING_CODE,
}

/// Checks capabilities, destination restrictions, conversation windows and
/// code expiry. Nothing that fails here reaches the provider.
pub fn preflight(
    capabilities: &ProviderCapabilities,
    limits: &AccountLimits,
    message: &OutboundMessage,
    now: DateTime<Utc>,
) -> Result<(), PreflightError> {
    if message.destination.kind != capabilities.recipient {
        return Err(PreflightError::WRONG_RECIPIENT_KIND);
    }
    if !capabilities.purposes.contains(&message.purpose) {
        return Err(PreflightError::UNSUPPORTED_PURPOSE);
    }
    if capabilities
        .template_required_for
        .contains(&message.purpose)
        && message.provider_template.is_none()
    {
        return Err(PreflightError::MISSING_APPROVED_TEMPLATE);
    }
    // Approved templates may be sent at any time. Messenger's out-of-window
    // mechanism (OutOfWindowPolicy::UTILITY_MESSAGES) is not sent until Meta
    // confirms it for the Page, so the window applies to every free-form
    // message.
    if let Some(hours) = capabilities.conversation_window_hours {
        let in_window = message
            .last_inbound_at
            .map(|at| now - at < Duration::hours(i64::from(hours)))
            .unwrap_or(false);
        if message.provider_template.is_none() && !in_window {
            return Err(PreflightError::OUTSIDE_CONVERSATION_WINDOW);
        }
    }
    if !limits.allowed_calling_codes.is_empty() {
        if let Some(code) = message.destination.calling_code() {
            if !limits.allowed_calling_codes.iter().any(|c| c == code) {
                return Err(PreflightError::DESTINATION_NOT_ALLOWED);
            }
        }
    }
    if message.purpose == MessagePurpose::OTP {
        match message.expires_at {
            Some(expires_at) if expires_at <= now => {
                return Err(PreflightError::CODE_EXPIRED);
            }
            None => return Err(PreflightError::CODE_EXPIRED),
            _ => {}
        }
        if message
            .content
            .code
            .as_deref()
            .unwrap_or_default()
            .is_empty()
        {
            return Err(PreflightError::MISSING_CODE);
        }
    }
    Ok(())
}

/// Maps an HTTP exchange with a provider to an outcome. Connection failures
/// happen before the provider sees the request; timeouts and server errors
/// after it may have processed it, so they are unknown.
pub(crate) fn outcome_from_http(
    result: Result<(u16, String), reqwest::Error>,
    accepted_id: impl Fn(&str) -> Option<String>,
    failure_code: impl Fn(&str) -> Option<String>,
) -> SendOutcome {
    match result {
        Err(error) if error.is_connect() || error.is_builder() => {
            SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::TRANSIENT,
                code: None,
                reason: "provider unreachable".to_string(),
            })
        }
        Err(_) => SendOutcome::Unknown {
            reason: "no answer from the provider".to_string(),
        },
        Ok((status, body)) if (200..300).contains(&status) => SendOutcome::Accepted {
            provider_message_id: accepted_id(&body),
        },
        Ok((429, body)) => SendOutcome::Rejected(ProviderFailure {
            kind: FailureKind::TRANSIENT,
            code: failure_code(&body),
            reason: "rate limited by the provider".to_string(),
        }),
        Ok((status, body)) if (400..500).contains(&status) => {
            SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::PERMANENT,
                code: failure_code(&body),
                reason: format!("rejected by the provider with status {status}"),
            })
        }
        Ok((status, _)) => SendOutcome::Unknown {
            reason: format!("provider answered with status {status}"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::types::messaging::MessagingProvider;

    fn message(channel: MessageChannel, purpose: MessagePurpose, to: &str) -> OutboundMessage {
        OutboundMessage {
            channel,
            purpose,
            destination: Destination::parse(channel, to).expect("destination"),
            language: Some("en".to_string()),
            content: MessageContent {
                subject: None,
                text: "Your code is 123456".to_string(),
                html: None,
                template_parameters: vec![],
                code: Some("123456".to_string()),
            },
            provider_template: None,
            idempotency_key: "key".to_string(),
            expires_at: Some(Utc::now() + Duration::minutes(5)),
            last_inbound_at: None,
            out_of_window: OutOfWindowPolicy::DISABLED,
        }
    }

    fn capabilities(provider: MessagingProvider, channel: MessageChannel) -> ProviderCapabilities {
        provider.capabilities(channel).expect("capabilities")
    }

    #[test]
    fn whatsapp_requires_an_approved_template() {
        let caps = capabilities(
            MessagingProvider::WHATSAPP_CLOUD_API,
            MessageChannel::WHATSAPP,
        );
        let mut whatsapp = message(
            MessageChannel::WHATSAPP,
            MessagePurpose::OTP,
            "+639171234567",
        );
        assert_eq!(
            preflight(&caps, &AccountLimits::default(), &whatsapp, Utc::now()),
            Err(PreflightError::MISSING_APPROVED_TEMPLATE)
        );
        whatsapp.provider_template = Some("otp_en".to_string());
        assert_eq!(
            preflight(&caps, &AccountLimits::default(), &whatsapp, Utc::now()),
            Ok(())
        );
    }

    #[test]
    fn expired_or_missing_codes_are_never_sent() {
        let caps = capabilities(MessagingProvider::AWS_SNS, MessageChannel::SMS);
        let mut sms = message(MessageChannel::SMS, MessagePurpose::OTP, "+639171234567");
        sms.expires_at = Some(Utc::now() - Duration::seconds(1));
        assert_eq!(
            preflight(&caps, &AccountLimits::default(), &sms, Utc::now()),
            Err(PreflightError::CODE_EXPIRED)
        );
        sms.expires_at = None;
        assert_eq!(
            preflight(&caps, &AccountLimits::default(), &sms, Utc::now()),
            Err(PreflightError::CODE_EXPIRED)
        );
        sms.expires_at = Some(Utc::now() + Duration::minutes(1));
        sms.content.code = None;
        assert_eq!(
            preflight(&caps, &AccountLimits::default(), &sms, Utc::now()),
            Err(PreflightError::MISSING_CODE)
        );
    }

    #[test]
    fn messenger_needs_a_recent_interaction() {
        let caps = capabilities(
            MessagingProvider::MESSENGER_SEND_API,
            MessageChannel::MESSENGER,
        );
        let now = Utc::now();
        let mut notice = message(
            MessageChannel::MESSENGER,
            MessagePurpose::NOTICE,
            "123456789",
        );
        notice.last_inbound_at = Some(now - Duration::hours(25));
        assert_eq!(
            preflight(&caps, &AccountLimits::default(), &notice, now),
            Err(PreflightError::OUTSIDE_CONVERSATION_WINDOW)
        );
        notice.last_inbound_at = Some(now - Duration::hours(2));
        assert_eq!(
            preflight(&caps, &AccountLimits::default(), &notice, now),
            Ok(())
        );

        // Not sent until Meta confirms the mechanism (D5).
        notice.last_inbound_at = None;
        notice.out_of_window = OutOfWindowPolicy::UTILITY_MESSAGES;
        assert_eq!(
            preflight(&caps, &AccountLimits::default(), &notice, now),
            Err(PreflightError::OUTSIDE_CONVERSATION_WINDOW)
        );
    }

    #[test]
    fn destinations_outside_the_allowed_calling_codes_are_refused() {
        let caps = capabilities(MessagingProvider::AWS_SNS, MessageChannel::SMS);
        let limits = AccountLimits {
            allowed_calling_codes: vec!["63".to_string()],
            ..Default::default()
        };
        let sms = message(MessageChannel::SMS, MessagePurpose::NOTICE, "+447700900123");
        assert_eq!(
            preflight(&caps, &limits, &sms, Utc::now()),
            Err(PreflightError::DESTINATION_NOT_ALLOWED)
        );
        let sms = message(MessageChannel::SMS, MessagePurpose::NOTICE, "+639171234567");
        assert_eq!(preflight(&caps, &limits, &sms, Utc::now()), Ok(()));
    }

    #[test]
    fn http_answers_map_to_outcomes() {
        let id = |body: &str| Some(body.to_string());
        let code = |_: &str| Some("E1".to_string());
        assert_eq!(
            outcome_from_http(Ok((200, "wamid.1".to_string())), id, code),
            SendOutcome::Accepted {
                provider_message_id: Some("wamid.1".to_string())
            }
        );
        assert_eq!(
            outcome_from_http(Ok((400, String::new())), id, code).state(),
            MessageAttemptState::FAILED
        );
        assert_eq!(
            outcome_from_http(Ok((429, String::new())), id, code),
            SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::TRANSIENT,
                code: Some("E1".to_string()),
                reason: "rate limited by the provider".to_string(),
            })
        );
        assert_eq!(
            outcome_from_http(Ok((502, String::new())), id, code).state(),
            MessageAttemptState::UNKNOWN
        );
    }
}
