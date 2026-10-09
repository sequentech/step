// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The hop between the public webhook component and harvest.
//!
//! The component (`messaging-webhooks`) is the only public entrypoint for
//! provider callbacks. It holds no secrets and verifies nothing: it wraps
//! each callback in a [`CallbackEnvelope`], with the raw body base64-encoded
//! so the provider's signature can be checked on the exact bytes, signs the
//! envelope with the relay secret and posts it to harvest's internal
//! `POST /messaging/callbacks`. Harvest checks the relay signature, runs the
//! provider's own verification and answers a [`RelayResponse`], which the
//! component returns to the provider as is.

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::BTreeMap;
use strum_macros::{Display, EnumString};

/// Header carrying the relay signature, `t=<unix>,v1=<hex>`.
pub const RELAY_SIGNATURE_HEADER: &str = "X-Messaging-Relay-Signature";

/// Shared secret of the component and harvest. Nothing else reads it.
pub const RELAY_SECRET_ENV: &str = "MESSAGING_RELAY_SECRET";

/// Harvest's internal route for relayed callbacks.
pub const CALLBACKS_PATH: &str = "/messaging/callbacks";

/// A relayed callback older or newer than this is refused.
pub const RELAY_MAX_SKEW_SECONDS: i64 = 300;

/// Version of [`CallbackEnvelope`] this code writes and reads.
pub const ENVELOPE_VERSION: u8 = 1;

/// Largest provider body the component takes, in bytes. Read by the
/// component, and by harvest to size the envelopes it accepts.
pub const MAX_BODY_BYTES_ENV: &str = "MESSAGING_WEBHOOKS_MAX_BODY_BYTES";

/// [`MAX_BODY_BYTES_ENV`] when it is unset: 256 KiB.
pub const DEFAULT_MAX_BODY_BYTES: u64 = 262_144;

/// Public base URL of the component, where providers send callbacks. Set on
/// the services that show or send callback URLs (windmill, harvest), never
/// needed by the component itself.
pub const WEBHOOKS_PUBLIC_URL_ENV: &str = "MESSAGING_WEBHOOKS_PUBLIC_URL";

/// Room for the envelope's JSON fields, the query and the forwarded headers
/// on top of the base64 body.
const ENVELOPE_OVERHEAD_BYTES: u64 = 64 * 1024;

/// [`MAX_BODY_BYTES_ENV`] read from its raw value: the default when unset or
/// blank.
///
/// # Errors
/// When the value is not a positive whole number of bytes.
pub fn max_body_bytes(value: Option<&str>) -> Result<u64> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(DEFAULT_MAX_BODY_BYTES),
        Some(value) => match value.parse::<u64>() {
            Ok(bytes) if bytes > 0 => Ok(bytes),
            _ => anyhow::bail!("{MAX_BODY_BYTES_ENV} must be a positive number of bytes"),
        },
    }
}

/// The largest envelope harvest reads for provider bodies of up to
/// `max_body_bytes`: base64 grows the body by 4/3, so twice the body plus
/// room for the other fields.
pub fn envelope_limit(max_body_bytes: u64) -> u64 {
    max_body_bytes
        .saturating_mul(2)
        .saturating_add(ENVELOPE_OVERHEAD_BYTES)
}

/// The only request headers the component forwards, lowercased. Viber
/// reports authenticate with `authorization` (Basic, from the Infobip
/// subscription); the others carry provider signatures and SNS metadata.
pub const FORWARDED_HEADERS: &[&str] = &[
    "content-type",
    "x-hub-signature-256",
    "x-amz-sns-message-type",
    "x-amz-sns-topic-arn",
    "x-amz-sns-subscription-arn",
    "authorization",
];

