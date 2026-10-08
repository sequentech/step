// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Callbacks of providers described by configuration: how they are
//! authenticated and how delivery reports and replies are read.

use super::{InboundMessage, StatusReport, WebhookEvent};
use crate::providers::http_api::{mapped_state, pointer_text, Placeholders};
use hmac::{Hmac, Mac};
use openssl::base64::decode_block;
use sequent_core::types::messaging::{DigestEncoding, HttpReports, HttpWebhookAuth};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn base64_url_decode(value: &str) -> Option<Vec<u8>> {
    let mut standard = value.replace('-', "+").replace('_', "/");
    while standard.len() % 4 != 0 {
        standard.push('=');
    }
    decode_block(&standard).ok()
}

fn hmac_matches(secret: &str, signed: &[u8], signature: &[u8]) -> bool {
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(signed);
    mac.verify_slice(signature).is_ok()
}

/// Whether a callback passes the account's configured authentication.
/// `headers` have lowercase names. `URL_KEY` accounts rely on the
/// unguessable webhook URL alone.
pub fn verify(
    auth: &HttpWebhookAuth,
    secret: Option<&str>,
    headers: &BTreeMap<String, String>,
    body: &[u8],
) -> bool {
    let header = |name: &str| headers.get(&name.to_lowercase()).map(String::as_str);
    match auth {
        HttpWebhookAuth::URL_KEY => true,
        HttpWebhookAuth::HEADER_SECRET { header: name } => {
            match (secret.filter(|s| !s.is_empty()), header(name)) {
                // Comparing MACs keeps the comparison constant time.
                (Some(secret), Some(given)) => {
                    let mut mac = match Hmac::<Sha256>::new_from_slice(secret.as_bytes()) {
                        Ok(mac) => mac,
                        Err(_) => return false,
                    };
                    mac.update(secret.as_bytes());
                    let expected = mac.finalize().into_bytes();
                    hmac_matches(secret, given.as_bytes(), &expected)
                }
                _ => false,
            }
        }
        HttpWebhookAuth::HMAC_SHA256 {
            header: name,
            prefix,
            encoding,
            signed,
        } => {
            let (Some(secret), Some(given)) = (secret.filter(|s| !s.is_empty()), header(name))
            else {
                return false;
            };
            let digest = match prefix {
                Some(prefix) => match given.strip_prefix(prefix.as_str()) {
                    Some(digest) => digest,
                    None => return false,
                },
                None => given,
            };
            let signature = match encoding {
                DigestEncoding::HEX => hex::decode(digest.trim()).ok(),
                DigestEncoding::BASE64 => decode_block(digest.trim()).ok(),
            };
            let Some(signature) = signature else {
                return false;
            };
            let signed_bytes = match signed {
                None => body.to_vec(),
                Some(template) => {
                    let mut placeholders = Placeholders::default();
                    placeholders.set("body", String::from_utf8_lossy(body).to_string());
                    for (name, value) in headers {
                        placeholders.set(&format!("header.{name}"), value.clone());
                    }
                    placeholders.render(template).into_bytes()
                }
            };
            hmac_matches(secret, &signed_bytes, &signature)
        }
        HttpWebhookAuth::JWT_HS256 { header: name } => {
            let (Some(secret), Some(given)) = (secret.filter(|s| !s.is_empty()), header(name))
            else {
                return false;
            };
            let token = given.strip_prefix("Bearer ").unwrap_or(given).trim();
            let mut parts = token.split('.');
            let (Some(head), Some(claims), Some(signature), None) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                return false;
            };
            let algorithm = base64_url_decode(head)
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .and_then(|header| header["alg"].as_str().map(str::to_string));
            if algorithm.as_deref() != Some("HS256") {
                return false;
            }
            let Some(signature) = base64_url_decode(signature) else {
                return false;
            };
            if !hmac_matches(secret, format!("{head}.{claims}").as_bytes(), &signature) {
                return false;
            }
            let Some(claims) = base64_url_decode(claims)
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            else {
                return false;
            };
            if claims["exp"]
                .as_i64()
                .is_some_and(|exp| exp < chrono::Utc::now().timestamp())
            {
                return false;
            }
            match claims["payload_hash"].as_str() {
                Some(hash) => hash.eq_ignore_ascii_case(&hex::encode(Sha256::digest(body))),
                None => true,
            }
        }
    }
}

