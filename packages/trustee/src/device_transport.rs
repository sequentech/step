// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An inert RFC 8628 client transport for Keycloak 26.7.5's own device
//! authorization grant (`DeviceEndpoint`, `DeviceGrantType`): it asks the
//! configured realm for a device code and polls its token endpoint, and
//! turns the replies into inputs of the [`crate::device`] model.
//!
//! Nothing at runtime can build it: the crate declares it under `cfg(test)`
//! only, and no login, board, config or legacy path calls it.
//!
//! - Every URL comes from the configured HTTPS issuer and exact realm. No
//!   URL in a reply is ever requested; discovery is not used.
//! - The `verification_uri` Keycloak returns must be spelled exactly as the
//!   realm's standard `/realms/{realm}/device` page derived here from the
//!   configured issuer, or the authorization is refused; only that locally
//!   derived address is ever shown. `verification_uri_complete` is dropped:
//!   the user code is only shown through [`DeviceAuthorization`]'s protected
//!   console accessor, never put in a URL or sent to a browser.
//! - A poll spends the model's [`PollTicket`], so it happens only when the
//!   model allows, and its reply can only settle that ticket.
//! - The client never follows redirects, uses no proxy, trusts only the
//!   site CA bundle the native trustee client already requires, and bounds
//!   connection, request and body size.
//! - Only a confidential client (`client_secret_post`) with PKCE S256 is
//!   supported; there is no public-client or plain-PKCE mode.
//! - Replies are parsed into typed, length- and charset-limited fields.
//!   Errors carry neither replies nor credentials, and nothing secret has a
//!   readable `Debug`.
//! - A token reply stays an [`UnverifiedTokenReply`]: its tokens leave it
//!   only consumed into the device verifier, which proves no second factor
//!   yet, so no reply becomes an approval.

use std::collections::HashSet;
use std::fmt;
use std::time::Duration;

use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::Url;
use serde::Deserialize;

use crate::device::verify::{DeviceNonce, NativeVerifier};
use crate::device::{
    ChallengeBinding, Clock, DeviceError, DeviceLogin, DeviceStatus,
    PollOutcome, PollTicket,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Bounds the whole exchange, reading the body included.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const DEVICE_RESPONSE_LIMIT: usize = 4 * 1024;
pub(crate) const ERROR_RESPONSE_LIMIT: usize = 4 * 1024;
pub(crate) const TOKEN_RESPONSE_LIMIT: usize = 64 * 1024;
const TOKEN_LIMIT: usize = 16 * 1024;
/// Upper sanity bounds; the model applies its own, stricter ones.
const MAX_EXPIRES_IN: u64 = 86_400;
const MAX_INTERVAL: u64 = 3_600;
const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const JSON: &str = "application/json";

/// Why the transport refused. Carries no reply text and no credential.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransportError {
    BadIssuer,
    BadRealm,
    BadClientId,
    BadClientSecret,
    BadPkce,
    /// The PKCE challenge was already used by this transport.
    PkceReused,
    /// No device request nonce could be made.
    Nonce,
    /// The device request nonce was already used by this transport.
    NonceReused,
    /// The poll ticket is not of the grant's own machine and challenge.
    ForeignTicket,
    BadCaBundle,
    ClientBuild,
    /// Network failure, timeout, HTTP 429 or 5xx: try again later.
    Unavailable,
    UnexpectedStatus,
    WrongContentType,
    TooLarge,
    Malformed,
    /// The IdP sent the user somewhere other than the realm's own page.
    VerificationUriMismatch,
    /// The IdP answered with this OAuth error.
    Refused(OAuthErrorCode),
}

/// The OAuth error codes of RFC 6749 5.2 and RFC 8628 3.5 Keycloak sends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OAuthErrorCode {
    AuthorizationPending,
    SlowDown,
    AccessDenied,
    ExpiredToken,
    InvalidGrant,
    InvalidClient,
    UnauthorizedClient,
    InvalidRequest,
    InvalidScope,
    UnsupportedGrantType,
    /// A well-formed code this client does not know.
    Other,
}

