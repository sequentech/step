// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Viber Business Messages through Infobip. Step sets the message ID, so an
//! attempt whose response was lost can be looked up by that ID.

use super::{exchange, Account};
use crate::parameters::{parse_all, TemplateParameter};
use crate::sender::{
    outcome_from_http, ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, MessageAttemptState, MessageChannel,
    MessagePurpose, MessagingProvider, ProviderCapabilities,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub struct InfobipViberSender {
    account: Account,
    http: reqwest::Client,
    base_url: String,
    sender: String,
    approved_templates: BTreeMap<MessagePurpose, BTreeMap<String, String>>,
}

impl InfobipViberSender {
    pub fn new(account: Account, http: reqwest::Client) -> Result<Self> {
        let AccountSender::VIBER_INFOBIP {
            base_url,
            sender,
            approved_templates,
        } = &account.sender
        else {
            return Err(anyhow!("not an Infobip Viber account"));
        };
        Ok(InfobipViberSender {
            base_url: base_url.trim_end_matches('/').to_string(),
            sender: sender.clone(),
            approved_templates: approved_templates.clone(),
            account,
            http,
        })
    }

    fn authorization(&self) -> Result<String> {
        Ok(format!(
            "App {}",
            self.account.credential(CredentialName::API_KEY)?
        ))
    }

    fn content(message: &OutboundMessage, template: &str) -> Value {
        let parameters: Map<String, Value> = if message.purpose == MessagePurpose::OTP {
            Map::from_iter([(
                "pin".to_string(),
                Value::String(message.content.code.clone().unwrap_or_default()),
            )])
        } else {
            // Infobip takes parameters by placeholder name; positional ones
            // are named 1, 2, …
            let mut positional = 0;
            parse_all(&message.content.template_parameters)
                .into_iter()
                .map(|parameter| match parameter {
                    TemplateParameter::Named { name, value } => (name, Value::String(value)),
                    TemplateParameter::Positional(value) => {
                        positional += 1;
                        (positional.to_string(), Value::String(value))
                    }
                })
                .collect()
        };
        json!({
            "type": "TEMPLATE",
            "templateId": template,
            "language": message.language.clone().unwrap_or_else(|| "en".to_string()),
            "parameters": parameters,
        })
    }
}

fn status_state(group: &str) -> Option<MessageAttemptState> {
    match group {
        "PENDING" | "ACCEPTED" => Some(MessageAttemptState::ACCEPTED),
        "DELIVERED" => Some(MessageAttemptState::DELIVERED),
        "UNDELIVERABLE" | "EXPIRED" | "REJECTED" => Some(MessageAttemptState::FAILED),
        _ => None,
    }
}

#[async_trait]
impl ChannelSender for InfobipViberSender {
    fn capabilities(&self) -> ProviderCapabilities {
        MessagingProvider::VIBER_INFOBIP
            .capabilities(MessageChannel::VIBER)
            .unwrap_or_else(|| unreachable!("Infobip Viber declares its own channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        let (Ok(authorization), Some(template)) =
            (self.authorization(), message.provider_template.as_deref())
        else {
            return SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::PERMANENT,
                code: None,
                reason: "missing API key or approved template".to_string(),
            });
        };
        let mut entry = json!({
            "sender": self.sender,
            "destinations": [{
                "to": message.destination.as_str().trim_start_matches('+'),
                "messageId": message.idempotency_key,
            }],
            "content": Self::content(message, template),
        });
        if let Some(callback) = &self.account.callback_url {
            entry["webhooks"] = json!({"delivery": {"url": callback}});
        }
        let request = self
            .http
            .post(format!("{}/viber/2/messages", self.base_url))
            .header("Authorization", authorization)
            .json(&json!({"messages": [entry]}));
        let key = message.idempotency_key.clone();
        let outcome = outcome_from_http(
            exchange(request).await,
            |body| {
                serde_json::from_str::<Value>(body).ok()?["messages"][0]["messageId"]
                    .as_str()
                    .map(str::to_string)
            },
            |body| {
                serde_json::from_str::<Value>(body).ok()?["requestError"]["serviceException"]
                    ["messageId"]
                    .as_str()
                    .map(str::to_string)
            },
        );
        match outcome {
            SendOutcome::Accepted {
                provider_message_id: None,
            } => SendOutcome::Accepted {
                provider_message_id: Some(key),
            },
            other => other,
        }
    }

    async fn check(&self) -> AccountCheck {
        let now = Some(Utc::now().to_rfc3339());
        let Ok(authorization) = self.authorization() else {
            return AccountCheck {
                checked_at: now,
                reason: Some("missing API key".to_string()),
                ..Default::default()
            };
        };
        let balance = exchange(
            self.http
                .get(format!("{}/account/1/balance", self.base_url))
                .header("Authorization", authorization),
        )
        .await;
        let reason = match balance {
            Ok((200, _)) => None,
            Ok((status, _)) => Some(format!("Infobip answered with status {status}")),
            Err(_) => Some("Infobip unreachable".to_string()),
        };
        let connected = reason.is_none();
        AccountCheck {
            connected,
            production_access: connected,
            approved_templates: self
                .approved_templates
                .iter()
                .map(|(purpose, languages)| (*purpose, languages.keys().cloned().collect()))
                .collect(),
            checked_at: now,
            reason,
        }
    }

    async fn reconcile(&self, provider_message_id: &str) -> Option<MessageAttemptState> {
        let authorization = self.authorization().ok()?;
        let (status, body) = exchange(
            self.http
                .get(format!("{}/viber/2/logs", self.base_url))
                .query(&[("messageId", provider_message_id)])
                .header("Authorization", authorization),
        )
        .await
        .ok()?;
        if status != 200 {
            return None;
        }
        let body: Value = serde_json::from_str(&body).ok()?;
        // Logs lag behind sends, so an empty answer leaves the outcome unknown.
        status_state(body["results"].as_array()?.first()?["status"]["groupName"].as_str()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destination::Destination;
    use crate::test_server::{Reply, TestServer};
    use chrono::Duration;
    use sequent_core::types::messaging::{AccountLimits, MessageContent, OutOfWindowPolicy};

    fn account(base_url: &str) -> Account {
        Account {
            id: "vb-1".to_string(),
            channel: MessageChannel::VIBER,
            sender: AccountSender::VIBER_INFOBIP {
                base_url: base_url.to_string(),
                sender: "COMELEC".to_string(),
                approved_templates: BTreeMap::from([(
                    MessagePurpose::OTP,
                    BTreeMap::from([("en".to_string(), "tpl-otp-en".to_string())]),
                )]),
            },
            credentials: BTreeMap::from([(CredentialName::API_KEY, "key-1".to_string())]),
            limits: AccountLimits::default(),
            callback_url: Some("https://step.example/webhooks/viber/abc".to_string()),
        }
    }

    fn code() -> OutboundMessage {
        OutboundMessage {
            channel: MessageChannel::VIBER,
            purpose: MessagePurpose::OTP,
            destination: Destination::parse(MessageChannel::VIBER, "+639171234567").unwrap(),
            language: Some("en".to_string()),
            content: MessageContent {
                subject: None,
                text: "Your code is 123456".to_string(),
                html: None,
                template_parameters: vec![],
                code: Some("123456".to_string()),
            },
            provider_template: Some("tpl-otp-en".to_string()),
            idempotency_key: "logical-1:1".to_string(),
            expires_at: Some(Utc::now() + Duration::minutes(5)),
            last_inbound_at: None,
            out_of_window: OutOfWindowPolicy::DISABLED,
        }
    }

    #[tokio::test]
    async fn codes_use_the_partner_template_and_our_message_id() {
        let server = TestServer::start(vec![Reply::Json(
            200,
            json!({"messages": [{"messageId": "logical-1:1", "status": {"groupName": "PENDING"}}]}),
        )])
        .await;
        let sender =
            InfobipViberSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        assert_eq!(
            sender.send(&code()).await,
            SendOutcome::Accepted {
                provider_message_id: Some("logical-1:1".to_string())
            }
        );
        let request = &server.requests()[0];
        assert_eq!(request.path, "/viber/2/messages");
        assert_eq!(request.header("authorization"), Some("App key-1"));
        let body = request.json();
        let entry = &body["messages"][0];
        assert_eq!(entry["destinations"][0]["to"], "639171234567");
        assert_eq!(entry["destinations"][0]["messageId"], "logical-1:1");
        assert_eq!(entry["content"]["templateId"], "tpl-otp-en");
        assert_eq!(entry["content"]["parameters"]["pin"], "123456");
        assert_eq!(
            entry["webhooks"]["delivery"]["url"],
            "https://step.example/webhooks/viber/abc"
        );
    }

    #[tokio::test]
    async fn unknown_outcomes_are_reconciled_by_message_id() {
        let server = TestServer::start(vec![
            Reply::Json(200, json!({"results": [{"messageId": "logical-1:1", "status": {"groupName": "DELIVERED"}}]})),
            Reply::Json(200, json!({"results": []})),
        ])
        .await;
        let sender =
            InfobipViberSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        assert_eq!(
            sender.reconcile("logical-1:1").await,
            Some(MessageAttemptState::DELIVERED)
        );
        assert_eq!(sender.reconcile("logical-1:1").await, None);
        assert!(server.requests()[0]
            .path
            .starts_with("/viber/2/logs?messageId=logical-1"));
    }

    #[tokio::test]
    async fn the_check_reports_the_manually_approved_templates() {
        let server = TestServer::start(vec![Reply::Json(200, json!({"balance": 10}))]).await;
        let sender =
            InfobipViberSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        let check = sender.check().await;
        assert!(check.connected);
        assert_eq!(
            check.approved_templates,
            BTreeMap::from([(MessagePurpose::OTP, vec!["en".to_string()])])
        );
    }
}