/// The provider family a public callback route belongs to.
#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, EnumString)]
pub enum CallbackProvider {
    /// WhatsApp Cloud and Messenger (`/webhooks/meta/{key}`).
    META,
    /// Viber through Infobip (`/webhooks/viber/{key}`).
    VIBER,
    /// SNS-wrapped SES events and SNS subscription confirmation
    /// (`/webhooks/aws/{key}`).
    AWS,
}

/// HTTP method of the relayed callback.
#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, EnumString)]
pub enum CallbackMethod {
    /// Only Meta's subscription handshake.
    GET,
    /// Every report and event.
    POST,
}

impl CallbackProvider {
    /// The provider's segment in the component's routes,
    /// `/webhooks/{segment}/{key}`.
    pub fn path_segment(self) -> &'static str {
        match self {
            Self::META => "meta",
            Self::VIBER => "viber",
            Self::AWS => "aws",
        }
    }

    /// The callback URL an administrator enters at the provider:
    /// `{component_url}/webhooks/{segment}/{key}`, where `component_url` is
    /// the component's public base URL ([`WEBHOOKS_PUBLIC_URL_ENV`]).
    pub fn callback_url(self, component_url: &str, key: &str) -> String {
        format!(
            "{}/webhooks/{}/{key}",
            component_url.trim_end_matches('/'),
            self.path_segment()
        )
    }
}

/// One provider callback, as the component received it.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CallbackEnvelope {
    /// Always [`ENVELOPE_VERSION`].
    pub version: u8,
    /// From the route.
    pub provider: CallbackProvider,
    /// The account's webhook key, from the route. The account is resolved
    /// from this only, never from the payload.
    pub key: String,
    /// From the request.
    pub method: CallbackMethod,
    /// The raw query string, without the `?`. Empty when there is none.
    pub query: String,
    /// Only [`FORWARDED_HEADERS`], with lowercase names.
    pub headers: BTreeMap<String, String>,
    /// The raw body, base64-encoded so it reaches harvest byte for byte.
    pub body_b64: String,
    /// When the component received it.
    pub received_at: DateTime<Utc>,
}

impl CallbackEnvelope {
    /// Wraps a received callback. Headers outside [`FORWARDED_HEADERS`] are
    /// dropped, and kept names are lowercased.
    pub fn new<'a>(
        provider: CallbackProvider,
        key: &str,
        method: CallbackMethod,
        query: &str,
        headers: impl IntoIterator<Item = (&'a str, &'a str)>,
        body: &[u8],
        received_at: DateTime<Utc>,
    ) -> Self {
        let headers = headers
            .into_iter()
            .map(|(name, value)| (name.to_ascii_lowercase(), value.to_string()))
            .filter(|(name, _)| FORWARDED_HEADERS.contains(&name.as_str()))
            .collect();
        Self {
            version: ENVELOPE_VERSION,
            provider,
            key: key.to_string(),
            method,
            query: query.to_string(),
            headers,
            body_b64: BASE64.encode(body),
            received_at,
        }
    }

    /// The original body bytes.
    ///
    /// # Errors
    /// When `body_b64` isn't valid base64.
    pub fn body(&self) -> Result<Vec<u8>> {
        BASE64
            .decode(&self.body_b64)
            .context("callback body is not valid base64")
    }

    /// A forwarded header, by its lowercase name.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

/// What the component answers the provider: harvest decides it.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct RelayResponse {
    /// HTTP status for the provider.
    pub status: u16,
    /// Content type of `body`.
    pub content_type: String,
    /// Response body; Meta's handshake needs the `hub.challenge` as plain
    /// text.
    pub body: String,
}

impl RelayResponse {
    /// A plain-text answer.
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: body.into(),
        }
    }
}

/// Whether `key` has the shape of an account webhook key: 64 lowercase hex
/// characters ([`crate::link::new_reference`]). Anything else is answered
/// 404 by the component and never forwarded.
pub fn is_webhook_key(key: &str) -> bool {
    key.len() == 64 && key.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Why a relayed callback was refused. Harvest answers 401 for each.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Display)]
