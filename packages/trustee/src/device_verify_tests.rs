// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic tests of the device token verifier. Each fixture generates its
//! own RSA key in memory and signs synthetic claims of a synthetic issuer;
//! no key, token or log of any real system is used, and nothing is stored.
//! The fixture is shared with the transport's tests, which drive the same
//! pairs through a real poll ticket.

use super::*;
use crate::device::{
    ChallengeId, Closure, DiscardReason, Monotonic, ScopeInput,
};
use aws_lc_rs::rand::SystemRandom;
use aws_lc_rs::rsa::{KeyPair, KeySize};
use aws_lc_rs::signature::KeyPair as _;
use aws_lc_rs::signature::RSA_PKCS1_SHA256;
use serde_json::{json, Map, Value};

pub(crate) const TENANT: &str = "00000000-0000-4000-8000-00000000000a";
pub(crate) const SUBJECT: &str = "00000000-0000-4000-8000-0000000000b1";
pub(crate) const OTHER_UUID: &str = "00000000-0000-4000-8000-0000000000c2";
pub(crate) const ISSUER: &str = "https://idp.synthetic.invalid/realms/tenant-00000000-0000-4000-8000-00000000000a";
const AUDIENCE: &str = "synthetic-ceremony-audience";
const CLIENT: &str = "synthetic-device-client";
const TRUSTEE: &str = "synthetic-trustee-a";
const ROLE: &str = "trustee-ceremony";
const SID: &str = "synthetic-session-0001";
const KID: &str = "synthetic-kid-1";
const HASURA: &str = "https://hasura.io/jwt/claims";
pub(crate) const START: UnixSeconds = 1_800_000_000;
const AUTH: UnixSeconds = START + 10;
pub(crate) const NOW: UnixSeconds = START + 100;

pub(crate) type ClaimSet = Map<String, Value>;

/// Unpadded base64url.
fn b64(bytes: &[u8]) -> String {
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

/// One DER element with `tag`: its content and what follows it.
fn tlv(input: &[u8], tag: u8) -> (&[u8], &[u8]) {
    assert_eq!(input[0], tag, "unexpected synthetic key DER");
    let (length, start) = match input[1] {
        short if short < 0x80 => (usize::from(short), 2),
        0x81 => (usize::from(input[2]), 3),
        0x82 => ((usize::from(input[2]) << 8) | usize::from(input[3]), 4),
        _ => panic!("unexpected synthetic key DER length"),
    };
    (&input[start..start + length], &input[start + length..])
}

/// The modulus and exponent of a PKCS#1 or SPKI RSA public key.
fn rsa_components(der: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let (outer, _) = tlv(der, 0x30);
    let pkcs1 = if outer.first() == Some(&0x30) {
        let (_, rest) = tlv(outer, 0x30);
        let (bits, _) = tlv(rest, 0x03);
        tlv(&bits[1..], 0x30).0
    } else {
        outer
    };
    let (modulus, rest) = tlv(pkcs1, 0x02);
    let (exponent, _) = tlv(rest, 0x02);
    let unsigned = |value: &[u8]| {
        let first = value.iter().position(|b| *b != 0).unwrap_or(0);
        value[first..].to_vec()
    };
    (unsigned(modulus), unsigned(exponent))
}

/// A synthetic RSA key made for one fixture.
pub(crate) struct Signer {
    pair: KeyPair,
    kid: &'static str,
}

impl Signer {
    fn new(kid: &'static str) -> Signer {
        Signer {
            pair: KeyPair::generate(KeySize::Rsa2048)
                .expect("synthetic RSA key"),
            kid,
        }
    }

    /// The base64url modulus and exponent, as a JWKS entry has them.
    fn components(&self) -> (String, String) {
        let (modulus, exponent) =
            rsa_components(self.pair.public_key().as_ref());
        (b64(&modulus), b64(&exponent))
    }

    fn trusted(&self) -> TrustedRsaKey {
        let (modulus, exponent) = self.components();
        TrustedRsaKey::from_components(self.kid, &modulus, &exponent)
            .expect("synthetic trusted key")
    }

    fn sign_raw(&self, header: &[u8], claims: &[u8]) -> String {
        let input = format!("{}.{}", b64(header), b64(claims));
        let mut signature = vec![0; self.pair.public_modulus_len()];
        self.pair
            .sign(
                &RSA_PKCS1_SHA256,
                &SystemRandom::new(),
                input.as_bytes(),
                &mut signature,
            )
            .expect("synthetic signature");
        format!("{input}.{}", b64(&signature))
    }

    fn header(&self) -> Value {
        json!({ "alg": "RS256", "typ": "JWT", "kid": self.kid })
    }

    pub(crate) fn sign(&self, claims: &ClaimSet) -> String {
        let claims = serde_json::to_vec(claims).expect("synthetic claims");
        self.sign_raw(self.header().to_string().as_bytes(), &claims)
    }
}

/// The model's clock `offset` seconds after its origin, at wall time `wall`.
pub(crate) fn clock(offset: u64, wall: UnixSeconds) -> Clock {
    Clock {
        mono: Monotonic::from_secs(1_000 + offset),
        wall,
    }
}

fn scope(edit: impl FnOnce(&mut ScopeInput<'static>)) -> DeviceScope {
    let mut input = ScopeInput {
        issuer: ISSUER,
        audience: AUDIENCE,
        client_id: CLIENT,
        subject: SUBJECT,
        tenant: TENANT,
        trustee_name: TRUSTEE,
        required_role: ROLE,
    };
    edit(&mut input);
    DeviceScope::new(input).expect("synthetic scope")
}

/// A machine with a challenge begun at wall time `started`, a verifier
/// made for it, and the key that verifier trusts.
pub(crate) struct Fixture {
    pub(crate) signer: Signer,
    pub(crate) machine: DeviceLogin,
    pub(crate) challenge: ChallengeId,
    pub(crate) verifier: NativeVerifier,
    /// The admission evidence proven fixtures verify with.
    admission: Option<AdmissionEvidence>,
}

/// Owned admission evidence: a post-token flow readback, and the trustee's
/// factors before the challenge and after the tokens.
struct AdmissionEvidence {
    readback: DriftEvidence,
    before: SubjectFactors,
    after: SubjectFactors,
}

impl AdmissionEvidence {
    fn admission(&self) -> Admission<'_> {
        Admission {
            readback: &self.readback,
            before: &self.before,
            after: &self.after,
        }
    }
}

fn fixture_started(started: UnixSeconds) -> Fixture {
    let signer = Signer::new(KID);
    let mut machine = DeviceLogin::new(scope(|_| {}));
    let challenge = machine
        .begin(clock(0, started), 600, None)
        .expect("synthetic challenge");
    let ring = TrustedKeyRing::new(ISSUER, vec![signer.trusted()])
        .expect("synthetic key ring");
    let verifier =
        NativeVerifier::for_machine(&machine, ring).expect("verifier");
    Fixture {
        signer,
        machine,
        challenge,
        verifier,
        admission: None,
    }
}

pub(crate) fn fixture() -> Fixture {
    fixture_started(START)
}

fn object(value: Value) -> ClaimSet {
    value.as_object().cloned().expect("synthetic claims object")
}

pub(crate) fn access_claims() -> ClaimSet {
    object(json!({
        "iss": ISSUER,
        "aud": AUDIENCE,
        "azp": CLIENT,
        "sub": SUBJECT,
        "typ": "Bearer",
        "iat": NOW - 20,
        "nbf": NOW - 20,
        "exp": NOW + 280,
        "auth_time": AUTH,
        "sid": SID,
        "preferred_username": TRUSTEE,
        "trustee": TRUSTEE,
        HASURA: {
            "x-hasura-tenant-id": TENANT,
            "x-hasura-allowed-roles": [ROLE],
            "x-hasura-default-role": ROLE,
            "x-hasura-user-id": SUBJECT,
        },
        // Present and never trusted: neither proves a fresh factor.
        "acr": "synthetic-gold",
        "amr": ["synthetic-pwd", "synthetic-otp"],
        "scope": "openid",
        "jti": "synthetic-jti-access",
    }))
}

pub(crate) fn id_claims() -> ClaimSet {
    object(json!({
        "iss": ISSUER,
        "aud": CLIENT,
        "azp": CLIENT,
        "sub": SUBJECT,
        "typ": "ID",
        "iat": NOW - 20,
        "exp": NOW + 280,
        "auth_time": AUTH,
        "sid": SID,
        "preferred_username": TRUSTEE,
        "acr": "synthetic-gold",
        "jti": "synthetic-jti-id",
    }))
}

pub(crate) fn set(claims: &mut ClaimSet, name: &str, value: Value) {
    claims.insert(name.to_owned(), value);
}

impl Fixture {
    /// Keycloak's token reply carrying these claims, signed by this key.
    pub(crate) fn token_reply(
        &self,
        access: &ClaimSet,
        id: &ClaimSet,
    ) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "access_token": self.signer.sign(access),
            "id_token": self.signer.sign(id),
            "refresh_token": "synthetic.refresh.token",
            "token_type": "Bearer",
            "expires_in": 280,
        }))
        .expect("synthetic reply")
    }

    /// A ticket for the machine's first poll.
    pub(crate) fn ticket(&mut self) -> PollTicket {
        self.machine
            .start_poll(clock(5, NOW).mono, self.challenge)
            .expect("synthetic ticket")
    }

    /// Verifies the signed pair at `NOW`, with this fixture's admission
    /// evidence when it has any.
    fn verify(
        &self,
        access: &ClaimSet,
        id: &ClaimSet,
    ) -> Result<VerifiedLogin, VerifyError> {
        let admission = self.admission.as_ref().map(AdmissionEvidence::admission);
        self.verifier.verify_pair_with(
            &self.signer.sign(access),
            &self.signer.sign(id),
            START,
            NOW,
            admission.as_ref().map(|admission| (admission, self.challenge)),
        )
    }

    /// Settles the first poll with this signed pair, through the admitted
    /// path when the fixture has admission evidence.
    fn settle(
        &mut self,
        access: &ClaimSet,
        id: &ClaimSet,
    ) -> Result<DeviceStatus, DeviceError> {
        let (access, id) = (self.signer.sign(access), self.signer.sign(id));
        let ticket = self.ticket();
        match &self.admission {
            Some(evidence) => self.verifier.settle_pair_admitted(
                &mut self.machine,
                clock(5, NOW),
                &ticket,
                (&access, &id),
                &evidence.admission(),
            ),
            None => self.verifier.settle_pair(
                &mut self.machine,
                clock(5, NOW),
                &ticket,
                &access,
                &id,
            ),
        }
    }

    /// The refusal of a validly signed pair edited by `access` and `id`.
    fn refusal(
        &self,
        access: impl FnOnce(&mut ClaimSet),
        id: impl FnOnce(&mut ClaimSet),
    ) -> Option<VerifyError> {
        let (mut access_set, mut id_set) = (access_claims(), id_claims());
        access(&mut access_set);
        id(&mut id_set);
        self.verify(&access_set, &id_set).err()
    }

    fn access_refusal(
        &self,
        edit: impl FnOnce(&mut ClaimSet),
    ) -> Option<VerifyError> {
        self.refusal(edit, |_| {})
    }

    fn id_refusal(&self, edit: impl FnOnce(&mut ClaimSet)) -> Option<VerifyError> {
        self.refusal(|_| {}, edit)
    }

    fn both_refusal(&self, edit: impl Fn(&mut ClaimSet)) -> Option<VerifyError> {
        self.refusal(&edit, &edit)
    }

    /// The refusal of the access token with this header.
    fn header_refusal(&self, header: &[u8]) -> Option<VerifyError> {
        let claims = serde_json::to_vec(&access_claims()).unwrap();
        let token = self.signer.sign_raw(header, &claims);
        let id = self.signer.sign(&id_claims());
        self.verifier.verify_pair(&token, &id, START, NOW).err()
    }
}

