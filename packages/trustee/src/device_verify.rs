// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An inert verifier of the access and ID token pair Keycloak's device grant
//! returns, which settles the poll that fetched the pair.
//!
//! It is a child of the device model, declared under `cfg(test)` only, so
//! it alone can build a [`VerifiedLogin`] and nothing at runtime can reach
//! it.
//!
//! - A verifier is made for one [`DeviceLogin`]: its expectations come from
//!   that machine's immutable [`DeviceScope`], and the challenge start from
//!   the machine's pending challenge, never from the caller.
//! - It only settles a poll in flight on that machine, through that poll's
//!   ticket: a pair that fails any check closes the challenge as denied.
//! - Signatures are checked by `jsonwebtoken` (AWS-LC backend), RS256 only,
//!   against a caller-injected, issuer-bound ring of at most 16 RSA keys
//!   with exponent 65537. There is no key fetch, cache, discovery or refresh.
//! - A token header may hold only `alg`, `kid` and `typ`: any other member,
//!   `crit` or a key source (`jku`, `jwk`, `x5u`, `x5c`, `x5t`) included and
//!   even when null, is refused, and none is ever followed.
//! - Every claim is checked after the signature, against an explicit wall
//!   time and the trusted challenge start, with no leeway.
//! - Every settlement checks the transaction's identity: the ID token's
//!   signed `nonce` is exactly the single-use [`DeviceNonce`] sent with
//!   this pending device request, and its signed `at_hash` is that of these
//!   exact access token bytes (as the stock Keycloak 26.7.5 fixture shows:
//!   ID nonce exact, no access nonce, at_hash matching, same reused SSO
//!   `sid`). That says which device request the tokens answer; it is not
//!   evidence of any factor, which stays the admission's alone.
//! - No generic claim proves a fresh second factor: `acr` is never read,
//!   and [`NativeVerifier::for_machine`] is `Unproven`, so every login it
//!   returns carries [`MfaEvidence::Absent`], which the model discards.
//! - The only other policy, `SignedFlowAmr`, needs a [`FlowPolicyProof`]:
//!   the installer's typed receipt of the whole installed shape (pinned
//!   Keycloak release, issuer, the device client's access, grants, browser
//!   flow and every client bound to that flow, every execution of the flow
//!   with its requirement and AMR configuration, and every mapper writing
//!   `amr`), which must be the one qualifying shape, and separate,
//!   current, accepted readback evidence that observed exactly that shape,
//!   field by field and by derived SHA-256 fingerprint. The receipt also
//!   covers the realm's default browser flow, its registered required
//!   actions and its OTP policy, and any other mapper writing `amr`.
//! - Admission under that proof also needs evidence given per poll: a flow
//!   readback made at or after both tokens' `iat` (never one from before
//!   the tokens), and the trustee's own factors observed before the
//!   challenge (one TOTP credential created earlier, no pending required
//!   action) and again after the tokens (the same credential). A TOTP
//!   enrolled inside the device run never counts as a preexisting factor.
//!   Without that evidence, no factor is proven. With it, both signed tokens must carry exactly those two `amr`
//!   references and be issued within the maximum age of a fresh
//!   `auth_time`; only then is the login a fresh TOTP. No constructor of
//!   that proof exists outside this module's tests, so nothing at runtime
//!   can reach it; the live readback, its freshness feed and production
//!   evidence remain to be built.

use std::collections::HashSet;
use std::sync::Arc;

use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use super::{
    ChallengeId, Clock, DeviceError, DeviceLogin, DeviceScope, DeviceStatus,
    FreshMfa, MfaEvidence, Owner, PollOutcome, PollTicket, SecondFactor, State,
    UnixSeconds, VerifiedLogin, MAX_GRANT_LIFETIME,
};

/// The longest token accepted, in bytes.
const TOKEN_LIMIT: usize = 16 * 1024;
const MAX_KEYS: usize = 16;
/// Base64url lengths of an RSA modulus of 2048 to 8192 bits.
const MODULUS_LENGTHS: std::ops::RangeInclusive<usize> = 342..=1_366;
/// 65537, the only RSA public exponent accepted, as Keycloak publishes it.
const EXPONENT: &str = "AQAB";
/// Header members that name a key source; never followed, always refused.
const KEY_SOURCES: [&str; 6] = ["jku", "jwk", "x5u", "x5c", "x5t", "x5t#S256"];
/// Keycloak's `typ` claim of an access token and of an ID token.
const ACCESS_TYPE: &str = "Bearer";
const ID_TYPE: &str = "ID";
/// The dedicated device flow's AMR references: one per required execution.
const PASSWORD_REFERENCE: &str =
    "https://sequent.vote/amr/trustee-device-password-v1";
const TOTP_REFERENCE: &str = "https://sequent.vote/amr/trustee-device-totp-v1";
/// The longest maximum age a flow receipt may set, in seconds.
const MAX_MFA_AGE: u64 = 60;
/// How old readback evidence may be and still count as current.
const READBACK_MAX_AGE: u64 = 300;

/// Why a pair, a scope or a key ring was refused. Names the check, never a
/// value from a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VerifyError {
    /// The machine's scope does not meet the verifier's stricter rules.
    BadPolicy,
    BadKeyRing,
    /// Not a bounded compact JWS.
    TokenShape,
    /// A header member other than `alg`, `kid` and `typ`, a duplicate, or
    /// an unexpected `typ`.
    Header,
    /// A `crit` member, whatever its value.
    CriticalExtension,
    Algorithm,
    /// The header names a key source; none is ever followed.
    KeyDiscovery,
    UnknownKey,
    Signature,
    /// Missing, mistyped or duplicate claims, or contradictory times.
    Malformed,
    Issuer,
    TokenType,
    Audience,
    AuthorizedParty,
    Subject,
    Session,
    IssuedInFuture,
    NotYetValid,
    Expired,
    Lifetime,
    AuthTimeMissing,
    AuthTimeStale,
    Tenant,
    Member,
    Role,
    /// The access and ID tokens disagree.
    Mismatch,
    /// The flow proof does not match the machine's issuer or client, or
    /// its receipt and readback disagree.
    FlowPolicy,
    /// The flow readback is not accepted, or not current.
    FlowDrift,
    /// `amr` is not exactly the two dedicated references, in both tokens.
    AmrMismatch,
    /// A token was issued later than the flow's maximum age after the
    /// authentication.
    MfaStale,
    /// A proven admission was asked for without its evidence.
    NoAdmissionEvidence,
    /// A readback was made before a signed token was issued: it cannot
    /// speak for the configuration the tokens were issued under.
    ReadbackBeforeTokens,
    /// A readback stamped in the same whole second as the later token: not
    /// yet after it, not before it either. Never admitted; a caller may
    /// keep the poll pending and ask again.
    ReadbackSameSecond,
    /// The trustee's TOTP was not enrolled, alone and with no pending
    /// required action, before this challenge began.
    FactorNotPreexisting,
    /// After the tokens, the trustee's TOTP or pending actions differ
    /// from before the challenge.
    FactorChanged,
    /// The evidence is not bound to this challenge by its own requests'
    /// nonces and phases.
    Binding,
    /// The ID token's signed `nonce` is not this device request's: the
    /// tokens were not issued for this challenge's device code.
    DeviceNonce,
    /// The ID token's signed `at_hash` is missing, or not that of the
    /// access token given with it.
    AccessHash,
}