impl OAuthErrorCode {
    fn parse(code: &str) -> Option<OAuthErrorCode> {
        if code.is_empty()
            || code.len() > 64
            || !code.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        {
            return None;
        }
        Some(match code {
            "authorization_pending" => OAuthErrorCode::AuthorizationPending,
            "slow_down" => OAuthErrorCode::SlowDown,
            "access_denied" => OAuthErrorCode::AccessDenied,
            "expired_token" => OAuthErrorCode::ExpiredToken,
            "invalid_grant" => OAuthErrorCode::InvalidGrant,
            "invalid_client" => OAuthErrorCode::InvalidClient,
            "unauthorized_client" => OAuthErrorCode::UnauthorizedClient,
            "invalid_request" => OAuthErrorCode::InvalidRequest,
            "invalid_scope" => OAuthErrorCode::InvalidScope,
            "unsupported_grant_type" => OAuthErrorCode::UnsupportedGrantType,
            _ => OAuthErrorCode::Other,
        })
    }
}

/// A secret string: no `Display`, and a `Debug` that shows nothing.
struct Secret(String);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

fn is_visible_ascii(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|b| b.is_ascii_graphic())
}

/// The realm's endpoints, all derived from the configured issuer.
pub(crate) struct IdpEndpoints {
    issuer: Url,
    device_authorization: Url,
    token: Url,
    verification: Url,
}

impl IdpEndpoints {
    /// From the configured Keycloak base URL (HTTPS, with an optional
    /// context path) and the exact realm name.
    pub(crate) fn new(
        base: &str,
        realm: &str,
    ) -> Result<IdpEndpoints, TransportError> {
        if !is_visible_ascii(base, 2_048) {
            return Err(TransportError::BadIssuer);
        }
        let parsed = Url::parse(base).map_err(|_| TransportError::BadIssuer)?;
        let path = parsed.path().trim_end_matches('/');
        if parsed.scheme() != "https"
            || parsed.cannot_be_a_base()
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || path.contains("//")
            || !path.bytes().all(|b| {
                b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_')
            })
        {
            return Err(TransportError::BadIssuer);
        }
        if realm.is_empty()
            || realm.len() > 128
            || !realm
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(TransportError::BadRealm);
        }
        let root = format!(
            "{}{path}/realms/{realm}",
            parsed.origin().ascii_serialization()
        );
        let endpoint = |suffix: &str| {
            Url::parse(&format!("{root}{suffix}"))
                .map_err(|_| TransportError::BadIssuer)
        };
        Ok(IdpEndpoints {
            issuer: endpoint("")?,
            device_authorization: endpoint(
                "/protocol/openid-connect/auth/device",
            )?,
            token: endpoint("/protocol/openid-connect/token")?,
            verification: endpoint("/device")?,
        })
    }

    /// The issuer the realm's signed tokens must name.
    pub(crate) fn issuer(&self) -> &str {
        self.issuer.as_str()
    }

    /// The realm's standard verification page, derived from the configured
    /// issuer: the only address ever shown to the trustee.
    pub(crate) fn verification_uri(&self) -> &str {
        self.verification.as_str()
    }

    /// Whether `value` is the realm's verification page spelled exactly as
    /// derived here. No normalization: dot segments, backslashes, case,
    /// an explicit default port or any other alternate spelling of the same
    /// address is refused.
    fn accepts_verification_uri(&self, value: &str) -> bool {
        value == self.verification_uri()
    }
}

/// The trustee's OAuth client id.
#[derive(Debug)]
pub(crate) struct ClientId(String);

impl ClientId {
    pub(crate) fn new(value: &str) -> Result<ClientId, TransportError> {
        if value.is_empty()
            || value.len() > 255
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        {
            return Err(TransportError::BadClientId);
        }
        Ok(ClientId(value.to_owned()))
    }
}

