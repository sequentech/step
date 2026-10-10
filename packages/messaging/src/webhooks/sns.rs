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

/// How long fetching an SNS signing certificate (or confirming a
/// subscription) may take: well inside the webhook component's 10 s.
pub const SNS_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Certificates remembered at once; SNS rotates them rarely.
const CACHED_CERTIFICATES: usize = 64;

/// SNS signing certificates by URL. Each URL names one certificate
/// (`SimpleNotificationService-<id>.pem`), so it is fetched once; a failed
/// fetch isn't remembered.
pub struct CertificateCache {
    http: reqwest::Client,
    certificates: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

impl CertificateCache {
    /// A cache whose fetches give up after `timeout`.
    pub fn new(timeout: std::time::Duration) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(timeout)
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_default(),
            certificates: Default::default(),
        }
    }

    /// The process's cache, with [`SNS_FETCH_TIMEOUT`].
    pub fn global() -> &'static Self {
        static CACHE: std::sync::OnceLock<CertificateCache> = std::sync::OnceLock::new();
        CACHE.get_or_init(|| CertificateCache::new(SNS_FETCH_TIMEOUT))
    }

    /// The certificate at `url`, fetched on first use.
    ///
    /// # Errors
    /// When it can't be fetched in time.
    pub async fn get(&self, url: &str) -> Result<Vec<u8>> {
        if let Some(certificate) = self
            .certificates
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(url)
        {
            return Ok(certificate.clone());
        }
        let certificate = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?
            .to_vec();
        let mut certificates = self.certificates.lock().unwrap_or_else(|e| e.into_inner());
        if certificates.len() >= CACHED_CERTIFICATES {
            certificates.clear();
        }
        certificates.insert(url.to_string(), certificate.clone());
        Ok(certificate)
    }

    /// Visits an SNS subscription URL, with the same timeout.
    ///
    /// # Errors
    /// When SNS doesn't answer with success in time.
    pub async fn confirm_subscription(&self, url: &str) -> Result<()> {
        self.http.get(url).send().await?.error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression (R1-18): every SES event fetched the certificate again,
    /// with no timeout, so a slow SNS host held harvest past the
    /// component's timeout.
    #[tokio::test]
    async fn certificates_are_fetched_once_per_url_and_a_slow_host_times_out() {
        use crate::test_server::{Reply, TestServer};
        let server = TestServer::start(vec![
            Reply::Raw(500, "text/plain", String::new()),
            Reply::Raw(200, "text/plain", "certificate".into()),
        ])
        .await;
        let cache = CertificateCache::new(std::time::Duration::from_millis(300));
        let url = format!("{}/SimpleNotificationService-1.pem", server.base_url);
        // A failed fetch isn't remembered.
        assert!(cache.get(&url).await.is_err());
        assert_eq!(cache.get(&url).await.unwrap(), b"certificate".to_vec());
        assert_eq!(cache.get(&url).await.unwrap(), b"certificate".to_vec());
        assert_eq!(server.requests().len(), 2);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let slow = format!("http://{}/slow.pem", listener.local_addr().unwrap());
        let held = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            drop(stream);
        });
        let started = std::time::Instant::now();
        assert!(cache.get(&slow).await.is_err());
        assert!(started.elapsed() < std::time::Duration::from_secs(3));
        held.abort();
        assert_eq!(SNS_FETCH_TIMEOUT, std::time::Duration::from_secs(5));
    }
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
