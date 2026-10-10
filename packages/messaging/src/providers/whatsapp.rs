// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! WhatsApp Cloud API. Codes use an approved authentication template whose
//! copy-code button carries the code; notices use approved utility
//! templates.

use super::{
    exchange, meta_app_secret_status, meta_outcome, meta_templates, meta_token_status,
    missing_credential_reason, provider_status_reason, Account, NUMBER_IN_SANDBOX,
    NUMBER_NAME_PREFIX, NUMBER_NOT_VERIFIED, NUMBER_STATUS_PREFIX, PROVIDER_UNREACHABLE,
    TEMPLATES_NOT_READ,
};
use crate::parameters::{meta_text_parameter, parse_all};
use crate::sender::{outcome_from_http, ChannelSender, OutboundMessage, SendOutcome};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use sequent_core::types::messaging::{
    AccountCheck, AccountCheckDetails, AccountSender, CredentialName, MessageChannel,
    MessagePurpose, MessagingProvider, ProviderApproval, ProviderCapabilities,
    WhatsAppNumberStatus,
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
            api_base_url,
            ..
        } = &account.sender
        else {
            return Err(anyhow!("not a WhatsApp account"));
        };
        let graph_url = api_base_url
            .as_deref()
            .map(|url| url.trim_end_matches('/').to_string())
            .unwrap_or(graph_url);
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
            let parameters: Vec<Value> = parse_all(&message.content.template_parameters)
                .iter()
                .map(meta_text_parameter)
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
        meta_outcome(outcome_from_http(
            exchange(request).await,
            |body| {
                serde_json::from_str::<Value>(body).ok()?["messages"][0]["id"]
                    .as_str()
                    .map(str::to_string)
            },
            error_code,
        ))
    }

    async fn check(&self) -> AccountCheck {
        let now = Some(Utc::now().to_rfc3339());
        let Ok(token) = self.token() else {
            return AccountCheck {
                checked_at: now,
                reason: Some(missing_credential_reason(CredentialName::ACCESS_TOKEN)),
                ..Default::default()
            };
        };
        // The number is looked up by ID alone first: a field the API version
        // does not know would fail the whole request.
        let number = exchange(
            self.http
                .get(format!("{}/{}", self.base_url, self.phone_number_id))
                .query(&[("fields", "id")])
                .bearer_auth(token),
        )
        .await;
        match number {
            Ok((200, _)) => {}
            Ok((status, _)) => {
                return AccountCheck {
                    checked_at: now,
                    reason: Some(provider_status_reason(status)),
                    ..Default::default()
                }
            }
            Err(_) => {
                return AccountCheck {
                    checked_at: now,
                    reason: Some(PROVIDER_UNREACHABLE.to_string()),
                    ..Default::default()
                }
            }
        };
        let mut details = AccountCheckDetails {
            token: meta_token_status(&self.http, &self.base_url, token, WHATSAPP_SCOPES).await,
            ..Default::default()
        };
        let status = exchange(
            self.http
                .get(format!("{}/{}", self.base_url, self.phone_number_id))
                .query(&[(
                    "fields",
                    "status,account_mode,code_verification_status,name_status",
                )])
                .bearer_auth(token),
        )
        .await;
        // In production unless the API says the number is a sandbox one or
        // its code is not verified.
        let mut number_status = WhatsAppNumberStatus::default();
        let (production_access, reason) = match status {
            Ok((200, body)) => {
                let number: Value = serde_json::from_str(&body).unwrap_or_default();
                number_status.status = number["status"].as_str().map(str::to_string);
                number_status.name_status = number["name_status"].as_str().map(str::to_string);
                let (approval, approval_reason) = number_approval(&number_status);
                details.provider_approval = approval;
                details.provider_approval_reason = approval_reason;
                let sandbox = number["account_mode"] == "SANDBOX";
                let unverified = number["code_verification_status"]
                    .as_str()
                    .is_some_and(|status| status != "VERIFIED");
                let reason = if sandbox {
                    Some(NUMBER_IN_SANDBOX.to_string())
                } else if unverified {
                    Some(NUMBER_NOT_VERIFIED.to_string())
                } else {
                    None
                };
                (reason.is_none(), reason)
            }
            _ => (false, Some(NUMBER_STATUS_NOT_READ.to_string())),
        };
        // Quality and limit are read on their own: older API versions don't
        // know `messaging_limit_tier`, and its absence mustn't hide the
        // quality rating.
        let quality = |fields: &'static str| {
            self.http
                .get(format!("{}/{}", self.base_url, self.phone_number_id))
                .query(&[("fields", fields)])
                .bearer_auth(token)
        };
        let quality_body = match exchange(quality("quality_rating,messaging_limit_tier")).await {
            Ok((200, body)) => Some(body),
            _ => match exchange(quality("quality_rating")).await {
                Ok((200, body)) => Some(body),
                _ => None,
            },
        };
        if let Some(body) = quality_body {
            let body: Value = serde_json::from_str(&body).unwrap_or_default();
            number_status.quality_rating = body["quality_rating"].as_str().map(str::to_string);
            number_status.messaging_limit =
                body["messaging_limit_tier"].as_str().map(str::to_string);
        }
        if number_status != WhatsAppNumberStatus::default() {
            details.whatsapp_number = Some(number_status);
        }
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
        let approved_templates = match templates {
            Ok((200, body)) => {
                let (templates, approved) = meta_templates(&body);
                details.templates = templates;
                approved
            }
            _ => {
                details.templates_reason = Some(TEMPLATES_NOT_READ.to_string());
                BTreeMap::new()
            }
        };
        // Callbacks are signed with the app secret: a wrong one breaks
        // them all, so it is checked too (R5-16).
        details.app_secret = meta_app_secret_status(
            &self.http,
            &self.base_url,
            &self.phone_number_id,
            token,
            self.account.credential(CredentialName::APP_SECRET).ok(),
        )
        .await;
        AccountCheck {
            connected: true,
            production_access,
            approved_templates,
            checked_at: now,
            reason,
            details,
        }
    }
}