/// The confidential client's secret.
#[derive(Debug)]
pub(crate) struct ClientSecret(Secret);

impl ClientSecret {
    pub(crate) fn new(value: String) -> Result<ClientSecret, TransportError> {
        if !is_visible_ascii(&value, 4_096) {
            return Err(TransportError::BadClientSecret);
        }
        Ok(ClientSecret(Secret(value)))
    }
}

/// How the client authenticates. Only a confidential client: there is no
/// public-client variant to fall back to.
#[derive(Debug)]
pub(crate) enum ClientAuthentication {
    /// `client_secret_post`, which Keycloak accepts on both endpoints.
    SecretPost(ClientSecret),
}

impl ClientAuthentication {
    fn secret(&self) -> &str {
        match self {
            ClientAuthentication::SecretPost(secret) => &secret.0 .0,
        }
    }
}

/// One flow's PKCE S256 pair (RFC 7636), made by the caller from at least
/// 32 fresh random bytes: this crate has no CSPRNG or SHA-256 of its own.
/// Keycloak checks the pair when the token is issued, so a challenge that
/// is not the verifier's S256 only makes the flow fail.
pub(crate) struct PkceS256 {
    verifier: Secret,
    challenge: String,
}

impl PkceS256 {
    pub(crate) fn from_parts(
        verifier: String,
        challenge: String,
    ) -> Result<PkceS256, TransportError> {
        let unreserved = |b: u8| {
            b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~')
        };
        let base64url = |b: u8| b.is_ascii_alphanumeric() || b"-_".contains(&b);
        // 43 characters: 32 bytes of entropy or a SHA-256, unpadded.
        if !(43..=128).contains(&verifier.len())
            || !verifier.bytes().all(unreserved)
            || challenge.len() != 43
            || !challenge.bytes().all(base64url)
            || challenge == verifier
        {
            return Err(TransportError::BadPkce);
        }
        Ok(PkceS256 {
            verifier: Secret(verifier),
            challenge,
        })
    }
}

impl fmt::Debug for PkceS256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PkceS256(<redacted>)")
    }
}

/// The PKCE challenges and device request nonces this transport has sent,
/// so that none is used twice.
#[derive(Default)]
pub(crate) struct ChallengeLedger(HashSet<String>, HashSet<String>);

impl ChallengeLedger {
    fn register(&mut self, pkce: &PkceS256) -> Result<(), TransportError> {
        if self.0.insert(pkce.challenge.clone()) {
            Ok(())
        } else {
            Err(TransportError::PkceReused)
        }
    }

    fn register_nonce(
        &mut self,
        nonce: &DeviceNonce,
    ) -> Result<(), TransportError> {
        if self.1.insert(nonce.form_value().to_owned()) {
            Ok(())
        } else {
            Err(TransportError::NonceReused)
        }
    }
}

/// Keycloak's device authorization reply, checked. Untrusted: it says what
/// to show and how to poll, nothing about who will approve.
pub(crate) struct DeviceAuthorization {
    device_code: Secret,
    user_code: Secret,
    /// Derived from the configured issuer, never the reply's spelling.
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}

impl DeviceAuthorization {
    /// The realm's own verification page as derived from the configured
    /// issuer, never as the reply spelled it.
    pub(crate) fn verification_uri(&self) -> &str {
        &self.verification_uri
    }

    /// The user code, for the trustee's protected console only: never a
    /// log, a URL, a file or a browser.
    pub(crate) fn user_code_for_protected_console(&self) -> &str {
        &self.user_code.0
    }

    /// For [`crate::device::DeviceLogin::begin`].
    pub(crate) fn expires_in(&self) -> u64 {
        self.expires_in
    }

    /// For [`crate::device::DeviceLogin::begin`]; `None` means RFC 8628's
    /// default, which the model applies.
    pub(crate) fn interval(&self) -> Option<u64> {
        self.interval
    }
}