pub enum RelayAuthError {
    /// No [`RELAY_SIGNATURE_HEADER`].
    MissingSignature,
    /// The header isn't `t=<unix>,v1=<hex>`.
    MalformedSignature,
    /// `t` is more than [`RELAY_MAX_SKEW_SECONDS`] away from now.
    StaleSignature,
    /// The HMAC doesn't match.
    BadSignature,
}

impl std::error::Error for RelayAuthError {}

fn relay_mac(secret: &[u8], timestamp: i64, body: &[u8]) -> Hmac<Sha256> {
    #[allow(clippy::expect_used)]
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    mac
}

/// The [`RELAY_SIGNATURE_HEADER`] value for a request body sent at
/// `timestamp` (unix seconds).
pub fn sign(secret: &[u8], timestamp: i64, body: &[u8]) -> String {
    let digest = relay_mac(secret, timestamp, body).finalize().into_bytes();
    format!("t={timestamp},v1={}", hex::encode(digest))
}

/// Checks a [`RELAY_SIGNATURE_HEADER`] value against the request body, in
/// constant time.
///
/// # Errors
/// The [`RelayAuthError`] that refuses the request.
pub fn verify(
    secret: &[u8],
    header: Option<&str>,
    body: &[u8],
    now: i64,
) -> Result<(), RelayAuthError> {
    let (timestamp, signature) = fresh_signature(header, now)?;
    relay_mac(secret, timestamp, body)
        .verify_slice(&signature)
        .map_err(|_| RelayAuthError::BadSignature)
}

/// The checks of [`verify`] that need neither the secret nor the body: the
/// header is present, well formed and fresh. Lets harvest refuse a request
/// before reading its body; [`verify`] still has the final word.
///
/// # Errors
/// [`RelayAuthError::MissingSignature`], `MalformedSignature` or
/// `StaleSignature`.
pub fn check_header(header: Option<&str>, now: i64) -> Result<(), RelayAuthError> {
    fresh_signature(header, now).map(|_| ())
}

