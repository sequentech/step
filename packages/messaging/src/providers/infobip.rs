// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Viber Business Messages through Infobip. Step sets the message ID, so an
//! attempt whose response was lost can be looked up by that ID.
//!
//! Delivery reports arrive only through the account's Infobip subscription
//! (notification profile), whose security setting is Basic auth with a user
//! name and password Step generates ([`new_report_credentials`]). Messages
//! never name a per-request report URL: one could bypass the subscription's
//! security setting.

use super::{
    exchange, missing_credential_reason, provider_status_reason, Account, PROVIDER_UNREACHABLE,
};
use crate::parameters::{parse_all, TemplateParameter};
use crate::sender::{
    outcome_from_http, ChannelSender, FailureKind, OutboundMessage, ProviderFailure, SendOutcome,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::Utc;
use sequent_core::types::messaging::{
    AccountCheck, AccountCheckDetails, AccountSender, CredentialName, MessageAttemptState,
    MessageChannel, MessagePurpose, MessagingProvider, PartnerLogin, ProviderCapabilities,
};

/// Why a Viber account isn't connected: Infobip refused its API key.
pub const PARTNER_LOGIN_REFUSED: &str = "Infobip refused the account's API key";
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The Basic auth an Infobip report subscription sends with each report.
/// Shown once to the administrator, then kept only in the vault.
#[derive(Clone, PartialEq, Eq)]
pub struct ReportCredentials {
    pub username: String,
    pub password: String,
}

impl std::fmt::Debug for ReportCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReportCredentials").finish_non_exhaustive()
    }
}

/// Random report credentials for a new subscription.
pub fn new_report_credentials() -> ReportCredentials {
    ReportCredentials {
        username: format!("step-reports-{}", &crate::link::new_reference()[..16]),
        password: crate::link::new_reference(),
    }
}

/// Why a report's `Authorization` header was refused. Each has one reason.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportAuthRefusal {
    /// The account has no report credentials yet.
    NOT_CONFIGURED,
    MISSING,
    MALFORMED,
    WRONG_CREDENTIALS,
}

impl ReportAuthRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NOT_CONFIGURED => "The account has no report credentials",
            Self::MISSING => "The report has no Basic authorization",
            Self::MALFORMED => "The report's authorization can't be read",
            Self::WRONG_CREDENTIALS => "The report's credentials are wrong",
        }
    }
}

/// Equal-time comparison: both sides are hashed first, so neither their
/// lengths nor their contents shape the time taken.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    openssl::memcmp::eq(&Sha256::digest(left), &Sha256::digest(right))
}