fn denied(challenge: ChallengeId) -> Result<DeviceStatus, DeviceError> {
    Ok(DeviceStatus::Closed {
        challenge,
        closure: Closure::Denied,
    })
}

#[test]
fn a_valid_pair_verifies_with_the_factor_unproven() {
    let fixture = fixture();
    let login = fixture.verify(&access_claims(), &id_claims()).unwrap();
    assert_eq!(login.issuer, ISSUER);
    assert_eq!(login.audiences, vec![AUDIENCE.to_owned()]);
    assert_eq!(login.authorized_party, CLIENT);
    assert_eq!(login.subject, SUBJECT);
    assert_eq!(login.tenant, TENANT);
    assert_eq!(login.trustee_name, TRUSTEE);
    assert_eq!(login.roles, vec![ROLE.to_owned()]);
    assert_eq!(login.session_id, SID);
    assert_eq!(login.auth_time, AUTH);
    assert_eq!(login.issued_at, NOW - 20);
    assert_eq!(login.expires_at, NOW + 280);
    assert!(login.session_expires_at.is_none());
    // acr "gold" and an amr list are present; neither is evidence.
    assert!(matches!(login.mfa, MfaEvidence::Absent));
}

#[test]
fn a_valid_pair_settles_its_poll_and_is_discarded_without_a_factor() {
    let mut fixture = fixture();
    let challenge = fixture.challenge;
    let ticket = fixture.ticket();
    let (access, id) = (
        fixture.signer.sign(&access_claims()),
        fixture.signer.sign(&id_claims()),
    );
    assert_eq!(
        fixture.verifier.settle_pair(
            &mut fixture.machine,
            clock(5, NOW),
            &ticket,
            &access,
            &id
        ),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: Closure::Discarded(DiscardReason::MfaAbsent),
        })
    );
    assert_eq!(
        fixture.machine.authorize(clock(6, NOW)),
        Err(DeviceError::NoGrant)
    );
    // The same ticket settles nothing twice.
    assert_eq!(
        fixture.verifier.settle_pair(
            &mut fixture.machine,
            clock(6, NOW),
            &ticket,
            &access,
            &id
        ),
        Err(DeviceError::AlreadySettled)
    );
}

#[test]
fn an_invalid_pair_closes_its_poll_as_denied() {
    let mut fixture = fixture();
    let challenge = fixture.challenge;
    let mut other = access_claims();
    set(&mut other, "sub", json!(OTHER_UUID));
    assert_eq!(fixture.settle(&other, &id_claims()), denied(challenge));
    assert_eq!(
        fixture.machine.authorize(clock(6, NOW)),
        Err(DeviceError::NoGrant)
    );

    // Signed by a key the verifier does not trust.
    let mut untrusted = self::fixture();
    let challenge = untrusted.challenge;
    let impostor = Signer::new(KID);
    let ticket = untrusted.ticket();
    let access = impostor.sign(&access_claims());
    let id = impostor.sign(&id_claims());
    assert_eq!(
        untrusted.verifier.settle_pair(
            &mut untrusted.machine,
            clock(5, NOW),
            &ticket,
            &access,
            &id
        ),
        denied(challenge)
    );
}

#[test]
fn the_challenge_start_comes_from_the_machine() {
    // A challenge begun after the pair's authentication: stale, denied.
    let mut fixture = fixture_started(AUTH + 1);
    let challenge = fixture.challenge;
    assert_eq!(
        fixture.settle(&access_claims(), &id_claims()),
        denied(challenge)
    );
}

#[test]
fn a_verifier_settles_only_its_own_machine() {
    let own = fixture();
    let mut other = fixture();
    let challenge = other.challenge;
    let ticket = other.ticket();
    let (access, id) = (
        other.signer.sign(&access_claims()),
        other.signer.sign(&id_claims()),
    );
    // `own.verifier` was made for `own.machine`: it verifies nothing for
    // `other.machine` and denies the poll.
    assert_eq!(
        own.verifier.settle_pair(
            &mut other.machine,
            clock(5, NOW),
            &ticket,
            &access,
            &id
        ),
        denied(challenge)
    );
    // Nor can it settle a ticket of another machine on its own.
    let mut own = own;
    let ticket = other.machine.start_poll(clock(20, NOW).mono, challenge);
    assert!(ticket.is_err());
    let own_ticket = own.ticket();
    assert_eq!(
        own.verifier.settle_pair(
            &mut other.machine,
            clock(5, NOW),
            &own_ticket,
            &access,
            &id
        ),
        Err(DeviceError::ForeignTicket)
    );
}

