// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An inert observation seam: turns one policy observation of the pinned
//! Keycloak 26.7.5 realm into the verifier's typed [`FlowReceipt`], as
//! readback evidence ([`DriftEvidence`]) for a [`super::FlowPolicyProof`].
//!
//! It is a test-only child of the verifier and is wired to nothing: the
//! device verifier made for a machine stays `Unproven`.
//!
//! # Where an observation comes from
//!
//! The runner never holds a Keycloak admin secret, a privileged client or a
//! network caller for the admin API. A separate trusted collector, which
//! owns those credentials, reads the realm and hands the runner a closed,
//! versioned envelope (`sequent.keycloak-device-policy-observation.v1`)
//! over an already authenticated local channel. That envelope format is
//! stipulated here: no Keycloak representation source is staged in this
//! tree, so this parser does not claim to read raw Admin API JSON. The
//! collector maps the Admin API into it; only the authenticator provider
//! ids and the `default.reference.value` / `default.reference.maxAge`
//! config keys are Keycloak's own names, and any other id is kept as
//! unknown, which disqualifies the flow.
//!
//! # What is required of an observation
//!
//! - It is bound to the expected issuer by its authenticated channel, and
//!   names the expected issuer, realm id (a UUID) and realm name.
//! - Each section (clients, flows, mappers) is `complete`: a `partial` or
//!   `read_denied` section is never evidence.
//! - The schema is closed: unknown members, unknown enum values and
//!   duplicates are refused, and sizes are bounded.
//! - The client inventory names the device client once, and every client's
//!   effective browser flow (its override, or the realm's browser flow when
//!   it inherits) is resolved, so every client bound to the dedicated flow
//!   is known.
//! - The dedicated flow appears once; its executions are in strictly
//!   increasing priority and, when there are two, at exactly the installed
//!   priorities.
//! - Every protocol mapper effective for the device client is listed. The
//!   AMR mapper is known by its provider id (`oidc-amr-mapper`, as in the
//!   staged `AmrProtocolMapper`) alone; any other mapper configured to
//!   write the `amr` claim is recorded as a foreign writer.
//! - The realm's OTP policy and every registered required action (alias,
//!   provider id, enabled, default action, priority, as the staged
//!   `RequiredActionProviderRepresentation` carries them) are listed.
//! - When asked for, the trustee's own factors: OTP credential ids and
//!   creation times, and pending required actions, for exactly one user.
//!
//! The result is evidence only: whether it qualifies, and whether it is the
//! installed shape, is decided by the verifier's own binding. Roles are
//! not observed here; they are checked from signed claims.
//!
//! # Binding to one transaction
//!
//! Each observation answers one [`ObservationRequest`]: a fresh 32-byte
//! nonce, a phase (before the authentication, or after the tokens) and the
//! machine's pending challenge. The collector echoes all three inside its
//! authenticated reply body; a reply to any other request is refused.
//! [`observe_before`] and [`settle_observed`] are the seam a poll loop
//! would use; they ask the [`AuthenticatedCollector`] and hand the bound
//! evidence to the verifier, which alone decides.
//!
//! A reply stamped in the tokens' own whole second is not yet after them:
//! it is never admitted, but for tokens of this transaction (this device
//! request's signed `nonce` and matching `at_hash`, checked first)
//! [`settle_observed`] keeps the poll pending ([`Observed::Retained`])
//! instead of closing an otherwise legitimate challenge, and a fresh
//! request answered in a later second decides. Tokens of another device
//! request are denied at once, never retained. Every other outcome settles
//! at once, and the challenge's expiry still ends it.
//!
//! # Still to be measured or built
//!
//! - The stock fixture showed a device login after an ordinary SSO login
//!   reuses that SSO session's `sid`, so a session is not evidence and is
//!   not read. The tokens are bound to this device request instead (see
//!   the verifier): the pinned stock Keycloak 26.7.5 fixture measured the
//!   ID token's signed `nonce` exactly equal to the device request's, no
//!   access token `nonce`, and a signed `at_hash` matching the exact access
//!   token bytes. That is the transaction's identity, not factor evidence.
//! - The pending device grant (device code, PKCE verifier, nonce) is bound,
//!   as its challenge begins, to that machine and challenge
//!   (`DeviceLogin::begin_bound`, transport `BoundDeviceGrant`); it polls
//!   only with that challenge's tickets. Inert and test-only like the rest:
//!   no runtime caller begins a bound challenge yet.
//! - Which executions approved this device code is not recorded per device
//!   in stock Keycloak: approval stores only the user session's id, and
//!   `amr`/`auth_time` belong to that user session. Binding the executions
//!   to the device code needs a server extension; none is built.
//! - No live collector, and no realm or interval lock between the two
//!   readbacks, exists yet.
//!
//! # Prerequisites another worker must provide
//!
//! - The installer's receipt: [`ExpectedInstall`] (issuer, realm id,
//!   client id, flow alias, the two execution priorities) and the
//!   installed [`FlowReceipt`], from the root installer's protected record.
//! - The authenticated receiver: an [`AuthenticatedObservation`] built only
//!   from a collector reply whose channel authenticated the collector for
//!   exactly that issuer, including a realm lock readback.
//!
//! Neither has a constructor outside the tests.

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