/// What proves the trustee's second factor.
pub(crate) enum SecondFactorPolicy {
    /// Nothing: every login is discarded for want of a factor.
    Unproven,
    /// The dedicated flow's signed `amr`, under a bound flow proof.
    SignedFlowAmr(BoundFlowProof),
}

/// The Keycloak release whose flow, client and mapper shapes were tested.
const PINNED_IDP_RELEASE: &str = "26.7.5";

/// How a client authenticates to the IdP.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ClientAccess {
    Confidential,
    Public,
    BearerOnly,
}

/// A grant a client is allowed to use.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Grant {
    DeviceAuthorization,
    StandardFlow,
    DirectAccess,
    Implicit,
    ServiceAccounts,
    TokenExchange,
}

/// The device client as installed.
#[derive(Clone, PartialEq, Eq)]
struct ClientShape {
    client_id: String,
    access: ClientAccess,
    /// Every grant enabled, sorted, without repeats.
    grants: Vec<Grant>,
    /// The client's browser flow override.
    browser_flow: String,
    /// Every client whose browser flow override is the dedicated flow,
    /// sorted, without repeats.
    flow_bound_clients: Vec<String>,
}

/// The kind of an authentication flow.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FlowProvider {
    BasicFlow,
    ClientFlow,
}

/// What one execution of the flow runs.
#[derive(Clone, PartialEq, Eq)]
enum Step {
    UsernamePasswordForm,
    OtpForm,
    Cookie,
    IdentityProviderRedirector,
    /// A nested flow, by alias.
    Subflow(String),
    /// Any other authenticator, by provider id.
    Unknown(String),
}

/// An execution's requirement.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Requirement {
    Required,
    Alternative,
    Conditional,
    Disabled,
}

/// One execution, in flow order, with its authenticator configuration's
/// AMR reference (`default.reference.value`) and maximum age
/// (`default.reference.maxAge`), when it has them.
#[derive(Clone, PartialEq, Eq)]
struct ExecutionShape {
    step: Step,
    requirement: Requirement,
    amr_reference: Option<String>,
    amr_max_age: Option<u64>,
}

/// The dedicated browser flow as installed: every execution, disabled ones
/// included, at the top level.
#[derive(Clone, PartialEq, Eq)]
struct FlowShape {
    alias: String,
    top_level: bool,
    provider: FlowProvider,
    executions: Vec<ExecutionShape>,
}

/// Which mapper implementation writes the `amr` claim.
#[derive(Clone, PartialEq, Eq)]
enum MapperProvider {
    /// Keycloak's built-in `oidc-amr-mapper`.
    OidcAmr,
    Other(String),
}

/// Where a mapper is attached.
#[derive(Clone, PartialEq, Eq)]
enum MapperHost {
    /// On the device client itself.
    Client,
    /// On a client scope, by name.
    ClientScope(String),
}

/// One protocol mapper that writes the `amr` claim for the client.
#[derive(Clone, PartialEq, Eq)]
struct AmrMapperShape {
    provider: MapperProvider,
    host: MapperHost,
    id_token: bool,
    access_token: bool,
}

/// Keycloak's Configure OTP required action: its alias and provider id.
const CONFIGURE_TOTP: &str = "CONFIGURE_TOTP";