#[test]
fn refuses_another_issuer_type_audience_or_client() {
    let fixture = fixture();
    let event_realm = format!("{ISSUER}-event-{OTHER_UUID}");
    assert_eq!(
        fixture.both_refusal(|c| set(c, "iss", json!(event_realm))),
        Some(VerifyError::Issuer)
    );
    assert_eq!(
        fixture.access_refusal(|c| set(c, "iss", json!(format!("{ISSUER}/")))),
        Some(VerifyError::Issuer)
    );
    for (access, id) in [("Refresh", "ID"), ("ID", "ID"), ("Bearer", "Bearer")]
    {
        assert_eq!(
            fixture.refusal(
                |c| set(c, "typ", json!(access)),
                |c| set(c, "typ", json!(id))
            ),
            Some(VerifyError::TokenType),
            "{access} {id}"
        );
    }
    for audience in [json!("account"), json!([]), json!(["account", CLIENT])] {
        assert_eq!(
            fixture.access_refusal(|c| set(c, "aud", audience)),
            Some(VerifyError::Audience)
        );
    }
    for audience in [json!(AUDIENCE), json!([CLIENT, AUDIENCE]), json!([])] {
        assert_eq!(
            fixture.id_refusal(|c| set(c, "aud", audience)),
            Some(VerifyError::Audience)
        );
    }
    let other_client = json!("synthetic-other-client");
    assert_eq!(
        fixture.access_refusal(|c| set(c, "azp", other_client.clone())),
        Some(VerifyError::AuthorizedParty)
    );
    assert_eq!(
        fixture.id_refusal(|c| set(c, "azp", other_client)),
        Some(VerifyError::AuthorizedParty)
    );
}

#[test]
fn refuses_another_subject_session_or_authentication() {
    let fixture = fixture();
    assert_eq!(
        fixture.access_refusal(|c| set(c, "sub", json!(OTHER_UUID))),
        Some(VerifyError::Subject)
    );
    assert_eq!(
        fixture.id_refusal(|c| set(c, "sub", json!(OTHER_UUID))),
        Some(VerifyError::Subject)
    );
    assert_eq!(
        fixture.access_refusal(|c| {
            c[HASURA]["x-hasura-user-id"] = json!(OTHER_UUID);
        }),
        Some(VerifyError::Subject)
    );
    assert_eq!(
        fixture.access_refusal(|c| drop(c.remove("sid"))),
        Some(VerifyError::Session)
    );
    assert_eq!(
        fixture.id_refusal(|c| set(c, "sid", json!("synthetic-session-0002"))),
        Some(VerifyError::Session)
    );
    assert_eq!(
        fixture.both_refusal(|c| set(c, "sid", json!(""))),
        Some(VerifyError::Session)
    );
    assert_eq!(
        fixture.id_refusal(|c| drop(c.remove("auth_time"))),
        Some(VerifyError::AuthTimeMissing)
    );
    assert_eq!(
        fixture.access_refusal(|c| set(c, "auth_time", json!(AUTH + 1))),
        Some(VerifyError::Mismatch)
    );
    assert_eq!(
        fixture.both_refusal(|c| set(c, "auth_time", json!(START - 1))),
        Some(VerifyError::AuthTimeStale)
    );
    // Authenticated after the token was issued.
    assert_eq!(
        fixture.both_refusal(|c| set(c, "auth_time", json!(NOW - 10))),
        Some(VerifyError::Malformed)
    );
}

#[test]
fn refuses_another_tenant_member_or_any_role_widening() {
    let fixture = fixture();
    assert_eq!(
        fixture.access_refusal(|c| {
            c[HASURA]["x-hasura-tenant-id"] = json!(OTHER_UUID);
        }),
        Some(VerifyError::Tenant)
    );
    assert_eq!(
        fixture.access_refusal(|c| drop(c.remove(HASURA))),
        Some(VerifyError::Tenant)
    );
    for roles in [
        json!([ROLE, "synthetic-admin"]),
        json!(["synthetic-admin", ROLE]),
        json!([ROLE, ROLE]),
        json!([]),
        json!(["synthetic-other-role"]),
    ] {
        assert_eq!(
            fixture.access_refusal(|c| {
                c[HASURA]["x-hasura-allowed-roles"] = roles;
            }),
            Some(VerifyError::Role)
        );
    }
    assert_eq!(
        fixture.access_refusal(|c| {
            c[HASURA]["x-hasura-default-role"] = json!("synthetic-admin");
        }),
        Some(VerifyError::Role)
    );
    let other = json!("synthetic-trustee-b");
    assert_eq!(
        fixture.access_refusal(|c| set(c, "trustee", other.clone())),
        Some(VerifyError::Member)
    );
    assert_eq!(
        fixture.access_refusal(|c| drop(c.remove("trustee"))),
        Some(VerifyError::Member)
    );
    assert_eq!(
        fixture.id_refusal(|c| set(c, "trustee", other.clone())),
        Some(VerifyError::Member)
    );
    // The username is checked, never relied on.
    assert_eq!(
        fixture.id_refusal(|c| set(c, "preferred_username", other)),
        Some(VerifyError::Member)
    );
    // Claims the ID token repeats must agree with the access token.
    assert_eq!(
        fixture.id_refusal(|c| {
            set(
                c,
                HASURA,
                json!({
                    "x-hasura-tenant-id": OTHER_UUID,
                    "x-hasura-allowed-roles": [ROLE],
                    "x-hasura-default-role": ROLE,
                }),
            );
        }),
        Some(VerifyError::Mismatch)
    );
}

#[test]
fn refuses_stale_future_or_overlong_tokens() {
    let fixture = fixture();
    assert_eq!(
        fixture.access_refusal(|c| set(c, "iat", json!(NOW + 1))),
        Some(VerifyError::IssuedInFuture)
    );
    assert_eq!(
        fixture.access_refusal(|c| set(c, "nbf", json!(NOW + 1))),
        Some(VerifyError::NotYetValid)
    );
    assert_eq!(
        fixture.access_refusal(|c| set(c, "exp", json!(NOW))),
        Some(VerifyError::Expired)
    );
    assert_eq!(
        fixture.id_refusal(|c| set(c, "exp", json!(NOW - 1))),
        Some(VerifyError::Expired)
    );
    let overlong = NOW - 20 + MAX_GRANT_LIFETIME + 1;
    assert_eq!(
        fixture.access_refusal(|c| set(c, "exp", json!(overlong))),
        Some(VerifyError::Lifetime)
    );
}

