// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Amazon SNS notifications carrying SES delivery events. The signature is
//! checked with the certificate SNS names, and only certificates served by
//! SNS itself are trusted.

use super::{StatusReport, WebhookEvent};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use openssl::base64::decode_block;
use openssl::hash::MessageDigest;
use openssl::sign::Verifier;
use openssl::x509::X509;
use sequent_core::types::messaging::MessageAttemptState;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SnsEnvelope {
    #[serde(rename = "Type")]
    pub kind: String,
    pub message_id: String,
    pub topic_arn: String,
    pub subject: Option<String>,
    pub message: String,
    pub timestamp: String,
    pub signature_version: String,
    pub signature: String,
    #[serde(rename = "SigningCertURL")]
    pub signing_cert_url: String,
    pub token: Option<String>,
    #[serde(rename = "SubscribeURL")]
    pub subscribe_url: Option<String>,
}

impl SnsEnvelope {
    pub fn parse(body: &[u8]) -> Result<SnsEnvelope> {
        serde_json::from_slice(body).context("invalid SNS envelope")
    }

    fn string_to_sign(&self) -> Result<String> {
        let mut fields: Vec<(&str, &str)> =
            vec![("Message", &self.message), ("MessageId", &self.message_id)];
        match self.kind.as_str() {
            "Notification" => {
                if let Some(subject) = &self.subject {
                    fields.push(("Subject", subject));
                }
            }
            "SubscriptionConfirmation" | "UnsubscribeConfirmation" => {
                fields.push((
                    "SubscribeURL",
                    self.subscribe_url
                        .as_deref()
                        .ok_or_else(|| anyhow!("missing SubscribeURL"))?,
                ));
            }
            other => return Err(anyhow!("unsupported SNS message type {other}")),
        }
        fields.push(("Timestamp", &self.timestamp));
        if self.kind != "Notification" {
            fields.push((
                "Token",
                self.token
                    .as_deref()
                    .ok_or_else(|| anyhow!("missing Token"))?,
            ));
        }
        fields.push(("TopicArn", &self.topic_arn));
        fields.push(("Type", &self.kind));
        Ok(fields
            .iter()
            .map(|(name, value)| format!("{name}\n{value}\n"))
            .collect())
    }

    /// Checks the signature against `certificate_pem`, which the caller
    /// fetched from [`SnsEnvelope::signing_cert_url`] after
    /// [`is_trusted_certificate_url`] accepted it.
    pub fn verify(&self, certificate_pem: &[u8]) -> Result<()> {
        let digest = match self.signature_version.as_str() {
            "1" => MessageDigest::sha1(),
            "2" => MessageDigest::sha256(),
            other => return Err(anyhow!("unsupported SNS signature version {other}")),
        };
        let certificate = X509::from_pem(certificate_pem)?;
        let key = certificate.public_key()?;
        let signature = decode_block(&self.signature)?;
        let mut verifier = Verifier::new(digest, &key)?;
        verifier.update(self.string_to_sign()?.as_bytes())?;
        if verifier.verify(&signature)? {
            Ok(())
        } else {
            Err(anyhow!("invalid SNS signature"))
        }
    }
}

/// Only HTTPS certificates served by an SNS regional endpoint are trusted.
pub fn is_trusted_certificate_url(url: &str) -> bool {
    sns_path(url).is_some_and(|path| path.ends_with(".pem"))
}

/// Whether `url` is an HTTPS URL of an SNS regional endpoint, such as a
/// subscription confirmation link.
pub fn is_sns_url(url: &str) -> bool {
    sns_path(url).is_some()
}

