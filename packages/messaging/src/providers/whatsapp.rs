// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WhatsApp Cloud API. Codes use an approved authentication template whose
//! copy-code button carries the code; notices use approved utility
//! templates.

use super::{exchange, Account};
use crate::sender::{outcome_from_http, ChannelSender, OutboundMessage, SendOutcome};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sequent_core::types::messaging::{
    AccountCheck, AccountSender, CredentialName, MessageChannel, MessagePurpose, MessagingProvider,
    ProviderCapabilities,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub struct WhatsAppSender {
    account: Account,
    http: reqwest::Client,
    base_url: String,
    business_account_id: String,
    phone_number_id: String,
}

impl WhatsAppSender {
    pub fn new(account: Account, http: reqwest::Client, graph_url: String) -> Result<Self> {
        let AccountSender::WHATSAPP_CLOUD_API {
            business_account_id,
            phone_number_id,
            api_version,
            ..
        } = &account.sender
        else {
            return Err(anyhow!("not a WhatsApp account"));
        };
        Ok(WhatsAppSender {
            base_url: format!("{graph_url}/{api_version}"),
            business_account_id: business_account_id.clone(),
            phone_number_id: phone_number_id.clone(),
            account,
            http,
        })
    }

    fn token(&self) -> Result<&str> {
        self.account.credential(CredentialName::ACCESS_TOKEN)
    }

    fn template_body(message: &OutboundMessage, template: &str) -> Value {
        let text = |value: &str| json!({"type": "text", "text": value});
        let code = message.content.code.clone().unwrap_or_default();
        let components = if message.purpose == MessagePurpose::OTP {
            json!([
                {"type": "body", "parameters": [text(&code)]},
                {"type": "button", "sub_type": "url", "index": "0", "parameters": [text(&code)]}
            ])
        } else {
            let parameters: Vec<Value> = message
                .content
                .template_parameters
                .iter()
                .map(|p| text(p))
                .collect();
            if parameters.is_empty() {
                json!([])
            } else {
                json!([{"type": "body", "parameters": parameters}])
            }
        };
        json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": message.destination.as_str(),
            "type": "template",
            "template": {
                "name": template,
                "language": {"code": message.language.clone().unwrap_or_else(|| "en".to_string())},
                "components": components,
            }
        })
    }
}

fn error_code(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get("error")?
        .get("code")?
        .as_i64()
        .map(|code| code.to_string())
}