/// A GET callback's query parameters as the flat object the report
/// pointers address.
pub fn query_as_json(query: &BTreeMap<String, String>) -> Value {
    Value::Object(
        query
            .iter()
            .map(|(name, value)| (name.clone(), Value::String(value.clone())))
            .collect(),
    )
}

/// Reads delivery reports and replies from a callback payload.
pub fn parse_events(reports: &HttpReports, payload: &Value) -> Vec<WebhookEvent> {
    let root = match &reports.items_pointer {
        Some(pointer) => match payload.pointer(pointer) {
            Some(items) => items,
            None => return vec![],
        },
        None => payload,
    };
    let items: Vec<&Value> = match root {
        Value::Array(items) => items.iter().collect(),
        item => vec![item],
    };
    items
        .into_iter()
        .filter_map(|item| {
            let message_id = pointer_text(item, &reports.status.message_id_pointer);
            if let (Some(id), Some(state)) = (&message_id, mapped_state(&reports.status, item)) {
                return Some(WebhookEvent::Status(StatusReport {
                    provider_message_id: id.clone(),
                    state,
                    error_code: reports
                        .status
                        .error_pointer
                        .as_deref()
                        .and_then(|pointer| pointer_text(item, pointer)),
                    billing: None,
                    at: None,
                }));
            }
            let from = pointer_text(item, reports.inbound_from_pointer.as_deref()?)?;
            Some(WebhookEvent::Inbound(InboundMessage {
                from: if from.chars().all(|c| c.is_ascii_digit()) && from.len() >= 8 {
                    format!("+{from}")
                } else {
                    from
                },
                provider_message_id: message_id.unwrap_or_default(),
                has_text: true,
                at: None,
            }))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::base64::encode_block;
    use sequent_core::types::messaging::MessageAttemptState;
    use serde_json::json;

    fn headers(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(name, value)| (name.to_lowercase(), value.to_string()))
            .collect()
    }

    fn sign(secret: &str, data: &[u8]) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    fn base64_url(bytes: &[u8]) -> String {
        encode_block(bytes)
            .replace('+', "-")
            .replace('/', "_")
            .trim_end_matches('=')
            .to_string()
    }

    #[test]
    fn header_secrets_must_match_exactly() {
        let auth = HttpWebhookAuth::HEADER_SECRET {
            header: "X-Secret".to_string(),
        };
        assert!(verify(
            &auth,
            Some("s3cret"),
            &headers(&[("X-Secret", "s3cret")]),
            b"{}"
        ));
        assert!(!verify(
            &auth,
            Some("s3cret"),
            &headers(&[("X-Secret", "other")]),
            b"{}"
        ));
        assert!(!verify(&auth, Some("s3cret"), &headers(&[]), b"{}"));
        assert!(!verify(&auth, None, &headers(&[("X-Secret", "")]), b"{}"));
        assert!(!verify(
            &auth,
            Some(""),
            &headers(&[("X-Secret", "")]),
            b"{}"
        ));
        assert!(verify(
            &HttpWebhookAuth::URL_KEY,
            None,
            &headers(&[]),
            b"{}"
        ));
    }

    #[test]
    fn hmac_signatures_cover_the_body_or_a_composed_string() {
        let body = br#"{"status":"delivered"}"#;
        let plain = HttpWebhookAuth::HMAC_SHA256 {
            header: "X-Signature".to_string(),
            prefix: Some("sha256=".to_string()),
            encoding: DigestEncoding::HEX,
            signed: None,
        };
        let digest = format!("sha256={}", hex::encode(sign("k", body)));
        assert!(verify(
            &plain,
            Some("k"),
            &headers(&[("X-Signature", &digest)]),
            body
        ));
        assert!(!verify(
            &plain,
            Some("k"),
            &headers(&[("X-Signature", &digest)]),
            b"{}"
        ));
        assert!(!verify(
            &plain,
            Some("other"),
            &headers(&[("X-Signature", &digest)]),
            body
        ));

        let composed = HttpWebhookAuth::HMAC_SHA256 {
            header: "x-sig".to_string(),
            prefix: None,
            encoding: DigestEncoding::BASE64,
            signed: Some("{{body}}.{{header.x-nonce}}.{{header.x-timestamp}}".to_string()),
        };
        let signed = [body.as_slice(), b".n1.1790000000"].concat();
        let digest = encode_block(&sign("k", &signed));
        let ok = headers(&[
            ("x-sig", &digest),
            ("X-Nonce", "n1"),
            ("X-Timestamp", "1790000000"),
        ]);
        assert!(verify(&composed, Some("k"), &ok, body));
        let replayed = headers(&[
            ("x-sig", &digest),
            ("X-Nonce", "n2"),
            ("X-Timestamp", "1790000000"),
        ]);
        assert!(!verify(&composed, Some("k"), &replayed, body));
    }

    #[test]
    fn signed_jwts_are_checked_with_their_payload_hash() {
        let body = br#"{"status":"delivered"}"#;
        let auth = HttpWebhookAuth::JWT_HS256 {
            header: "Authorization".to_string(),
        };
        let token = |claims: Value, secret: &str| {
            let input = format!(
                "{}.{}",
                base64_url(br#"{"alg":"HS256","typ":"JWT"}"#),
                base64_url(claims.to_string().as_bytes())
            );
            format!(
                "Bearer {input}.{}",
                base64_url(&sign(secret, input.as_bytes()))
            )
        };
        let hash = hex::encode(Sha256::digest(body));
        let good = token(json!({"payload_hash": hash}), "k");
        assert!(verify(
            &auth,
            Some("k"),
            &headers(&[("Authorization", &good)]),
            body
        ));
        assert!(!verify(
            &auth,
            Some("k"),
            &headers(&[("Authorization", &good)]),
            b"{}"
        ));
        let forged = token(json!({"payload_hash": hash}), "other");
        assert!(!verify(
            &auth,
            Some("k"),
            &headers(&[("Authorization", &forged)]),
            body
        ));
        let expired = token(json!({"exp": 1}), "k");
        assert!(!verify(
            &auth,
            Some("k"),
            &headers(&[("Authorization", &expired)]),
            body
        ));
        let unsigned = format!(
            "Bearer {}.{}.",
            base64_url(br#"{"alg":"none"}"#),
            base64_url(b"{}")
        );
        assert!(!verify(
            &auth,
            Some("k"),
            &headers(&[("Authorization", &unsigned)]),
            body
        ));
    }

    fn reports(extra: Value) -> HttpReports {
        let mut value = json!({
            "status": {
                "message_id_pointer": "/payload/umid",
                "state_pointer": "/payload/status/state",
                "states": {"delivered": "DELIVERED", "undelivered": "FAILED", "sent": "ACCEPTED"},
                "error_pointer": "/payload/status/detail"
            }
        });
        for (key, extra_value) in extra.as_object().cloned().unwrap_or_default() {
            value[key] = extra_value;
        }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn nested_and_listed_reports_are_read() {
        let single = json!({"payload": {"umid": "u-1", "status": {"state": "Delivered"}}});
        assert!(matches!(
            parse_events(&reports(json!({})), &single).as_slice(),
            [WebhookEvent::Status(StatusReport { state: MessageAttemptState::DELIVERED, provider_message_id, .. })]
                if provider_message_id == "u-1"
        ));
        let listed = json!({"results": [
            {"payload": {"umid": "u-2", "status": {"state": "undelivered", "detail": "blocked"}}},
            {"payload": {"umid": "u-3", "status": {"state": "seen"}}},
        ]});
        let events = parse_events(&reports(json!({"items_pointer": "/results"})), &listed);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            WebhookEvent::Status(StatusReport { state: MessageAttemptState::FAILED, error_code: Some(code), .. })
                if code == "blocked"
        ));
    }

    #[test]
    fn query_string_reports_and_numeric_statuses_are_read() {
        let query = BTreeMap::from([
            ("transid".to_string(), "t-9".to_string()),
            ("status_code".to_string(), "1".to_string()),
        ]);
        let reports: HttpReports = serde_json::from_value(json!({
            "status": {"message_id_pointer": "/transid", "state_pointer": "/status_code",
                       "states": {"1": "DELIVERED", "2": "FAILED", "16": "FAILED"}}
        }))
        .unwrap();
        assert!(matches!(
            parse_events(&reports, &query_as_json(&query)).as_slice(),
            [WebhookEvent::Status(StatusReport { state: MessageAttemptState::DELIVERED, provider_message_id, .. })]
                if provider_message_id == "t-9"
        ));
    }

    #[test]
    fn replies_are_recorded_as_inbound_messages() {
        let reports = reports(json!({"inbound_from_pointer": "/payload/user/msisdn"}));
        let inbound = json!({"payload": {"umid": "in-1", "user": {"msisdn": "639171234567"}}});
        assert!(matches!(
            parse_events(&reports, &inbound).as_slice(),
            [WebhookEvent::Inbound(InboundMessage { from, .. })] if from == "+639171234567"
        ));
    }
}