/// One registered required action, as `RequiredActionProviderRepresentation`
/// carries it (alias, providerId, enabled, defaultAction, priority).
#[derive(Clone, PartialEq, Eq)]
struct RequiredActionShape {
    alias: String,
    provider_id: String,
    enabled: bool,
    default_action: bool,
    priority: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OtpKind {
    Totp,
    Hotp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OtpAlgorithm {
    HmacSha1,
    HmacSha256,
    HmacSha512,
}

/// The realm's OTP policy.
#[derive(Clone, Copy, PartialEq, Eq)]
struct OtpPolicyShape {
    kind: OtpKind,
    algorithm: OtpAlgorithm,
    digits: u8,
    period: u32,
    look_ahead: u8,
    code_reusable: bool,
}

/// The stock OTP policy the enrolled TOTP was tested under.
const STOCK_OTP_POLICY: OtpPolicyShape = OtpPolicyShape {
    kind: OtpKind::Totp,
    algorithm: OtpAlgorithm::HmacSha1,
    digits: 6,
    period: 30,
    look_ahead: 1,
    code_reusable: false,
};

/// The installed shape the `amr` claim's meaning depends on: the release,
/// the issuer, the realm's default browser flow, its registered required
/// actions and OTP policy, the device client, its dedicated flow, every
/// `oidc-amr-mapper` (by provider id) for that client, and the provider ids
/// of any other mapper configured to write the `amr` claim. Compared whole
/// and by fingerprint; never reduced to a flag or a supplied digest.
#[derive(Clone, PartialEq, Eq)]
struct FlowReceipt {
    idp_release: String,
    issuer: String,
    realm_browser_flow: String,
    /// Sorted by alias, without repeats.
    required_actions: Vec<RequiredActionShape>,
    otp_policy: OtpPolicyShape,
    client: ClientShape,
    flow: FlowShape,
    amr_mappers: Vec<AmrMapperShape>,
    /// Provider ids of other mappers writing `amr`, sorted.
    foreign_amr_writers: Vec<String>,
    /// Aliases of every other flow with an execution configured with
    /// either dedicated reference, sorted. Keycloak's AMR mapper names the
    /// references of every execution the user session completed, in any
    /// flow, so a reference is only proof of the dedicated execution while
    /// no other execution carries it.
    reference_collisions: Vec<String>,
}

/// Appends `field` so that no two field sequences encode alike.
fn push_field(out: &mut String, field: &str) {
    out.push_str(&field.len().to_string());
    out.push(':');
    out.push_str(field);
    out.push(';');
}

impl FlowReceipt {
    /// A canonical, injective encoding of every field.
    fn canonical(&self) -> String {
        let mut out = String::new();
        push_field(&mut out, "flow-receipt-v2");
        push_field(&mut out, &self.idp_release);
        push_field(&mut out, &self.issuer);
        push_field(&mut out, &self.realm_browser_flow);
        push_field(&mut out, &self.required_actions.len().to_string());
        for action in &self.required_actions {
            push_field(&mut out, &action.alias);
            push_field(&mut out, &action.provider_id);
            push_field(&mut out, if action.enabled { "enabled" } else { "-" });
            push_field(
                &mut out,
                if action.default_action {
                    "default"
                } else {
                    "-"
                },
            );
            push_field(&mut out, &action.priority.to_string());
        }
        let otp = &self.otp_policy;
        push_field(&mut out, &format!("{}", otp.kind as u8));
        push_field(&mut out, &format!("{}", otp.algorithm as u8));
        push_field(&mut out, &otp.digits.to_string());
        push_field(&mut out, &otp.period.to_string());
        push_field(&mut out, &otp.look_ahead.to_string());
        push_field(&mut out, if otp.code_reusable { "reusable" } else { "-" });
        push_field(&mut out, &self.foreign_amr_writers.len().to_string());
        for writer in &self.foreign_amr_writers {
            push_field(&mut out, writer);
        }
        push_field(&mut out, &self.reference_collisions.len().to_string());
        for alias in &self.reference_collisions {
            push_field(&mut out, alias);
        }
        let client = &self.client;
        push_field(&mut out, &client.client_id);
        push_field(&mut out, &format!("{}", client.access as u8));
        push_field(&mut out, &client.grants.len().to_string());
        for grant in &client.grants {
            push_field(&mut out, &format!("{}", *grant as u8));
        }
        push_field(&mut out, &client.browser_flow);
        push_field(&mut out, &client.flow_bound_clients.len().to_string());
        for bound in &client.flow_bound_clients {
            push_field(&mut out, bound);
        }
        let flow = &self.flow;
        push_field(&mut out, &flow.alias);
        push_field(&mut out, if flow.top_level { "top" } else { "nested" });
        push_field(&mut out, &format!("{}", flow.provider as u8));
        push_field(&mut out, &flow.executions.len().to_string());
        for execution in &flow.executions {
            let (kind, name) = match &execution.step {
                Step::UsernamePasswordForm => ("password", ""),
                Step::OtpForm => ("otp", ""),
                Step::Cookie => ("cookie", ""),
                Step::IdentityProviderRedirector => ("idp-redirector", ""),
                Step::Subflow(alias) => ("subflow", alias.as_str()),
                Step::Unknown(provider) => ("unknown", provider.as_str()),
            };
            push_field(&mut out, kind);
            push_field(&mut out, name);
            push_field(&mut out, &format!("{}", execution.requirement as u8));
            match &execution.amr_reference {
                Some(reference) => push_field(&mut out, reference),
                None => push_field(&mut out, "-"),
            }
            match execution.amr_max_age {
                Some(age) => push_field(&mut out, &age.to_string()),
                None => push_field(&mut out, "-"),
            }
        }
        push_field(&mut out, &self.amr_mappers.len().to_string());
        for mapper in &self.amr_mappers {
            match &mapper.provider {
                MapperProvider::OidcAmr => {
                    push_field(&mut out, "oidc-amr-mapper")
                }
                MapperProvider::Other(provider) => {
                    push_field(&mut out, "other");
                    push_field(&mut out, provider);
                }
            }
            match &mapper.host {
                MapperHost::Client => push_field(&mut out, "client"),
                MapperHost::ClientScope(scope) => {
                    push_field(&mut out, "client-scope");
                    push_field(&mut out, scope);
                }
            }
            push_field(&mut out, if mapper.id_token { "id" } else { "-" });
            push_field(
                &mut out,
                if mapper.access_token { "access" } else { "-" },
            );
        }
        out
    }

    /// SHA-256 of the canonical encoding: what an installer records and a
    /// readback recomputes. Always derived here, never accepted as input.
    fn fingerprint(&self) -> Vec<u8> {
        aws_lc_rs::digest::digest(
            &aws_lc_rs::digest::SHA256,
            self.canonical().as_bytes(),
        )
        .as_ref()
        .to_vec()
    }

    /// The one shape that qualifies, for `policy`'s issuer and client:
    /// - the tested Keycloak release;
    /// - a confidential client allowed the device grant only, whose browser
    ///   flow is the dedicated flow, and the only client bound to it;
    /// - a top-level basic flow of exactly two REQUIRED executions, the
    ///   password form then the OTP form, carrying the two dedicated
    ///   references with one maximum age of at most 60 s, and nothing
    ///   else: no cookie, alternative, conditional, disabled, nested or
    ///   unknown step;
    /// - exactly one `amr` mapper, Keycloak's own, on the client, writing
    ///   both the ID and the access token, and no other mapper writing
    ///   `amr`;
    /// - a realm default browser flow other than the dedicated flow;
    /// - Configure OTP registered and enabled, and no required action a
    ///   default action for new users;
    /// - the stock OTP policy: TOTP, HMAC-SHA1, 6 digits, 30 s, one step
    ///   of look-ahead, codes not reusable.
    ///
    /// The maximum age, when it qualifies.
    fn qualifying_max_age(&self, policy: &ClaimPolicy) -> Option<u64> {
        let client = &self.client;
        let flow = &self.flow;
        let client_ok = client.client_id == policy.client_id
            && client.access == ClientAccess::Confidential
            && client.grants == [Grant::DeviceAuthorization]
            && client.browser_flow == flow.alias
            && client.flow_bound_clients == [policy.client_id.as_str()];
        let flow_ok = is_visible_ascii(&flow.alias, 255)
            && flow.top_level
            && flow.provider == FlowProvider::BasicFlow;
        let [password, otp] = flow.executions.as_slice() else {
            return None;
        };
        let max_age = password.amr_max_age?;
        let executions_ok = password.step == Step::UsernamePasswordForm
            && password.requirement == Requirement::Required
            && password.amr_reference.as_deref() == Some(PASSWORD_REFERENCE)
            && otp.step == Step::OtpForm
            && otp.requirement == Requirement::Required
            && otp.amr_reference.as_deref() == Some(TOTP_REFERENCE)
            && otp.amr_max_age == Some(max_age)
            && (1..=MAX_MFA_AGE).contains(&max_age);
        let mapper_ok = self.amr_mappers
            == [AmrMapperShape {
                provider: MapperProvider::OidcAmr,
                host: MapperHost::Client,
                id_token: true,
                access_token: true,
            }]
            && self.foreign_amr_writers.is_empty()
            && self.reference_collisions.is_empty();
        let realm_ok = is_visible_ascii(&self.realm_browser_flow, 255)
            && self.realm_browser_flow != flow.alias;
        let actions = &self.required_actions;
        let actions_ok =
            actions.windows(2).all(|pair| pair[0].alias < pair[1].alias)
                && actions.iter().all(|action| !action.default_action)
                && actions.iter().any(|action| {
                    action.alias == CONFIGURE_TOTP
                        && action.provider_id == CONFIGURE_TOTP
                        && action.enabled
                });
        (self.idp_release == PINNED_IDP_RELEASE
            && self.issuer == policy.issuer
            && realm_ok
            && actions_ok
            && self.otp_policy == STOCK_OTP_POLICY
            && client_ok
            && flow_ok
            && executions_ok
            && mapper_ok)
            .then_some(max_age)
    }
}

/// What an authenticated readback of the live flow concluded.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Readback {
    /// The live flow and its lock match the receipt.
    Accepted,
    /// The live flow differs from the receipt.
    Drifted,
    /// No readback could be made.
    Unavailable,
}

/// A separate, authenticated readback of the live flow configuration.
struct DriftEvidence {
    observed: FlowReceipt,
    checked_at: UnixSeconds,
    readback: Readback,
    /// The request it answered; none for an installation-time readback.
    binding: Option<RequestBinding>,
}

/// The typed, prechecked evidence that tokens' `amr` can be trusted: the
/// installer's receipt and current readback evidence agreeing with it.
/// Its fields are private and it has no constructor: only this module's
/// tests build one. Production evidence is not implemented.
pub(crate) struct FlowPolicyProof {
    receipt: FlowReceipt,
    drift: DriftEvidence,
}

/// A proof whose binding was checked, with the flow's maximum age.
pub(crate) struct BoundFlowProof {
    proof: FlowPolicyProof,
    max_age: u64,
}

/// When an observation was asked for, relative to the challenge.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// After the challenge began, before the trustee authenticated.
    BeforeAuthentication,
    /// After both tokens were issued.
    AfterTokens,
}

