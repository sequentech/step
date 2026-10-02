// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Provider callbacks. Each module verifies its provider's mechanism and
//! turns a payload into [`WebhookEvent`]s. A callback never authenticates a
//! voter.

pub mod infobip;
pub mod meta;
pub mod sns;

use chrono::{DateTime, Utc};
use sequent_core::types::messaging::MessageAttemptState;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum WebhookEvent {
    Status(StatusReport),
    Inbound(InboundMessage),
    MessengerReferral(MessengerReferral),
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

/// A message a voter sent to an account. Only metadata is kept.
#[derive(Debug, Clone, PartialEq)]
pub struct InboundMessage {
    /// Phone number (E.164) or Page-scoped ID.
    pub from: String,
    pub provider_message_id: String,
    pub has_text: bool,
    pub at: Option<DateTime<Utc>>,
}

/// A Messenger interaction that may complete a link: an `m.me` referral
/// (`reference`) or a typed linking word.
#[derive(Debug, Clone, PartialEq)]
pub struct MessengerReferral {
    pub page_scoped_id: String,
    pub reference: String,
    pub linking_word: Option<String>,
    pub at: Option<DateTime<Utc>>,
}
