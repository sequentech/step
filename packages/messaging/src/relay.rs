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

/// How strictly [`RELAY_SECRET_ENV`] is checked ([`RelaySecretPolicy`]).
/// Set only in the development compose file.
pub const RELAY_SECRET_POLICY_ENV: &str = "MESSAGING_RELAY_SECRET_POLICY";

/// The shortest relay secret the component and harvest accept, in bytes:
/// as long as the HMAC-SHA256 output.
pub const MIN_RELAY_SECRET_BYTES: usize = 32;

/// The relay secret committed in the development environment. Refused
/// unless [`RelaySecretPolicy::ALLOW_DEV_SECRET`] is set.
pub const DEV_RELAY_SECRET: &str = "dev-messaging-relay-secret-not-for-production";

/// Whether the committed development secret may be used.
#[allow(non_camel_case_types)]
#[derive(Display, EnumString, Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum RelaySecretPolicy {
    /// Every deployment: the development value is refused.
    #[default]
    STRICT,
    /// The development stack only, where the value is public anyway.
    ALLOW_DEV_SECRET,
}

/// Why a relay secret can't be used. The component doesn't start, and
/// harvest refuses every relayed callback with 503.
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RelaySecretRefusal {
    NOT_SET,
    TOO_SHORT,
    DEVELOPMENT_VALUE,
    UNKNOWN_POLICY,
}

impl RelaySecretRefusal {
    /// The one reason, naming the variable and never its value.
    pub fn reason(self) -> &'static str {
        match self {
            Self::NOT_SET => "MESSAGING_RELAY_SECRET is not set",
            Self::TOO_SHORT => "MESSAGING_RELAY_SECRET must be at least 32 bytes",
            Self::DEVELOPMENT_VALUE => {
                "MESSAGING_RELAY_SECRET is the development value; generate one (openssl rand -hex 32)"
            }
            Self::UNKNOWN_POLICY => "MESSAGING_RELAY_SECRET_POLICY must be STRICT or ALLOW_DEV_SECRET",
        }
    }
}

impl std::fmt::Display for RelaySecretRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.reason())
    }
}

impl std::error::Error for RelaySecretRefusal {}

/// The relay secret from the raw values of [`RELAY_SECRET_ENV`] and
/// [`RELAY_SECRET_POLICY_ENV`]: set, at least [`MIN_RELAY_SECRET_BYTES`]
/// long, and not [`DEV_RELAY_SECRET`] unless the policy allows it.
///
/// # Errors
/// The [`RelaySecretRefusal`] that refuses it.
pub fn relay_secret(
    secret: Option<&str>,
    policy: Option<&str>,
) -> Result<Vec<u8>, RelaySecretRefusal> {
    let policy = match policy.map(str::trim).filter(|policy| !policy.is_empty()) {
        None => RelaySecretPolicy::default(),
        Some(policy) => policy
            .parse::<RelaySecretPolicy>()
            .map_err(|_| RelaySecretRefusal::UNKNOWN_POLICY)?,
    };
    let secret = secret
        .filter(|secret| !secret.trim().is_empty())
        .ok_or(RelaySecretRefusal::NOT_SET)?;
    if secret.len() < MIN_RELAY_SECRET_BYTES {
        return Err(RelaySecretRefusal::TOO_SHORT);
    }
    if secret == DEV_RELAY_SECRET && policy != RelaySecretPolicy::ALLOW_DEV_SECRET {
        return Err(RelaySecretRefusal::DEVELOPMENT_VALUE);
    }
    Ok(secret.as_bytes().to_vec())
}

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

/// How many callbacks the webhook component relays at once (R7-03). More
/// are answered 503, so the provider retries them later, and harvest never
/// holds more of them.
pub const MAX_IN_FLIGHT_ENV: &str = "MESSAGING_WEBHOOKS_MAX_IN_FLIGHT";

/// [`MAX_IN_FLIGHT_ENV`] when it is unset.
pub const DEFAULT_MAX_IN_FLIGHT: usize = 16;

/// How many database connections harvest gives callbacks at once (R7-03),
/// out of the pool that codes and notices use too. A callback that gets
/// none within [`CALLBACK_CONNECTION_WAIT_MS`] is answered 503, so the
/// provider retries it, and codes keep the rest of the pool.
pub const CALLBACK_CONNECTIONS_ENV: &str = "MESSAGING_CALLBACK_CONNECTIONS";

/// [`CALLBACK_CONNECTIONS_ENV`] when it is unset.
pub const DEFAULT_CALLBACK_CONNECTIONS: usize = 4;

