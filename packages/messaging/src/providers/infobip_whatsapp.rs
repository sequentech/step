// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WhatsApp through Infobip: Meta's approved templates, sent with the
//! account's Infobip API key. Step sets the message ID.
//!
//! Delivery reports arrive through the account's Infobip subscription, with
//! the same Basic auth as Viber's ([`super::infobip`]). Infobip has no
//! WhatsApp message lookup, so an unknown outcome waits for its report.

use super::infobip::{authorization, PARTNER_LOGIN_REFUSED};
use super::{
    exchange, missing_credential_reason, provider_status_reason, whatsapp_templates, Account,
    PROVIDER_UNREACHABLE,
};
use crate::parameters::parse_all;
use crate::sender::{
    outcome_from_http, ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sequent_core::types::messaging::{
    AccountCheck, AccountCheckDetails, AccountSender, CredentialName, MessageChannel,
    MessagePurpose, PartnerLogin, ProviderCapabilities,
};
use serde_json::{json, Value};

pub struct InfobipWhatsAppSender {
    account: Account,
    http: reqwest::Client,
    base_url: String,
    sender: String,
}

impl InfobipWhatsAppSender {
    pub fn new(account: Account, http: reqwest::Client) -> Result<Self> {
        let AccountSender::WHATSAPP_INFOBIP {
            base_url,
            sender_number,
            ..
        } = &account.sender
        else {
            return Err(anyhow!("not an Infobip WhatsApp account"));
        };
        Ok(InfobipWhatsAppSender {
            base_url: base_url.trim_end_matches('/').to_string(),
            sender: sender_number.trim().trim_start_matches('+').to_string(),
            account,
            http,
        })
    }

    /// The template's content. A code fills the authentication template's
    /// body and its copy-code button, which Infobip takes as a URL button;
    /// a notice's parameters fill the body's placeholders in order.
    fn template_content(message: &OutboundMessage, template: &str) -> Value {
        let language = message.language.clone().unwrap_or_else(|| "en".to_string());
        let template_data = if message.purpose == MessagePurpose::OTP {
            let code = message.content.code.clone().unwrap_or_default();
            json!({
                "body": {"placeholders": [code]},
                "buttons": [{"type": "URL", "parameter": code}],
            })
        } else {
            let placeholders: Vec<String> = parse_all(&message.content.template_parameters)
                .iter()
                .map(|parameter| parameter.value().to_string())
                .collect();
            json!({"body": {"placeholders": placeholders}})
        };
        json!({
            "templateName": template,
            "templateData": template_data,
            "language": language,
        })
    }
}

#[async_trait]
impl ChannelSender for InfobipWhatsAppSender {
    fn capabilities(&self) -> ProviderCapabilities {
        self.account
            .sender
            .capabilities(MessageChannel::WHATSAPP)
            .unwrap_or_else(|| unreachable!("Infobip WhatsApp declares its own channel"))
    }

    async fn send(&self, message: &OutboundMessage) -> SendOutcome {
        let (Ok(authorization), Some(template)) = (
            authorization(&self.account),
            message.provider_template.as_deref(),
        ) else {
            return SendOutcome::Rejected(ProviderFailure {
                kind: FailureKind::PERMANENT,
                code: None,
                reason: "missing API key or approved template".to_string(),
            });
        };
        // No `notifyUrl`: reports come only through the account's
        // subscription, with its Basic auth.
        let entry = json!({
            "from": self.sender,
            "to": message.destination.as_str().trim_start_matches('+'),
            "messageId": message.idempotency_key,
            "content": Self::template_content(message, template),
        });
        let request = self
            .http
            .post(format!("{}/whatsapp/1/message/template", self.base_url))
            .header("Authorization", authorization)
            .json(&json!({"messages": [entry]}));
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
                provider_message_id: Some(message.idempotency_key.clone()),
            },
            other => other,
        }
    }

    async fn check(&self) -> AccountCheck {
        let now = Some(Utc::now().to_rfc3339());
        let Ok(authorization) = authorization(&self.account) else {
            return AccountCheck {
                checked_at: now,
                reason: Some(missing_credential_reason(CredentialName::API_KEY)),
                ..Default::default()
            };
        };
        // The sender's templates: the request checks the key against this
        // sender, and its answer holds the approvals.
        let sender = urlencoding::encode(&self.sender);
        let answer = exchange(
            self.http
                .get(format!(
                    "{}/whatsapp/2/senders/{sender}/templates",
                    self.base_url
                ))
                .header("Authorization", authorization),
        )
        .await;
        let (reason, body) = match answer {
            Ok((200, body)) => (None, Some(body)),
            Ok((401 | 403, _)) => (Some(PARTNER_LOGIN_REFUSED.to_string()), None),
            Ok((status, _)) => (Some(provider_status_reason(status)), None),
            Err(_) => (Some(PROVIDER_UNREACHABLE.to_string()), None),
        };
        let connected = reason.is_none();
        let mut details = AccountCheckDetails {
            partner_login: Some(PartnerLogin {
                accepted: connected,
                reason: reason.clone(),
            }),
            ..Default::default()
        };
        let body: Value = body
            .and_then(|body| serde_json::from_str(&body).ok())
            .unwrap_or_default();
        let (templates, approved_templates) = whatsapp_templates(
            body["templates"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default(),
        );
        details.templates = templates;
        AccountCheck {
            connected,
            production_access: connected,
            approved_templates,
            checked_at: now,
            reason,
            details,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::destination::Destination;
    use crate::providers::infobip::PARTNER_LOGIN_REFUSED;
    use crate::providers::provider_status_reason;
    use crate::sender::{FailureKind, ProviderFailure};
    use crate::test_server::{Reply, TestServer};
    use chrono::{Duration, Utc};
    use sequent_core::types::messaging::{
        AccountLimits, CredentialName, MessageContent, MessagePurpose, OutOfWindowPolicy,
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn account(base_url: &str) -> Account {
        Account {
            id: "wa-1".to_string(),
            channel: MessageChannel::WHATSAPP,
            sender: AccountSender::WHATSAPP_INFOBIP {
                base_url: format!("{base_url}/"),
                sender_number: "+447796344125".to_string(),
                display_name: Some("COMELEC".to_string()),
            },
            credentials: BTreeMap::from([(CredentialName::API_KEY, "key-1".to_string())]),
            limits: AccountLimits::default(),
            callback_url: None,
        }
    }

    fn code() -> OutboundMessage {
        OutboundMessage {
            channel: MessageChannel::WHATSAPP,
            purpose: MessagePurpose::OTP,
            destination: Destination::parse(MessageChannel::WHATSAPP, "+639171234567").unwrap(),
            language: Some("en_GB".to_string()),
            content: MessageContent {
                subject: None,
                text: "Your code is 123456".to_string(),
                html: None,
                template_parameters: vec![],
                code: Some("123456".to_string()),
            },
            provider_template: Some("comelec_otp".to_string()),
            idempotency_key: "logical-1:1".to_string(),
            expires_at: Some(Utc::now() + Duration::minutes(5)),
            last_inbound_at: None,
            out_of_window: OutOfWindowPolicy::DISABLED,
        }
    }

    fn accepted(message_id: &str) -> Reply {
        Reply::Json(
            200,
            json!({"messages": [{"to": "639171234567", "messageCount": 1, "messageId": message_id,
                "status": {"groupName": "PENDING", "name": "PENDING_ENROUTE"}}]}),
        )
    }

    #[tokio::test]
    async fn codes_fill_the_authentication_templates_body_and_copy_code_button() {
        let server = TestServer::start(vec![accepted("logical-1:1")]).await;
        let sender =
            InfobipWhatsAppSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        assert_eq!(
            sender.send(&code()).await,
            SendOutcome::Accepted {
                provider_message_id: Some("logical-1:1".to_string())
            }
        );
        let request = &server.requests()[0];
        assert_eq!(request.path, "/whatsapp/1/message/template");
        assert_eq!(request.header("authorization"), Some("App key-1"));
        assert_eq!(
            request.json(),
            json!({"messages": [{
                "from": "447796344125",
                "to": "639171234567",
                "messageId": "logical-1:1",
                "content": {
                    "templateName": "comelec_otp",
                    "templateData": {
                        "body": {"placeholders": ["123456"]},
                        "buttons": [{"type": "URL", "parameter": "123456"}],
                    },
                    "language": "en_GB",
                },
            }]})
        );
    }

    #[tokio::test]
    async fn notices_fill_the_body_placeholders_in_order() {
        let server = TestServer::start(vec![accepted("logical-2:1")]).await;
        let sender =
            InfobipWhatsAppSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        let mut notice = code();
        notice.purpose = MessagePurpose::NOTICE;
        notice.idempotency_key = "logical-2:1".to_string();
        notice.language = None;
        notice.provider_template = Some("polls_close".to_string());
        notice.content.code = None;
        // WhatsApp templates through Infobip take placeholders by position:
        // a named parameter keeps its place.
        notice.content.template_parameters = vec!["Ana".to_string(), "@hour=8 pm".to_string()];
        assert!(matches!(
            sender.send(&notice).await,
            SendOutcome::Accepted { .. }
        ));
        assert_eq!(
            server.requests()[0].json()["messages"][0]["content"],
            json!({
                "templateName": "polls_close",
                "templateData": {"body": {"placeholders": ["Ana", "8 pm"]}},
                "language": "en",
            })
        );
    }

    #[tokio::test]
    async fn nothing_is_sent_without_an_api_key_or_template() {
        let server = TestServer::start(vec![]).await;
        let sender =
            InfobipWhatsAppSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        let mut untemplated = code();
        untemplated.provider_template = None;
        let mut keyless = account(&server.base_url);
        keyless.credentials.clear();
        let keyless = InfobipWhatsAppSender::new(keyless, reqwest::Client::new()).unwrap();
        for outcome in [sender.send(&untemplated).await, keyless.send(&code()).await] {
            assert!(matches!(
                outcome,
                SendOutcome::Rejected(ProviderFailure {
                    kind: FailureKind::PERMANENT,
                    ..
                })
            ));
        }
        assert!(server.requests().is_empty());
    }

    #[tokio::test]
    async fn a_refused_send_keeps_infobips_reason_code() {
        let server = TestServer::start(vec![Reply::Json(
            400,
            json!({"requestError": {"serviceException": {"messageId": "BAD_REQUEST", "text": "Bad request"}}}),
        )])
        .await;
        let sender =
            InfobipWhatsAppSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        let SendOutcome::Rejected(failure) = sender.send(&code()).await else {
            panic!("rejected");
        };
        assert_eq!(failure.code.as_deref(), Some("BAD_REQUEST"));
    }

    #[tokio::test]
    async fn the_check_reads_the_senders_templates_from_infobip() {
        let server = TestServer::start(vec![
            Reply::Json(
                200,
                json!({"templates": [
                    {"id": "1", "name": "comelec_otp", "language": "en", "status": "APPROVED", "category": "AUTHENTICATION"},
                    {"id": "2", "name": "comelec_otp", "language": "tl", "status": "PENDING", "category": "AUTHENTICATION"},
                    {"id": "3", "name": "polls_close", "language": "en", "status": "APPROVED", "category": "UTILITY"},
                    {"id": "4", "name": "promo", "language": "en", "status": "APPROVED", "category": "MARKETING"}
                ]}),
            ),
        ])
        .await;
        let sender =
            InfobipWhatsAppSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        let check = sender.check().await;
        assert!(check.connected);
        assert!(check.production_access);
        assert_eq!(check.reason, None);
        assert_eq!(
            check.approved_templates,
            BTreeMap::from([
                (MessagePurpose::OTP, vec!["en".to_string()]),
                (MessagePurpose::NOTICE, vec!["en".to_string()]),
            ])
        );
        assert_eq!(check.details.templates.len(), 4);
        assert!(check
            .details
            .partner_login
            .is_some_and(|login| login.accepted));
        // One request checks the key against this sender and reads its
        // templates; the account's balance is not asked for.
        let requests = server.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(
            requests[0].path,
            "/whatsapp/2/senders/447796344125/templates"
        );
        assert_eq!(requests[0].header("authorization"), Some("App key-1"));
    }

    #[tokio::test]
    async fn a_refused_api_key_or_sender_is_not_connected() {
        for (status, reason) in [
            (401, PARTNER_LOGIN_REFUSED.to_string()),
            (403, PARTNER_LOGIN_REFUSED.to_string()),
            (404, provider_status_reason(404)),
        ] {
            let server = TestServer::start(vec![Reply::Json(status, json!({}))]).await;
            let sender =
                InfobipWhatsAppSender::new(account(&server.base_url), reqwest::Client::new())
                    .unwrap();
            let check = sender.check().await;
            assert!(!check.connected, "{status}");
            assert!(!check.production_access, "{status}");
            assert_eq!(check.reason, Some(reason.clone()), "{status}");
            assert_eq!(
                check.details.partner_login,
                Some(PartnerLogin {
                    accepted: false,
                    reason: Some(reason)
                })
            );
            assert!(check.approved_templates.is_empty());
        }
    }
}