/// The path of an HTTPS URL on an SNS regional endpoint.
fn sns_path(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let (host, path) = rest.split_once('/')?;
    let region = host.strip_prefix("sns.").and_then(|h| {
        h.strip_suffix(".amazonaws.com")
            .or_else(|| h.strip_suffix(".amazonaws.com.cn"))
    });
    let valid_region = region
        .map(|r| {
            !r.is_empty()
                && r.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .unwrap_or(false);
    (valid_region && !path.contains("..") && !host.contains('@')).then_some(path)
}

/// Delivery events of an SES notification. Transient bounces are left
/// alone: SES keeps retrying them.
pub fn ses_events(envelope: &SnsEnvelope) -> Result<Vec<WebhookEvent>> {
    let event: Value = serde_json::from_str(&envelope.message)?;
    let kind = event["eventType"]
        .as_str()
        .or_else(|| event["notificationType"].as_str())
        .unwrap_or_default();
    let Some(message_id) = event["mail"]["messageId"].as_str() else {
        return Ok(vec![]);
    };
    let state = match kind {
        "Delivery" => Some(MessageAttemptState::DELIVERED),
        "Bounce" if event["bounce"]["bounceType"] == "Permanent" => {
            Some(MessageAttemptState::FAILED)
        }
        "Reject" => Some(MessageAttemptState::FAILED),
        _ => None,
    };
    Ok(state
        .map(|state| {
            vec![WebhookEvent::Status(StatusReport {
                provider_message_id: message_id.to_string(),
                state,
                error_code: event["bounce"]["bounceSubType"]
                    .as_str()
                    .map(str::to_string),
                billing: None,
                at: DateTime::parse_from_rfc3339(&envelope.timestamp)
                    .ok()
                    .map(|d| d.with_timezone(&Utc)),
            })]
        })
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::asn1::Asn1Time;
    use openssl::base64::encode_block;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::sign::Signer;
    use openssl::x509::X509Name;
    use serde_json::json;

    fn certificate() -> (PKey<openssl::pkey::Private>, Vec<u8>) {
        let key = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();
        let mut name = X509Name::builder().unwrap();
        name.append_entry_by_text("CN", "sns.amazonaws.com")
            .unwrap();
        let name = name.build();
        let mut builder = X509::builder().unwrap();
        builder.set_subject_name(&name).unwrap();
        builder.set_issuer_name(&name).unwrap();
        builder.set_pubkey(&key).unwrap();
        builder
            .set_not_before(&Asn1Time::days_from_now(0).unwrap())
            .unwrap();
        builder
            .set_not_after(&Asn1Time::days_from_now(1).unwrap())
            .unwrap();
        builder.sign(&key, MessageDigest::sha256()).unwrap();
        (key, builder.build().to_pem().unwrap())
    }

    fn envelope(message: Value, version: &str, key: &PKey<openssl::pkey::Private>) -> SnsEnvelope {
        let mut envelope = SnsEnvelope {
            kind: "Notification".to_string(),
            message_id: "msg-1".to_string(),
            topic_arn: "arn:aws:sns:eu-west-1:123:ses-events".to_string(),
            subject: None,
            message: message.to_string(),
            timestamp: "2026-10-02T10:00:00.000Z".to_string(),
            signature_version: version.to_string(),
            signature: String::new(),
            signing_cert_url:
                "https://sns.eu-west-1.amazonaws.com/SimpleNotificationService-abc.pem".to_string(),
            token: None,
            subscribe_url: None,
        };
        let digest = if version == "1" {
            MessageDigest::sha1()
        } else {
            MessageDigest::sha256()
        };
        let mut signer = Signer::new(digest, key).unwrap();
        signer
            .update(envelope.string_to_sign().unwrap().as_bytes())
            .unwrap();
        envelope.signature = encode_block(&signer.sign_to_vec().unwrap());
        envelope
    }

    #[test]
    fn signatures_of_both_versions_verify_and_tampering_fails() {
        let (key, pem) = certificate();
        for version in ["1", "2"] {
            let mut envelope = envelope(json!({"eventType": "Delivery"}), version, &key);
            assert!(envelope.verify(&pem).is_ok());
            envelope.message = json!({"eventType": "Bounce"}).to_string();
            assert!(envelope.verify(&pem).is_err());
        }
        let (other_key, _) = certificate();
        let forged = envelope(json!({}), "2", &other_key);
        assert!(forged.verify(&pem).is_err());
    }

    #[test]
    fn only_sns_certificate_urls_are_trusted() {
        assert!(is_trusted_certificate_url(
            "https://sns.eu-west-1.amazonaws.com/SimpleNotificationService-abc.pem"
        ));
        assert!(is_trusted_certificate_url(
            "https://sns.cn-north-1.amazonaws.com.cn/SimpleNotificationService-abc.pem"
        ));
        for url in [
            "http://sns.eu-west-1.amazonaws.com/a.pem",
            "https://sns.eu-west-1.amazonaws.com.evil.example/a.pem",
            "https://evil.example/sns.eu-west-1.amazonaws.com/a.pem",
            "https://sns.eu-west-1.amazonaws.com/a.txt",
            "https://user@sns.eu-west-1.amazonaws.com/a.pem",
            "https://sns..amazonaws.com/a.pem",
        ] {
            assert!(!is_trusted_certificate_url(url), "{url}");
        }
        assert!(is_sns_url(
            "https://sns.eu-west-1.amazonaws.com/?Action=ConfirmSubscription&Token=t"
        ));
        assert!(!is_sns_url(
            "https://sns.evil.example/?Action=ConfirmSubscription"
        ));
    }

    #[test]
    fn ses_events_map_to_attempt_states() {
        let (key, _) = certificate();
        let events = |message: Value| ses_events(&envelope(message, "2", &key)).unwrap();
        assert!(matches!(
            events(json!({"eventType": "Delivery", "mail": {"messageId": "ses-1"}})).as_slice(),
            [WebhookEvent::Status(StatusReport {
                state: MessageAttemptState::DELIVERED,
                ..
            })]
        ));
        assert!(matches!(
            events(json!({"notificationType": "Bounce", "mail": {"messageId": "ses-1"},
                          "bounce": {"bounceType": "Permanent", "bounceSubType": "NoEmail"}})).as_slice(),
            [WebhookEvent::Status(StatusReport { state: MessageAttemptState::FAILED, error_code: Some(code), .. })]
                if code == "NoEmail"
        ));
        assert!(events(
            json!({"eventType": "Bounce", "mail": {"messageId": "ses-1"},
                              "bounce": {"bounceType": "Transient"}})
        )
        .is_empty());
    }
}