#[test]
fn refuses_missing_mistyped_or_duplicate_claims() {
    let fixture = fixture();
    for name in ["iss", "aud", "azp", "sub", "typ", "iat", "exp"] {
        assert_eq!(
            fixture.access_refusal(|c| drop(c.remove(name))),
            Some(VerifyError::Malformed),
            "{name}"
        );
    }
    for (name, value) in [
        ("exp", json!("synthetic")),
        ("iat", json!(-1)),
        ("aud", json!(7)),
        ("sid", json!(["synthetic"])),
    ] {
        assert_eq!(
            fixture.access_refusal(|c| set(c, name, value)),
            Some(VerifyError::Malformed),
            "{name}"
        );
    }

    let claims = serde_json::to_string(&access_claims()).unwrap();
    let duplicate = format!(r#"{{"sub":"{OTHER_UUID}",{}"#, &claims[1..]);
    let token = fixture.signer.sign_raw(
        fixture.signer.header().to_string().as_bytes(),
        duplicate.as_bytes(),
    );
    let id = fixture.signer.sign(&id_claims());
    assert_eq!(
        fixture.verifier.verify_pair(&token, &id, START, NOW).err(),
        Some(VerifyError::Malformed)
    );
}

#[test]
fn refuses_any_header_beyond_alg_kid_and_typ() {
    let fixture = fixture();
    let refusal = |header: Value| {
        fixture.header_refusal(header.to_string().as_bytes())
    };
    for alg in ["HS256", "RS512", "PS256", "ES256", "none"] {
        assert_eq!(
            refusal(json!({ "alg": alg, "typ": "JWT", "kid": KID })),
            Some(VerifyError::Algorithm),
            "{alg}"
        );
    }
    // Critical extensions, whatever their value, even null.
    for crit in [json!(["synthetic-ext"]), json!([]), Value::Null] {
        let mut header = fixture.signer.header();
        header["crit"] = crit;
        assert_eq!(refusal(header), Some(VerifyError::CriticalExtension));
    }
    // Key sources, even null: never followed.
    for name in ["jku", "jwk", "x5u", "x5c", "x5t", "x5t#S256"] {
        for value in [json!("synthetic"), Value::Null] {
            let mut header = fixture.signer.header();
            header[name] = value;
            assert_eq!(
                refusal(header),
                Some(VerifyError::KeyDiscovery),
                "{name}"
            );
        }
    }
    // Any other member, even null.
    for (name, value) in [
        ("cty", json!("JWT")),
        ("b64", json!(false)),
        ("zip", json!("DEF")),
        ("synthetic-extension", Value::Null),
    ] {
        let mut header = fixture.signer.header();
        header[name] = value;
        assert_eq!(refusal(header), Some(VerifyError::Header), "{name}");
    }
    let mut header = fixture.signer.header();
    header["typ"] = json!("at+jwt");
    assert_eq!(refusal(header), Some(VerifyError::Header));
    // A duplicate member.
    let duplicate =
        format!(r#"{{"alg":"RS256","alg":"RS256","kid":"{KID}"}}"#);
    assert_eq!(
        fixture.header_refusal(duplicate.as_bytes()),
        Some(VerifyError::Header)
    );
    assert_eq!(
        refusal(json!({ "alg": "RS256", "typ": "JWT" })),
        Some(VerifyError::UnknownKey)
    );
    assert_eq!(
        refusal(json!({ "alg": "RS256", "kid": "synthetic-kid-unknown" })),
        Some(VerifyError::UnknownKey)
    );
    assert_eq!(fixture.header_refusal(b"[]"), Some(VerifyError::Header));
}

#[test]
fn decodes_only_canonical_base64url() {
    assert_eq!(decode_base64url("").as_deref(), Some(&b""[..]));
    assert_eq!(decode_base64url("AQAB").as_deref(), Some(&[1, 0, 1][..]));
    assert_eq!(decode_base64url("YQ").as_deref(), Some(&b"a"[..]));
    // Padding, another alphabet, stray bits, impossible length.
    for input in ["YQ==", "Y+/A", "YR", "Y", " YQ"] {
        assert_eq!(decode_base64url(input), None, "{input}");
    }
}

#[test]
fn refuses_a_signature_no_trusted_key_made() {
    let fixture = fixture();
    let id = fixture.signer.sign(&id_claims());
    // Another synthetic key claiming the trusted key id.
    let impostor = Signer::new(KID).sign(&access_claims());
    assert_eq!(
        fixture.verifier.verify_pair(&impostor, &id, START, NOW).err(),
        Some(VerifyError::Signature)
    );
    // A trusted signature over different claims.
    let access = fixture.signer.sign(&access_claims());
    let parts: Vec<&str> = access.split('.').collect();
    let id_parts: Vec<&str> = id.split('.').collect();
    let spliced = format!("{}.{}.{}", parts[0], id_parts[1], parts[2]);
    assert_eq!(
        fixture.verifier.verify_pair(&spliced, &id, START, NOW).err(),
        Some(VerifyError::Signature)
    );
    let truncated = &access[..access.len() - 4];
    assert_eq!(
        fixture.verifier.verify_pair(truncated, &id, START, NOW).err(),
        Some(VerifyError::Signature)
    );
}

#[test]
fn refuses_tokens_that_are_not_bounded_compact_jws() {
    let fixture = fixture();
    let id = fixture.signer.sign(&id_claims());
    let access = fixture.signer.sign(&access_claims());
    let oversized = format!("a.{}.a", "a".repeat(TOKEN_LIMIT));
    for token in [
        String::new(),
        oversized,
        format!("{access}.extra"),
        format!(" {access}"),
        "synthetic".to_owned(),
    ] {
        assert_eq!(
            fixture.verifier.verify_pair(&token, &id, START, NOW).err(),
            Some(VerifyError::TokenShape)
        );
    }
}

#[test]
fn takes_its_expectations_from_the_machine_scope_only() {
    let signer = Signer::new(KID);
    let ring = || TrustedKeyRing::new(ISSUER, vec![signer.trusted()]).unwrap();
    let edits: [fn(&mut ScopeInput<'static>); 6] = [
        // A neighbouring event realm of the same tenant.
        |i| i.issuer = "https://idp.synthetic.invalid/realms/tenant-00000000-0000-4000-8000-00000000000a-event-00000000-0000-4000-8000-0000000000c2",
        |i| i.issuer = "http://idp.synthetic.invalid/realms/tenant-00000000-0000-4000-8000-00000000000a",
        // The tenant disagrees with the issuer's realm.
        |i| i.tenant = OTHER_UUID,
        |i| i.subject = "synthetic-subject",
        |i| i.tenant = "00000000-0000-4000-8000-00000000000A",
        |i| i.client_id = "synthetic client",
    ];
    for edit in edits {
        let machine = DeviceLogin::new(scope(edit));
        assert_eq!(
            NativeVerifier::for_machine(&machine, ring()).err(),
            Some(VerifyError::BadPolicy)
        );
    }
    // Keys trusted for another issuer never serve this machine's.
    let machine = DeviceLogin::new(scope(|_| {}));
    let (modulus, exponent) = signer.components();
    let elsewhere = TrustedKeyRing::new(
        "https://idp.synthetic.invalid/realms/tenant-00000000-0000-4000-8000-0000000000c2",
        vec![TrustedRsaKey::from_components(KID, &modulus, &exponent).unwrap()],
    )
    .unwrap();
    assert_eq!(
        NativeVerifier::for_machine(&machine, elsewhere).err(),
        Some(VerifyError::BadKeyRing)
    );
}

const DEVICE_FLOW: &str = "synthetic-device-flow";
/// Signed half a minute after the authentication, within the 60 s age.
const SIGNED_AT: UnixSeconds = AUTH + 30;

fn execution(step: Step, reference: &str, max_age: u64) -> ExecutionShape {
    ExecutionShape {
        step,
        requirement: Requirement::Required,
        amr_reference: Some(reference.to_owned()),
        amr_max_age: Some(max_age),
    }
}

/// A bare execution with no AMR configuration.
fn bare(step: Step, requirement: Requirement) -> ExecutionShape {
    ExecutionShape {
        step,
        requirement,
        amr_reference: None,
        amr_max_age: None,
    }
}

fn amr_mapper() -> AmrMapperShape {
    AmrMapperShape {
        provider: MapperProvider::OidcAmr,
        host: MapperHost::Client,
        id_token: true,
        access_token: true,
    }
}

/// A registered required action that is not a default action.
fn required_action(alias: &str, enabled: bool, priority: i64) -> RequiredActionShape {
    RequiredActionShape {
        alias: alias.to_owned(),
        provider_id: alias.to_owned(),
        enabled,
        default_action: false,
        priority,
    }
}

/// The one qualifying installed shape, as tested on the pinned release.
fn receipt() -> FlowReceipt {
    FlowReceipt {
        idp_release: PINNED_IDP_RELEASE.to_owned(),
        issuer: ISSUER.to_owned(),
        realm_browser_flow: "browser".to_owned(),
        required_actions: vec![
            required_action(CONFIGURE_TOTP, true, 10),
            required_action("UPDATE_PASSWORD", true, 30),
        ],
        otp_policy: STOCK_OTP_POLICY,
        foreign_amr_writers: Vec::new(),
        reference_collisions: Vec::new(),
        client: ClientShape {
            client_id: CLIENT.to_owned(),
            access: ClientAccess::Confidential,
            grants: vec![Grant::DeviceAuthorization],
            browser_flow: DEVICE_FLOW.to_owned(),
            flow_bound_clients: vec![CLIENT.to_owned()],
        },
        flow: FlowShape {
            alias: DEVICE_FLOW.to_owned(),
            top_level: true,
            provider: FlowProvider::BasicFlow,
            executions: vec![
                execution(Step::UsernamePasswordForm, PASSWORD_REFERENCE, MAX_MFA_AGE),
                execution(Step::OtpForm, TOTP_REFERENCE, MAX_MFA_AGE),
            ],
        },
        amr_mappers: vec![amr_mapper()],
    }
}

/// A synthetic proof: the receipt, and an accepted readback of it taken
/// shortly before the poll.
fn flow_proof() -> FlowPolicyProof {
    FlowPolicyProof {
        receipt: receipt(),
        drift: DriftEvidence {
            observed: receipt(),
            checked_at: NOW - 30,
            readback: Readback::Accepted,
            binding: None,
        },
    }
}

const OTP_CREDENTIAL: &str = "synthetic-otp-credential-1";

/// The synthetic nonces of the two requests of the fixture's challenge.
const BEFORE_NONCE: &str = "synthetic-before-request-nonce-000000000000";
const AFTER_NONCE: &str = "synthetic-after--request-nonce-000000000000";
/// When the fixture's own session started: after the challenge, before the
/// authentication completed.
const SESSION_STARTED: UnixSeconds = START + 8;

fn binding(nonce: &str, phase: Phase) -> RequestBinding {
    RequestBinding {
        nonce: nonce.to_owned(),
        phase,
        challenge: ChallengeId(1),
    }
}

/// The tokens' session: started with this challenge, used by the device
/// client alone.
fn device_session() -> SessionShape {
    SessionShape {
        id: SID.to_owned(),
        started_at: SESSION_STARTED,
        clients: vec![CLIENT.to_owned()],
    }
}

/// The trustee's factors: one TOTP enrolled a day before the challenge, no
/// pending required action, observed at `observed_at` for `binding`.
fn subject_factors(binding: RequestBinding, observed_at: UnixSeconds) -> SubjectFactors {
    SubjectFactors {
        binding,
        subject: SUBJECT.to_owned(),
        otp_credentials: vec![OtpCredentialShape {
            id: OTP_CREDENTIAL.to_owned(),
            created_at: START - 86_400,
        }],
        pending_actions: Vec::new(),
        sessions: vec![device_session()],
        observed_at,
    }
}

/// Good admission evidence for `receipt`: the factors after the challenge
/// began and before the authentication, then one reply after the tokens
/// with the flow readback and the factors.
fn admission_for(receipt: &FlowReceipt) -> AdmissionEvidence {
    let after = binding(AFTER_NONCE, Phase::AfterTokens);
    AdmissionEvidence {
        readback: DriftEvidence {
            observed: receipt.clone(),
            checked_at: NOW - 5,
            readback: Readback::Accepted,
            binding: Some(after.clone()),
        },
        before: subject_factors(
            binding(BEFORE_NONCE, Phase::BeforeAuthentication),
            START + 5,
        ),
        after: subject_factors(after, NOW - 5),
    }
}

/// The fixture, its verifier holding `flow_proof()` changed by `edit`, and
/// good admission evidence for that receipt changed by `admit`.
fn admitted_fixture_with(
    edit: impl FnOnce(&mut FlowPolicyProof),
    admit: impl FnOnce(&mut AdmissionEvidence),
) -> Result<Fixture, VerifyError> {
    let signer = Signer::new(KID);
    let mut machine = DeviceLogin::new(scope(|_| {}));
    let challenge = machine
        .begin(clock(0, START), 600, None)
        .expect("synthetic challenge");
    let ring = TrustedKeyRing::new(ISSUER, vec![signer.trusted()])
        .expect("synthetic key ring");
    let mut proof = flow_proof();
    edit(&mut proof);
    let mut admission = admission_for(&proof.receipt);
    admit(&mut admission);
    let verifier = NativeVerifier::with_flow_proof(&machine, ring, proof)?;
    Ok(Fixture {
        signer,
        machine,
        challenge,
        verifier,
        admission: Some(admission),
    })
}

/// The fixture, its verifier holding `flow_proof()` changed by `edit`.
fn proven_fixture_with(
    edit: impl FnOnce(&mut FlowPolicyProof),
) -> Result<Fixture, VerifyError> {
    admitted_fixture_with(edit, |_| {})
}

fn admitted_fixture(admit: impl FnOnce(&mut AdmissionEvidence)) -> Fixture {
    admitted_fixture_with(|_| {}, admit).expect("proven fixture")
}

fn proven_fixture() -> Fixture {
    proven_fixture_with(|_| {}).expect("proven fixture")
}

/// Claims as the dedicated flow signs them: both references, issued
/// within the maximum age of the authentication.
fn dedicated(mut claims: ClaimSet) -> ClaimSet {
    set(&mut claims, "amr", json!([PASSWORD_REFERENCE, TOTP_REFERENCE]));
    set(&mut claims, "iat", json!(SIGNED_AT));
    if claims.contains_key("nbf") {
        set(&mut claims, "nbf", json!(SIGNED_AT));
    }
    claims
}

fn dedicated_access() -> ClaimSet {
    dedicated(access_claims())
}

fn dedicated_id() -> ClaimSet {
    dedicated(id_claims())
}

impl Fixture {
    /// The refusal of a dedicated, validly signed pair edited by `access`
    /// and `id`.
    fn dedicated_refusal(
        &self,
        access: impl FnOnce(&mut ClaimSet),
        id: impl FnOnce(&mut ClaimSet),
    ) -> Option<VerifyError> {
        let (mut access_set, mut id_set) = (dedicated_access(), dedicated_id());
        access(&mut access_set);
        id(&mut id_set);
        self.verify(&access_set, &id_set).err()
    }
}

fn fresh_totp(login: &VerifiedLogin) -> bool {
    matches!(
        login.mfa,
        MfaEvidence::Verified(mfa) if mfa == FreshMfa {
            factor: SecondFactor::Totp,
            verified_at: AUTH,
        }
    )
}

#[test]
fn a_proven_flow_turns_the_exact_signed_amr_into_a_fresh_totp() {
    let mut fixture = proven_fixture();
    let login = fixture.verify(&dedicated_access(), &dedicated_id()).unwrap();
    assert!(fresh_totp(&login));
    // In either order.
    let mut swapped = dedicated_id();
    set(&mut swapped, "amr", json!([TOTP_REFERENCE, PASSWORD_REFERENCE]));
    let login = fixture.verify(&dedicated_access(), &swapped).unwrap();
    assert!(fresh_totp(&login));
    // Exactly at the maximum age.
    let mut oldest = dedicated_access();
    set(&mut oldest, "iat", json!(AUTH + MAX_MFA_AGE));
    let login = fixture.verify(&oldest, &dedicated_id()).unwrap();
    assert!(fresh_totp(&login));

    // Through the poll's own ticket, the model approves it.
    let challenge = fixture.challenge;
    let status = fixture.settle(&dedicated_access(), &dedicated_id());
    assert!(matches!(
        status,
        Ok(DeviceStatus::Approved { challenge: approved, .. }) if approved == challenge
    ));
    assert!(fixture.machine.authorize(clock(6, NOW)).is_ok());
}

#[test]
fn the_production_verifier_stays_unproven_even_for_the_exact_amr() {
    let mut fixture = fixture();
    let login = fixture.verify(&dedicated_access(), &dedicated_id()).unwrap();
    assert!(matches!(login.mfa, MfaEvidence::Absent));
    let challenge = fixture.challenge;
    assert_eq!(
        fixture.settle(&dedicated_access(), &dedicated_id()),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: Closure::Discarded(DiscardReason::MfaAbsent),
        })
    );
    assert_eq!(
        fixture.machine.authorize(clock(6, NOW)),
        Err(DeviceError::NoGrant)
    );
}

#[test]
fn refuses_any_amr_but_exactly_the_two_dedicated_references() {
    let fixture = proven_fixture();
    let other_totp = "https://sequent.vote/amr/trustee-device-totp-v2";
    let generic = [
        Value::Null,
        json!([]),
        json!(PASSWORD_REFERENCE),
        json!([PASSWORD_REFERENCE]),
        json!([PASSWORD_REFERENCE, PASSWORD_REFERENCE]),
        json!([TOTP_REFERENCE, TOTP_REFERENCE]),
        json!([PASSWORD_REFERENCE, TOTP_REFERENCE, "pwd"]),
        json!([PASSWORD_REFERENCE, other_totp]),
        json!(["pwd", "otp"]),
        json!([PASSWORD_REFERENCE, 7]),
        json!({ "password": PASSWORD_REFERENCE, "totp": TOTP_REFERENCE }),
    ];
    for amr in generic {
        assert_eq!(
            fixture.dedicated_refusal(|c| set(c, "amr", amr.clone()), |_| {}),
            Some(VerifyError::AmrMismatch),
            "access {amr}"
        );
        assert_eq!(
            fixture.dedicated_refusal(|_| {}, |c| set(c, "amr", amr.clone())),
            Some(VerifyError::AmrMismatch),
            "id {amr}"
        );
    }
    // Missing from either token; an assurance level is no substitute.
    assert_eq!(
        fixture.dedicated_refusal(|c| drop(c.remove("amr")), |_| {}),
        Some(VerifyError::AmrMismatch)
    );
    assert_eq!(
        fixture.dedicated_refusal(
            |_| {},
            |c| {
                c.remove("amr");
                set(c, "acr", json!("synthetic-gold"));
            }
        ),
        Some(VerifyError::AmrMismatch)
    );
}

#[test]
fn refuses_a_proven_login_outside_its_age_or_challenge() {
    let fixture = proven_fixture();
    let late = |c: &mut ClaimSet| {
        set(c, "iat", json!(AUTH + MAX_MFA_AGE + 1));
        if c.contains_key("nbf") {
            set(c, "nbf", json!(AUTH + MAX_MFA_AGE + 1));
        }
    };
    assert_eq!(
        fixture.dedicated_refusal(late, |_| {}),
        Some(VerifyError::MfaStale)
    );
    assert_eq!(
        fixture.dedicated_refusal(|_| {}, late),
        Some(VerifyError::MfaStale)
    );
    // Authenticated before this challenge began.
    let before = |c: &mut ClaimSet| set(c, "auth_time", json!(START - 1));
    assert_eq!(
        fixture.dedicated_refusal(before, before),
        Some(VerifyError::AuthTimeStale)
    );
    // Issued in the future, or authenticated after issue.
    assert_eq!(
        fixture.dedicated_refusal(|c| set(c, "iat", json!(NOW + 1)), |_| {}),
        Some(VerifyError::IssuedInFuture)
    );
    let after = |c: &mut ClaimSet| set(c, "auth_time", json!(SIGNED_AT + 1));
    assert_eq!(
        fixture.dedicated_refusal(after, after),
        Some(VerifyError::Malformed)
    );
    // The two tokens disagree on the authentication.
    assert_eq!(
        fixture.dedicated_refusal(|c| set(c, "auth_time", json!(AUTH + 1)), |_| {}),
        Some(VerifyError::Mismatch)
    );
}

#[test]
fn refuses_a_proven_pair_of_another_session_subject_or_member() {
    let fixture = proven_fixture();
    assert_eq!(
        fixture.dedicated_refusal(
            |_| {},
            |c| set(c, "sid", json!("synthetic-session-0002"))
        ),
        Some(VerifyError::Session)
    );
    let other = |c: &mut ClaimSet| set(c, "sub", json!(OTHER_UUID));
    assert_eq!(
        fixture.dedicated_refusal(other, other),
        Some(VerifyError::Subject)
    );
    assert_eq!(
        fixture.dedicated_refusal(
            |c| set(c, "trustee", json!("synthetic-trustee-b")),
            |_| {}
        ),
        Some(VerifyError::Member)
    );
    assert_eq!(
        fixture.dedicated_refusal(|c| set(c, "azp", json!("synthetic-other")), |_| {}),
        Some(VerifyError::AuthorizedParty)
    );
}

/// Changes that disqualify an installed shape, applied to the receipt and
/// the readback alike: each must fail before any MFA is admitted.
fn disqualifying() -> Vec<(&'static str, fn(&mut FlowReceipt))> {
    let cases: &[(&str, fn(&mut FlowReceipt))] = &[
        // Release, issuer, client.
        ("other release", |r| r.idp_release = "26.7.4".to_owned()),
        ("event realm", |r| r.issuer = format!("{ISSUER}-event-{OTHER_UUID}")),
        ("other client", |r| r.client.client_id = "synthetic-other".to_owned()),
        ("public client", |r| r.client.access = ClientAccess::Public),
        ("bearer-only client", |r| r.client.access = ClientAccess::BearerOnly),
        ("no grant", |r| r.client.grants.clear()),
        ("repeated grant", |r| r.client.grants.push(Grant::DeviceAuthorization)),
        ("standard flow too", |r| r.client.grants.push(Grant::StandardFlow)),
        ("direct grant too", |r| r.client.grants.push(Grant::DirectAccess)),
        ("implicit too", |r| r.client.grants.push(Grant::Implicit)),
        ("service accounts too", |r| r.client.grants.push(Grant::ServiceAccounts)),
        ("token exchange too", |r| r.client.grants.push(Grant::TokenExchange)),
        ("stock browser flow", |r| r.client.browser_flow = "browser".to_owned()),
        // Another client accepting logins through the dedicated flow.
        ("extra bound client", |r| {
            r.client.flow_bound_clients.push("synthetic-portal".to_owned());
        }),
        ("no bound client", |r| r.client.flow_bound_clients.clear()),
        // The flow itself.
        ("empty alias", |r| {
            r.flow.alias = String::new();
            r.client.browser_flow = String::new();
        }),
        ("nested flow", |r| r.flow.top_level = false),
        ("client flow", |r| r.flow.provider = FlowProvider::ClientFlow),
        ("cookie first", |r| {
            r.flow.executions.insert(0, bare(Step::Cookie, Requirement::Alternative));
        }),
        ("disabled cookie", |r| {
            r.flow.executions.push(bare(Step::Cookie, Requirement::Disabled));
        }),
        ("identity provider", |r| {
            r.flow
                .executions
                .push(bare(Step::IdentityProviderRedirector, Requirement::Alternative));
        }),
        ("inherited subflow", |r| {
            r.flow.executions.push(bare(
                Step::Subflow("browser forms".to_owned()),
                Requirement::Alternative,
            ));
        }),
        ("unknown step", |r| {
            r.flow.executions.push(bare(
                Step::Unknown("synthetic-authenticator".to_owned()),
                Requirement::Required,
            ));
        }),
        ("otp skipped", |r| drop(r.flow.executions.pop())),
        ("otp conditional", |r| {
            r.flow.executions[1].requirement = Requirement::Conditional;
        }),
        ("password alternative", |r| {
            r.flow.executions[0].requirement = Requirement::Alternative;
        }),
        ("otp disabled", |r| {
            r.flow.executions[1].requirement = Requirement::Disabled;
        }),
        ("otp before password", |r| r.flow.executions.swap(0, 1)),
        ("references swapped", |r| {
            r.flow.executions[0].amr_reference = Some(TOTP_REFERENCE.to_owned());
            r.flow.executions[1].amr_reference = Some(PASSWORD_REFERENCE.to_owned());
        }),
        ("generic reference", |r| {
            r.flow.executions[0].amr_reference = Some("pwd".to_owned());
        }),
        ("no reference", |r| r.flow.executions[1].amr_reference = None),
        ("no maximum age", |r| r.flow.executions[0].amr_max_age = None),
        ("unequal ages", |r| r.flow.executions[1].amr_max_age = Some(59)),
        ("age zero", |r| {
            r.flow.executions[0].amr_max_age = Some(0);
            r.flow.executions[1].amr_max_age = Some(0);
        }),
        ("age over 60", |r| {
            r.flow.executions[0].amr_max_age = Some(MAX_MFA_AGE + 1);
            r.flow.executions[1].amr_max_age = Some(MAX_MFA_AGE + 1);
        }),
        // The AMR mappers.
        ("no mapper", |r| r.amr_mappers.clear()),
        ("second mapper", |r| r.amr_mappers.push(amr_mapper())),
        ("hard-coded mapper", |r| {
            r.amr_mappers[0].provider =
                MapperProvider::Other("oidc-hardcoded-claim-mapper".to_owned());
        }),
        ("scope mapper", |r| {
            r.amr_mappers[0].host = MapperHost::ClientScope("acr".to_owned());
        }),
        ("not in ID token", |r| r.amr_mappers[0].id_token = false),
        ("not in access token", |r| r.amr_mappers[0].access_token = false),
        ("foreign amr writer", |r| {
            r.foreign_amr_writers.push("oidc-hardcoded-claim-mapper".to_owned());
        }),
        ("reference reused by another flow", |r| {
            r.reference_collisions.push("browser forms".to_owned());
        }),
        // The realm default browser flow.
        ("realm default is the flow", |r| {
            r.realm_browser_flow = DEVICE_FLOW.to_owned();
        }),
        ("no realm default", |r| r.realm_browser_flow = String::new()),
        // Required actions.
        ("configure otp by default", |r| {
            r.required_actions[0].default_action = true;
        }),
        ("configure otp disabled", |r| r.required_actions[0].enabled = false),
        ("no configure otp", |r| {
            r.required_actions.remove(0);
        }),
        ("configure otp by another provider", |r| {
            r.required_actions[0].provider_id = "synthetic-totp".to_owned();
        }),
        ("another default action", |r| {
            r.required_actions[1].default_action = true;
        }),
        ("unsorted actions", |r| r.required_actions.swap(0, 1)),
        // The OTP policy.
        ("hotp", |r| r.otp_policy.kind = OtpKind::Hotp),
        ("sha-256", |r| r.otp_policy.algorithm = OtpAlgorithm::HmacSha256),
        ("sha-512", |r| r.otp_policy.algorithm = OtpAlgorithm::HmacSha512),
        ("eight digits", |r| r.otp_policy.digits = 8),
        ("sixty-second period", |r| r.otp_policy.period = 60),
        ("wide look-ahead", |r| r.otp_policy.look_ahead = 3),
        ("reusable codes", |r| r.otp_policy.code_reusable = true),
    ];
    cases.to_vec()
}

#[test]
fn qualifies_only_the_exact_installed_shape() {
    let policy = ClaimPolicy::from_scope(&scope(|_| {})).unwrap();
    assert_eq!(receipt().qualifying_max_age(&policy), Some(MAX_MFA_AGE));
    for (name, change) in disqualifying() {
        let mut shape = receipt();
        change(&mut shape);
        assert_eq!(shape.qualifying_max_age(&policy), None, "{name}");
        // Installed alike and read back alike: still refused.
        let refused = proven_fixture_with(|p| {
            change(&mut p.receipt);
            change(&mut p.drift.observed);
        });
        assert_eq!(refused.err(), Some(VerifyError::FlowPolicy), "{name}");
    }
}

#[test]
fn refuses_a_readback_that_differs_from_the_receipt() {
    for (name, change) in disqualifying() {
        // The receipt is the qualifying shape; the live flow drifted.
        let refused = proven_fixture_with(|p| change(&mut p.drift.observed));
        assert_eq!(refused.err(), Some(VerifyError::FlowPolicy), "{name}");
    }
    // Even a qualifying live shape must be the very one installed.
    let refused = proven_fixture_with(|p| {
        for execution in &mut p.drift.observed.flow.executions {
            execution.amr_max_age = Some(30);
        }
    });
    assert_eq!(refused.err(), Some(VerifyError::FlowPolicy));
}

#[test]
fn fingerprints_every_field_of_the_installed_shape() {
    let exact = receipt();
    assert_eq!(exact.fingerprint(), receipt().fingerprint());
    assert_eq!(exact.fingerprint().len(), 32);
    for (name, change) in disqualifying() {
        let mut shape = receipt();
        change(&mut shape);
        assert_ne!(shape.fingerprint(), exact.fingerprint(), "{name}");
    }
    // Field boundaries are encoded: moving text between fields differs.
    let mut left = receipt();
    left.flow.executions.push(bare(Step::Subflow("a;b".to_owned()), Requirement::Disabled));
    let mut right = receipt();
    right.flow.executions.push(bare(Step::Subflow("a".to_owned()), Requirement::Disabled));
    assert_ne!(left.canonical(), right.canonical());
}

#[test]
fn a_shorter_qualifying_age_is_enforced_from_the_flow() {
    let fixture = proven_fixture_with(|p| {
        for shape in [&mut p.receipt, &mut p.drift.observed] {
            for execution in &mut shape.flow.executions {
                execution.amr_max_age = Some(30);
            }
        }
    })
    .expect("qualifying 30 s flow");
    // Issued 30 s after the authentication: admitted.
    assert!(fixture.verify(&dedicated_access(), &dedicated_id()).is_ok());
    let late = |c: &mut ClaimSet| set(c, "iat", json!(AUTH + 31));
    assert_eq!(
        fixture.dedicated_refusal(|_| {}, late),
        Some(VerifyError::MfaStale)
    );
}

#[test]
fn refuses_a_drifted_unavailable_or_future_readback() {
    let refusals: [fn(&mut AdmissionEvidence); 3] = [
        |a| a.readback.readback = Readback::Drifted,
        |a| a.readback.readback = Readback::Unavailable,
        |a| a.readback.checked_at = NOW + 1,
    ];
    for refusal in refusals {
        let fixture = admitted_fixture(refusal);
        assert_eq!(
            fixture.dedicated_refusal(|_| {}, |_| {}),
            Some(VerifyError::FlowDrift)
        );
    }

    // Through the ticket, a drifted flow closes the challenge as denied.
    let mut drifted =
        admitted_fixture(|a| a.readback.readback = Readback::Drifted);
    let challenge = drifted.challenge;
    assert_eq!(
        drifted.settle(&dedicated_access(), &dedicated_id()),
        denied(challenge)
    );
    assert_eq!(
        drifted.machine.authorize(clock(6, NOW)),
        Err(DeviceError::NoGrant)
    );
}

#[test]
fn admits_a_fresh_totp_only_with_a_readback_after_both_tokens() {
    let fixture = admitted_fixture(|_| {});
    let login = fixture.verify(&dedicated_access(), &dedicated_id()).unwrap();
    assert!(fresh_totp(&login));
    // The second after the tokens' issue is the first that is after it.
    let next_second = admitted_fixture(|a| {
        a.readback.checked_at = SIGNED_AT + 1;
        a.after.observed_at = SIGNED_AT + 1;
    });
    assert!(next_second.verify(&dedicated_access(), &dedicated_id()).is_ok());
    // In the same second it is not yet after the issue: never admitted,
    // and told apart from a readback that is truly earlier.
    let same_second = admitted_fixture(|a| a.readback.checked_at = SIGNED_AT);
    assert_eq!(
        same_second.dedicated_refusal(|_| {}, |_| {}),
        Some(VerifyError::ReadbackSameSecond)
    );
    let same_second = admitted_fixture(|a| a.after.observed_at = SIGNED_AT);
    assert_eq!(
        same_second.dedicated_refusal(|_| {}, |_| {}),
        Some(VerifyError::ReadbackSameSecond)
    );

    // Made before the access token was issued.
    let stale = admitted_fixture(|a| a.readback.checked_at = SIGNED_AT - 1);
    assert_eq!(
        stale.dedicated_refusal(|_| {}, |_| {}),
        Some(VerifyError::ReadbackBeforeTokens)
    );
    // After the access token, but before the ID token.
    let between = admitted_fixture(|a| a.readback.checked_at = SIGNED_AT + 5);
    assert_eq!(
        between.dedicated_refusal(|_| {}, |c| set(c, "iat", json!(SIGNED_AT + 10))),
        Some(VerifyError::ReadbackBeforeTokens)
    );
    // The trustee's factors, observed again only before the tokens.
    let early = admitted_fixture(|a| a.after.observed_at = SIGNED_AT - 1);
    assert_eq!(
        early.dedicated_refusal(|_| {}, |_| {}),
        Some(VerifyError::ReadbackBeforeTokens)
    );
    let future = admitted_fixture(|a| a.after.observed_at = NOW + 1);
    assert_eq!(
        future.dedicated_refusal(|_| {}, |_| {}),
        Some(VerifyError::ReadbackBeforeTokens)
    );

    // No evidence at all: the plain path never proves a factor.
    let mut plain = admitted_fixture(|_| {});
    let (access, id) = (
        plain.signer.sign(&dedicated_access()),
        plain.signer.sign(&dedicated_id()),
    );
    assert_eq!(
        plain.verifier.verify_pair(&access, &id, START, NOW).err(),
        Some(VerifyError::NoAdmissionEvidence)
    );
    let challenge = plain.challenge;
    let ticket = plain.ticket();
    assert_eq!(
        plain.verifier.settle_pair(
            &mut plain.machine,
            clock(5, NOW),
            &ticket,
            &access,
            &id
        ),
        denied(challenge)
    );
}

#[test]
fn never_takes_an_enrollment_in_this_run_for_a_preexisting_factor() {
    let refusals: [(&str, fn(&mut AdmissionEvidence)); 8] = [
        ("configure otp pending", |a| {
            a.before.pending_actions.push(CONFIGURE_TOTP.to_owned());
        }),
        ("update password pending", |a| {
            a.before.pending_actions.push("UPDATE_PASSWORD".to_owned());
        }),
        ("no credential yet", |a| a.before.otp_credentials.clear()),
        ("enrolled at the start", |a| {
            a.before.otp_credentials[0].created_at = START;
        }),
        ("enrolled during the run", |a| {
            a.before.otp_credentials[0].created_at = START + 5;
            a.after.otp_credentials[0].created_at = START + 5;
        }),
        // A snapshot from before this challenge, or not before the
        // authentication, does not show the state at its start.
        ("observed before the challenge", |a| a.before.observed_at = START - 1),
        ("observed at the authentication", |a| a.before.observed_at = AUTH),
        ("two credentials", |a| {
            a.before.otp_credentials.push(OtpCredentialShape {
                id: "synthetic-otp-credential-2".to_owned(),
                created_at: START - 10,
            });
        }),
    ];
    for (name, refusal) in refusals {
        let fixture = admitted_fixture(refusal);
        assert_eq!(
            fixture.dedicated_refusal(|_| {}, |_| {}),
            Some(VerifyError::FactorNotPreexisting),
            "{name}"
        );
    }
    // Through the ticket, an enrollment inside the run is denied.
    let mut enrolling = admitted_fixture(|a| a.before.otp_credentials.clear());
    let challenge = enrolling.challenge;
    assert_eq!(
        enrolling.settle(&dedicated_access(), &dedicated_id()),
        denied(challenge)
    );
}

#[test]
fn refuses_factors_that_changed_by_the_time_of_the_tokens() {
    let refusals: [(&str, fn(&mut AdmissionEvidence)); 5] = [
        ("other credential", |a| {
            a.after.otp_credentials[0].id = "synthetic-otp-credential-2".to_owned();
        }),
        ("re-created credential", |a| {
            a.after.otp_credentials[0].created_at = SIGNED_AT;
        }),
        ("added credential", |a| {
            a.after.otp_credentials.push(OtpCredentialShape {
                id: "synthetic-otp-credential-2".to_owned(),
                created_at: SIGNED_AT,
            });
        }),
        ("removed credential", |a| a.after.otp_credentials.clear()),
        ("action now pending", |a| {
            a.after.pending_actions.push(CONFIGURE_TOTP.to_owned());
        }),
    ];
    for (name, refusal) in refusals {
        let fixture = admitted_fixture(refusal);
        assert_eq!(
            fixture.dedicated_refusal(|_| {}, |_| {}),
            Some(VerifyError::FactorChanged),
            "{name}"
        );
    }
    let subjects: [fn(&mut AdmissionEvidence); 2] = [
        |a| a.before.subject = OTHER_UUID.to_owned(),
        |a| a.after.subject = OTHER_UUID.to_owned(),
    ];
    for subject in subjects {
        assert_eq!(
            admitted_fixture(subject).dedicated_refusal(|_| {}, |_| {}),
            Some(VerifyError::Subject)
        );
    }
}

#[test]
fn binds_every_piece_of_evidence_to_this_challenges_requests() {
    let refusals: [(&str, fn(&mut AdmissionEvidence)); 9] = [
        ("unbound readback", |a| a.readback.binding = None),
        ("readback of another challenge", |a| {
            a.readback.binding = Some(RequestBinding {
                challenge: ChallengeId(2),
                ..binding(AFTER_NONCE, Phase::AfterTokens)
            });
            a.after.binding = a.readback.binding.clone().unwrap();
        }),
        ("readback in the wrong phase", |a| {
            let early = binding(AFTER_NONCE, Phase::BeforeAuthentication);
            a.readback.binding = Some(early.clone());
            a.after.binding = early;
        }),
        ("factors from another reply", |a| {
            a.after.binding = binding(
                "synthetic-other--request-nonce-000000000000",
                Phase::AfterTokens,
            );
        }),
        ("earlier factors in the wrong phase", |a| {
            a.before.binding = binding(BEFORE_NONCE, Phase::AfterTokens);
        }),
        ("earlier factors of another challenge", |a| {
            a.before.binding.challenge = ChallengeId(2);
        }),
        // The same reply cannot serve as both observations.
        ("one reply twice", |a| {
            a.before.binding.nonce = AFTER_NONCE.to_owned();
        }),
        ("short nonce", |a| {
            let short = binding("synthetic-short-nonce", Phase::AfterTokens);
            a.readback.binding = Some(short.clone());
            a.after.binding = short;
        }),
        ("nonce outside base64url", |a| {
            a.before.binding.nonce =
                "synthetic+before+request+nonce+000000000000".to_owned();
        }),
    ];
    for (name, refusal) in refusals {
        assert_eq!(
            admitted_fixture(refusal).dedicated_refusal(|_| {}, |_| {}),
            Some(VerifyError::Binding),
            "{name}"
        );
    }
}

#[test]
fn requires_the_tokens_own_fresh_device_only_session() {
    let refusals: [(&str, fn(&mut AdmissionEvidence)); 6] = [
        ("no session", |a| a.after.sessions.clear()),
        ("another session only", |a| {
            a.after.sessions[0].id = "synthetic-session-0002".to_owned();
        }),
        // A prior SSO session the device login joined.
        ("started before the challenge", |a| {
            a.after.sessions[0].started_at = START - 1;
        }),
        ("started after the authentication", |a| {
            a.after.sessions[0].started_at = AUTH + 1;
        }),
        ("shared with an ordinary client", |a| {
            a.after.sessions[0].clients =
                vec!["fixture-ordinary-sso".to_owned(), CLIENT.to_owned()];
        }),
        ("no client", |a| a.after.sessions[0].clients.clear()),
    ];
    for (name, refusal) in refusals {
        assert_eq!(
            admitted_fixture(refusal).dedicated_refusal(|_| {}, |_| {}),
            Some(VerifyError::SessionNotFresh),
            "{name}"
        );
    }
    // Other sessions of the trustee may exist; the tokens' own decides.
    let with_other = admitted_fixture(|a| {
        a.after.sessions.push(SessionShape {
            id: "synthetic-session-0002".to_owned(),
            started_at: START - 3_600,
            clients: vec!["fixture-ordinary-sso".to_owned()],
        });
    });
    assert!(with_other.verify(&dedicated_access(), &dedicated_id()).is_ok());
}

#[test]
fn refuses_a_post_token_readback_of_a_changed_policy() {
    for (name, change) in disqualifying() {
        let fixture = admitted_fixture(|a| change(&mut a.readback.observed));
        assert_eq!(
            fixture.dedicated_refusal(|_| {}, |_| {}),
            Some(VerifyError::FlowDrift),
            "{name}"
        );
    }
    // Even a qualifying shape must be the one installed.
    let other = admitted_fixture(|a| {
        for execution in &mut a.readback.observed.flow.executions {
            execution.amr_max_age = Some(30);
        }
    });
    assert_eq!(
        other.dedicated_refusal(|_| {}, |_| {}),
        Some(VerifyError::FlowDrift)
    );
}

#[test]
fn the_unproven_verifier_ignores_any_admission_evidence() {
    let mut unproven = fixture();
    unproven.admission = Some(admission_for(&receipt()));
    let login = unproven.verify(&dedicated_access(), &dedicated_id()).unwrap();
    assert!(matches!(login.mfa, MfaEvidence::Absent));
    let challenge = unproven.challenge;
    assert_eq!(
        unproven.settle(&dedicated_access(), &dedicated_id()),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: Closure::Discarded(DiscardReason::MfaAbsent),
        })
    );
}