use aws_lc_rs::rand::{SecureRandom, SystemRandom};

use super::{
    is_uuid, is_visible_ascii, Admission, AmrMapperShape, BoundPollResult,
    ChallengeId, ClaimPolicy, ClientAccess, ClientShape, Clock, DeviceError,
    DeviceLogin, DeviceStatus, DriftEvidence, ExecutionShape, FlowProvider,
    FlowReceipt, FlowShape, Grant, MapperHost, MapperProvider, NativeVerifier,
    Opener, OtpAlgorithm, OtpCredentialShape, OtpKind, OtpPolicyShape, Phase,
    Readback, RequestBinding, RequiredActionShape, Requirement, State, Step,
    SubjectFactors, UnixSeconds, PASSWORD_REFERENCE, TOTP_REFERENCE,
};

const SCHEMA: &str = "sequent.keycloak-device-policy-observation.v1";
const BODY_LIMIT: usize = 256 * 1024;
const MAX_CLIENTS: usize = 512;
const MAX_FLOWS: usize = 128;
const MAX_EXECUTIONS: usize = 64;
const MAX_MAPPERS: usize = 1_024;
const MAX_REQUIRED_ACTIONS: usize = 64;
const MAX_CREDENTIALS: usize = 16;
const NAME_LIMIT: usize = 255;
const REFERENCE_LIMIT: usize = 512;
/// Keycloak's authenticator provider ids for the steps the flow may hold.
const PASSWORD_FORM: &str = "auth-username-password-form";
const OTP_FORM: &str = "auth-otp-form";
const COOKIE: &str = "auth-cookie";
const IDP_REDIRECTOR: &str = "identity-provider-redirector";
/// Keycloak's AMR protocol mapper.
const AMR_MAPPER: &str = "oidc-amr-mapper";

/// Why an observation is not evidence. Names the check, never a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObservationError {
    TooLarge,
    /// Not the closed schema: unknown or duplicate members, wrong types.
    Schema,
    /// Contradictory or out-of-range values.
    Malformed,
    /// The channel was not authenticated for the expected issuer.
    Attestation,
    Issuer,
    Realm,
    /// A section was only partly read.
    Incomplete,
    /// A section could not be read.
    ReadDenied,
    /// The device client or the dedicated flow is missing or repeated.
    Inventory,
    /// The two executions are not at the installed priorities, or the
    /// executions are not in strictly increasing priority.
    Priority,
    /// The reply does not answer this request: another nonce, phase or
    /// challenge, or no nonce could be made.
    Binding,
}

/// A request for one observation, bound to a challenge and phase by a
/// fresh opaque nonce the collector must echo inside its authenticated
/// reply.
pub(crate) struct ObservationRequest {
    binding: RequestBinding,
}