impl fmt::Debug for DeviceAuthorization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceAuthorization")
            .field("expires_in", &self.expires_in)
            .field("interval", &self.interval)
            .finish_non_exhaustive()
    }
}

/// What the trustee keeps to poll: the device code, the PKCE verifier, and
/// the single-use nonce sent with exactly this device request, which the
/// tokens issued for it must carry back signed.
#[derive(Debug)]
pub(crate) struct PendingDeviceGrant {
    authorization: DeviceAuthorization,
    pkce: PkceS256,
    nonce: DeviceNonce,
}

impl PendingDeviceGrant {
    pub(crate) fn authorization(&self) -> &DeviceAuthorization {
        &self.authorization
    }

    /// Begins `machine`'s challenge from this authorization's lifetime and
    /// interval, binding the grant to exactly that machine and challenge.
    /// Consumes the grant only on success: if the machine cannot begin, the
    /// grant comes back unchanged with the reason.
    pub(crate) fn begin(
        self,
        machine: &mut DeviceLogin,
        clock: Clock,
    ) -> Result<BoundDeviceGrant, (DeviceError, PendingDeviceGrant)> {
        let (expires_in, interval) =
            (self.authorization.expires_in, self.authorization.interval);
        match machine.begin_bound(clock, expires_in, interval) {
            Ok(binding) => Ok(BoundDeviceGrant {
                grant: self,
                binding,
            }),
            Err(error) => Err((error, self)),
        }
    }
}

/// A pending device grant (device code, PKCE verifier, nonce) bound, when
/// its challenge began, to one machine's one challenge. Its polls take
/// only that machine's tickets for that challenge; nothing in it can be
/// read or swapped from outside. Inert: no runtime caller yet.
pub(crate) struct BoundDeviceGrant {
    grant: PendingDeviceGrant,
    binding: ChallengeBinding,
}

impl BoundDeviceGrant {
    pub(crate) fn authorization(&self) -> &DeviceAuthorization {
        &self.grant.authorization
    }

    /// The ticket, if it is this grant's machine's for this grant's
    /// challenge; otherwise it is handed back unspent, so a foreign or
    /// stale call costs the proper poll nothing. Checked before any
    /// network request.
    fn claim_ticket(
        &self,
        ticket: PollTicket,
    ) -> Result<PollTicket, (TransportError, PollTicket)> {
        if self.binding.matches(&ticket) {
            Ok(ticket)
        } else {
            Err((TransportError::ForeignTicket, ticket))
        }
    }

    /// The poll's reply, paired with the ticket this grant accepted and
    /// this grant's own nonce: the only way a [`Polled`] is made.
    fn polled(&self, ticket: PollTicket, reply: PollReply) -> Polled {
        Polled {
            ticket,
            reply,
            nonce: self.grant.nonce.clone(),
        }
    }
}

impl fmt::Debug for BoundDeviceGrant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoundDeviceGrant")
            .field("grant", &self.grant)
            .field("binding", &self.binding)
            .finish()
    }
}

/// A token reply nobody has verified: not a login, a factor or an approval.
pub(crate) struct UnverifiedTokenReply {
    access_token: Secret,
    id_token: Secret,
    refresh_token: Option<Secret>,
    expires_in: u64,
}

impl fmt::Debug for UnverifiedTokenReply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UnverifiedTokenReply(<redacted>)")
    }
}

/// One poll's result.
#[derive(Debug)]
pub(crate) enum PollReply {
    Pending,
    SlowDown,
    Denied,
    Expired,
    /// Any other OAuth error: the flow is over.
    Refused(OAuthErrorCode),
    /// Network failure, timeout, HTTP 429 or 5xx: back off and poll again.
    Unavailable,
    /// A reply that breaks the protocol: the flow is over.
    Malformed,
    TokenIssued(UnverifiedTokenReply),
}

