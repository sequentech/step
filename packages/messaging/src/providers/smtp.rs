// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Email through an SMTP relay.

use super::Account;
use crate::sender::{ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use lettre::message::{header::ContentType, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, MessageChannel, MessagingProvider,
    ProviderCapabilities,
};
use std::sync::Arc;

pub struct SmtpSender {
    transport: Arc<SmtpTransport>,
    from: Mailbox,
}

impl SmtpSender {
    pub fn new(account: &Account) -> Result<Self> {
        let AccountSender::SMTP {
            from_address,
            from_name,
            server_url,
        } = &account.sender
        else {
            return Err(anyhow!("not an SMTP account"));
        };
        let from = Mailbox::new(
            from_name.clone(),
            from_address
                .parse()
                .map_err(|_| anyhow!("invalid From address"))?,
        );
        let password = account.credential(CredentialName::SMTP_PASSWORD)?;
        let transport = SmtpTransport::from_url(server_url)
            .map_err(|_| anyhow!("invalid SMTP server URL"))?
            .credentials(Credentials::new(from_address.clone(), password.to_string()))
            .timeout(Some(super::PROVIDER_TIMEOUT))
            .build();
        Ok(SmtpSender {
            transport: Arc::new(transport),
            from,
        })
    }

    fn email(&self, message: &OutboundMessage) -> Result<Message> {
        let builder = Message::builder()
            .from(self.from.clone())
            .to(message
                .destination
                .as_str()
                .parse()
                .map_err(|_| anyhow!("invalid recipient"))?)
            .subject(message.content.subject.clone().unwrap_or_default());
        let text = SinglePart::builder()
            .header(ContentType::TEXT_PLAIN)
            .body(message.content.text.clone());
        Ok(match &message.content.html {
            Some(html) => builder.multipart(
                MultiPart::alternative().singlepart(text).singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html.clone()),
                ),
            )?,
            None => builder.singlepart(text)?,
        })
    }
}

#[async_trait]
impl ChannelSender for SmtpSender {
    fn capabilities(&self) -> ProviderCapabilities {
        MessagingProvider::SMTP
            .capabilities(MessageChannel::EMAIL)
            .unwrap_or_else(|| unreachable!("SMTP declares its own channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        let email = match self.email(message) {
            Ok(email) => email,
            Err(error) => {
                return SendOutcome::Rejected(ProviderFailure {
                    kind: FailureKind::PERMANENT,
                    code: None,
                    reason: error.to_string(),
                })
            }
        };
        let transport = self.transport.clone();
        let result = tokio::task::spawn_blocking(move || transport.send(&email)).await;
        match result {
            Ok(Ok(response)) => SendOutcome::Accepted {
                provider_message_id: response.message().next().map(str::to_string),
            },
            Ok(Err(error)) if error.is_permanent() || error.is_transient() => {
                SendOutcome::Rejected(ProviderFailure {
                    kind: if error.is_permanent() {
                        FailureKind::PERMANENT
                    } else {
                        FailureKind::TRANSIENT
                    },
                    code: error.status().map(|code| code.to_string()),
                    reason: "refused by the SMTP server".to_string(),
                })
            }
            _ => SendOutcome::Unknown {
                reason: "no answer from the SMTP server".to_string(),
            },
        }
    }

    async fn check(&self) -> AccountCheck {
        let transport = self.transport.clone();
        let connected = tokio::task::spawn_blocking(move || transport.test_connection())
            .await
            .map(|result| result.unwrap_or(false))
            .unwrap_or(false);
        AccountCheck {
            connected,
            production_access: connected,
            approved_templates: Default::default(),
            checked_at: Some(Utc::now().to_rfc3339()),
            reason: (!connected)
                .then(|| "SMTP server unreachable or credentials rejected".to_string()),
        }
    }
}