/// What a request for an observation was bound to: a fresh opaque nonce
/// the runner made for it, its phase, and the challenge. The collector
/// echoes these inside its authenticated reply; nothing outside the reply
/// body (no extra header) counts.
#[derive(Clone, PartialEq, Eq)]
struct RequestBinding {
    nonce: String,
    phase: Phase,
    challenge: ChallengeId,
}

/// A request nonce: 32 random bytes, unpadded base64url.
fn is_request_nonce(nonce: &str) -> bool {
    nonce.len() == 43 && is_base64url(nonce)
}

/// Unpadded base64url.
fn encode_base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);
        let n = (u32::from(buf[0]) << 16)
            | (u32::from(buf[1]) << 8)
            | u32::from(buf[2]);
        for i in 0..=chunk.len() {
            let index = (n >> (18 - 6 * i)) & 63;
            out.push(char::from(ALPHABET[index as usize]));
        }
    }
    out
}

/// The single-use nonce of one device authorization request: 32 bytes from
/// the system's random source, made for exactly one pending device request
/// and sent only in its form. Keycloak keeps it with that device code and
/// signs it back as the ID token's `nonce` for the token issued for that
/// code. It identifies the transaction; it proves no factor. No `Display`;
/// its `Debug` shows nothing. In this test-only module the random source is
/// AWS-LC; a runtime one needs a reviewed dependency.
#[derive(Clone)]
pub(crate) struct DeviceNonce(String);

impl DeviceNonce {
    pub(crate) fn generate() -> Result<DeviceNonce, VerifyError> {
        use aws_lc_rs::rand::{SecureRandom, SystemRandom};
        let mut bytes = [0u8; 32];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| VerifyError::DeviceNonce)?;
        Ok(DeviceNonce(encode_base64url(&bytes)))
    }

    /// The value for the device authorization form's `nonce` field.
    pub(crate) fn form_value(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for DeviceNonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DeviceNonce(<redacted>)")
    }
}

/// Sealing of [`BoundPollResult`]: only this module names it, so only the
/// types it lists here can be bound poll results.
mod sealed {
    pub trait Sealed {}
}

impl sealed::Sealed for crate::device_transport::AwaitingVerification {}

/// The key to a [`BoundPollResult`]: only this module makes one, so only
/// this module (and its children) can open a result.
pub(crate) struct Opener(());

/// What a bound poll result opens to: the poll's own ticket, the pair it
/// fetched, and the nonce of the device request the poll belongs to.
pub(crate) struct BoundParts<'a> {
    pub(crate) ticket: &'a PollTicket,
    pub(crate) access_token: &'a str,
    pub(crate) id_token: &'a str,
    pub(crate) nonce: &'a DeviceNonce,
}

/// The owned, opaque result of one successful bound poll, settled once by
/// the verifier. Sealed: only the transport's own awaiting reply (built only
/// through a bound grant's accepted ticket) implements it, and only the
/// verifier, holding an [`Opener`], can read its parts.
#[allow(private_bounds)] // The sealing supertrait is private on purpose.
pub(crate) trait BoundPollResult: sealed::Sealed {
    /// The poll's own ticket alone: no token or nonce. Read first, to
    /// check which machine the result belongs to before anything else.
    fn ticket(&self, opener: &Opener) -> &PollTicket;
    fn open(&self, opener: &Opener) -> BoundParts<'_>;
}

/// Whether `ticket` was issued by `machine`: its identity, nothing secret.
fn is_own_ticket(machine: &DeviceLogin, ticket: &PollTicket) -> bool {
    Arc::ptr_eq(&machine.owner.0, &ticket.owner.0)
}

/// What a terminal settlement did with the sealed result it was given.
pub(crate) enum Settlement<R> {
    /// Settled on the result's own machine, or refused there; either way
    /// the result is spent.
    Settled(Result<DeviceStatus, DeviceError>),
    /// Offered to a machine that did not issue its ticket. Only the
    /// ticket's machine identity was read; the result comes back unspent
    /// for its own machine, which it still can settle.
    WrongMachine(R),
}

/// The OIDC `at_hash` of an access token signed with RS256: the left half
/// of the SHA-256 of its exact ASCII bytes, unpadded base64url.
fn rs256_access_hash(access_token: &str) -> String {
    let digest = aws_lc_rs::digest::digest(
        &aws_lc_rs::digest::SHA256,
        access_token.as_bytes(),
    );
    encode_base64url(&digest.as_ref()[..16])
}

/// One OTP credential of the trustee: its id and creation time.
#[derive(Clone, PartialEq, Eq)]
struct OtpCredentialShape {
    id: String,
    created_at: UnixSeconds,
}

/// An authenticated observation of the trustee's own factors: OTP
/// credentials and pending required actions, for exactly one subject.
#[derive(Clone, PartialEq, Eq)]
struct SubjectFactors {
    binding: RequestBinding,
    subject: String,
    otp_credentials: Vec<OtpCredentialShape>,
    pending_actions: Vec<String>,
    observed_at: UnixSeconds,
}