/// What the [`crate::device`] model is told about a poll.
pub(crate) enum ModelStep {
    /// `DeviceLogin::settle` with this outcome.
    Settle(PollOutcome),
    /// `DeviceLogin::poll_failed`.
    PollFailed,
    /// Hold the ticket until the verifier checks this reply, or discard it.
    AwaitVerification(UnverifiedTokenReply),
}

impl PollReply {
    pub(crate) fn into_model_step(self) -> ModelStep {
        match self {
            PollReply::Pending => {
                ModelStep::Settle(PollOutcome::AuthorizationPending)
            }
            PollReply::SlowDown => ModelStep::Settle(PollOutcome::SlowDown),
            PollReply::Denied => ModelStep::Settle(PollOutcome::AccessDenied),
            PollReply::Expired => ModelStep::Settle(PollOutcome::ExpiredToken),
            // The model has no other terminal outcome without a grant.
            PollReply::Refused(_) | PollReply::Malformed => {
                ModelStep::Settle(PollOutcome::AccessDenied)
            }
            PollReply::Unavailable => ModelStep::PollFailed,
            PollReply::TokenIssued(reply) => {
                ModelStep::AwaitVerification(reply)
            }
        }
    }
}

/// One poll's reply with the model ticket the poll spent. The ticket can't
/// be taken back out: it can only settle the model, once.
pub(crate) struct Polled {
    ticket: PollTicket,
    reply: PollReply,
    /// The nonce of the device request this poll belongs to.
    nonce: DeviceNonce,
}

/// What settling a [`Polled`] left.
pub(crate) enum Settled {
    Status(DeviceStatus),
    /// A token reply, with its ticket still in flight, for the verifier.
    AwaitingVerification(AwaitingVerification),
}

/// A token reply waiting for the verifier, holding the ticket of the poll
/// that fetched it; only verifying this reply or discarding it settles it.
/// The refresh token is never used: there is no refresh yet.
pub(crate) struct AwaitingVerification {
    ticket: PollTicket,
    reply: UnverifiedTokenReply,
    nonce: DeviceNonce,
}

impl Polled {
    pub(crate) fn reply(&self) -> &PollReply {
        &self.reply
    }

    /// Settles the model with this reply through the spent ticket.
    pub(crate) fn settle(
        self,
        machine: &mut DeviceLogin,
        clock: Clock,
    ) -> Result<Settled, DeviceError> {
        let Polled {
            ticket,
            reply,
            nonce,
        } = self;
        match reply.into_model_step() {
            ModelStep::Settle(outcome) => {
                machine.settle(clock, &ticket, outcome).map(Settled::Status)
            }
            ModelStep::PollFailed => machine
                .poll_failed(clock.mono, &ticket)
                .map(Settled::Status),
            ModelStep::AwaitVerification(reply) => {
                Ok(Settled::AwaitingVerification(AwaitingVerification {
                    ticket,
                    reply,
                    nonce,
                }))
            }
        }
    }
}

impl AwaitingVerification {
    /// Consumes this reply and its own ticket: the verifier made for
    /// `machine` checks the pair against that machine's scope and pending
    /// challenge, and settles that same poll. Any failed check closes the
    /// challenge as denied. The pair must have been issued for this poll's
    /// own device request (signed `nonce`) and belong together (signed
    /// `at_hash`); that identifies the transaction, it proves no factor.
    /// With the second factor unproven, a valid pair is discarded by the
    /// model too: no grant can result.
    pub(crate) fn verify_and_settle(
        self,
        machine: &mut DeviceLogin,
        clock: Clock,
        verifier: &NativeVerifier,
    ) -> Result<DeviceStatus, DeviceError> {
        let AwaitingVerification {
            ticket,
            reply,
            nonce,
        } = self;
        verifier.settle_pair(
            machine,
            clock,
            &ticket,
            (&reply.access_token.0, &reply.id_token.0),
            &nonce,
        )
    }

