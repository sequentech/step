// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Development transport: records that a message would be sent, without
//! its content.

use super::Account;
use crate::sender::{ChannelSender, OutboundMessage, SendOutcome};
use async_trait::async_trait;
use chrono::Utc;
use sequent_core::types::messaging::{
    AccountCheck, MessageChannel, MessagingProvider, ProviderCapabilities,
};
use tracing::info;

pub struct ConsoleSender {
    account_id: String,
    channel: MessageChannel,
}

impl ConsoleSender {
    pub fn new(account: &Account) -> Self {
        ConsoleSender {
            account_id: account.id.clone(),
            channel: account.channel,
        }
    }
}

#[async_trait]
impl ChannelSender for ConsoleSender {
    fn capabilities(&self) -> ProviderCapabilities {
        MessagingProvider::CONSOLE
            .capabilities(self.channel)
            .unwrap_or_else(|| unreachable!("the console accepts every channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        info!(
            account = %self.account_id,
            channel = %message.channel,
            purpose = %message.purpose,
            destination = %message.destination.masked(),
            "console transport: message not sent"
        );
        SendOutcome::Accepted {
            provider_message_id: Some(format!("console-{}", crate::link::new_reference())),
        }
    }

    async fn check(&self) -> AccountCheck {
        AccountCheck {
            connected: true,
            production_access: true,
            approved_templates: Default::default(),
            checked_at: Some(Utc::now().to_rfc3339()),
            reason: None,
        }
    }
}