#[test]
fn refuses_an_unsafe_key_ring() {
    let signer = Signer::new(KID);
    let (modulus, exponent) = signer.components();
    assert_eq!(exponent, EXPONENT);
    let key = |kid: &str| {
        TrustedRsaKey::from_components(kid, &modulus, &exponent).unwrap()
    };
    for (kid, n, e) in [
        ("", modulus.as_str(), EXPONENT),
        (KID, "AQAB", EXPONENT),
        (KID, &modulus[..341], EXPONENT),
        (KID, format!("{}+", &modulus[..341]).as_str(), EXPONENT),
        // Only 65537, canonically spelled.
        (KID, modulus.as_str(), "Aw"),
        (KID, modulus.as_str(), "AQAA"),
        (KID, modulus.as_str(), "AAEAAQ"),
        (KID, modulus.as_str(), "AQAB "),
        (KID, modulus.as_str(), ""),
    ] {
        assert_eq!(
            TrustedRsaKey::from_components(kid, n, e).err(),
            Some(VerifyError::BadKeyRing),
            "{e}"
        );
    }
    assert_eq!(
        TrustedKeyRing::new(ISSUER, Vec::new()).err(),
        Some(VerifyError::BadKeyRing)
    );
    assert_eq!(
        TrustedKeyRing::new(ISSUER, vec![key(KID), key(KID)]).err(),
        Some(VerifyError::BadKeyRing)
    );
    let too_many = (0..=MAX_KEYS)
        .map(|index| key(&format!("synthetic-kid-{index}")))
        .collect();
    assert_eq!(
        TrustedKeyRing::new(ISSUER, too_many).err(),
        Some(VerifyError::BadKeyRing)
    );
}