/// What the tokens themselves say that the admission checks against.
struct IssuedPair {
    issued: [UnixSeconds; 2],
    auth_time: UnixSeconds,
}

/// What a proven admission needs besides the tokens: a flow readback made
/// after both tokens were issued, and the trustee's factors observed before
/// the challenge and again after the tokens. This is factor evidence; the
/// transaction's identity is checked apart, for every settlement.
struct Admission<'a> {
    readback: &'a DriftEvidence,
    before: &'a SubjectFactors,
    after: &'a SubjectFactors,
}

impl FlowPolicyProof {
    /// Binds the proof to `policy`: the receipt is the one qualifying shape
    /// for this issuer and client, normalized, and the readback observed
    /// exactly it, field by field and by fingerprint.
    fn bind(self, policy: &ClaimPolicy) -> Result<BoundFlowProof, VerifyError> {
        let receipt = &self.receipt;
        let observed = &self.drift.observed;
        let max_age = receipt
            .qualifying_max_age(policy)
            .ok_or(VerifyError::FlowPolicy)?;
        if observed != receipt
            || observed.fingerprint() != receipt.fingerprint()
            || observed.qualifying_max_age(policy) != Some(max_age)
        {
            return Err(VerifyError::FlowPolicy);
        }
        Ok(BoundFlowProof {
            proof: self,
            max_age,
        })
    }

    /// Exactly the flow's two references, in either order.
    fn amr_exact(amr: Option<&serde_json::Value>) -> bool {
        let Some(serde_json::Value::Array(values)) = amr else {
            return false;
        };
        let references = [PASSWORD_REFERENCE, TOTP_REFERENCE];
        values.len() == 2
            && values.first() != values.last()
            && values.iter().all(|value| {
                value
                    .as_str()
                    .is_some_and(|reference| references.contains(&reference))
            })
    }
}

/// The machine's scope, checked against the verifier's stricter rules.
struct ClaimPolicy {
    issuer: String,
    access_audience: String,
    client_id: String,
    subject: String,
    tenant: String,
    trustee_name: String,
    role: String,
}

fn is_visible_ascii(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value.bytes().all(|b| b.is_ascii_graphic())
}

/// A lower-case, hyphenated UUID.
fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, b)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

fn is_base64url(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Canonical unpadded base64url: no padding, no other alphabet, and no
/// stray bits in the last character.
fn decode_base64url(input: &str) -> Option<Vec<u8>> {
    if input.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(input.len() / 4 * 3 + 2);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(u8::try_from(buffer >> bits).ok()?);
            buffer &= (1 << bits) - 1;
        }
    }
    (buffer == 0).then_some(out)
}

impl ClaimPolicy {
    /// The scope's values, which must also be: an HTTPS issuer of exactly
    /// the tenant's realm, lower-case UUIDs for tenant and subject, and
    /// visible ASCII names.
    fn from_scope(scope: &DeviceScope) -> Result<ClaimPolicy, VerifyError> {
        let tenant_realm = format!("/realms/tenant-{}", scope.tenant);
        let names_ok = [
            &scope.audience,
            &scope.client_id,
            &scope.trustee_name,
            &scope.required_role,
        ]
        .iter()
        .all(|value| is_visible_ascii(value, 255));
        if !names_ok
            || !is_uuid(&scope.tenant)
            || !is_uuid(&scope.subject)
            || !is_visible_ascii(&scope.issuer, 2_048)
            || !scope.issuer.starts_with("https://")
            || !scope.issuer.ends_with(&tenant_realm)
            || scope.issuer.contains(['?', '#', '\\'])
        {
            return Err(VerifyError::BadPolicy);
        }
        Ok(ClaimPolicy {
            issuer: scope.issuer.clone(),
            access_audience: scope.audience.clone(),
            client_id: scope.client_id.clone(),
            subject: scope.subject.clone(),
            tenant: scope.tenant.clone(),
            trustee_name: scope.trustee_name.clone(),
            role: scope.required_role.clone(),
        })
    }
}

/// One RSA signing key the caller trusts for the issuer, as a JWKS entry
/// spells it.
pub(crate) struct TrustedRsaKey {
    kid: String,
    key: DecodingKey,
}

impl TrustedRsaKey {
    /// From the key id and the base64url modulus and exponent. Only 65537
    /// (`AQAB`) is accepted as exponent. Keys under 2048 bits are refused
    /// here and again by AWS-LC when verifying.
    pub(crate) fn from_components(
        kid: &str,
        modulus: &str,
        exponent: &str,
    ) -> Result<TrustedRsaKey, VerifyError> {
        if !is_visible_ascii(kid, 128)
            || !MODULUS_LENGTHS.contains(&modulus.len())
            || !is_base64url(modulus)
            || exponent != EXPONENT
        {
            return Err(VerifyError::BadKeyRing);
        }
        let key = DecodingKey::from_rsa_components(modulus, exponent)
            .map_err(|_| VerifyError::BadKeyRing)?;
        Ok(TrustedRsaKey {
            kid: kid.to_owned(),
            key,
        })
    }
}

/// The keys trusted for exactly one issuer.
pub(crate) struct TrustedKeyRing {
    issuer: String,
    keys: Vec<TrustedRsaKey>,
}

impl TrustedKeyRing {
    pub(crate) fn new(
        issuer: &str,
        keys: Vec<TrustedRsaKey>,
    ) -> Result<TrustedKeyRing, VerifyError> {
        let kids: HashSet<&str> =
            keys.iter().map(|key| key.kid.as_str()).collect();
        if keys.is_empty() || keys.len() > MAX_KEYS || kids.len() != keys.len()
        {
            return Err(VerifyError::BadKeyRing);
        }
        Ok(TrustedKeyRing {
            issuer: issuer.to_owned(),
            keys,
        })
    }

    fn find(&self, kid: &str) -> Option<&DecodingKey> {
        self.keys
            .iter()
            .find(|key| key.kid == kid)
            .map(|key| &key.key)
    }
}

/// The only JOSE header members accepted; duplicates are refused.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JoseHeader {
    alg: String,
    kid: Option<String>,
    typ: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn values(&self) -> &[String] {
        match self {
            Audience::One(value) => std::slice::from_ref(value),
            Audience::Many(values) => values,
        }
    }
}

/// Hasura's claims, as the platform's mappers add them.
#[derive(Clone, Deserialize, PartialEq, Eq)]
struct HasuraClaims {
    #[serde(rename = "x-hasura-tenant-id")]
    tenant_id: String,
    #[serde(rename = "x-hasura-allowed-roles")]
    allowed_roles: Vec<String>,
    #[serde(rename = "x-hasura-default-role")]
    default_role: String,
    #[serde(rename = "x-hasura-user-id")]
    user_id: Option<String>,
}