/// How long a callback waits for one of its connections.
pub const CALLBACK_CONNECTION_WAIT_MS: u64 = 2_000;

/// A positive count read from the raw value of `name`: `default` when unset
/// or blank.
///
/// # Errors
/// When the value is not a positive whole number.
pub fn positive_count(name: &str, value: Option<&str>, default: usize) -> Result<usize> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(default),
        Some(value) => match value.parse::<usize>() {
            Ok(count) if count > 0 => Ok(count),
            _ => anyhow::bail!("{name} must be a positive whole number"),
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

    /// `received_at` must be within [`RELAY_MAX_SKEW_SECONDS`] of the time
    /// the relay signature names: the component signs what it just
    /// received.
    ///
    /// # Errors
    /// [`RelayAuthError::ReceivedAtMismatch`].
    pub fn check_received_at(&self, signed_at: i64) -> Result<(), RelayAuthError> {
        if self.received_at.timestamp().abs_diff(signed_at) > RELAY_MAX_SKEW_SECONDS.unsigned_abs()
        {
            return Err(RelayAuthError::ReceivedAtMismatch);
        }
        Ok(())
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
    /// The envelope's `received_at` is far from the signature's time.
    ReceivedAtMismatch,
}

impl std::error::Error for RelayAuthError {}

/// A relay signature that verified: its time and its MAC.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct VerifiedSignature {
    /// `t`, in unix seconds.
    pub timestamp: i64,
    /// `v1`, the HMAC bytes.
    pub mac: Vec<u8>,
}

/// Why [`ReplayGuard`] refused a verified request.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Display)]
pub enum ReplayRefusal {
    /// The same signed request was taken before (401).
    Replayed,
    /// Too many fresh signatures to remember another (503: the provider
    /// retries).
    Busy,
}

/// The relay signatures taken in the last [`RELAY_MAX_SKEW_SECONDS`]: a
/// captured relayed request can't be sent again while it is fresh. Older
/// ones are refused by their age, so they are forgotten.
pub struct ReplayGuard {
    seen: std::sync::Mutex<std::collections::HashMap<Vec<u8>, i64>>,
    capacity: usize,
}

/// How many fresh signatures [`ReplayGuard::default`] remembers.
pub const REPLAY_GUARD_CAPACITY: usize = 100_000;

impl Default for ReplayGuard {
    fn default() -> Self {
        Self::new(REPLAY_GUARD_CAPACITY)
    }
}

impl ReplayGuard {
    /// Remembers at most `capacity` fresh signatures.
    pub fn new(capacity: usize) -> Self {
        Self {
            seen: std::sync::Mutex::new(std::collections::HashMap::new()),
            capacity,
        }
    }

    /// Takes a verified signature once.
    ///
    /// # Errors
    /// [`ReplayRefusal::Replayed`] when it was taken before,
    /// [`ReplayRefusal::Busy`] when too many fresh ones are remembered.
    pub fn admit(&self, signature: &VerifiedSignature, now: i64) -> Result<(), ReplayRefusal> {
        let mut seen = self.seen.lock().unwrap_or_else(|e| e.into_inner());
        if seen.contains_key(&signature.mac) {
            return Err(ReplayRefusal::Replayed);
        }
        if seen.len() >= self.capacity {
            seen.retain(|_, at| now.abs_diff(*at) <= RELAY_MAX_SKEW_SECONDS.unsigned_abs());
            if seen.len() >= self.capacity {
                return Err(ReplayRefusal::Busy);
            }
        }
        seen.insert(signature.mac.clone(), signature.timestamp);
        Ok(())
    }
}

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
    verify_signature(secret, header, body, now).map(|_| ())
}