    /// Drops the unverified reply and ends the flow without a grant.
    pub(crate) fn discard(
        self,
        machine: &mut DeviceLogin,
        clock: Clock,
    ) -> Result<DeviceStatus, DeviceError> {
        let AwaitingVerification { ticket, reply, .. } = self;
        drop(reply);
        machine.settle(clock, &ticket, PollOutcome::AccessDenied)
    }
}

fn is_json(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|value| {
        value
            .split(';')
            .next()
            .is_some_and(|media| media.trim().eq_ignore_ascii_case(JSON))
    })
}

/// Whether a declared `Content-Length` fits; an absent one is bounded while
/// reading.
fn declared_length_fits(length: Option<u64>, limit: usize) -> bool {
    !length.is_some_and(|length| length > limit as u64)
}

fn append_bounded(
    body: &mut Vec<u8>,
    chunk: &[u8],
    limit: usize,
) -> Result<(), TransportError> {
    if chunk.len() > limit.saturating_sub(body.len()) {
        return Err(TransportError::TooLarge);
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn json_body<'a, T: Deserialize<'a>>(
    content_type: Option<&str>,
    body: &'a [u8],
    limit: usize,
) -> Result<T, TransportError> {
    if !is_json(content_type) {
        return Err(TransportError::WrongContentType);
    }
    if body.len() > limit {
        return Err(TransportError::TooLarge);
    }
    serde_json::from_slice(body).map_err(|_| TransportError::Malformed)
}

#[derive(Deserialize)]
struct ErrorWire {
    error: String,
}

fn parse_error(
    content_type: Option<&str>,
    body: &[u8],
) -> Result<OAuthErrorCode, TransportError> {
    let wire: ErrorWire = json_body(content_type, body, ERROR_RESPONSE_LIMIT)?;
    OAuthErrorCode::parse(&wire.error).ok_or(TransportError::Malformed)
}

/// RFC 8628 3.2 as Keycloak sends it. `verification_uri_complete` is not
/// read; other unknown members are ignored, duplicates refused.
#[derive(Deserialize)]
struct DeviceAuthorizationWire {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}

/// Checks the device authorization endpoint's reply.
pub(crate) fn parse_device_authorization(
    endpoints: &IdpEndpoints,
    status: u16,
    content_type: Option<&str>,
    body: &[u8],
) -> Result<DeviceAuthorization, TransportError> {
    match status {
        200 => {}
        400 | 401 => {
            return Err(TransportError::Refused(parse_error(
                content_type,
                body,
            )?))
        }
        429 | 500..=599 => return Err(TransportError::Unavailable),
        _ => return Err(TransportError::UnexpectedStatus),
    }
    let wire: DeviceAuthorizationWire =
        json_body(content_type, body, DEVICE_RESPONSE_LIMIT)?;
    // Keycloak: base64url of 32 random bytes.
    let device_code_ok = (22..=256).contains(&wire.device_code.len())
        && wire
            .device_code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b));
    // Keycloak: "XXXX-XXXX" by default.
    let user_code_ok = (4..=32).contains(&wire.user_code.len())
        && wire
            .user_code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-');
    let interval_ok = !wire
        .interval
        .is_some_and(|interval| !(1..=MAX_INTERVAL).contains(&interval));
    if !device_code_ok
        || !user_code_ok
        || !(1..=MAX_EXPIRES_IN).contains(&wire.expires_in)
        || !interval_ok
    {
        return Err(TransportError::Malformed);
    }
    if !endpoints.accepts_verification_uri(&wire.verification_uri) {
        return Err(TransportError::VerificationUriMismatch);
    }
    Ok(DeviceAuthorization {
        device_code: Secret(wire.device_code),
        user_code: Secret(wire.user_code),
        verification_uri: endpoints.verification_uri().to_owned(),
        expires_in: wire.expires_in,
        interval: wire.interval,
    })
}