/// The claims read; others are ignored, `acr` and `amr` included.
/// Duplicates of these are refused.
#[derive(Clone, Deserialize)]
struct Claims {
    iss: String,
    aud: Audience,
    azp: String,
    sub: String,
    typ: String,
    iat: UnixSeconds,
    exp: UnixSeconds,
    nbf: Option<UnixSeconds>,
    auth_time: Option<UnixSeconds>,
    sid: Option<String>,
    preferred_username: Option<String>,
    trustee: Option<String>,
    #[serde(rename = "https://hasura.io/jwt/claims")]
    hasura: Option<HasuraClaims>,
    /// Read only under a flow proof, and only from signed tokens.
    amr: Option<serde_json::Value>,
    /// The device request's nonce, as the IdP signed it back.
    nonce: Option<String>,
    /// The ID token's hash of its access token.
    at_hash: Option<String>,
}

/// Verifies device grant token pairs for one machine's trustee.
pub(crate) struct NativeVerifier {
    policy: ClaimPolicy,
    keys: TrustedKeyRing,
    /// The machine this verifier settles, and no other.
    owner: Owner,
    second_factor: SecondFactorPolicy,
}

impl NativeVerifier {
    /// A verifier for `machine`, expecting exactly its scope, with keys
    /// trusted for exactly its issuer.
    pub(crate) fn for_machine(
        machine: &DeviceLogin,
        keys: TrustedKeyRing,
    ) -> Result<NativeVerifier, VerifyError> {
        let policy = ClaimPolicy::from_scope(&machine.scope)?;
        if keys.issuer != policy.issuer {
            return Err(VerifyError::BadKeyRing);
        }
        Ok(NativeVerifier {
            policy,
            keys,
            owner: machine.owner.clone(),
            second_factor: SecondFactorPolicy::Unproven,
        })
    }

    /// A verifier for `machine` that trusts the dedicated flow's signed
    /// `amr` under `proof`, once the proof is bound to the machine's issuer
    /// and client. Private, and its proof has no constructor outside the
    /// tests: unreachable at runtime.
    fn with_flow_proof(
        machine: &DeviceLogin,
        keys: TrustedKeyRing,
        proof: FlowPolicyProof,
    ) -> Result<NativeVerifier, VerifyError> {
        let mut verifier = NativeVerifier::for_machine(machine, keys)?;
        let bound = proof.bind(&verifier.policy)?;
        verifier.second_factor = SecondFactorPolicy::SignedFlowAmr(bound);
        Ok(verifier)
    }

    /// The terminal settlement of one successful bound poll: consumes its
    /// sealed result, opened only here, and settles the poll its own ticket
    /// holds with the pair that poll fetched and the nonce of its own
    /// device request. No caller hands in tokens, a ticket or a nonce. A
    /// result offered to another machine is handed back unspent, unopened
    /// beyond its ticket's machine identity; every other refusal spends it.
    pub(crate) fn settle_bound<R: BoundPollResult>(
        &self,
        machine: &mut DeviceLogin,
        clock: Clock,
        result: R,
    ) -> Settlement<R> {
        let opener = Opener(());
        if !is_own_ticket(machine, result.ticket(&opener)) {
            return Settlement::WrongMachine(result);
        }
        let parts = result.open(&opener);
        Settlement::Settled(self.settle_pair(
            machine,
            clock,
            parts.ticket,
            (parts.access_token, parts.id_token),
            parts.nonce,
        ))
    }

    /// Settles the poll `ticket` holds on `machine` with the pair that poll
    /// fetched: approved with the verified login, which the model then
    /// judges, or denied when any check fails. A verifier made for another
    /// machine verifies nothing and denies.
    /// `device_nonce` is the nonce of the pending device request this poll
    /// belongs to; tokens not issued for it are denied. Private to this
    /// module and its children: everyone else settles a sealed result.
    fn settle_pair(
        &self,
        machine: &mut DeviceLogin,
        clock: Clock,
        ticket: &PollTicket,
        tokens: (&str, &str),
        device_nonce: &DeviceNonce,
    ) -> Result<DeviceStatus, DeviceError> {
        self.settle_pair_with(
            machine,
            clock,
            ticket,
            tokens,
            device_nonce,
            None,
        )
    }

    /// As `settle_pair`, with the evidence a proven admission needs. Private
    /// and reachable only from the tests: no evidence exists at runtime.
    fn settle_pair_admitted(
        &self,
        machine: &mut DeviceLogin,
        clock: Clock,
        ticket: &PollTicket,
        tokens: (&str, &str),
        device_nonce: &DeviceNonce,
        admission: &Admission,
    ) -> Result<DeviceStatus, DeviceError> {
        self.settle_pair_with(
            machine,
            clock,
            ticket,
            tokens,
            device_nonce,
            Some(admission),
        )
    }

    /// Whether settling the machine's pending poll with this pair and
    /// evidence would admit it, without settling anything: the same checks,
    /// at the same clock. Private and test-only, like the evidence.
    fn try_admission(
        &self,
        machine: &DeviceLogin,
        clock: Clock,
        tokens: (&str, &str),
        device_nonce: &DeviceNonce,
        admission: &Admission,
    ) -> Result<(), VerifyError> {
        let (started, challenge) = match &machine.state {
            State::Pending(pending) => {
                (pending.started_wall, pending.challenge)
            }
            _ => return Err(VerifyError::Binding),
        };
        if !Arc::ptr_eq(&self.owner.0, &machine.owner.0) {
            return Err(VerifyError::BadPolicy);
        }
        self.verify_pair_with(
            tokens.0,
            tokens.1,
            started,
            clock.wall,
            device_nonce,
            Some((admission, challenge)),
        )
        .map(|_| ())
    }

    fn settle_pair_with(
        &self,
        machine: &mut DeviceLogin,
        clock: Clock,
        ticket: &PollTicket,
        tokens: (&str, &str),
        device_nonce: &DeviceNonce,
        admission: Option<&Admission>,
    ) -> Result<DeviceStatus, DeviceError> {
        let (access_token, id_token) = tokens;
        machine.claim(ticket)?;
        let (started, challenge) = match &machine.state {
            State::Pending(pending) => {
                (pending.started_wall, pending.challenge)
            }
            _ => return Err(DeviceError::NoChallenge),
        };
        let verified = if Arc::ptr_eq(&self.owner.0, &machine.owner.0) {
            // The challenge the evidence must be bound to is the machine's.
            self.verify_pair_with(
                access_token,
                id_token,
                started,
                clock.wall,
                device_nonce,
                admission.map(|admission| (admission, challenge)),
            )
        } else {
            Err(VerifyError::BadPolicy)
        };
        let outcome = match verified {
            Ok(login) => PollOutcome::Approved(login),
            Err(_) => PollOutcome::AccessDenied,
        };
        machine.settle(clock, ticket, outcome)
    }

