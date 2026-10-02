// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Messenger Send API: a Page messaging a Page-scoped ID, as a response
//! within the conversation window.

use super::{exchange, Account};
use crate::sender::{
    outcome_from_http, ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, MessageChannel, MessagingProvider,
    ProviderCapabilities,
};
use serde_json::{json, Value};

pub struct MessengerSender {
    account: Account,
    http: reqwest::Client,
    base_url: String,
    page_id: String,
}

impl MessengerSender {
    pub fn new(account: Account, http: reqwest::Client, graph_url: String) -> Result<Self> {
        let AccountSender::MESSENGER_SEND_API {
            page_id,
            api_version,
            ..
        } = &account.sender
        else {
            return Err(anyhow!("not a Messenger account"));
        };
        Ok(MessengerSender {
            base_url: format!("{graph_url}/{api_version}"),
            page_id: page_id.clone(),
            account,
            http,
        })
    }
}

fn error_code(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body).ok()?["error"]["code"]
        .as_i64()
        .map(|code| code.to_string())
}

#[async_trait]
impl ChannelSender for MessengerSender {
    fn capabilities(&self) -> ProviderCapabilities {
        MessagingProvider::MESSENGER_SEND_API
            .capabilities(MessageChannel::MESSENGER)
            .unwrap_or_else(|| unreachable!("Messenger declares its own channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        let Ok(token) = self.account.credential(CredentialName::ACCESS_TOKEN) else {
            return SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::PERMANENT,
                code: None,
                reason: "missing Page access token".to_string(),
            });
        };
        let request = self
            .http
            .post(format!("{}/{}/messages", self.base_url, self.page_id))
            .bearer_auth(token)
            .json(&json!({
                "recipient": {"id": message.destination.as_str()},
                "messaging_type": "RESPONSE",
                "message": {"text": message.content.text},
            }));
        outcome_from_http(
            exchange(request).await,
            |body| {
                serde_json::from_str::<Value>(body).ok()?["message_id"]
                    .as_str()
                    .map(str::to_string)
            },
            error_code,
        )
    }

    async fn check(&self) -> AccountCheck {
        let now = Some(Utc::now().to_rfc3339());
        let Ok(token) = self.account.credential(CredentialName::ACCESS_TOKEN) else {
            return AccountCheck {
                checked_at: now,
                reason: Some("missing Page access token".to_string()),
                ..Default::default()
            };
        };
        let page = exchange(
            self.http
                .get(format!("{}/{}", self.base_url, self.page_id))
                .query(&[("fields", "name,username")])
                .bearer_auth(token),
        )
        .await;
        match page {
            Ok((200, _)) => {}
            Ok((status, _)) => {
                return AccountCheck {
                    checked_at: now,
                    reason: Some(format!("Page lookup failed with status {status}")),
                    ..Default::default()
                }
            }
            Err(_) => {
                return AccountCheck {
                    checked_at: now,
                    reason: Some("Messenger Platform unreachable".to_string()),
                    ..Default::default()
                }
            }
        }
        let subscribed = match exchange(
            self.http
                .get(format!(
                    "{}/{}/subscribed_apps",
                    self.base_url, self.page_id
                ))
                .bearer_auth(token),
        )
        .await
        {
            Ok((200, body)) => serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|body| body["data"].as_array().map(|apps| !apps.is_empty()))
                .unwrap_or(false),
            _ => false,
        };
        AccountCheck {
            connected: true,
            production_access: subscribed,
            approved_templates: Default::default(),
            checked_at: now,
            reason: (!subscribed).then(|| "the app is not subscribed to the Page".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destination::Destination;
    use crate::test_server::{Reply, TestServer};
    use sequent_core::types::messaging::{
        AccountLimits, MessageContent, MessagePurpose, OutOfWindowPolicy,
    };
    use std::collections::BTreeMap;

    fn account() -> Account {
        Account {
            id: "ms-1".to_string(),
            channel: MessageChannel::MESSENGER,
            sender: AccountSender::MESSENGER_SEND_API {
                page_id: "page-1".to_string(),
                page_name: Some("COMELEC".to_string()),
                page_username: None,
                api_version: "v23.0".to_string(),
            },
            credentials: BTreeMap::from([(CredentialName::ACCESS_TOKEN, "page-token".to_string())]),
            limits: AccountLimits::default(),
            callback_url: None,
        }
    }

    #[tokio::test]
    async fn messages_are_sent_as_responses_to_the_page_scoped_id() {
        let server = TestServer::start(vec![Reply::Json(
            200,
            json!({"recipient_id": "6543210", "message_id": "m_1"}),
        )])
        .await;
        let sender =
            MessengerSender::new(account(), reqwest::Client::new(), server.base_url.clone())
                .unwrap();
        let message = OutboundMessage {
            channel: MessageChannel::MESSENGER,
            purpose: MessagePurpose::NOTICE,
            destination: Destination::parse(MessageChannel::MESSENGER, "6543210").unwrap(),
            language: None,
            content: MessageContent {
                subject: None,
                text: "Your enrollment was approved.".to_string(),
                html: None,
                template_parameters: vec![],
                code: None,
            },
            provider_template: None,
            idempotency_key: "k".to_string(),
            expires_at: None,
            last_inbound_at: Some(Utc::now()),
            out_of_window: OutOfWindowPolicy::DISABLED,
        };
        assert_eq!(
            sender.send(&message).await,
            SendOutcome::Accepted {
                provider_message_id: Some("m_1".to_string())
            }
        );
        let request = &server.requests()[0];
        assert_eq!(request.path, "/v23.0/page-1/messages");
        assert_eq!(request.json()["recipient"]["id"], "6543210");
        assert_eq!(request.json()["messaging_type"], "RESPONSE");
    }

    #[tokio::test]
    async fn a_page_without_the_webhook_subscription_is_not_ready() {
        let server = TestServer::start(vec![
            Reply::Json(200, json!({"name": "COMELEC"})),
            Reply::Json(200, json!({"data": []})),
        ])
        .await;
        let sender =
            MessengerSender::new(account(), reqwest::Client::new(), server.base_url.clone())
                .unwrap();
        let check = sender.check().await;
        assert!(check.connected);
        assert!(!check.production_access);
        assert_eq!(
            check.reason.as_deref(),
            Some("the app is not subscribed to the Page")
        );
    }
}