#[derive(Deserialize)]
struct TokenWire {
    access_token: String,
    token_type: String,
    expires_in: u64,
    id_token: Option<String>,
    refresh_token: Option<String>,
}

fn parse_tokens(
    content_type: Option<&str>,
    body: &[u8],
) -> Result<UnverifiedTokenReply, TransportError> {
    let wire: TokenWire = json_body(content_type, body, TOKEN_RESPONSE_LIMIT)?;
    let token = |value: String| {
        if is_visible_ascii(&value, TOKEN_LIMIT) {
            Ok(Secret(value))
        } else {
            Err(TransportError::Malformed)
        }
    };
    if !wire.token_type.eq_ignore_ascii_case("Bearer")
        || !(1..=MAX_EXPIRES_IN).contains(&wire.expires_in)
    {
        return Err(TransportError::Malformed);
    }
    Ok(UnverifiedTokenReply {
        access_token: token(wire.access_token)?,
        // The later verifier needs the ID token's session and MFA claims.
        id_token: token(wire.id_token.ok_or(TransportError::Malformed)?)?,
        refresh_token: wire.refresh_token.map(token).transpose()?,
        expires_in: wire.expires_in,
    })
}

/// Classifies the token endpoint's reply to one device code poll.
pub(crate) fn parse_poll(
    status: u16,
    content_type: Option<&str>,
    body: &[u8],
) -> PollReply {
    match status {
        200 => parse_tokens(content_type, body)
            .map_or(PollReply::Malformed, PollReply::TokenIssued),
        400 | 401 => match parse_error(content_type, body) {
            Ok(OAuthErrorCode::AuthorizationPending) => PollReply::Pending,
            Ok(OAuthErrorCode::SlowDown) => PollReply::SlowDown,
            Ok(OAuthErrorCode::AccessDenied) => PollReply::Denied,
            Ok(OAuthErrorCode::ExpiredToken) => PollReply::Expired,
            Ok(code) => PollReply::Refused(code),
            Err(_) => PollReply::Malformed,
        },
        429 | 500..=599 => PollReply::Unavailable,
        // Redirects included: they are never followed.
        _ => PollReply::Malformed,
    }
}

/// The device authorization request: never the verifier or a device code.
/// Its `nonce` is this request's own; stock Keycloak keeps it with the
/// device code and signs it back in the ID token issued for that code.
fn authorization_form<'a>(
    client_id: &'a ClientId,
    auth: &'a ClientAuthentication,
    pkce: &'a PkceS256,
    nonce: &'a DeviceNonce,
) -> Vec<(&'static str, &'a str)> {
    vec![
        ("client_id", client_id.0.as_str()),
        ("client_secret", auth.secret()),
        ("scope", "openid"),
        ("code_challenge", pkce.challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("nonce", nonce.form_value()),
    ]
}

/// One device code poll of the token endpoint.
fn token_form<'a>(
    client_id: &'a ClientId,
    auth: &'a ClientAuthentication,
    grant: &'a PendingDeviceGrant,
) -> Vec<(&'static str, &'a str)> {
    vec![
        ("grant_type", DEVICE_GRANT_TYPE),
        ("client_id", client_id.0.as_str()),
        ("client_secret", auth.secret()),
        ("device_code", grant.authorization.device_code.0.as_str()),
        ("code_verifier", grant.pkce.verifier.0.as_str()),
    ]
}

/// The HTTP client: the site CA bundle only, HTTPS only, no redirects, no
/// proxy, bounded in time.
pub(crate) fn build_client(
    ca_pem: &[u8],
) -> Result<reqwest::Client, TransportError> {
    let certificates = reqwest::Certificate::from_pem_bundle(ca_pem)
        .map_err(|_| TransportError::BadCaBundle)?;
    if certificates.is_empty() {
        return Err(TransportError::BadCaBundle);
    }
    let mut builder = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .referer(false)
        .https_only(true)
        .no_proxy()
        .min_tls_version(reqwest::tls::Version::TLS_1_2)
        .tls_built_in_root_certs(false);
    for certificate in certificates {
        builder = builder.add_root_certificate(certificate);
    }
    builder.build().map_err(|_| TransportError::ClientBuild)
}

