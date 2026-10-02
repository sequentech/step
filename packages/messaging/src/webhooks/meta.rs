// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Meta webhooks (WhatsApp Cloud API and Messenger): the subscription
//! handshake, payload signatures and the events Step uses.

use super::{InboundMessage, MessengerReferral, StatusReport, WebhookEvent};
use anyhow::{anyhow, Result};
use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use sequent_core::types::messaging::{AccountSender, MessageAttemptState};
use serde_json::Value;
use sha2::Sha256;

/// Answers Meta's subscription request (`hub.mode`, `hub.verify_token`,
/// `hub.challenge`). Returns the challenge to echo, or `None` to refuse.
pub fn verify_subscription(
    mode: Option<&str>,
    token: Option<&str>,
    challenge: Option<&str>,
    expected_token: &str,
) -> Option<String> {
    let token = token?;
    let matches: bool = hmac_equal(token.as_bytes(), expected_token.as_bytes());
    (mode == Some("subscribe") && matches && !expected_token.is_empty())
        .then(|| challenge.map(str::to_string))
        .flatten()
}

/// Constant-time comparison: `b` is checked as the MAC of `a` under a key
/// derived from `b` itself.
fn hmac_equal(a: &[u8], b: &[u8]) -> bool {
    let tag = |value: &[u8]| -> Option<Vec<u8>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(b).ok()?;
        mac.update(value);
        Some(mac.finalize().into_bytes().to_vec())
    };
    let (Some(expected), Ok(mut mac)) = (tag(b), Hmac::<Sha256>::new_from_slice(b)) else {
        return false;
    };
    mac.update(a);
    mac.verify_slice(&expected).is_ok()
}

/// Verifies `X-Hub-Signature-256` (`sha256=<hex>`) over the raw body with
/// the app secret.
pub fn verify_signature(app_secret: &str, signature_header: Option<&str>, body: &[u8]) -> bool {
    let Some(hex_signature) = signature_header.and_then(|h| h.strip_prefix("sha256=")) else {
        return false;
    };
    let Ok(signature) = hex::decode(hex_signature) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&signature).is_ok()
}

fn timestamp(value: &Value) -> Option<DateTime<Utc>> {
    let seconds = match value {
        Value::String(s) => s.parse::<i64>().ok()?,
        Value::Number(n) => n.as_i64()?,
        _ => return None,
    };
    // Messenger sends milliseconds, WhatsApp seconds.
    if seconds > 10_000_000_000 {
        Utc.timestamp_millis_opt(seconds).single()
    } else {
        Utc.timestamp_opt(seconds, 0).single()
    }
}

fn whatsapp_status(status: &str) -> Option<MessageAttemptState> {
    match status {
        "sent" => Some(MessageAttemptState::ACCEPTED),
        "delivered" | "read" => Some(MessageAttemptState::DELIVERED),
        "failed" => Some(MessageAttemptState::FAILED),
        _ => None,
    }
}

/// Parses a verified payload. Entries for another business account,
/// number or Page are dropped: the account comes from the webhook URL, never
/// from the payload.
pub fn parse_events(account: &AccountSender, body: &[u8]) -> Result<Vec<WebhookEvent>> {
    let payload: Value = serde_json::from_slice(body)?;
    let object = payload["object"].as_str().unwrap_or_default();
    let entries = payload["entry"].as_array().cloned().unwrap_or_default();
    match (object, account) {
        (
            "whatsapp_business_account",
            AccountSender::WHATSAPP_CLOUD_API {
                business_account_id,
                phone_number_id,
                ..
            },
        ) => Ok(entries
            .iter()
            .filter(|entry| entry["id"].as_str() == Some(business_account_id.as_str()))
            .flat_map(|entry| entry["changes"].as_array().cloned().unwrap_or_default())
            .filter(|change| change["field"] == "messages")
            .filter(|change| {
                change["value"]["metadata"]["phone_number_id"].as_str()
                    == Some(phone_number_id.as_str())
            })
            .flat_map(|change| whatsapp_events(&change["value"]))
            .collect()),
        ("page", AccountSender::MESSENGER_SEND_API { page_id, .. }) => Ok(entries
            .iter()
            .filter(|entry| entry["id"].as_str() == Some(page_id.as_str()))
            .flat_map(|entry| entry["messaging"].as_array().cloned().unwrap_or_default())
            .filter(|event| event["recipient"]["id"].as_str() == Some(page_id.as_str()))
            .flat_map(|event| messenger_events(&event))
            .collect()),
        _ => Err(anyhow!("webhook payload does not belong to this account")),
    }
}