/// The timestamp and MAC of a present, well-formed and fresh header.
fn fresh_signature(header: Option<&str>, now: i64) -> Result<(i64, Vec<u8>), RelayAuthError> {
    let header = header.ok_or(RelayAuthError::MissingSignature)?;
    let mut timestamp = None;
    let mut signature = None;
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
            Some(("v1", value)) => signature = hex::decode(value).ok(),
            _ => {}
        }
    }
    let (Some(timestamp), Some(signature)) = (timestamp, signature) else {
        return Err(RelayAuthError::MalformedSignature);
    };
    if now.abs_diff(timestamp) > RELAY_MAX_SKEW_SECONDS.unsigned_abs() {
        return Err(RelayAuthError::StaleSignature);
    }
    Ok((timestamp, signature))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"relay-secret";
    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn signature_round_trip() {
        let body = br#"{"version":1}"#;
        let header = sign(SECRET, 1_000, body);
        assert_eq!(verify(SECRET, Some(&header), body, 1_000), Ok(()));
        assert_eq!(verify(SECRET, Some(&header), body, 1_300), Ok(()));
    }

    #[test]
    fn signature_refusals() {
        let body = b"payload";
        let header = sign(SECRET, 1_000, body);
        assert_eq!(
            verify(SECRET, None, body, 1_000),
            Err(RelayAuthError::MissingSignature)
        );
        assert_eq!(
            verify(SECRET, Some("v1=abcd"), body, 1_000),
            Err(RelayAuthError::MalformedSignature)
        );
        assert_eq!(
            verify(SECRET, Some(&header), body, 1_301),
            Err(RelayAuthError::StaleSignature)
        );
        assert_eq!(
            verify(SECRET, Some(&header), b"payload!", 1_000),
            Err(RelayAuthError::BadSignature)
        );
        assert_eq!(
            verify(b"other", Some(&header), body, 1_000),
            Err(RelayAuthError::BadSignature)
        );
    }

    #[test]
    fn envelope_keeps_bytes_and_allowed_headers_only() {
        // Meta escapes slashes and non-ASCII; the signature covers these bytes.
        let body = "{\"a\":\"x\\/y\",\"n\":\"\\u00f1 ñ\"}".as_bytes();
        let envelope = CallbackEnvelope::new(
            CallbackProvider::META,
            KEY,
            CallbackMethod::POST,
            "",
            [
                ("X-Hub-Signature-256", "sha256=00"),
                ("Cookie", "secret"),
                ("Content-Type", "application/json"),
            ],
            body,
            Utc::now(),
        );
        let json = serde_json::to_string(&envelope).unwrap();
        let back: CallbackEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.body().unwrap(), body);
        assert_eq!(back.header("x-hub-signature-256"), Some("sha256=00"));
        assert_eq!(back.header("content-type"), Some("application/json"));
        assert_eq!(back.headers.len(), 2);
        assert!(json.contains("\"provider\":\"META\""));
    }

    #[test]
    fn header_check_needs_no_secret_or_body() {
        let header = sign(SECRET, 1_000, b"payload");
        assert_eq!(check_header(Some(&header), 1_000), Ok(()));
        assert_eq!(
            check_header(None, 1_000),
            Err(RelayAuthError::MissingSignature)
        );
        assert_eq!(
            check_header(Some("t=1000"), 1_000),
            Err(RelayAuthError::MalformedSignature)
        );
        assert_eq!(
            check_header(Some("t=1000,v1=zz"), 1_000),
            Err(RelayAuthError::MalformedSignature)
        );
        assert_eq!(
            check_header(Some(&header), 699),
            Err(RelayAuthError::StaleSignature)
        );
    }

    #[test]
    fn body_limits() {
        assert_eq!(max_body_bytes(None).unwrap(), DEFAULT_MAX_BODY_BYTES);
        assert_eq!(max_body_bytes(Some(" ")).unwrap(), DEFAULT_MAX_BODY_BYTES);
        assert_eq!(max_body_bytes(Some("1024")).unwrap(), 1024);
        assert!(max_body_bytes(Some("0")).is_err());
        assert!(max_body_bytes(Some("-1")).is_err());
        assert!(max_body_bytes(Some("1k")).is_err());
        // The envelope of a body at the limit fits harvest's limit.
        let body = vec![0xffu8; usize::try_from(DEFAULT_MAX_BODY_BYTES).unwrap()];
        let envelope = CallbackEnvelope::new(
            CallbackProvider::AWS,
            KEY,
            CallbackMethod::POST,
            "",
            [("content-type", "text/plain; charset=UTF-8")],
            &body,
            Utc::now(),
        );
        let size = serde_json::to_vec(&envelope).unwrap().len() as u64;
        assert!(size < envelope_limit(DEFAULT_MAX_BODY_BYTES));
        assert_eq!(envelope_limit(u64::MAX), u64::MAX);
    }

    #[test]
    fn callback_urls_follow_the_component_routes() {
        assert_eq!(
            CallbackProvider::META.callback_url("https://callbacks.example.org/", KEY),
            format!("https://callbacks.example.org/webhooks/meta/{KEY}")
        );
        assert_eq!(
            CallbackProvider::VIBER.callback_url("http://messaging-webhooks:8410", KEY),
            format!("http://messaging-webhooks:8410/webhooks/viber/{KEY}")
        );
        assert_eq!(CallbackProvider::AWS.path_segment(), "aws");
    }

    #[test]
    fn webhook_key_shape() {
        assert!(is_webhook_key(KEY));
        assert!(is_webhook_key(&crate::link::new_reference()));
        assert!(!is_webhook_key(&KEY.to_uppercase()));
        assert!(!is_webhook_key(&KEY[..32]));
        assert!(!is_webhook_key("../../etc/passwd"));
    }
}
