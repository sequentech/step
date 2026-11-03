// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Provider callbacks. Each module verifies its provider's mechanism and
//! turns a payload into [`WebhookEvent`]s. A callback never authenticates a
//! voter.
//!
//! Only statuses, Messenger's Get Started and referrals, and WhatsApp's
//! number-change signals are kept.
//! Messages voters write are acknowledged and dropped: nothing they send is
//! stored or answered.

pub mod infobip;
pub mod meta;
pub mod sns;

use crate::sender::FailureKind;
use chrono::{DateTime, Utc};
use sequent_core::types::messaging::{MessageAttemptState, MessagingProvider};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum WebhookEvent {
    Status(StatusReport),
    /// The recipient pressed Get Started on the Page without a reference.
    /// Opens the conversation window; binds no link.
    MessengerGetStarted(MessengerContact),
    MessengerReferral(MessengerReferral),
    DeliveredUpTo(DeliveredUpTo),
    /// WhatsApp's `user_changed_number` system message: the person who had
    /// `old_number` now uses `new_number`. Voters at the old number are
    /// flagged for verification again; nothing changes automatically.
    NumberChanged(NumberChange),
}

/// A WhatsApp `user_changed_number` signal, with the numbers as Meta sends
/// them (digits, no `+`). Only their masked forms and digests are kept.
#[derive(Debug, Clone, PartialEq)]
pub struct NumberChange {
    pub old_number: String,
    pub new_number: Option<String>,
    pub at: Option<DateTime<Utc>>,
}

/// Every message accepted for `recipient` up to an instant was delivered.
#[derive(Debug, Clone, PartialEq)]
pub struct DeliveredUpTo {
    /// Page-scoped ID.
    pub recipient: String,
    pub up_to: DateTime<Utc>,
}

/// A provider's report about a message Step sent.
#[derive(Debug, Clone, PartialEq)]
pub struct StatusReport {
    pub provider_message_id: String,
    pub state: MessageAttemptState,
    pub error_code: Option<String>,
    /// Billing evidence as the provider reports it.
    pub billing: Option<Value>,
    pub at: Option<DateTime<Utc>>,
}

/// A Messenger recipient's Get Started, which opens the 24-hour window.
#[derive(Debug, Clone, PartialEq)]
pub struct MessengerContact {
    pub page_scoped_id: String,
    pub at: Option<DateTime<Utc>>,
}

/// A Messenger interaction that may complete a link: an `m.me` referral,
/// on its own or with Get Started. Also opens the 24-hour window.
#[derive(Debug, Clone, PartialEq)]
pub struct MessengerReferral {
    pub page_scoped_id: String,
    pub reference: String,
    pub at: Option<DateTime<Utc>>,
}

/// WhatsApp and Messenger codes of failures reported later by webhook that
/// may pass: throttling, pair and spam rate limits, and temporary outages.
const META_TRANSIENT_REPORT_CODES: &[&str] = &[
    "1", "2", "4", "17", "32", "613", "80007", "130429", "131000", "131016", "131048", "131049",
    "131056", "133004",
];

/// Infobip error names (or parts of them) that mean the failure may pass.
const INFOBIP_TRANSIENT_ERRORS: &[&str] = &[
    "ABSENT_SUBSCRIBER",
    "MEMORY_CAPACITY",
    "RATE_LIMIT",
    "SYSTEM_FAILURE",
    "TEMPORAR",
    "THROTTL",
    "TIMEOUT",
    "TIME_OUT",
];

/// Whether a failure a provider reported later, by webhook, may be retried
/// on the same channel. Anything not known to pass is permanent, so a
/// failure is never retried blindly.
pub fn report_failure_kind(provider: MessagingProvider, error_code: Option<&str>) -> FailureKind {
    let Some(code) = error_code else {
        return FailureKind::PERMANENT;
    };
    let transient = match provider {
        MessagingProvider::WHATSAPP_CLOUD_API | MessagingProvider::MESSENGER_SEND_API => {
            META_TRANSIENT_REPORT_CODES.contains(&code)
        }
        MessagingProvider::VIBER_INFOBIP => {
            let code = code.to_ascii_uppercase();
            INFOBIP_TRANSIENT_ERRORS
                .iter()
                .any(|part| code.contains(part))
        }
        // A transient SES bounce whose mailbox may free up.
        MessagingProvider::AWS_SES => code == "MailboxFull",
        _ => false,
    };
    if transient {
        FailureKind::TRANSIENT
    } else {
        FailureKind::PERMANENT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_failures_are_transient_only_for_known_codes() {
        use MessagingProvider::*;
        assert_eq!(
            report_failure_kind(WHATSAPP_CLOUD_API, Some("130429")),
            FailureKind::TRANSIENT
        );
        assert_eq!(
            report_failure_kind(WHATSAPP_CLOUD_API, Some("131026")),
            FailureKind::PERMANENT
        );
        assert_eq!(
            report_failure_kind(VIBER_INFOBIP, Some("EC_ABSENT_SUBSCRIBER")),
            FailureKind::TRANSIENT
        );
        assert_eq!(
            report_failure_kind(VIBER_INFOBIP, Some("EC_UNKNOWN_SUBSCRIBER")),
            FailureKind::PERMANENT
        );
        assert_eq!(
            report_failure_kind(AWS_SES, Some("MailboxFull")),
            FailureKind::TRANSIENT
        );
        assert_eq!(
            report_failure_kind(AWS_SES, Some("NoEmail")),
            FailureKind::PERMANENT
        );
        assert_eq!(
            report_failure_kind(WHATSAPP_CLOUD_API, None),
            FailureKind::PERMANENT
        );
    }
}