fn whatsapp_events(value: &Value) -> Vec<WebhookEvent> {
    let statuses = value["statuses"].as_array().cloned().unwrap_or_default();
    let messages = value["messages"].as_array().cloned().unwrap_or_default();
    let mut events: Vec<WebhookEvent> = statuses
        .iter()
        .filter_map(|status| {
            Some(WebhookEvent::Status(StatusReport {
                provider_message_id: status["id"].as_str()?.to_string(),
                state: whatsapp_status(status["status"].as_str()?)?,
                error_code: status["errors"][0]["code"]
                    .as_i64()
                    .map(|code| code.to_string()),
                billing: (!status["pricing"].is_null()).then(|| status["pricing"].clone()),
                at: timestamp(&status["timestamp"]),
            }))
        })
        .collect();
    events.extend(messages.iter().filter_map(|message| {
        Some(WebhookEvent::Inbound(InboundMessage {
            from: format!("+{}", message["from"].as_str()?),
            provider_message_id: message["id"].as_str()?.to_string(),
            has_text: message["type"] == "text",
            at: timestamp(&message["timestamp"]),
        }))
    }));
    events
}

fn messenger_events(event: &Value) -> Vec<WebhookEvent> {
    let Some(psid) = event["sender"]["id"].as_str() else {
        return vec![];
    };
    let at = timestamp(&event["timestamp"]);
    let mut events = vec![];
    let referral = event["referral"]["ref"]
        .as_str()
        .or_else(|| event["postback"]["referral"]["ref"].as_str());
    if let Some(reference) = referral {
        events.push(WebhookEvent::MessengerReferral(MessengerReferral {
            page_scoped_id: psid.to_string(),
            reference: reference.to_string(),
            linking_word: None,
            at,
        }));
    }
    if let Some(mids) = event["delivery"]["mids"].as_array() {
        events.extend(mids.iter().filter_map(|mid| {
            Some(WebhookEvent::Status(StatusReport {
                provider_message_id: mid.as_str()?.to_string(),
                state: MessageAttemptState::DELIVERED,
                error_code: None,
                billing: None,
                at,
            }))
        }));
    }
    let message = &event["message"];
    if !message.is_null() && message["is_echo"] != true {
        if let Some(mid) = message["mid"].as_str() {
            events.push(WebhookEvent::Inbound(InboundMessage {
                from: psid.to_string(),
                provider_message_id: mid.to_string(),
                has_text: message["text"].is_string(),
                at,
            }));
            if referral.is_none() {
                if let Some(text) = message["text"].as_str() {
                    events.push(WebhookEvent::MessengerReferral(MessengerReferral {
                        page_scoped_id: psid.to_string(),
                        reference: String::new(),
                        linking_word: Some(text.trim().to_string()),
                        at,
                    }));
                }
            }
        }
    }
    if !event["postback"].is_null() && referral.is_none() {
        events.push(WebhookEvent::Inbound(InboundMessage {
            from: psid.to_string(),
            provider_message_id: event["postback"]["mid"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            has_text: false,
            at,
        }));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn whatsapp_account() -> AccountSender {
        AccountSender::WHATSAPP_CLOUD_API {
            business_account_id: "waba-1".to_string(),
            phone_number_id: "pn-1".to_string(),
            display_phone_number: "+63 2 8123 4567".to_string(),
            display_name: Some("COMELEC".to_string()),
            api_version: "v23.0".to_string(),
        }
    }

    fn messenger_account() -> AccountSender {
        AccountSender::MESSENGER_SEND_API {
            page_id: "page-1".to_string(),
            page_name: Some("COMELEC".to_string()),
            page_username: Some("comelec".to_string()),
            api_version: "v23.0".to_string(),
        }
    }

    fn sign(secret: &str, body: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
    }

    #[test]
    fn the_subscription_handshake_needs_the_exact_token() {
        assert_eq!(
            verify_subscription(Some("subscribe"), Some("t0k"), Some("42"), "t0k"),
            Some("42".to_string())
        );
        assert_eq!(
            verify_subscription(Some("subscribe"), Some("other"), Some("42"), "t0k"),
            None
        );
        assert_eq!(
            verify_subscription(Some("unsubscribe"), Some("t0k"), Some("42"), "t0k"),
            None
        );
        assert_eq!(
            verify_subscription(Some("subscribe"), Some(""), Some("42"), ""),
            None
        );
    }

    #[test]
    fn signatures_are_checked_over_the_raw_body() {
        let body = br#"{"object":"page"}"#;
        assert!(verify_signature(
            "secret",
            Some(&sign("secret", body)),
            body
        ));
        assert!(!verify_signature(
            "secret",
            Some(&sign("other", body)),
            body
        ));
        assert!(!verify_signature(
            "secret",
            Some(&sign("secret", body)),
            br#"{"object":"page "}"#
        ));
        assert!(!verify_signature("secret", None, body));
        assert!(!verify_signature("secret", Some("sha1=abc"), body));
    }

    #[test]
    fn whatsapp_statuses_and_messages_are_parsed() {
        let body = json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "id": "waba-1",
                "changes": [{
                    "field": "messages",
                    "value": {
                        "metadata": {"phone_number_id": "pn-1"},
                        "statuses": [
                            {"id": "wamid.1", "status": "delivered", "timestamp": "1790000000",
                             "pricing": {"billable": true, "category": "authentication"}},
                            {"id": "wamid.2", "status": "failed", "timestamp": "1790000001",
                             "errors": [{"code": 131026}]},
                            {"id": "wamid.3", "status": "deleted", "timestamp": "1790000002"}
                        ],
                        "messages": [{"from": "639171234567", "id": "wamid.in", "type": "text",
                                      "timestamp": "1790000003", "text": {"body": "hello"}}]
                    }
                }]
            }]
        });
        let events = parse_events(&whatsapp_account(), body.to_string().as_bytes()).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(
            events[0],
            WebhookEvent::Status(StatusReport {
                provider_message_id: "wamid.1".to_string(),
                state: MessageAttemptState::DELIVERED,
                error_code: None,
                billing: Some(json!({"billable": true, "category": "authentication"})),
                at: Utc.timestamp_opt(1_790_000_000, 0).single(),
            })
        );
        assert!(matches!(
            &events[1],
            WebhookEvent::Status(StatusReport { state: MessageAttemptState::FAILED, error_code: Some(code), .. })
                if code == "131026"
        ));
        assert!(matches!(
            &events[2],
            WebhookEvent::Inbound(InboundMessage { from, has_text: true, .. }) if from == "+639171234567"
        ));
    }

    #[test]
    fn entries_of_other_accounts_are_ignored() {
        let body = json!({
            "object": "whatsapp_business_account",
            "entry": [{
                "id": "waba-1",
                "changes": [{"field": "messages", "value": {
                    "metadata": {"phone_number_id": "another-number"},
                    "statuses": [{"id": "wamid.1", "status": "delivered", "timestamp": "1"}]
                }}]
            }, {
                "id": "another-waba",
                "changes": [{"field": "messages", "value": {
                    "metadata": {"phone_number_id": "pn-1"},
                    "statuses": [{"id": "wamid.2", "status": "delivered", "timestamp": "1"}]
                }}]
            }]
        });
        assert!(
            parse_events(&whatsapp_account(), body.to_string().as_bytes())
                .unwrap()
                .is_empty()
        );
        assert!(parse_events(&messenger_account(), body.to_string().as_bytes()).is_err());
    }

    #[test]
    fn messenger_referrals_deliveries_and_linking_words_are_parsed() {
        let body = json!({
            "object": "page",
            "entry": [{
                "id": "page-1",
                "messaging": [
                    {"sender": {"id": "psid-1"}, "recipient": {"id": "page-1"}, "timestamp": 1790000000000_i64,
                     "postback": {"title": "Get Started", "payload": "GET_STARTED",
                                  "referral": {"ref": "ref-abc", "source": "SHORTLINK", "type": "OPEN_THREAD"}}},
                    {"sender": {"id": "psid-2"}, "recipient": {"id": "page-1"}, "timestamp": 1790000000001_i64,
                     "delivery": {"mids": ["m_1", "m_2"], "watermark": 1790000000000_i64}},
                    {"sender": {"id": "psid-3"}, "recipient": {"id": "page-1"}, "timestamp": 1790000000002_i64,
                     "message": {"mid": "m_in", "text": " VOTE-4821 "}},
                    {"sender": {"id": "page-1"}, "recipient": {"id": "page-1"}, "timestamp": 1790000000003_i64,
                     "message": {"mid": "m_echo", "text": "echo", "is_echo": true}},
                    {"sender": {"id": "psid-4"}, "recipient": {"id": "another-page"}, "timestamp": 1790000000004_i64,
                     "referral": {"ref": "ref-other"}}
                ]
            }]
        });
        let events = parse_events(&messenger_account(), body.to_string().as_bytes()).unwrap();
        assert_eq!(
            events[0],
            WebhookEvent::MessengerReferral(MessengerReferral {
                page_scoped_id: "psid-1".to_string(),
                reference: "ref-abc".to_string(),
                linking_word: None,
                at: Utc.timestamp_millis_opt(1_790_000_000_000).single(),
            })
        );
        assert!(matches!(&events[1], WebhookEvent::Status(s) if s.provider_message_id == "m_1"));
        assert!(matches!(&events[2], WebhookEvent::Status(s) if s.provider_message_id == "m_2"));
        assert!(matches!(&events[3], WebhookEvent::Inbound(m) if m.from == "psid-3"));
        assert!(matches!(
            &events[4],
            WebhookEvent::MessengerReferral(MessengerReferral { linking_word: Some(word), .. })
                if word == "VOTE-4821"
        ));
        assert_eq!(events.len(), 5);
    }
}