/// The device grant's HTTP exchanges with one realm.
pub(crate) struct DeviceTransport {
    http: reqwest::Client,
    endpoints: IdpEndpoints,
    client_id: ClientId,
    auth: ClientAuthentication,
    ledger: ChallengeLedger,
}

// Inert: no runtime caller until a reviewed integration.
#[allow(dead_code)]
impl DeviceTransport {
    pub(crate) fn new(
        endpoints: IdpEndpoints,
        client_id: ClientId,
        auth: ClientAuthentication,
        ca_pem: &[u8],
    ) -> Result<DeviceTransport, TransportError> {
        Ok(DeviceTransport {
            http: build_client(ca_pem)?,
            endpoints,
            client_id,
            auth,
            ledger: ChallengeLedger::default(),
        })
    }

    /// Starts a flow with a fresh PKCE pair and a nonce made here for this
    /// request alone; the caller never supplies it.
    pub(crate) async fn authorize(
        &mut self,
        pkce: PkceS256,
    ) -> Result<PendingDeviceGrant, TransportError> {
        self.ledger.register(&pkce)?;
        let nonce =
            DeviceNonce::generate().map_err(|_| TransportError::Nonce)?;
        self.ledger.register_nonce(&nonce)?;
        let form =
            authorization_form(&self.client_id, &self.auth, &pkce, &nonce);
        let response = self
            .http
            .post(self.endpoints.device_authorization.clone())
            .header(ACCEPT, JSON)
            .form(&form)
            .send()
            .await
            .map_err(|_| TransportError::Unavailable)?;
        let (status, content_type, body) =
            read_bounded(response, DEVICE_RESPONSE_LIMIT).await?;
        let authorization = parse_device_authorization(
            &self.endpoints,
            status,
            content_type.as_deref(),
            &body,
        )?;
        Ok(PendingDeviceGrant {
            authorization,
            pkce,
            nonce,
        })
    }

    /// Polls once with the bound grant, spending the ticket the model
    /// issued when its interval allowed; there is no other way to poll. A
    /// ticket of another machine or challenge is refused before any request
    /// and handed back unspent.
    pub(crate) async fn poll(
        &self,
        bound: &BoundDeviceGrant,
        ticket: PollTicket,
    ) -> Result<Polled, (TransportError, PollTicket)> {
        let ticket = bound.claim_ticket(ticket)?;
        let reply = self.exchange(&bound.grant).await;
        Ok(bound.polled(ticket, reply))
    }

    async fn exchange(&self, grant: &PendingDeviceGrant) -> PollReply {
        let form = token_form(&self.client_id, &self.auth, grant);
        let Ok(response) = self
            .http
            .post(self.endpoints.token.clone())
            .header(ACCEPT, JSON)
            .form(&form)
            .send()
            .await
        else {
            return PollReply::Unavailable;
        };
        match read_bounded(response, TOKEN_RESPONSE_LIMIT).await {
            Ok((status, content_type, body)) => {
                parse_poll(status, content_type.as_deref(), &body)
            }
            Err(TransportError::Unavailable) => PollReply::Unavailable,
            Err(_) => PollReply::Malformed,
        }
    }
}

/// The reply's status, media type and body, refusing a body over `limit`.
#[allow(dead_code)] // Inert, as `DeviceTransport`.
async fn read_bounded(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<(u16, Option<String>, Vec<u8>), TransportError> {
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if !declared_length_fits(response.content_length(), limit) {
        return Err(TransportError::TooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| TransportError::Unavailable)?
    {
        append_bounded(&mut body, &chunk, limit)?;
    }
    Ok((status, content_type, body))
}

#[cfg(test)]
#[path = "../tests/support/device_transport_tests.rs"]
mod tests;