    /// The login an access and ID token pair proves at wall time `now`,
    /// for a challenge started at wall time `challenge_started`, with no
    /// admission evidence: never a proven factor.
    fn verify_pair(
        &self,
        access_token: &str,
        id_token: &str,
        challenge_started: UnixSeconds,
        now: UnixSeconds,
        device_nonce: &DeviceNonce,
    ) -> Result<VerifiedLogin, VerifyError> {
        self.verify_pair_with(
            access_token,
            id_token,
            challenge_started,
            now,
            device_nonce,
            None,
        )
    }

    /// The admission checks of a proven factor, after the tokens' own:
    /// - every piece of evidence answers a request of this challenge: the
    ///   flow readback and the later factors one after-tokens request, the
    ///   earlier factors a different before-authentication request;
    /// - the flow readback was made strictly after both tokens' `iat`
    ///   (`iat` has whole seconds, so the same second is not after),
    ///   accepted, recent and not in the future, of exactly the installed
    ///   receipt;
    /// - the earlier factors were observed at or after the challenge began
    ///   and strictly before the authentication: one OTP credential,
    ///   created before the challenge, and no pending required action, so
    ///   an enrollment inside this run never counts;
    /// - the later factors, observed strictly after both `iat`: the same
    ///   credential, still no pending action.
    ///
    /// The transaction's identity (signed `nonce` and `at_hash`) is checked
    /// before, for every settlement; it is no factor evidence.
    fn check_admission(
        &self,
        bound: &BoundFlowProof,
        admission: &Admission,
        challenge: ChallengeId,
        pair: &IssuedPair,
        challenge_started: UnixSeconds,
        now: UnixSeconds,
    ) -> Result<(), VerifyError> {
        let readback = admission.readback;
        let (before, after) = (admission.before, admission.after);
        let bound_to = |binding: &RequestBinding, phase: Phase| {
            binding.phase == phase
                && binding.challenge == challenge
                && is_request_nonce(&binding.nonce)
        };
        let Some(after_binding) = &readback.binding else {
            return Err(VerifyError::Binding);
        };
        if !bound_to(after_binding, Phase::AfterTokens)
            || after.binding != *after_binding
            || !bound_to(&before.binding, Phase::BeforeAuthentication)
            || before.binding.nonce == after_binding.nonce
        {
            return Err(VerifyError::Binding);
        }
        if readback.readback != Readback::Accepted
            || readback.checked_at > now
            || now - readback.checked_at > READBACK_MAX_AGE
        {
            return Err(VerifyError::FlowDrift);
        }
        let latest = pair
            .issued
            .iter()
            .copied()
            .max()
            .unwrap_or(UnixSeconds::MAX);
        if readback.checked_at < latest {
            return Err(VerifyError::ReadbackBeforeTokens);
        }
        if readback.checked_at == latest {
            return Err(VerifyError::ReadbackSameSecond);
        }
        let receipt = &bound.proof.receipt;
        if readback.observed != *receipt
            || readback.observed.fingerprint() != receipt.fingerprint()
            || readback.observed.qualifying_max_age(&self.policy)
                != Some(bound.max_age)
        {
            return Err(VerifyError::FlowDrift);
        }
        if before.subject != self.policy.subject
            || after.subject != self.policy.subject
        {
            return Err(VerifyError::Subject);
        }
        let [credential] = before.otp_credentials.as_slice() else {
            return Err(VerifyError::FactorNotPreexisting);
        };
        if before.observed_at < challenge_started
            || before.observed_at >= pair.auth_time
            || !before.pending_actions.is_empty()
            || credential.created_at >= challenge_started
        {
            return Err(VerifyError::FactorNotPreexisting);
        }
        if after.observed_at > now || after.observed_at < latest {
            return Err(VerifyError::ReadbackBeforeTokens);
        }
        if after.observed_at == latest {
            return Err(VerifyError::ReadbackSameSecond);
        }
        if after.otp_credentials != before.otp_credentials
            || !after.pending_actions.is_empty()
        {
            return Err(VerifyError::FactorChanged);
        }
        Ok(())
    }

    /// The transaction's identity, for every settlement, proven or not: the
    /// tokens were issued for this pending device request and belong
    /// together. The ID token carries, signed, exactly the nonce sent with
    /// that request (and so does the access token if it carries one; stock
    /// Keycloak's does not), and its signed `at_hash` is that of these exact
    /// access token bytes, so no other signed access token can stand in.
    /// The user session may be an earlier SSO session the device login
    /// reused, so the session proves nothing here. This identifies the
    /// transaction; it is not evidence of any factor.
    fn check_transaction(
        access_token: &str,
        access: &Claims,
        id: &Claims,
        device_nonce: &DeviceNonce,
    ) -> Result<(), VerifyError> {
        let nonce = device_nonce.0.as_str();
        if !is_request_nonce(nonce)
            || id.nonce.as_deref() != Some(nonce)
            || access.nonce.as_deref().is_some_and(|other| other != nonce)
        {
            return Err(VerifyError::DeviceNonce);
        }
        if id.at_hash.as_deref()
            != Some(rs256_access_hash(access_token).as_str())
        {
            return Err(VerifyError::AccessHash);
        }
        Ok(())
    }