/// Unpadded base64url.
fn base64url(bytes: &[u8]) -> String {
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

impl ObservationRequest {
    /// A request for `phase` of `challenge`, with a nonce of 32 bytes from
    /// the system's random source. In this test-only module that source is
    /// AWS-LC; a runtime one needs a reviewed dependency.
    fn new(
        phase: Phase,
        challenge: ChallengeId,
    ) -> Result<Self, ObservationError> {
        let mut bytes = [0u8; 32];
        SystemRandom::new()
            .fill(&mut bytes)
            .map_err(|_| ObservationError::Binding)?;
        Ok(ObservationRequest {
            binding: RequestBinding {
                nonce: base64url(&bytes),
                phase,
                challenge,
            },
        })
    }
}

/// What the installer recorded and the observation must match.
pub(crate) struct ExpectedInstall {
    issuer: String,
    realm_id: String,
    client_id: String,
    flow_alias: String,
    /// The password and OTP executions' priorities.
    priorities: [i64; 2],
}

/// One collector reply, received over a channel authenticated for
/// `issuer_binding`. No constructor outside the tests.
pub(crate) struct AuthenticatedObservation {
    issuer_binding: String,
    body: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    schema: String,
    /// The request this reply answers, echoed.
    request: RequestWire,
    collector: CollectorWire,
    realm: RealmWire,
    required_actions: Section<RequiredActionWire>,
    clients: Section<ClientWire>,
    flows: Section<FlowWire>,
    mappers: Section<MapperWire>,
    /// The trustee's own factors, when the observation was asked for them.
    subject: Option<SubjectWire>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OtpKindWire {
    Totp,
    Hotp,
}

#[derive(Clone, Copy, Deserialize)]
enum OtpAlgorithmWire {
    #[serde(rename = "HmacSHA1")]
    HmacSha1,
    #[serde(rename = "HmacSHA256")]
    HmacSha256,
    #[serde(rename = "HmacSHA512")]
    HmacSha512,
}

/// The realm's OTP policy.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OtpPolicyWire {
    kind: OtpKindWire,
    algorithm: OtpAlgorithmWire,
    digits: u8,
    period: u32,
    look_ahead_window: u8,
    code_reusable: bool,
}

/// One registered required action, from `RequiredActionProviderRepresentation`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequiredActionWire {
    alias: String,
    provider_id: String,
    enabled: bool,
    default_action: bool,
    priority: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialWire {
    id: String,
    created_at: UnixSeconds,
}

/// The trustee's own factors: OTP credentials and pending required actions.
/// User sessions are not read: the stock device login reuses an earlier SSO
/// session, so a session's shape proves nothing about this device request.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SubjectWire {
    completeness: Completeness,
    user_id: String,
    otp_credentials: Vec<CredentialWire>,
    pending_required_actions: Vec<String>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PhaseWire {
    BeforeAuthentication,
    AfterTokens,
}

/// The request a reply answers.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestWire {
    nonce: String,
    phase: PhaseWire,
    challenge: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectorWire {
    observed_at: UnixSeconds,
    keycloak_release: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RealmWire {
    issuer: String,
    id: String,
    name: String,
    /// The realm's browser flow binding, inherited by clients without an
    /// override.
    browser_flow: String,
    otp_policy: OtpPolicyWire,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Completeness {
    Complete,
    Partial,
    ReadDenied,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Section<T> {
    completeness: Completeness,
    items: Vec<T>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GrantWire {
    DeviceAuthorization,
    StandardFlow,
    DirectAccess,
    Implicit,
    ServiceAccounts,
    TokenExchange,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientWire {
    client_id: String,
    public_client: bool,
    bearer_only: bool,
    grants: Vec<GrantWire>,
    browser_flow_override: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
enum RequirementWire {
    #[serde(rename = "REQUIRED")]
    Required,
    #[serde(rename = "ALTERNATIVE")]
    Alternative,
    #[serde(rename = "CONDITIONAL")]
    Conditional,
    #[serde(rename = "DISABLED")]
    Disabled,
}

/// Keycloak's authenticator config keys for an execution's AMR reference.
const REFERENCE_VALUE: &str = "default.reference.value";
const REFERENCE_MAX_AGE: &str = "default.reference.maxAge";
const MAX_CONFIG_ENTRIES: usize = 64;

/// An authenticator config, as Keycloak stores it: string keys to string
/// values. Stock flows carry other keys; only the dedicated flow's must be
/// exactly the two reference keys. A repeated key is refused.
struct ConfigWire(BTreeMap<String, String>);

impl<'de> Deserialize<'de> for ConfigWire {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct Entries;
        impl<'de> serde::de::Visitor<'de> for Entries {
            type Value = ConfigWire;
            fn expecting(
                &self,
                f: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                f.write_str("an authenticator config of string entries")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<ConfigWire, A::Error> {
                let mut entries = BTreeMap::new();
                while let Some((key, value)) =
                    map.next_entry::<String, String>()?
                {
                    if entries.len() >= MAX_CONFIG_ENTRIES
                        || entries.insert(key, value).is_some()
                    {
                        return Err(serde::de::Error::custom(
                            "repeated or too many config entries",
                        ));
                    }
                }
                Ok(ConfigWire(entries))
            }
        }
        deserializer.deserialize_map(Entries)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionWire {
    priority: i64,
    requirement: RequirementWire,
    /// The authenticator provider id; absent for a nested flow.
    authenticator: Option<String>,
    /// The nested flow's alias; absent for an authenticator.
    subflow_alias: Option<String>,
    config: Option<ConfigWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FlowWire {
    alias: String,
    top_level: bool,
    provider_id: String,
    executions: Vec<ExecutionWire>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
enum HostWire {
    Client,
    ClientScope(String),
}

/// One protocol mapper effective for a client, from the client itself or
/// one of its client scopes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MapperWire {
    client_id: String,
    provider_id: String,
    host: HostWire,
    claim_name: Option<String>,
    id_token: bool,
    access_token: bool,
}

fn section<T>(
    section: &Section<T>,
    limit: usize,
) -> Result<&[T], ObservationError> {
    match section.completeness {
        Completeness::Complete => {}
        Completeness::Partial => return Err(ObservationError::Incomplete),
        Completeness::ReadDenied => return Err(ObservationError::ReadDenied),
    }
    if section.items.len() > limit {
        return Err(ObservationError::TooLarge);
    }
    Ok(&section.items)
}

fn name(value: &str) -> Result<&str, ObservationError> {
    if is_visible_ascii(value, NAME_LIMIT) {
        Ok(value)
    } else {
        Err(ObservationError::Malformed)
    }
}

/// A flow alias. Keycloak's own aliases hold inner spaces (its stock
/// nested browser flow is `browser forms`), so unlike other names an alias
/// may: printable ASCII and single inner spaces, never leading, trailing
/// or doubled, never a control or non-ASCII character.
fn alias(value: &str) -> Result<&str, ObservationError> {
    let printable = value.bytes().all(|b| b == b' ' || b.is_ascii_graphic());
    if value.is_empty()
        || value.len() > NAME_LIMIT
        || !printable
        || value.starts_with(' ')
        || value.ends_with(' ')
        || value.contains("  ")
    {
        return Err(ObservationError::Malformed);
    }
    Ok(value)
}

fn grant(wire: GrantWire) -> Grant {
    match wire {
        GrantWire::DeviceAuthorization => Grant::DeviceAuthorization,
        GrantWire::StandardFlow => Grant::StandardFlow,
        GrantWire::DirectAccess => Grant::DirectAccess,
        GrantWire::Implicit => Grant::Implicit,
        GrantWire::ServiceAccounts => Grant::ServiceAccounts,
        GrantWire::TokenExchange => Grant::TokenExchange,
    }
}

fn requirement(wire: RequirementWire) -> Requirement {
    match wire {
        RequirementWire::Required => Requirement::Required,
        RequirementWire::Alternative => Requirement::Alternative,
        RequirementWire::Conditional => Requirement::Conditional,
        RequirementWire::Disabled => Requirement::Disabled,
    }
}

fn execution(wire: &ExecutionWire) -> Result<ExecutionShape, ObservationError> {
    let step = match (&wire.authenticator, &wire.subflow_alias) {
        (Some(provider), None) => match name(provider)? {
            PASSWORD_FORM => Step::UsernamePasswordForm,
            OTP_FORM => Step::OtpForm,
            COOKIE => Step::Cookie,
            IDP_REDIRECTOR => Step::IdentityProviderRedirector,
            other => Step::Unknown(other.to_owned()),
        },
        (None, Some(nested)) => Step::Subflow(alias(nested)?.to_owned()),
        _ => return Err(ObservationError::Malformed),
    };
    let (amr_reference, amr_max_age) = match &wire.config {
        None => (None, None),
        Some(ConfigWire(config)) => {
            // The dedicated flow's configs hold the two reference keys only.
            let (Some(reference), Some(max_age), 2) = (
                config.get(REFERENCE_VALUE),
                config.get(REFERENCE_MAX_AGE),
                config.len(),
            ) else {
                return Err(ObservationError::Malformed);
            };
            if !is_visible_ascii(reference, REFERENCE_LIMIT)
                || max_age.is_empty()
                || max_age.len() > 6
                || !max_age.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(ObservationError::Malformed);
            }
            let age =
                max_age.parse().map_err(|_| ObservationError::Malformed)?;
            (Some(reference.clone()), Some(age))
        }
    };
    Ok(ExecutionShape {
        step,
        requirement: requirement(wire.requirement),
        amr_reference,
        amr_max_age,
    })
}

/// A sort key for mappers, so that the observed order never matters.
fn mapper_key(mapper: &AmrMapperShape) -> (String, String, bool, bool) {
    let provider = match &mapper.provider {
        MapperProvider::OidcAmr => AMR_MAPPER.to_owned(),
        MapperProvider::Other(id) => format!("other:{id}"),
    };
    let host = match &mapper.host {
        MapperHost::Client => "client".to_owned(),
        MapperHost::ClientScope(scope) => format!("scope:{scope}"),
    };
    (provider, host, mapper.id_token, mapper.access_token)
}

/// The envelope of an authenticated observation for the expected issuer.
fn envelope(
    expected: &ExpectedInstall,
    observation: &AuthenticatedObservation,
    request: &ObservationRequest,
) -> Result<Envelope, ObservationError> {
    if observation.body.len() > BODY_LIMIT {
        return Err(ObservationError::TooLarge);
    }
    if observation.issuer_binding != expected.issuer {
        return Err(ObservationError::Attestation);
    }
    let envelope: Envelope = serde_json::from_slice(&observation.body)
        .map_err(|_| ObservationError::Schema)?;
    if envelope.schema != SCHEMA {
        return Err(ObservationError::Schema);
    }
    // The reply answers exactly this request, inside its own body.
    let asked = &request.binding;
    let phase = match envelope.request.phase {
        PhaseWire::BeforeAuthentication => Phase::BeforeAuthentication,
        PhaseWire::AfterTokens => Phase::AfterTokens,
    };
    if envelope.request.nonce != asked.nonce
        || phase != asked.phase
        || envelope.request.challenge != asked.challenge.0
    {
        return Err(ObservationError::Binding);
    }
    let realm = &envelope.realm;
    if realm.issuer != expected.issuer {
        return Err(ObservationError::Issuer);
    }
    if !is_uuid(&realm.id)
        || realm.id != expected.realm_id
        || !realm
            .issuer
            .ends_with(&format!("/realms/{}", name(&realm.name)?))
    {
        return Err(ObservationError::Realm);
    }
    Ok(envelope)
}

/// The trustee's own factors, as one authenticated observation saw them at
/// the collector's observation time: for exactly `subject`, complete.
pub(crate) fn observe_subject(
    expected: &ExpectedInstall,
    observation: &AuthenticatedObservation,
    subject: &str,
    request: &ObservationRequest,
) -> Result<SubjectFactors, ObservationError> {
    let envelope = envelope(expected, observation, request)?;
    let wire = envelope
        .subject
        .as_ref()
        .ok_or(ObservationError::Inventory)?;
    match wire.completeness {
        Completeness::Complete => {}
        Completeness::Partial => return Err(ObservationError::Incomplete),
        Completeness::ReadDenied => return Err(ObservationError::ReadDenied),
    }
    if !is_uuid(&wire.user_id) || wire.user_id != subject {
        return Err(ObservationError::Inventory);
    }
    if wire.otp_credentials.len() > MAX_CREDENTIALS
        || wire.pending_required_actions.len() > MAX_REQUIRED_ACTIONS
    {
        return Err(ObservationError::TooLarge);
    }
    let mut otp_credentials = wire
        .otp_credentials
        .iter()
        .map(|credential| {
            Ok(OtpCredentialShape {
                id: name(&credential.id)?.to_owned(),
                created_at: credential.created_at,
            })
        })
        .collect::<Result<Vec<_>, ObservationError>>()?;
    otp_credentials.sort_by(|a, b| a.id.cmp(&b.id));
    if otp_credentials
        .windows(2)
        .any(|pair| pair[0].id == pair[1].id)
    {
        return Err(ObservationError::Malformed);
    }
    let mut pending_actions = wire
        .pending_required_actions
        .iter()
        .map(|action| name(action).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    pending_actions.sort();
    pending_actions.dedup();
    Ok(SubjectFactors {
        binding: request.binding.clone(),
        subject: wire.user_id.clone(),
        otp_credentials,
        pending_actions,
        observed_at: envelope.collector.observed_at,
    })
}

fn otp_policy(wire: &OtpPolicyWire) -> OtpPolicyShape {
    OtpPolicyShape {
        kind: match wire.kind {
            OtpKindWire::Totp => OtpKind::Totp,
            OtpKindWire::Hotp => OtpKind::Hotp,
        },
        algorithm: match wire.algorithm {
            OtpAlgorithmWire::HmacSha1 => OtpAlgorithm::HmacSha1,
            OtpAlgorithmWire::HmacSha256 => OtpAlgorithm::HmacSha256,
            OtpAlgorithmWire::HmacSha512 => OtpAlgorithm::HmacSha512,
        },
        digits: wire.digits,
        period: wire.period,
        look_ahead: wire.look_ahead_window,
        code_reusable: wire.code_reusable,
    }
}

/// The readback evidence one authenticated, complete observation gives:
/// the installed shape as observed, at the collector's observation time.
pub(crate) fn observe(
    expected: &ExpectedInstall,
    observation: &AuthenticatedObservation,
    request: &ObservationRequest,
) -> Result<DriftEvidence, ObservationError> {
    let envelope = envelope(expected, observation, request)?;
    let realm = &envelope.realm;
    let realm_browser_flow = alias(&realm.browser_flow)?;
    let release = name(&envelope.collector.keycloak_release)?.to_owned();
    let actions = section(&envelope.required_actions, MAX_REQUIRED_ACTIONS)?;
    let clients = section(&envelope.clients, MAX_CLIENTS)?;
    let flows = section(&envelope.flows, MAX_FLOWS)?;
    let mappers = section(&envelope.mappers, MAX_MAPPERS)?;

    // The complete client inventory: unique ids, one device client.
    let mut ids = HashSet::new();
    for client in clients {
        if !ids.insert(name(&client.client_id)?) {
            return Err(ObservationError::Inventory);
        }
    }
    let device = clients
        .iter()
        .find(|client| client.client_id == expected.client_id)
        .ok_or(ObservationError::Inventory)?;
    let access = match (device.public_client, device.bearer_only) {
        (false, false) => ClientAccess::Confidential,
        (true, false) => ClientAccess::Public,
        (false, true) => ClientAccess::BearerOnly,
        (true, true) => return Err(ObservationError::Malformed),
    };
    let mut grants: Vec<Grant> =
        device.grants.iter().copied().map(grant).collect();
    grants.sort();
    let distinct = grants.len();
    grants.dedup();
    if grants.len() != distinct {
        return Err(ObservationError::Malformed);
    }
    let effective = |client: &ClientWire| -> Result<String, ObservationError> {
        match &client.browser_flow_override {
            Some(flow) => Ok(alias(flow)?.to_owned()),
            None => Ok(realm_browser_flow.to_owned()),
        }
    };
    let mut flow_bound_clients = Vec::new();
    for client in clients {
        if effective(client)? == expected.flow_alias {
            flow_bound_clients.push(client.client_id.clone());
        }
    }
    flow_bound_clients.sort();

    // The dedicated flow, once, among uniquely named flows.
    let mut aliases = HashSet::new();
    for flow in flows {
        if !aliases.insert(alias(&flow.alias)?) {
            return Err(ObservationError::Inventory);
        }
    }
    let flow = flows
        .iter()
        .find(|flow| flow.alias == expected.flow_alias)
        .ok_or(ObservationError::Inventory)?;
    let provider = match flow.provider_id.as_str() {
        "basic-flow" => FlowProvider::BasicFlow,
        "client-flow" => FlowProvider::ClientFlow,
        _ => return Err(ObservationError::Malformed),
    };
    if flow.executions.len() > MAX_EXECUTIONS {
        return Err(ObservationError::TooLarge);
    }
    if !flow.executions.windows(2).all(|pair| {
        pair.first().map(|e| e.priority) < pair.last().map(|e| e.priority)
    }) {
        return Err(ObservationError::Priority);
    }
    let executions = flow
        .executions
        .iter()
        .map(execution)
        .collect::<Result<Vec<_>, _>>()?;
    if flow.executions.len() == 2 {
        let priorities: Vec<i64> =
            flow.executions.iter().map(|e| e.priority).collect();
        if priorities != expected.priorities {
            return Err(ObservationError::Priority);
        }
    }

    // Every other flow, nested ones included, configuring a dedicated
    // reference: its executions would yield the same `amr` value.
    let mut reference_collisions = Vec::new();
    for other in flows {
        if other.alias == expected.flow_alias {
            continue;
        }
        if other.executions.len() > MAX_EXECUTIONS {
            return Err(ObservationError::TooLarge);
        }
        let collides = other.executions.iter().any(|execution| {
            execution.config.as_ref().is_some_and(|ConfigWire(config)| {
                config.get(REFERENCE_VALUE).is_some_and(|reference| {
                    reference == PASSWORD_REFERENCE
                        || reference == TOTP_REFERENCE
                })
            })
        });
        if collides {
            reference_collisions.push(other.alias.clone());
        }
    }
    reference_collisions.sort();

    // The AMR mapper is known by its provider id alone. Any other mapper
    // configured to write the `amr` claim is recorded as a foreign writer,
    // never taken for the AMR mapper.
    let mut amr_mappers = Vec::new();
    let mut foreign_amr_writers = Vec::new();
    for mapper in mappers {
        let provider_id = name(&mapper.provider_id)?;
        if mapper.client_id != expected.client_id {
            continue;
        }
        if provider_id == AMR_MAPPER {
            amr_mappers.push(AmrMapperShape {
                provider: MapperProvider::OidcAmr,
                host: match &mapper.host {
                    HostWire::Client => MapperHost::Client,
                    HostWire::ClientScope(scope) => {
                        MapperHost::ClientScope(name(scope)?.to_owned())
                    }
                },
                id_token: mapper.id_token,
                access_token: mapper.access_token,
            });
        } else if mapper.claim_name.as_deref() == Some("amr") {
            foreign_amr_writers.push(provider_id.to_owned());
        }
    }
    amr_mappers.sort_by_key(mapper_key);
    foreign_amr_writers.sort();

    // Every registered required action, by unique alias.
    let mut required_actions = actions
        .iter()
        .map(|action| {
            Ok(RequiredActionShape {
                alias: name(&action.alias)?.to_owned(),
                provider_id: name(&action.provider_id)?.to_owned(),
                enabled: action.enabled,
                default_action: action.default_action,
                priority: action.priority,
            })
        })
        .collect::<Result<Vec<_>, ObservationError>>()?;
    required_actions.sort_by(|a, b| a.alias.cmp(&b.alias));
    if required_actions
        .windows(2)
        .any(|pair| pair[0].alias == pair[1].alias)
    {
        return Err(ObservationError::Inventory);
    }

    Ok(DriftEvidence {
        observed: FlowReceipt {
            idp_release: release,
            issuer: realm.issuer.clone(),
            realm_browser_flow: realm_browser_flow.to_owned(),
            required_actions,
            otp_policy: otp_policy(&realm.otp_policy),
            foreign_amr_writers,
            client: ClientShape {
                client_id: device.client_id.clone(),
                access,
                grants,
                browser_flow: effective(device)?,
                flow_bound_clients,
            },
            flow: FlowShape {
                alias: flow.alias.clone(),
                top_level: flow.top_level,
                provider,
                executions,
            },
            amr_mappers,
            reference_collisions,
        },
        checked_at: envelope.collector.observed_at,
        readback: Readback::Accepted,
        binding: Some(request.binding.clone()),
    })
}

/// The trusted collector, reached over a channel authenticated for the
/// expected issuer. It owns the admin credentials; the runner only asks.
/// Nothing implements it outside the tests: no such channel exists yet.
pub(crate) trait AuthenticatedCollector {
    /// The collector's reply to `request`, or none when it cannot answer.
    fn collect(
        &self,
        request: &ObservationRequest,
    ) -> Option<AuthenticatedObservation>;
}

/// The challenge a machine has pending.
fn pending_challenge(machine: &DeviceLogin) -> Option<ChallengeId> {
    match &machine.state {
        State::Pending(pending) => Some(pending.challenge),
        _ => None,
    }
}

/// After the challenge began and before the trustee authenticates: asks
/// the collector, with a fresh request bound to the machine's pending
/// challenge, for the trustee's own factors.
pub(crate) fn observe_before(
    collector: &dyn AuthenticatedCollector,
    expected: &ExpectedInstall,
    verifier: &NativeVerifier,
    machine: &DeviceLogin,
) -> Result<SubjectFactors, ObservationError> {
    let challenge =
        pending_challenge(machine).ok_or(ObservationError::Binding)?;
    let request =
        ObservationRequest::new(Phase::BeforeAuthentication, challenge)?;
    let observation = collector
        .collect(&request)
        .ok_or(ObservationError::ReadDenied)?;
    observe_subject(expected, &observation, &verifier.policy.subject, &request)
}

/// What one call of [`settle_observed`] did with the sealed result it was
/// given.
pub(crate) enum Observed<R> {
    /// The poll was settled; the result is spent.
    Settled(DeviceStatus),
    /// The after-tokens reply was stamped in the same whole second as the
    /// later token, so it is not yet after it. The poll stays in flight and
    /// the sealed result comes back unspent; settle it again, with a fresh
    /// request, in a later second. Nothing was admitted, and the
    /// challenge's own expiry still ends it.
    Retained(R),
    /// Offered to a machine that did not issue its ticket: nothing was
    /// asked of the collector, only the ticket's machine identity was read,
    /// and the result comes back unspent for its own machine.
    WrongMachine(R),
}

/// Settles one successful bound poll's sealed result. With factors observed
/// before the authentication, it asks the collector, with a fresh request
/// bound to the same challenge, for the flow and the factors after the
/// tokens, and settles with that evidence, unless the reply lands in the
/// tokens' own second, when the result is handed back instead. Without any
/// evidence the poll is settled with none, which never proves a factor.
/// The ticket, the pair and the device request's nonce are the result's
/// own, read only through the verifier's opener; every settlement, admitted
/// or not, checks the tokens against that nonce.
pub(crate) fn settle_observed<R: BoundPollResult>(
    verifier: &NativeVerifier,
    machine: &mut DeviceLogin,
    clock: Clock,
    result: R,
    expected: &ExpectedInstall,
    collector: &dyn AuthenticatedCollector,
    before: Option<&SubjectFactors>,
) -> Result<Observed<R>, DeviceError> {
    let opener = Opener(());
    if !super::is_own_ticket(machine, result.ticket(&opener)) {
        return Ok(Observed::WrongMachine(result));
    }
    let parts = result.open(&opener);
    let (ticket, tokens, device_nonce) = (
        parts.ticket,
        (parts.access_token, parts.id_token),
        parts.nonce,
    );
    let after_tokens = || -> Option<(DriftEvidence, SubjectFactors)> {
        let challenge = pending_challenge(machine)?;
        let request =
            ObservationRequest::new(Phase::AfterTokens, challenge).ok()?;
        let observation = collector.collect(&request)?;
        let readback = observe(expected, &observation, &request).ok()?;
        let after = observe_subject(
            expected,
            &observation,
            &verifier.policy.subject,
            &request,
        )
        .ok()?;
        Some((readback, after))
    };
    let evidence = before.and_then(|before| Some((before, after_tokens()?)));
    let status = match &evidence {
        Some((before, (readback, after))) => {
            let admission = Admission {
                readback,
                before,
                after,
            };
            // Only the reply's own second is retained: every other outcome,
            // admitted or not, is settled at once.
            if verifier.try_admission(
                machine,
                clock,
                tokens,
                device_nonce,
                &admission,
            ) == Err(super::VerifyError::ReadbackSameSecond)
            {
                return Ok(Observed::Retained(result));
            }
            verifier.settle_pair_admitted(
                machine,
                clock,
                ticket,
                tokens,
                device_nonce,
                &admission,
            )
        }
        None => {
            verifier.settle_pair(machine, clock, ticket, tokens, device_nonce)
        }
    };
    status.map(Observed::Settled)
}

/// Facts for a health check: never authority, never a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PolicyFacts {
    /// The observation parsed and every section was complete.
    pub(crate) observed: bool,
    /// The observed shape is the one qualifying shape for the policy.
    pub(crate) qualifies: bool,
    /// The observed shape is exactly the installed receipt.
    pub(crate) matches_receipt: bool,
}

/// Health facts of one observation against the policy and the receipt.
fn facts(
    expected: &ExpectedInstall,
    observation: &AuthenticatedObservation,
    request: &ObservationRequest,
    policy: &ClaimPolicy,
    receipt: &FlowReceipt,
) -> PolicyFacts {
    match observe(expected, observation, request) {
        Err(_) => PolicyFacts {
            observed: false,
            qualifies: false,
            matches_receipt: false,
        },
        Ok(evidence) => PolicyFacts {
            observed: true,
            qualifies: evidence.observed.qualifying_max_age(policy).is_some(),
            matches_receipt: evidence.observed == *receipt
                && evidence.observed.fingerprint() == receipt.fingerprint(),
        },
    }
}

#[cfg(test)]
#[path = "../tests/support/device_policy_keycloak_tests.rs"]
mod tests;