/// [`verify`], returning the signature that verified, for
/// [`ReplayGuard`] and [`CallbackEnvelope::check_received_at`].
///
/// # Errors
/// The [`RelayAuthError`] that refuses the request.
pub fn verify_signature(
    secret: &[u8],
    header: Option<&str>,
    body: &[u8],
    now: i64,
) -> Result<VerifiedSignature, RelayAuthError> {
    let (timestamp, mac) = fresh_signature(header, now)?;
    relay_mac(secret, timestamp, body)
        .verify_slice(&mac)
        .map_err(|_| RelayAuthError::BadSignature)?;
    Ok(VerifiedSignature { timestamp, mac })
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
    fn relay_secrets_are_long_and_never_the_development_value_by_default() {
        let strong = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            relay_secret(Some(strong), None),
            Ok(strong.as_bytes().to_vec())
        );
        assert_eq!(
            relay_secret(Some(strong), Some("STRICT")),
            Ok(strong.as_bytes().to_vec())
        );
        for (secret, policy, refusal) in [
            (None, None, RelaySecretRefusal::NOT_SET),
            (Some("  "), None, RelaySecretRefusal::NOT_SET),
            (Some("s3cret"), None, RelaySecretRefusal::TOO_SHORT),
            (Some(&strong[..31]), None, RelaySecretRefusal::TOO_SHORT),
            (
                Some(DEV_RELAY_SECRET),
                None,
                RelaySecretRefusal::DEVELOPMENT_VALUE,
            ),
            (
                Some(DEV_RELAY_SECRET),
                Some("STRICT"),
                RelaySecretRefusal::DEVELOPMENT_VALUE,
            ),
            (
                Some(strong),
                Some("lax"),
                RelaySecretRefusal::UNKNOWN_POLICY,
            ),
            // The development policy permits the development value, never a
            // short one.
            (
                Some("s3cret"),
                Some("ALLOW_DEV_SECRET"),
                RelaySecretRefusal::TOO_SHORT,
            ),
        ] {
            assert_eq!(
                relay_secret(secret, policy),
                Err(refusal),
                "{secret:?} {policy:?}"
            );
        }
        assert_eq!(
            relay_secret(Some(DEV_RELAY_SECRET), Some("ALLOW_DEV_SECRET")),
            Ok(DEV_RELAY_SECRET.as_bytes().to_vec())
        );
        // One reason each, naming the variable; never the value.
        assert_eq!(
            RelaySecretRefusal::TOO_SHORT.to_string(),
            "MESSAGING_RELAY_SECRET must be at least 32 bytes"
        );
        assert_eq!(
            RelaySecretRefusal::NOT_SET.to_string(),
            "MESSAGING_RELAY_SECRET is not set"
        );
    }

    #[test]
    fn a_verified_signature_names_its_time_and_mac() {
        let body = b"payload";
        let header = sign(SECRET, 1_000, body);
        let verified = verify_signature(SECRET, Some(&header), body, 1_100).unwrap();
        assert_eq!(verified.timestamp, 1_000);
        assert_eq!(format!("t=1000,v1={}", hex::encode(&verified.mac)), header);
    }

    #[test]
    fn a_relayed_request_is_taken_once() {
        let guard = ReplayGuard::new(2);
        let first =
            verify_signature(SECRET, Some(&sign(SECRET, 1_000, b"a")), b"a", 1_000).unwrap();
        let second =
            verify_signature(SECRET, Some(&sign(SECRET, 1_000, b"b")), b"b", 1_000).unwrap();
        assert_eq!(guard.admit(&first, 1_000), Ok(()));
        assert_eq!(guard.admit(&first, 1_200), Err(ReplayRefusal::Replayed));
        assert_eq!(guard.admit(&second, 1_000), Ok(()));
        // Full of signatures that are still fresh: refused for now, so the
        // provider retries, rather than forgetting one that could replay.
        let third =
            verify_signature(SECRET, Some(&sign(SECRET, 1_001, b"c")), b"c", 1_001).unwrap();
        assert_eq!(guard.admit(&third, 1_001), Err(ReplayRefusal::Busy));
        // Once stale, a signature is refused by its age anyway; it is
        // forgotten and makes room.
        assert_eq!(guard.admit(&third, 1_301), Ok(()));
    }

    #[test]
    fn the_envelope_time_is_the_signatures() {
        let envelope = |received: i64| {
            CallbackEnvelope::new(
                CallbackProvider::META,
                KEY,
                CallbackMethod::POST,
                "",
                [],
                b"",
                DateTime::from_timestamp(received, 0).unwrap(),
            )
        };
        assert_eq!(envelope(1_000).check_received_at(1_000), Ok(()));
        assert_eq!(envelope(700).check_received_at(1_000), Ok(()));
        assert_eq!(
            envelope(699).check_received_at(1_000),
            Err(RelayAuthError::ReceivedAtMismatch)
        );
        assert_eq!(
            envelope(1_301).check_received_at(1_000),
            Err(RelayAuthError::ReceivedAtMismatch)
        );
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
        assert_eq!(positive_count(MAX_IN_FLIGHT_ENV, None, 16).unwrap(), 16);
        assert_eq!(
            positive_count(MAX_IN_FLIGHT_ENV, Some(" 3 "), 16).unwrap(),
            3
        );
        assert_eq!(
            positive_count(MAX_IN_FLIGHT_ENV, Some("0"), 16)
                .unwrap_err()
                .to_string(),
            "MESSAGING_WEBHOOKS_MAX_IN_FLIGHT must be a positive whole number"
        );
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