    fn verify_pair_with(
        &self,
        access_token: &str,
        id_token: &str,
        challenge_started: UnixSeconds,
        now: UnixSeconds,
        device_nonce: &DeviceNonce,
        admission: Option<(&Admission, ChallengeId)>,
    ) -> Result<VerifiedLogin, VerifyError> {
        let access = self.verified_claims(access_token)?;
        let id = self.verified_claims(id_token)?;
        self.check_common(&access, now)?;
        self.check_common(&id, now)?;
        Self::check_transaction(access_token, &access, &id, device_nonce)?;
        if access.typ != ACCESS_TYPE || id.typ != ID_TYPE {
            return Err(VerifyError::TokenType);
        }
        if !access
            .aud
            .values()
            .iter()
            .any(|aud| *aud == self.policy.access_audience)
            || id.aud.values() != [self.policy.client_id.as_str()]
        {
            return Err(VerifyError::Audience);
        }
        let session_id = match (&access.sid, &id.sid) {
            (Some(access_sid), Some(id_sid))
                if access_sid == id_sid
                    && is_visible_ascii(access_sid, 255) =>
            {
                access_sid.clone()
            }
            _ => return Err(VerifyError::Session),
        };
        let auth_time = id.auth_time.ok_or(VerifyError::AuthTimeMissing)?;
        if access.auth_time.is_some_and(|time| time != auth_time) {
            return Err(VerifyError::Mismatch);
        }
        if auth_time > access.iat || auth_time > id.iat {
            return Err(VerifyError::Malformed);
        }
        if auth_time < challenge_started {
            return Err(VerifyError::AuthTimeStale);
        }
        let hasura = access.hasura.as_ref().ok_or(VerifyError::Tenant)?;
        self.check_hasura(hasura)?;
        if id.hasura.as_ref().is_some_and(|claims| claims != hasura) {
            return Err(VerifyError::Mismatch);
        }
        let name = self.policy.trustee_name.as_str();
        if access.trustee.as_deref() != Some(name)
            || id.trustee.as_deref().is_some_and(|other| other != name)
        {
            return Err(VerifyError::Member);
        }
        let mfa = match &self.second_factor {
            SecondFactorPolicy::Unproven => MfaEvidence::Absent,
            SecondFactorPolicy::SignedFlowAmr(bound) => {
                if !FlowPolicyProof::amr_exact(access.amr.as_ref())
                    || !FlowPolicyProof::amr_exact(id.amr.as_ref())
                {
                    return Err(VerifyError::AmrMismatch);
                }
                // `auth_time` is at or before both `iat`, checked above.
                let max_age = bound.max_age;
                if access.iat - auth_time > max_age
                    || id.iat - auth_time > max_age
                {
                    return Err(VerifyError::MfaStale);
                }
                let (admission, challenge) =
                    admission.ok_or(VerifyError::NoAdmissionEvidence)?;
                let pair = IssuedPair {
                    issued: [access.iat, id.iat],
                    auth_time,
                };
                self.check_admission(
                    bound,
                    admission,
                    challenge,
                    &pair,
                    challenge_started,
                    now,
                )?;
                MfaEvidence::Verified(FreshMfa {
                    factor: SecondFactor::Totp,
                    verified_at: auth_time,
                })
            }
        };
        Ok(VerifiedLogin {
            issuer: access.iss.clone(),
            audiences: access.aud.values().to_vec(),
            authorized_party: access.azp.clone(),
            subject: access.sub.clone(),
            tenant: hasura.tenant_id.clone(),
            trustee_name: self.policy.trustee_name.clone(),
            roles: hasura.allowed_roles.clone(),
            issued_at: access.iat,
            expires_at: access.exp.min(id.exp),
            session_expires_at: None,
            session_id,
            auth_time,
            mfa,
        })
    }

    /// The key a token's header selects, once the header is only an RS256
    /// `alg`, a known `kid` and at most `typ: JWT`.
    fn header_key(&self, token: &str) -> Result<&DecodingKey, VerifyError> {
        let encoded = token.split('.').next().unwrap_or_default();
        let raw = decode_base64url(encoded).ok_or(VerifyError::Header)?;
        let members: serde_json::Map<String, serde_json::Value> =
            serde_json::from_slice(&raw).map_err(|_| VerifyError::Header)?;
        if members.contains_key("crit") {
            return Err(VerifyError::CriticalExtension);
        }
        if KEY_SOURCES.iter().any(|name| members.contains_key(*name)) {
            return Err(VerifyError::KeyDiscovery);
        }
        let header: JoseHeader =
            serde_json::from_slice(&raw).map_err(|_| VerifyError::Header)?;
        if header.alg != "RS256" {
            return Err(VerifyError::Algorithm);
        }
        if header.typ.as_deref().is_some_and(|typ| typ != "JWT") {
            return Err(VerifyError::Header);
        }
        header
            .kid
            .as_deref()
            .and_then(|kid| self.keys.find(kid))
            .ok_or(VerifyError::UnknownKey)
    }

    /// The claims of a token whose RS256 signature a trusted key verifies.
    fn verified_claims(&self, token: &str) -> Result<Claims, VerifyError> {
        if !is_visible_ascii(token, TOKEN_LIMIT)
            || token.bytes().filter(|b| *b == b'.').count() != 2
        {
            return Err(VerifyError::TokenShape);
        }
        let key = self.header_key(token)?;
        // The signature only: every claim is checked afterwards, against
        // the explicit clock.
        let mut validation = Validation::new(Algorithm::RS256);
        validation.algorithms = vec![Algorithm::RS256];
        validation.leeway = 0;
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.validate_aud = false;
        validation.required_spec_claims = HashSet::new();
        jsonwebtoken::decode::<Claims>(token, key, &validation)
            .map(|data| data.claims)
            .map_err(|err| match err.kind() {
                ErrorKind::Json(_) => VerifyError::Malformed,
                _ => VerifyError::Signature,
            })
    }

    /// The rules both tokens share.
    fn check_common(
        &self,
        claims: &Claims,
        now: UnixSeconds,
    ) -> Result<(), VerifyError> {
        let policy = &self.policy;
        if claims.iss != policy.issuer {
            return Err(VerifyError::Issuer);
        }
        if claims.azp != policy.client_id {
            return Err(VerifyError::AuthorizedParty);
        }
        if claims.sub != policy.subject {
            return Err(VerifyError::Subject);
        }
        if claims.iat > now {
            return Err(VerifyError::IssuedInFuture);
        }
        if claims.nbf.is_some_and(|nbf| nbf > now) {
            return Err(VerifyError::NotYetValid);
        }
        if claims.exp <= now {
            return Err(VerifyError::Expired);
        }
        if claims.exp <= claims.iat
            || claims.exp - claims.iat > MAX_GRANT_LIFETIME
        {
            return Err(VerifyError::Lifetime);
        }
        if claims
            .preferred_username
            .as_deref()
            .is_some_and(|name| name != policy.trustee_name)
        {
            return Err(VerifyError::Member);
        }
        Ok(())
    }

    /// Exactly the configured tenant and the one configured role.
    fn check_hasura(&self, hasura: &HasuraClaims) -> Result<(), VerifyError> {
        let policy = &self.policy;
        if hasura.tenant_id != policy.tenant {
            return Err(VerifyError::Tenant);
        }
        if hasura
            .user_id
            .as_deref()
            .is_some_and(|user| user != policy.subject)
        {
            return Err(VerifyError::Subject);
        }
        if hasura.allowed_roles != [policy.role.as_str()]
            || hasura.default_role != policy.role
        {
            return Err(VerifyError::Role);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/support/device_verify_tests.rs"]
pub(crate) mod tests;

/// The inert Keycloak policy observation seam: a child, so it alone can
/// build readback evidence. Wired to nothing.
#[cfg(test)]
#[path = "device_policy_keycloak.rs"]
mod keycloak_policy;