#[async_trait]
impl ChannelSender for WhatsAppSender {
    fn capabilities(&self) -> ProviderCapabilities {
        MessagingProvider::WHATSAPP_CLOUD_API
            .capabilities(MessageChannel::WHATSAPP)
            .unwrap_or_else(|| unreachable!("WhatsApp declares its own channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        let (Ok(token), Some(template)) = (self.token(), message.provider_template.as_deref())
        else {
            return SendOutcome::Rejected(crate::sender::ProviderFailure {
                kind: crate::sender::FailureKind::PERMANENT,
                code: None,
                reason: "missing credentials or approved template".to_string(),
            });
        };
        let request = self
            .http
            .post(format!(
                "{}/{}/messages",
                self.base_url, self.phone_number_id
            ))
            .bearer_auth(token)
            .json(&Self::template_body(message, template));
        outcome_from_http(
            exchange(request).await,
            |body| {
                serde_json::from_str::<Value>(body).ok()?["messages"][0]["id"]
                    .as_str()
                    .map(str::to_string)
            },
            error_code,
        )
    }

    async fn check(&self) -> AccountCheck {
        let now = Some(Utc::now().to_rfc3339());
        let Ok(token) = self.token() else {
            return AccountCheck {
                checked_at: now,
                reason: Some("missing access token".to_string()),
                ..Default::default()
            };
        };
        let number = exchange(
            self.http
                .get(format!("{}/{}", self.base_url, self.phone_number_id))
                .query(&[(
                    "fields",
                    "verified_name,code_verification_status,quality_rating,messaging_limit_tier",
                )])
                .bearer_auth(token),
        )
        .await;
        let number = match number {
            Ok((200, body)) => serde_json::from_str::<Value>(&body).unwrap_or_default(),
            Ok((status, _)) => {
                return AccountCheck {
                    checked_at: now,
                    reason: Some(format!("phone number lookup failed with status {status}")),
                    ..Default::default()
                }
            }
            Err(_) => {
                return AccountCheck {
                    checked_at: now,
                    reason: Some("WhatsApp Cloud API unreachable".to_string()),
                    ..Default::default()
                }
            }
        };
        let templates = exchange(
            self.http
                .get(format!(
                    "{}/{}/message_templates",
                    self.base_url, self.business_account_id
                ))
                .query(&[
                    ("fields", "name,language,status,category"),
                    ("limit", "200"),
                ])
                .bearer_auth(token),
        )
        .await;
        let mut approved_templates: BTreeMap<MessagePurpose, Vec<String>> = BTreeMap::new();
        if let Ok((200, body)) = templates {
            let body: Value = serde_json::from_str(&body).unwrap_or_default();
            for template in body["data"].as_array().cloned().unwrap_or_default() {
                if template["status"] != "APPROVED" {
                    continue;
                }
                let purpose = match template["category"].as_str() {
                    Some("AUTHENTICATION") => MessagePurpose::OTP,
                    Some("UTILITY") => MessagePurpose::NOTICE,
                    _ => continue,
                };
                if let Some(language) = template["language"].as_str() {
                    let languages = approved_templates.entry(purpose).or_default();
                    if !languages.iter().any(|l| l == language) {
                        languages.push(language.to_string());
                    }
                }
            }
        }
        AccountCheck {
            connected: true,
            production_access: number["code_verification_status"] == "VERIFIED",
            approved_templates,
            checked_at: now,
            reason: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destination::Destination;
    use crate::test_server::{Reply, TestServer};
    use chrono::Duration;
    use sequent_core::types::messaging::{
        AccountLimits, MessageAttemptState, MessageContent, OutOfWindowPolicy,
    };

    fn account() -> Account {
        Account {
            id: "wa-1".to_string(),
            channel: MessageChannel::WHATSAPP,
            sender: AccountSender::WHATSAPP_CLOUD_API {
                business_account_id: "waba-1".to_string(),
                phone_number_id: "pn-1".to_string(),
                display_phone_number: "+63 2 8123 4567".to_string(),
                display_name: None,
                api_version: "v23.0".to_string(),
            },
            credentials: BTreeMap::from([(CredentialName::ACCESS_TOKEN, "token-1".to_string())]),
            limits: AccountLimits::default(),
            callback_url: None,
        }
    }

    fn code_message() -> OutboundMessage {
        OutboundMessage {
            channel: MessageChannel::WHATSAPP,
            purpose: MessagePurpose::OTP,
            destination: Destination::parse(MessageChannel::WHATSAPP, "+639171234567").unwrap(),
            language: Some("en".to_string()),
            content: MessageContent {
                subject: None,
                text: "123456 is your verification code.".to_string(),
                html: None,
                template_parameters: vec![],
                code: Some("123456".to_string()),
            },
            provider_template: Some("comelec_otp".to_string()),
            idempotency_key: "key-1".to_string(),
            expires_at: Some(Utc::now() + Duration::minutes(5)),
            last_inbound_at: None,
            out_of_window: OutOfWindowPolicy::DISABLED,
        }
    }

    #[tokio::test]
    async fn codes_are_sent_with_the_authentication_template() {
        let server = TestServer::start(vec![Reply::Json(
            200,
            json!({"messages": [{"id": "wamid.1"}]}),
        )])
        .await;
        let sender =
            WhatsAppSender::new(account(), reqwest::Client::new(), server.base_url.clone())
                .unwrap();
        let outcome = sender.send(&code_message()).await;
        assert_eq!(
            outcome,
            SendOutcome::Accepted {
                provider_message_id: Some("wamid.1".to_string())
            }
        );
        let request = &server.requests()[0];
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/v23.0/pn-1/messages");
        assert_eq!(request.header("authorization"), Some("Bearer token-1"));
        let body = request.json();
        assert_eq!(body["to"], "+639171234567");
        assert_eq!(body["template"]["name"], "comelec_otp");
        assert_eq!(
            body["template"]["components"][1]["parameters"][0]["text"],
            "123456"
        );
    }

    #[tokio::test]
    async fn a_lost_response_is_an_unknown_outcome() {
        let server = TestServer::start(vec![Reply::Drop]).await;
        let sender =
            WhatsAppSender::new(account(), reqwest::Client::new(), server.base_url.clone())
                .unwrap();
        assert_eq!(
            sender.send(&code_message()).await.state(),
            MessageAttemptState::UNKNOWN
        );
    }

    #[tokio::test]
    async fn rejected_requests_keep_the_provider_code_only() {
        let server = TestServer::start(vec![Reply::Json(
            400,
            json!({"error": {"code": 131026, "message": "Message undeliverable to +639171234567"}}),
        )])
        .await;
        let sender =
            WhatsAppSender::new(account(), reqwest::Client::new(), server.base_url.clone())
                .unwrap();
        match sender.send(&code_message()).await {
            SendOutcome::Rejected(failure) => {
                assert_eq!(failure.code.as_deref(), Some("131026"));
                assert!(!failure.reason.contains("+63"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[tokio::test]
    async fn the_check_reports_verification_and_approved_templates() {
        let server = TestServer::start(vec![
            Reply::Json(200, json!({"verified_name": "COMELEC", "code_verification_status": "VERIFIED"})),
            Reply::Json(
                200,
                json!({"data": [
                    {"name": "otp", "language": "en", "status": "APPROVED", "category": "AUTHENTICATION"},
                    {"name": "otp", "language": "tl", "status": "PENDING", "category": "AUTHENTICATION"},
                    {"name": "approved", "language": "en", "status": "APPROVED", "category": "UTILITY"},
                    {"name": "promo", "language": "en", "status": "APPROVED", "category": "MARKETING"}
                ]}),
            ),
        ])
        .await;
        let sender =
            WhatsAppSender::new(account(), reqwest::Client::new(), server.base_url.clone())
                .unwrap();
        let check = sender.check().await;
        assert!(check.connected);
        assert!(check.production_access);
        assert_eq!(
            check.approved_templates,
            BTreeMap::from([
                (MessagePurpose::OTP, vec!["en".to_string()]),
                (MessagePurpose::NOTICE, vec!["en".to_string()]),
            ])
        );
        assert_eq!(
            server.requests()[1].path.split('?').next(),
            Some("/v23.0/waba-1/message_templates")
        );
    }

    #[tokio::test]
    async fn rejected_credentials_are_not_connected() {
        let server =
            TestServer::start(vec![Reply::Json(401, json!({"error": {"code": 190}}))]).await;
        let sender =
            WhatsAppSender::new(account(), reqwest::Client::new(), server.base_url.clone())
                .unwrap();
        let check = sender.check().await;
        assert!(!check.connected);
        assert_eq!(
            check.reason.as_deref(),
            Some("phone number lookup failed with status 401")
        );
    }
}