/// Permissions the account's token needs: sending, and reading templates.
const WHATSAPP_SCOPES: &[&str] = &[
    "whatsapp_business_messaging",
    "whatsapp_business_management",
];

/// The reason given when the number's status couldn't be read.
pub const NUMBER_STATUS_NOT_READ: &str = "The number's status couldn't be read";

/// The number states in which Meta doesn't deliver its messages. Any other
/// state sends: `CONNECTED`, and `FLAGGED` or `RATE_LIMITED`, which are
/// quality warnings shown with the number's status, not a missing approval
/// (R5-17).
const NOT_SENDING_STATES: &[&str] = &[
    "PENDING",
    "RESTRICTED",
    "BANNED",
    "DISCONNECTED",
    "DELETED",
    "UNVERIFIED",
];

/// The provider's approval of the number, from its status and its display
/// name's review: pending while the number is in a state that doesn't
/// send ([`NOT_SENDING_STATES`]) or its name isn't approved. `None` when
/// Meta reported neither, so the administrator's entry is used.
fn number_approval(number: &WhatsAppNumberStatus) -> (Option<ProviderApproval>, Option<String>) {
    if number.status.is_none() && number.name_status.is_none() {
        return (None, None);
    }
    let reason = match (number.status.as_deref(), number.name_status.as_deref()) {
        (Some(status), _) if NOT_SENDING_STATES.contains(&status) => {
            Some(format!("{NUMBER_STATUS_PREFIX}{status}"))
        }
        (None, _) => Some(format!("{NUMBER_STATUS_PREFIX}UNKNOWN")),
        (_, Some("APPROVED" | "AVAILABLE_WITHOUT_REVIEW")) => None,
        (_, name) => Some(format!(
            "{NUMBER_NAME_PREFIX}{}",
            name.unwrap_or("NOT_REVIEWED")
        )),
    };
    match reason {
        None => (Some(ProviderApproval::CONFIRMED), None),
        Some(reason) => (Some(ProviderApproval::PENDING), Some(reason)),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{APP_SECRET_NOT_CHECKED, APP_SECRET_REFUSED};
    use super::*;
    use crate::destination::Destination;
    use crate::test_server::{Reply, TestServer};
    use chrono::Duration;
    use sequent_core::types::messaging::{
        AccountLimits, AppSecretStatus, MessageAttemptState, MessageContent, OutOfWindowPolicy,
        TemplateStatus, TokenStatus,
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
                api_base_url: None,
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
            Reply::Json(200, json!({"id": "pn-1"})),
            Reply::Json(
                200,
                json!({"data": {"is_valid": true, "expires_at": 1893456000,
                    "scopes": ["whatsapp_business_messaging"]}}),
            ),
            Reply::Json(
                200,
                json!({"account_mode": "LIVE", "code_verification_status": "VERIFIED",
                    "status": "CONNECTED", "name_status": "APPROVED"}),
            ),
            Reply::Json(
                200,
                json!({"quality_rating": "GREEN", "messaging_limit_tier": "TIER_1K"}),
            ),
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
        let requests = server.requests();
        assert!(requests[1].path.starts_with("/v23.0/debug_token?"));
        assert!(requests[1].path.contains("input_token=token-1"));
        assert_eq!(
            requests[4].path.split('?').next(),
            Some("/v23.0/waba-1/message_templates")
        );
        let details = &check.details;
        assert_eq!(
            details.token,
            Some(TokenStatus {
                valid: true,
                expires_at: Some("2030-01-01T00:00:00+00:00".to_string()),
                missing_scopes: vec!["whatsapp_business_management".to_string()],
            })
        );
        assert_eq!(
            details.whatsapp_number,
            Some(WhatsAppNumberStatus {
                status: Some("CONNECTED".to_string()),
                name_status: Some("APPROVED".to_string()),
                quality_rating: Some("GREEN".to_string()),
                messaging_limit: Some("TIER_1K".to_string()),
            })
        );
        assert_eq!(details.provider_approval, Some(ProviderApproval::CONFIRMED));
        assert_eq!(details.templates.len(), 4);
        assert_eq!(
            details.templates[1],
            TemplateStatus {
                name: "otp".to_string(),
                language: "tl".to_string(),
                category: Some("AUTHENTICATION".to_string()),
                status: "PENDING".to_string(),
            }
        );
        assert_eq!(details.templates_reason, None);
    }

    /// What the check couldn't read is absent, with its reason; an older
    /// API version without `messaging_limit_tier` still gives the quality.
    #[tokio::test]
    async fn the_check_says_what_it_could_not_read() {
        let server = TestServer::start(vec![
            Reply::Json(200, json!({"id": "pn-1"})),
            Reply::Json(400, json!({"error": {"code": 100}})),
            Reply::Json(
                200,
                json!({"account_mode": "LIVE", "code_verification_status": "VERIFIED",
                    "status": "CONNECTED", "name_status": "PENDING_REVIEW"}),
            ),
            Reply::Json(400, json!({"error": {"code": 100}})),
            Reply::Json(200, json!({"quality_rating": "YELLOW"})),
            Reply::Json(500, json!({})),
        ])
        .await;
        let sender =
            WhatsAppSender::new(account(), reqwest::Client::new(), server.base_url.clone())
                .unwrap();
        let check = sender.check().await;
        assert!(check.connected);
        let details = &check.details;
        assert_eq!(details.token, None);
        assert_eq!(
            details
                .whatsapp_number
                .as_ref()
                .and_then(|n| n.quality_rating.as_deref()),
            Some("YELLOW")
        );
        assert_eq!(
            details
                .whatsapp_number
                .as_ref()
                .and_then(|n| n.messaging_limit.as_deref()),
            None
        );
        assert_eq!(details.provider_approval, Some(ProviderApproval::PENDING));
        assert_eq!(
            details.provider_approval_reason.as_deref(),
            Some("The number's display name is PENDING_REVIEW")
        );
        assert!(details.templates.is_empty());
        assert_eq!(
            details.templates_reason.as_deref(),
            Some("The provider's templates couldn't be read")
        );
        assert!(check.approved_templates.is_empty());
    }

    #[test]
    fn the_number_is_approved_only_when_connected_with_an_approved_name() {
        let number = |status: Option<&str>, name: Option<&str>| WhatsAppNumberStatus {
            status: status.map(str::to_string),
            name_status: name.map(str::to_string),
            ..Default::default()
        };
        assert_eq!(number_approval(&number(None, None)), (None, None));
        assert_eq!(
            number_approval(&number(Some("CONNECTED"), Some("AVAILABLE_WITHOUT_REVIEW"))),
            (Some(ProviderApproval::CONFIRMED), None)
        );
        for blocked in [
            "PENDING",
            "RESTRICTED",
            "BANNED",
            "DISCONNECTED",
            "DELETED",
            "UNVERIFIED",
        ] {
            assert_eq!(
                number_approval(&number(Some(blocked), Some("APPROVED"))).0,
                Some(ProviderApproval::PENDING),
                "{blocked}"
            );
        }
    }

    /// R5-16: a replaced app secret is checked with Meta's proof; a
    /// refused proof says so, and one Meta didn't answer says why it isn't
    /// known.
    #[tokio::test]
    async fn the_app_secret_is_checked_with_its_proof() {
        let mut with_secret = account();
        with_secret
            .credentials
            .insert(CredentialName::APP_SECRET, "secret-1".to_string());
        let proof_refused = json!({"error": {"message":
            "Invalid appsecret_proof provided in the API argument", "code": 100}});
        for (reply, expected) in [
            (
                Reply::Json(200, json!({"id": "pn-1"})),
                AppSecretStatus {
                    valid: Some(true),
                    reason: None,
                },
            ),
            (
                Reply::Json(400, proof_refused),
                AppSecretStatus {
                    valid: Some(false),
                    reason: Some(APP_SECRET_REFUSED.to_string()),
                },
            ),
            (
                Reply::Json(500, json!({})),
                AppSecretStatus {
                    valid: None,
                    reason: Some(APP_SECRET_NOT_CHECKED.to_string()),
                },
            ),
        ] {
            let server = TestServer::start(vec![reply]).await;
            let status = meta_app_secret_status(
                &reqwest::Client::new(),
                &server.base_url,
                "pn-1",
                "token-1",
                with_secret.credential(CredentialName::APP_SECRET).ok(),
            )
            .await;
            assert_eq!(status, Some(expected));
            let request = &server.requests()[0];
            // HMAC-SHA256("token-1") keyed by "secret-1": never the secret.
            assert!(
                request.path.contains("appsecret_proof="),
                "{}",
                request.path
            );
            assert!(!request.path.contains("secret-1"));
        }
        assert_eq!(
            meta_app_secret_status(&reqwest::Client::new(), "http://unused", "pn-1", "t", None)
                .await,
            None
        );
    }

    /// R5-17 regression: a quality warning (`FLAGGED`) or a rate limit
    /// read as "pending approval", which the send gate would turn into no
    /// codes at all, while Meta still delivers. Only the states in which
    /// the number can't send block.
    #[test]
    fn a_quality_warning_is_not_a_pending_approval() {
        let number = |status: &str| WhatsAppNumberStatus {
            status: Some(status.to_string()),
            name_status: Some("APPROVED".to_string()),
            ..Default::default()
        };
        for sending in ["FLAGGED", "RATE_LIMITED"] {
            assert_eq!(
                number_approval(&number(sending)),
                (Some(ProviderApproval::CONFIRMED), None),
                "{sending}"
            );
        }
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
            Some("The provider answered with status 401")
        );
    }
}