/// Checks a report's `Authorization` header against the account's report
/// credentials.
pub fn verify_report_authorization(
    authorization: Option<&str>,
    expected: Option<&ReportCredentials>,
) -> std::result::Result<(), ReportAuthRefusal> {
    let expected = expected
        .filter(|c| !c.username.is_empty() && !c.password.is_empty())
        .ok_or(ReportAuthRefusal::NOT_CONFIGURED)?;
    let authorization = authorization.ok_or(ReportAuthRefusal::MISSING)?.trim();
    let (scheme, encoded) = authorization
        .split_once(' ')
        .ok_or(ReportAuthRefusal::MALFORMED)?;
    if !scheme.eq_ignore_ascii_case("basic") {
        return Err(ReportAuthRefusal::MALFORMED);
    }
    let decoded = STANDARD
        .decode(encoded.trim())
        .map_err(|_| ReportAuthRefusal::MALFORMED)?;
    let wanted = format!("{}:{}", expected.username, expected.password);
    if constant_time_eq(&decoded, wanted.as_bytes()) {
        Ok(())
    } else {
        Err(ReportAuthRefusal::WRONG_CREDENTIALS)
    }
}

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
        // No `webhooks.delivery.url`: reports come only through the
        // account's subscription, with its Basic auth.
        let entry = json!({
            "sender": self.sender,
            "destinations": [{
                "to": message.destination.as_str().trim_start_matches('+'),
                "messageId": message.idempotency_key,
            }],
            "content": Self::content(message, template),
        });
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
                reason: Some(missing_credential_reason(CredentialName::API_KEY)),
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
            Ok((401 | 403, _)) => Some(PARTNER_LOGIN_REFUSED.to_string()),
            Ok((status, _)) => Some(provider_status_reason(status)),
            Err(_) => Some(PROVIDER_UNREACHABLE.to_string()),
        };
        let connected = reason.is_none();
        // Infobip has no API for Viber template approval: the templates are
        // the ones an administrator entered, and are shown as such.
        AccountCheck {
            connected,
            production_access: connected,
            approved_templates: self
                .approved_templates
                .iter()
                .map(|(purpose, languages)| (*purpose, languages.keys().cloned().collect()))
                .collect(),
            checked_at: now,
            details: AccountCheckDetails {
                partner_login: Some(PartnerLogin {
                    accepted: connected,
                    reason: reason.clone(),
                }),
                ..Default::default()
            },
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
            callback_url: Some("https://callbacks.example.org/webhooks/viber/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()),
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
        // The account has a callback address, and still no message names
        // one: reports come through the subscription's Basic auth only.
        assert!(sender.account.callback_url.is_some());
        assert_eq!(entry.get("webhooks"), None, "{body}");
    }

    fn basic(user_and_password: &str) -> String {
        format!("Basic {}", STANDARD.encode(user_and_password))
    }

    #[test]
    fn reports_need_the_subscriptions_basic_auth() {
        let expected = ReportCredentials {
            username: "step-reports-1".to_string(),
            password: "secret".to_string(),
        };
        assert_eq!(
            verify_report_authorization(Some(&basic("step-reports-1:secret")), Some(&expected)),
            Ok(())
        );
        assert_eq!(
            verify_report_authorization(
                Some(&format!(
                    "basic {}",
                    STANDARD.encode("step-reports-1:secret")
                )),
                Some(&expected)
            ),
            Ok(())
        );
        for (header, refusal) in [
            (None, ReportAuthRefusal::MISSING),
            (
                Some("Bearer token".to_string()),
                ReportAuthRefusal::MALFORMED,
            ),
            (Some("Basic".to_string()), ReportAuthRefusal::MALFORMED),
            (Some("Basic !!!".to_string()), ReportAuthRefusal::MALFORMED),
            (
                Some(basic("step-reports-1:wrong")),
                ReportAuthRefusal::WRONG_CREDENTIALS,
            ),
            (
                Some(basic("step-reports-1:secret ")),
                ReportAuthRefusal::WRONG_CREDENTIALS,
            ),
            (
                Some(basic("other:secret")),
                ReportAuthRefusal::WRONG_CREDENTIALS,
            ),
            (Some(basic(":")), ReportAuthRefusal::WRONG_CREDENTIALS),
        ] {
            assert_eq!(
                verify_report_authorization(header.as_deref(), Some(&expected)),
                Err(refusal),
                "{header:?}"
            );
        }
        // Before the credentials are generated, every report is refused.
        let empty = ReportCredentials {
            username: String::new(),
            password: String::new(),
        };
        for credentials in [None, Some(&empty)] {
            assert_eq!(
                verify_report_authorization(Some(&basic(":")), credentials),
                Err(ReportAuthRefusal::NOT_CONFIGURED)
            );
        }
    }

    #[test]
    fn generated_report_credentials_are_random_and_never_printed() {
        let first = new_report_credentials();
        let second = new_report_credentials();
        assert_ne!(first, second);
        assert!(first.username.starts_with("step-reports-"));
        assert_eq!(first.password.len(), 64);
        let printed = format!("{first:?}");
        assert!(!printed.contains(&first.password) && !printed.contains(&first.username));
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
        assert_eq!(
            check.details.partner_login,
            Some(PartnerLogin {
                accepted: true,
                reason: None
            })
        );
    }

    #[tokio::test]
    async fn a_refused_partner_login_is_not_connected() {
        let server = TestServer::start(vec![Reply::Json(401, json!({}))]).await;
        let sender =
            InfobipViberSender::new(account(&server.base_url), reqwest::Client::new()).unwrap();
        let check = sender.check().await;
        assert!(!check.connected);
        assert_eq!(
            check.reason.as_deref(),
            Some("Infobip refused the account's API key")
        );
        assert_eq!(
            check.details.partner_login,
            Some(PartnerLogin {
                accepted: false,
                reason: Some("Infobip refused the account's API key".to_string())
            })
        );
    }
}
