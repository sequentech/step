// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic tests of the device grant transport's pure parts. Every value is
//! synthetic: no real IdP, code, token or secret, and no network.

use super::*;
use serde_json::{json, Map, Value};

const BASE: &str = "https://idp.synthetic.invalid";
const REALM: &str = "tenant-synthetic";
const VERIFICATION: &str =
    "https://idp.synthetic.invalid/realms/tenant-synthetic/device";
const USER_CODE: &str = "SYNT-HETC";
const SECRET: &str = "synthetic-client-secret";
const ACCESS: &str = "synthetic.access.token";
const ID: &str = "synthetic.id.token";
const REFRESH: &str = "synthetic.refresh.token";
const DESCRIPTION: &str = "synthetic-description-must-not-leak";

fn device_code() -> String {
    format!("synthetic-device-code-{}", "A".repeat(21))
}

fn verifier() -> String {
    format!("synthetic-pkce-verifier-{}", "0".repeat(22))
}

fn challenge() -> String {
    format!("synthetic-pkce-challenge-{}", "0".repeat(18))
}

fn pkce() -> PkceS256 {
    PkceS256::from_parts(verifier(), challenge()).unwrap()
}

fn endpoints() -> IdpEndpoints {
    IdpEndpoints::new(BASE, REALM).unwrap()
}

/// Keycloak's device authorization reply, changed by `edit`.
fn device_body(edit: impl FnOnce(&mut Map<String, Value>)) -> Vec<u8> {
    let mut reply = json!({
        "device_code": device_code(),
        "user_code": USER_CODE,
        "verification_uri": VERIFICATION,
        "verification_uri_complete":
            format!("{VERIFICATION}?user_code={USER_CODE}"),
        "expires_in": 600,
        "interval": 5,
    })
    .as_object()
    .unwrap()
    .clone();
    edit(&mut reply);
    serde_json::to_vec(&reply).unwrap()
}

fn parse_device(body: &[u8]) -> Result<DeviceAuthorization, TransportError> {
    parse_device_authorization(&endpoints(), 200, Some(JSON), body)
}

/// Keycloak's token reply, changed by `edit`.
fn token_body(edit: impl FnOnce(&mut Map<String, Value>)) -> Vec<u8> {
    let mut reply = json!({
        "access_token": ACCESS,
        "expires_in": 300,
        "refresh_expires_in": 1800,
        "refresh_token": REFRESH,
        "token_type": "Bearer",
        "id_token": ID,
        "not-before-policy": 0,
        "session_state": "synthetic-session-state",
        "scope": "openid",
    })
    .as_object()
    .unwrap()
    .clone();
    edit(&mut reply);
    serde_json::to_vec(&reply).unwrap()
}

fn error_body(code: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "error": code,
        "error_description": DESCRIPTION,
    }))
    .unwrap()
}

#[test]
fn derives_every_endpoint_from_the_configured_issuer_and_realm() {
    let endpoints = endpoints();
    let realm = "https://idp.synthetic.invalid/realms/tenant-synthetic";
    assert_eq!(endpoints.issuer(), realm);
    assert_eq!(
        endpoints.device_authorization.as_str(),
        format!("{realm}/protocol/openid-connect/auth/device")
    );
    assert_eq!(
        endpoints.token.as_str(),
        format!("{realm}/protocol/openid-connect/token")
    );
    assert_eq!(endpoints.verification.as_str(), VERIFICATION);

    let with_path =
        IdpEndpoints::new("https://idp.synthetic.invalid:8443/auth/", REALM)
            .unwrap();
    assert_eq!(
        with_path.issuer(),
        "https://idp.synthetic.invalid:8443/auth/realms/tenant-synthetic"
    );
    let default_port =
        IdpEndpoints::new("https://idp.synthetic.invalid:443", REALM).unwrap();
    assert_eq!(default_port.issuer(), realm);
}

#[test]
fn refuses_an_unsafe_issuer_or_realm() {
    for base in [
        "",
        "http://idp.synthetic.invalid",
        "https://user@idp.synthetic.invalid",
        "https://user:pw@idp.synthetic.invalid",
        "https://idp.synthetic.invalid/?q=1",
        "https://idp.synthetic.invalid/#f",
        "https://idp.synthetic.invalid/a%2Fb",
        " https://idp.synthetic.invalid",
        "file:///synthetic",
        "not a url",
    ] {
        assert_eq!(
            IdpEndpoints::new(base, REALM).err(),
            Some(TransportError::BadIssuer),
            "{base}"
        );
    }
    for realm in ["", "a/b", "../master", "tenant synthetic", "tenant%2F"] {
        assert_eq!(
            IdpEndpoints::new(BASE, realm).err(),
            Some(TransportError::BadRealm),
            "{realm}"
        );
    }
}

#[test]
fn accepts_only_the_realms_own_verification_page() {
    let endpoints = endpoints();
    assert!(endpoints.accepts_verification_uri(VERIFICATION));
    for wrong in [
        // Alternate spellings of the same address.
        "https://idp.synthetic.invalid:443/realms/tenant-synthetic/device",
        "https://IDP.synthetic.invalid/realms/tenant-synthetic/device",
        "HTTPS://idp.synthetic.invalid/realms/tenant-synthetic/device",
        "https://idp.synthetic.invalid./realms/tenant-synthetic/device",
        "https://idp.synthetic.invalid/realms/x/../tenant-synthetic/device",
        "https://idp.synthetic.invalid/realms/./tenant-synthetic/device",
        "https://idp.synthetic.invalid/realms/tenant-synthetic/%64evice",
        r"https://idp.synthetic.invalid\realms\tenant-synthetic\device",
        r"https:\\idp.synthetic.invalid/realms/tenant-synthetic/device",
        "https:idp.synthetic.invalid/realms/tenant-synthetic/device",
        // Origin.
        "https://other.synthetic.invalid/realms/tenant-synthetic/device",
        "http://idp.synthetic.invalid/realms/tenant-synthetic/device",
        // Port.
        "https://idp.synthetic.invalid:8443/realms/tenant-synthetic/device",
        // Path: a realm's short verification URI, another realm, variants.
        "https://idp.synthetic.invalid/device",
        "https://idp.synthetic.invalid/realms/other-synthetic/device",
        "https://idp.synthetic.invalid/realms/tenant-synthetic/device/",
        "https://idp.synthetic.invalid/realms/tenant-synthetic/device/status",
        // Query, fragment, user information, padding.
        "https://idp.synthetic.invalid/realms/tenant-synthetic/device?a=b",
        "https://idp.synthetic.invalid/realms/tenant-synthetic/device#a",
        "https://u@idp.synthetic.invalid/realms/tenant-synthetic/device",
        " https://idp.synthetic.invalid/realms/tenant-synthetic/device",
        "",
    ] {
        assert!(!endpoints.accepts_verification_uri(wrong), "{wrong}");
    }

    let on_port =
        IdpEndpoints::new("https://idp.synthetic.invalid:8443", REALM).unwrap();
    assert!(!on_port.accepts_verification_uri(VERIFICATION));
}

#[test]
fn parses_keycloaks_device_authorization_and_drops_the_complete_uri() {
    let authorization = parse_device(&device_body(|_| {})).unwrap();
    assert_eq!(authorization.verification_uri(), VERIFICATION);
    assert_eq!(authorization.user_code_for_protected_console(), USER_CODE);
    assert_eq!(authorization.expires_in(), 600);
    assert_eq!(authorization.interval(), Some(5));
    assert_eq!(authorization.device_code.0, device_code());

    let without_interval = device_body(|reply| {
        reply.remove("interval");
        reply.remove("verification_uri_complete");
    });
    assert_eq!(parse_device(&without_interval).unwrap().interval(), None);

    let charset = parse_device_authorization(
        &endpoints(),
        200,
        Some("Application/JSON; charset=utf-8"),
        &device_body(|_| {}),
    );
    assert!(charset.is_ok());
}

#[test]
fn refuses_missing_or_malformed_device_authorization_fields() {
    let mut bodies: Vec<Vec<u8>> =
        ["device_code", "user_code", "expires_in", "verification_uri"]
            .into_iter()
            .map(|member| {
                device_body(|r| {
                    r.remove(member);
                })
            })
            .collect();
    for (member, value) in [
        ("expires_in", json!("600")),
        ("expires_in", json!(-1)),
        ("expires_in", json!(600.5)),
        ("expires_in", json!(0)),
        ("expires_in", json!(86_401)),
        ("interval", json!(0)),
        ("interval", json!(3_601)),
        ("device_code", json!("short")),
        ("device_code", json!(format!("{}+/=", device_code()))),
        ("device_code", json!("A".repeat(257))),
        ("user_code", json!("SYNT HETC")),
        ("user_code", json!("SY")),
        ("user_code", json!(1234)),
    ] {
        bodies.push(device_body(|r| {
            r.insert(member.into(), value);
        }));
    }
    for body in bodies {
        assert_eq!(parse_device(&body).err(), Some(TransportError::Malformed));
    }

    // Duplicate members, trailing data, not JSON.
    let duplicate = format!(
        r#"{{"device_code":"{code}","device_code":"{code}","user_code":"{USER_CODE}","verification_uri":"{VERIFICATION}","expires_in":600}}"#,
        code = device_code()
    );
    for body in [
        duplicate.as_bytes(),
        b"{}{}".as_slice(),
        b"synthetic".as_slice(),
    ] {
        assert_eq!(parse_device(body).err(), Some(TransportError::Malformed));
    }
}

#[test]
fn refuses_a_device_authorization_sending_the_user_elsewhere() {
    for address in [
        "https://other.synthetic.invalid/realms/tenant-synthetic/device",
        "https://idp.synthetic.invalid/realms/x/../tenant-synthetic/device",
        "https://IDP.synthetic.invalid/realms/tenant-synthetic/device",
        r"https://idp.synthetic.invalid\realms\tenant-synthetic\device",
        // Another realm's page: the realm identity must match too.
        "https://idp.synthetic.invalid/realms/tenant-synthetic-2/device",
    ] {
        let elsewhere = device_body(|r| {
            r.insert("verification_uri".into(), json!(address));
        });
        assert_eq!(
            parse_device(&elsewhere).err(),
            Some(TransportError::VerificationUriMismatch),
            "{address}"
        );
    }

    // This realm's page sent to a transport configured for another realm.
    let other_realm = IdpEndpoints::new(BASE, "tenant-synthetic-2").unwrap();
    assert_eq!(
        parse_device_authorization(
            &other_realm,
            200,
            Some(JSON),
            &device_body(|_| {})
        )
        .err(),
        Some(TransportError::VerificationUriMismatch)
    );
}

#[test]
fn shows_only_the_locally_derived_verification_address() {
    // Configured with an explicit default port: the derived address drops
    // it, and only that spelling is accepted and shown.
    let endpoints =
        IdpEndpoints::new("https://idp.synthetic.invalid:443/", REALM).unwrap();
    assert_eq!(endpoints.verification_uri(), VERIFICATION);
    let spelled = device_body(|r| {
        r.insert(
            "verification_uri".into(),
            json!("https://idp.synthetic.invalid:443/realms/tenant-synthetic/device"),
        );
    });
    assert_eq!(
        parse_device_authorization(&endpoints, 200, Some(JSON), &spelled).err(),
        Some(TransportError::VerificationUriMismatch)
    );
    let authorization = parse_device_authorization(
        &endpoints,
        200,
        Some(JSON),
        &device_body(|_| {}),
    )
    .unwrap();
    assert_eq!(
        authorization.verification_uri(),
        endpoints.verification_uri()
    );
}

/// A model machine for the synthetic scope, with a challenge begun at 0.
fn machine() -> (DeviceLogin, crate::device::ChallengeId) {
    let scope = crate::device::DeviceScope::new(crate::device::ScopeInput {
        issuer: "https://idp.synthetic.invalid/realms/tenant-synthetic",
        audience: "synthetic-audience",
        client_id: "synthetic-device-client",
        subject: "synthetic-subject-0001",
        tenant: "synthetic-tenant",
        trustee_name: "synthetic-trustee-a",
        required_role: "synthetic-trustee-role",
    })
    .unwrap();
    let mut machine = DeviceLogin::new(scope);
    let challenge = machine.begin(at(0), 600, None).unwrap();
    (machine, challenge)
}

fn at(offset: u64) -> Clock {
    Clock {
        mono: crate::device::Monotonic::from_secs(1_000 + offset),
        wall: 1_800_000_000 + offset,
    }
}

/// A poll's reply holding a real ticket of `machine`, as `poll` returns it.
fn polled(
    machine: &mut DeviceLogin,
    challenge: crate::device::ChallengeId,
    offset: u64,
    reply: PollReply,
) -> Polled {
    let ticket = machine.start_poll(at(offset).mono, challenge).unwrap();
    Polled {
        ticket,
        reply,
        nonce: DeviceNonce::generate().unwrap(),
    }
}

#[test]
fn a_poll_reply_settles_only_through_the_ticket_it_spent() {
    let (mut machine, challenge) = machine();
    // No ticket before the interval: no poll.
    assert_eq!(
        machine.start_poll(at(4).mono, challenge).err(),
        Some(DeviceError::TooEarly { wait: 1 })
    );
    let pending = polled(&mut machine, challenge, 5, PollReply::Pending);
    assert!(matches!(pending.reply(), PollReply::Pending));
    assert!(matches!(
        pending.settle(&mut machine, at(5)),
        Ok(Settled::Status(DeviceStatus::Pending { poll_in: 5, .. }))
    ));

    let failed = polled(&mut machine, challenge, 10, PollReply::Unavailable);
    assert!(matches!(
        failed.settle(&mut machine, at(10)),
        Ok(Settled::Status(DeviceStatus::Pending { poll_in: 10, .. }))
    ));

    let denied = polled(&mut machine, challenge, 20, PollReply::Denied);
    assert!(matches!(
        denied.settle(&mut machine, at(20)),
        Ok(Settled::Status(DeviceStatus::Closed {
            closure: crate::device::Closure::Denied,
            ..
        }))
    ));
}

#[test]
fn a_token_reply_waits_for_a_verifier_or_is_discarded() {
    let (mut machine, challenge) = machine();
    let body = token_body(|_| {});
    let reply = parse_poll(200, Some(JSON), &body);
    let polled = polled(&mut machine, challenge, 5, reply);
    let Ok(Settled::AwaitingVerification(waiting)) =
        polled.settle(&mut machine, at(5))
    else {
        panic!("expected a reply awaiting verification");
    };
    // Still in flight: no other poll meanwhile, and no grant.
    assert_eq!(
        machine.start_poll(at(6).mono, challenge).err(),
        Some(DeviceError::OutOfOrder)
    );
    assert_eq!(machine.authorize(at(6)), Err(DeviceError::NoGrant));
    assert!(matches!(
        waiting.discard(&mut machine, at(6)),
        Ok(DeviceStatus::Closed {
            closure: crate::device::Closure::Denied,
            ..
        })
    ));
    assert_eq!(machine.authorize(at(6)), Err(DeviceError::NoGrant));
}

#[test]
fn a_poll_reply_cannot_settle_another_machine() {
    let (mut own, challenge) = machine();
    let (mut other, _) = machine();
    let polled = polled(&mut own, challenge, 5, PollReply::Denied);
    assert!(matches!(
        polled.settle(&mut other, at(5)),
        Err(DeviceError::ForeignTicket)
    ));
    assert!(matches!(other.status(at(5)), DeviceStatus::Pending { .. }));
}

// The verifier's synthetic fixture: a machine, a verifier made for it and
// a synthetic key that signs real RS256 pairs.
use crate::device::verify::tests as signed;

/// Polls the fixture's machine once with a token reply carrying this pair,
/// and settles it to the point of verification.
fn awaiting_pair(
    fixture: &mut signed::Fixture,
    access: &signed::ClaimSet,
    id: &signed::ClaimSet,
) -> AwaitingVerification {
    let nonce = fixture.device_nonce.clone();
    awaiting_pair_for(fixture, access, id, nonce)
}

/// The fixture's own grant (its binding, with `nonce` as the device
/// request's), polled through the bound path with the tokens' reply.
fn awaiting_pair_for(
    fixture: &mut signed::Fixture,
    access: &signed::ClaimSet,
    id: &signed::ClaimSet,
    nonce: DeviceNonce,
) -> AwaitingVerification {
    let body = fixture.token_reply(access, id);
    let bound = BoundDeviceGrant {
        grant: grant_with(nonce),
        binding: fixture.binding.take().expect("fixture binding"),
    };
    let Ok(ticket) = bound.claim_ticket(fixture.ticket()) else {
        panic!("the fixture's own ticket");
    };
    let polled = bound.polled(ticket, parse_poll(200, Some(JSON), &body));
    let Ok(Settled::AwaitingVerification(waiting)) =
        polled.settle(&mut fixture.machine, signed::clock(5, signed::NOW))
    else {
        panic!("expected a reply awaiting verification");
    };
    waiting
}

/// A pending grant as `authorize` makes it, with `nonce` as its request's.
fn grant_with(nonce: DeviceNonce) -> PendingDeviceGrant {
    PendingDeviceGrant {
        authorization: parse_device(&device_body(|_| {})).unwrap(),
        pkce: pkce(),
        nonce,
    }
}

fn grant() -> PendingDeviceGrant {
    grant_with(DeviceNonce::generate().unwrap())
}

/// A model machine for the synthetic scope with no challenge yet.
fn idle_machine() -> DeviceLogin {
    let scope = crate::device::DeviceScope::new(crate::device::ScopeInput {
        issuer: "https://idp.synthetic.invalid/realms/tenant-synthetic",
        audience: "synthetic-audience",
        client_id: "synthetic-device-client",
        subject: "synthetic-subject-0001",
        tenant: "synthetic-tenant",
        trustee_name: "synthetic-trustee-a",
        required_role: "synthetic-trustee-role",
    })
    .unwrap();
    DeviceLogin::new(scope)
}

/// The grant, begun and bound on `machine` at `at(0)`.
fn bound_on(machine: &mut DeviceLogin) -> BoundDeviceGrant {
    match grant().begin(machine, at(0)) {
        Ok(bound) => bound,
        Err((error, _)) => panic!("begin: {error:?}"),
    }
}

/// Whether `bound` refused `ticket`, handing it back unspent.
fn refused(bound: &BoundDeviceGrant, ticket: PollTicket) -> Option<PollTicket> {
    match bound.claim_ticket(ticket) {
        Ok(_) => None,
        Err((TransportError::ForeignTicket, ticket)) => Some(ticket),
        Err(_) => None,
    }
}

#[test]
fn a_bound_grant_polls_only_with_its_own_machines_challenge_ticket() {
    let mut own = idle_machine();
    let mut other = idle_machine();
    let bound = bound_on(&mut own);
    let _other_bound = bound_on(&mut other);
    // Both machines are at an equal-looking challenge, with the same timing.
    let own_challenge = pending_challenge(&mut own, 0);
    assert_eq!(pending_challenge(&mut other, 0), own_challenge);
    assert!(matches!(
        own.start_poll(at(4).mono, own_challenge).err(),
        Some(DeviceError::TooEarly { wait: 1 })
    ));
    let other_ticket = other.start_poll(at(5).mono, own_challenge).unwrap();
    // The equal-looking foreign ticket is refused before any request and
    // comes back unspent: the other machine settles with it as usual.
    let other_ticket = refused(&bound, other_ticket).expect("refused");
    assert!(matches!(
        other.settle(at(5), &other_ticket, PollOutcome::AuthorizationPending),
        Ok(DeviceStatus::Pending { .. })
    ));
    // The grant's own ticket is taken.
    let own_ticket = own.start_poll(at(5).mono, own_challenge).unwrap();
    let Ok(own_ticket) = bound.claim_ticket(own_ticket) else {
        panic!("own ticket refused");
    };
    let polled = bound.polled(own_ticket, PollReply::Pending);
    assert!(matches!(
        polled.settle(&mut own, at(5)),
        Ok(Settled::Status(DeviceStatus::Pending { .. }))
    ));
}

#[test]
fn a_replaced_authorization_or_restart_never_takes_the_old_grant() {
    // The machine's challenge ended and a new authorization began another:
    // the old grant cannot poll the new challenge.
    let mut machine = idle_machine();
    let first = bound_on(&mut machine);
    let first_challenge = pending_challenge(&mut machine, 0);
    let ticket = machine.start_poll(at(5).mono, first_challenge).unwrap();
    machine
        .settle(at(5), &ticket, PollOutcome::ExpiredToken)
        .unwrap();
    let second = match grant().begin(&mut machine, at(6)) {
        Ok(bound) => bound,
        Err((error, _)) => panic!("begin: {error:?}"),
    };
    let second_challenge = pending_challenge(&mut machine, 6);
    assert_ne!(second_challenge, first_challenge);
    let ticket = machine.start_poll(at(11).mono, second_challenge).unwrap();
    let ticket = refused(&first, ticket).expect("old grant refused");
    assert!(second.claim_ticket(ticket).is_ok());

    // After a restart, a new machine at challenge 1 shares nothing with the
    // old grant.
    let mut restarted = idle_machine();
    let _fresh = bound_on(&mut restarted);
    let restarted_challenge = pending_challenge(&mut restarted, 0);
    let ticket = restarted
        .start_poll(at(5).mono, restarted_challenge)
        .unwrap();
    assert!(refused(&first, ticket).is_some());
}

#[test]
fn a_grant_that_cannot_begin_comes_back_unchanged() {
    let mut machine = idle_machine();
    let _bound = bound_on(&mut machine);
    let pending = pending_challenge(&mut machine, 0);
    // The machine's challenge is pending: another grant cannot begin on it,
    // and is handed back, with the proper pending challenge untouched.
    let Err((error, returned)) = grant().begin(&mut machine, at(1)) else {
        panic!("began on a busy machine");
    };
    assert_eq!(error, DeviceError::Busy);
    assert_eq!(returned.authorization().expires_in(), 600);
    assert_eq!(pending_challenge(&mut machine, 1), pending);
    assert!(machine.start_poll(at(5).mono, pending).is_ok());
}

/// The challenge `machine` has pending, read from its status at `at(offset)`
/// (no earlier than the machine's last observed time).
fn pending_challenge(
    machine: &mut DeviceLogin,
    offset: u64,
) -> crate::device::ChallengeId {
    match machine.status(at(offset)) {
        DeviceStatus::Pending { challenge, .. } => challenge,
        _ => panic!("no pending challenge"),
    }
}

#[test]
fn a_bound_grant_shows_no_secret() {
    let mut machine = idle_machine();
    let nonce = DeviceNonce::generate().unwrap();
    let nonce_value = nonce.form_value().to_owned();
    let bound = match grant_with(nonce).begin(&mut machine, at(0)) {
        Ok(bound) => bound,
        Err((error, _)) => panic!("begin: {error:?}"),
    };
    let shown = format!("{bound:?}");
    for secret in [
        USER_CODE.to_owned(),
        device_code(),
        verifier(),
        challenge(),
        nonce_value,
    ] {
        assert!(!shown.contains(&secret), "{shown}");
    }
}

#[test]
fn the_bound_path_settles_valid_and_wrong_scope_pairs() {
    // A valid pair through the bound path: discarded for want of a factor.
    let mut fixture = signed::fixture();
    let challenge = fixture.challenge;
    let waiting = awaiting_pair(
        &mut fixture,
        &signed::access_claims(),
        &signed::id_claims(),
    );
    let clock = signed::clock(6, signed::NOW);
    assert_eq!(
        waiting.verify_and_settle(
            &mut fixture.machine,
            clock,
            &fixture.verifier
        ),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: crate::device::Closure::Discarded(
                crate::device::DiscardReason::MfaAbsent
            ),
        })
    );
    // Another trustee's pair for the same request: wrong scope, denied.
    let mut fixture = signed::fixture();
    let challenge = fixture.challenge;
    let mut other = signed::access_claims();
    signed::set(&mut other, "sub", json!(signed::OTHER_UUID));
    let waiting = awaiting_pair(&mut fixture, &other, &signed::id_claims());
    assert_eq!(
        waiting.verify_and_settle(
            &mut fixture.machine,
            clock,
            &fixture.verifier
        ),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: crate::device::Closure::Denied,
        })
    );
}

#[test]
fn a_signed_pair_settles_its_own_ticket_and_is_discarded_unproven() {
    let mut fixture = signed::fixture();
    let challenge = fixture.challenge;
    let waiting = awaiting_pair(
        &mut fixture,
        &signed::access_claims(),
        &signed::id_claims(),
    );
    let clock = signed::clock(6, signed::NOW);
    assert_eq!(
        waiting.verify_and_settle(
            &mut fixture.machine,
            clock,
            &fixture.verifier
        ),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: crate::device::Closure::Discarded(
                crate::device::DiscardReason::MfaAbsent
            ),
        })
    );
    assert_eq!(fixture.machine.authorize(clock), Err(DeviceError::NoGrant));
}

#[test]
fn a_pair_of_another_device_request_closes_this_ticket_as_denied() {
    // The poll belongs to one device request's nonce; the tokens were
    // issued for another (stock: same reused session, other device code).
    let mut fixture = signed::fixture();
    let challenge = fixture.challenge;
    let waiting = awaiting_pair_for(
        &mut fixture,
        &signed::access_claims(),
        &signed::id_claims(),
        DeviceNonce::generate().unwrap(),
    );
    let clock = signed::clock(5, signed::NOW);
    assert_eq!(
        waiting.verify_and_settle(
            &mut fixture.machine,
            clock,
            &fixture.verifier
        ),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: crate::device::Closure::Denied,
        })
    );
}

#[test]
fn an_unverifiable_pair_closes_its_own_ticket_as_denied() {
    let denied = |challenge| {
        Ok(DeviceStatus::Closed {
            challenge,
            closure: crate::device::Closure::Denied,
        })
    };
    let clock = signed::clock(6, signed::NOW);

    // Another actor's identity, validly signed.
    let mut fixture = signed::fixture();
    let challenge = fixture.challenge;
    let mut other = signed::access_claims();
    signed::set(&mut other, "sub", json!(signed::OTHER_UUID));
    let waiting = awaiting_pair(&mut fixture, &other, &signed::id_claims());
    assert_eq!(
        waiting.verify_and_settle(
            &mut fixture.machine,
            clock,
            &fixture.verifier
        ),
        denied(challenge)
    );
    assert_eq!(fixture.machine.authorize(clock), Err(DeviceError::NoGrant));

    // A verifier made for another machine.
    let mut fixture = signed::fixture();
    let stranger = signed::fixture();
    let challenge = fixture.challenge;
    let waiting = awaiting_pair(
        &mut fixture,
        &signed::access_claims(),
        &signed::id_claims(),
    );
    assert_eq!(
        waiting.verify_and_settle(
            &mut fixture.machine,
            clock,
            &stranger.verifier
        ),
        denied(challenge)
    );

    // Expired by the time it is verified.
    let mut fixture = signed::fixture();
    let challenge = fixture.challenge;
    let waiting = awaiting_pair(
        &mut fixture,
        &signed::access_claims(),
        &signed::id_claims(),
    );
    let late = signed::clock(6, signed::NOW + 280);
    assert_eq!(
        waiting.verify_and_settle(
            &mut fixture.machine,
            late,
            &fixture.verifier
        ),
        denied(challenge)
    );
}

#[test]
fn classifies_device_authorization_statuses_and_media() {
    let parse = |status, content_type, body: &[u8]| {
        parse_device_authorization(&endpoints(), status, content_type, body)
            .err()
    };
    let body = device_body(|_| {});
    assert_eq!(
        parse(200, Some("text/html"), &body),
        Some(TransportError::WrongContentType)
    );
    assert_eq!(
        parse(200, None, &body),
        Some(TransportError::WrongContentType)
    );
    let oversized = vec![b' '; DEVICE_RESPONSE_LIMIT + 1];
    assert_eq!(
        parse(200, Some(JSON), &oversized),
        Some(TransportError::TooLarge)
    );
    assert_eq!(
        parse(401, Some(JSON), &error_body("invalid_client")),
        Some(TransportError::Refused(OAuthErrorCode::InvalidClient))
    );
    assert_eq!(
        parse(400, Some(JSON), &error_body("invalid_grant")),
        Some(TransportError::Refused(OAuthErrorCode::InvalidGrant))
    );
    assert_eq!(
        parse(400, Some(JSON), &error_body("Not A Code")),
        Some(TransportError::Malformed)
    );
    assert_eq!(parse(503, None, b""), Some(TransportError::Unavailable));
    assert_eq!(parse(429, None, b""), Some(TransportError::Unavailable));
    for status in [201, 302, 403, 404] {
        assert_eq!(
            parse(status, Some(JSON), &body),
            Some(TransportError::UnexpectedStatus)
        );
    }
    // The refusal shows the code, never the IdP's description.
    let refused = parse(401, Some(JSON), &error_body("invalid_client"));
    assert!(!format!("{refused:?}").contains(DESCRIPTION));
}

#[test]
fn feeds_poll_replies_to_the_model() {
    let poll = |status, body: &[u8]| {
        parse_poll(status, Some(JSON), body).into_model_step()
    };
    assert!(matches!(
        poll(400, &error_body("authorization_pending")),
        ModelStep::Settle(PollOutcome::AuthorizationPending)
    ));
    assert!(matches!(
        poll(400, &error_body("slow_down")),
        ModelStep::Settle(PollOutcome::SlowDown)
    ));
    assert!(matches!(
        poll(400, &error_body("access_denied")),
        ModelStep::Settle(PollOutcome::AccessDenied)
    ));
    assert!(matches!(
        poll(400, &error_body("expired_token")),
        ModelStep::Settle(PollOutcome::ExpiredToken)
    ));
    // Terminal errors and broken replies end the flow without a grant.
    for body in [
        error_body("invalid_grant"),
        error_body("invalid_client"),
        error_body("synthetic_unknown"),
        error_body(""),
        b"synthetic".to_vec(),
    ] {
        assert!(matches!(
            poll(400, &body),
            ModelStep::Settle(PollOutcome::AccessDenied)
        ));
    }
    for status in [302, 403, 404] {
        assert!(matches!(
            poll(status, b""),
            ModelStep::Settle(PollOutcome::AccessDenied)
        ));
    }
    // Transport trouble backs off and keeps the flow.
    for status in [429, 500, 502, 503] {
        assert!(matches!(poll(status, b""), ModelStep::PollFailed));
    }
    assert!(matches!(
        parse_poll(400, Some(JSON), &error_body("invalid_grant")),
        PollReply::Refused(OAuthErrorCode::InvalidGrant)
    ));
}

#[test]
fn keeps_a_token_reply_unverified() {
    let reply = parse_poll(200, Some(JSON), &token_body(|_| {}));
    let PollReply::TokenIssued(tokens) = reply else {
        panic!("expected a token reply");
    };
    assert_eq!(tokens.access_token.0, ACCESS);
    assert_eq!(tokens.id_token.0, ID);
    let refresh = tokens.refresh_token.as_ref().map(|t| t.0.as_str());
    assert_eq!(refresh, Some(REFRESH));
    assert_eq!(tokens.expires_in, 300);
    let shown = format!("{tokens:?}");
    for secret in [ACCESS, ID, REFRESH] {
        assert!(!shown.contains(secret));
    }
    // Never an approval: the model only hears of it through a verifier.
    assert!(matches!(
        PollReply::TokenIssued(tokens).into_model_step(),
        ModelStep::AwaitVerification(_)
    ));

    let without_refresh = token_body(|r| {
        r.remove("refresh_token");
    });
    assert!(matches!(
        parse_poll(200, Some(JSON), &without_refresh),
        PollReply::TokenIssued(_)
    ));
}

#[test]
fn refuses_a_malformed_token_reply() {
    let mut bodies: Vec<Vec<u8>> = ["id_token", "access_token", "expires_in"]
        .into_iter()
        .map(|member| {
            token_body(|r| {
                r.remove(member);
            })
        })
        .collect();
    for (member, value) in [
        ("token_type", json!("mac")),
        ("access_token", json!("")),
        ("access_token", json!("a b")),
        ("id_token", json!("a".repeat(TOKEN_LIMIT + 1))),
        ("expires_in", json!(0)),
    ] {
        bodies.push(token_body(|r| {
            r.insert(member.into(), value);
        }));
    }
    for body in bodies {
        assert!(matches!(
            parse_poll(200, Some(JSON), &body),
            PollReply::Malformed
        ));
    }
    assert!(matches!(
        parse_poll(200, Some("text/plain"), &token_body(|_| {})),
        PollReply::Malformed
    ));
    let oversized = vec![b' '; TOKEN_RESPONSE_LIMIT + 1];
    assert!(matches!(
        parse_poll(200, Some(JSON), &oversized),
        PollReply::Malformed
    ));
}

#[test]
fn bounds_the_body_while_reading() {
    assert!(declared_length_fits(None, 10));
    assert!(declared_length_fits(Some(10), 10));
    assert!(!declared_length_fits(Some(11), 10));

    let mut body = Vec::new();
    append_bounded(&mut body, b"12345", 10).unwrap();
    append_bounded(&mut body, b"67890", 10).unwrap();
    assert_eq!(
        append_bounded(&mut body, b"x", 10),
        Err(TransportError::TooLarge)
    );
    assert_eq!(body, b"1234567890");
}

#[test]
fn requires_an_explicit_s256_pair_used_once() {
    assert!(PkceS256::from_parts(verifier(), challenge()).is_ok());
    for (verifier, challenge) in [
        ("a".repeat(42), challenge()),
        ("a".repeat(129), challenge()),
        (format!("{} ", verifier()), challenge()),
        (verifier(), challenge()[..42].to_owned()),
        (verifier(), format!("{}~", &challenge()[..42])),
        // A "plain" challenge equal to its verifier.
        ("A".repeat(43), "A".repeat(43)),
    ] {
        assert_eq!(
            PkceS256::from_parts(verifier, challenge).err(),
            Some(TransportError::BadPkce)
        );
    }

    let mut ledger = ChallengeLedger::default();
    ledger.register(&pkce()).unwrap();
    assert_eq!(ledger.register(&pkce()), Err(TransportError::PkceReused));

    // Each device request's nonce is single-use too.
    let nonce = DeviceNonce::generate().unwrap();
    ledger.register_nonce(&nonce).unwrap();
    assert_eq!(
        ledger.register_nonce(&nonce.clone()),
        Err(TransportError::NonceReused)
    );
    ledger
        .register_nonce(&DeviceNonce::generate().unwrap())
        .unwrap();
}

#[test]
fn sends_each_secret_only_where_keycloak_expects_it() {
    let client_id = ClientId::new("synthetic-device-client").unwrap();
    let auth = ClientAuthentication::SecretPost(
        ClientSecret::new(SECRET.to_owned()).unwrap(),
    );
    let pkce = pkce();
    let nonce = DeviceNonce::generate().unwrap();
    let authorization = authorization_form(&client_id, &auth, &pkce, &nonce);
    // The request's own nonce goes in its own stock form, and nowhere else.
    assert_eq!(
        authorization,
        vec![
            ("client_id", "synthetic-device-client"),
            ("client_secret", SECRET),
            ("scope", "openid"),
            ("code_challenge", challenge().as_str()),
            ("code_challenge_method", "S256"),
            ("nonce", nonce.form_value()),
        ]
    );

    let grant = PendingDeviceGrant {
        authorization: parse_device(&device_body(|_| {})).unwrap(),
        pkce,
        nonce,
    };
    let code = device_code();
    let verifier = verifier();
    assert_eq!(
        token_form(&client_id, &auth, &grant),
        vec![
            ("grant_type", DEVICE_GRANT_TYPE),
            ("client_id", "synthetic-device-client"),
            ("client_secret", SECRET),
            ("device_code", code.as_str()),
            ("code_verifier", verifier.as_str()),
        ]
    );
    assert_eq!(grant.authorization().expires_in(), 600);
}

#[test]
fn shows_no_secret_in_debug() {
    let auth = ClientAuthentication::SecretPost(
        ClientSecret::new(SECRET.to_owned()).unwrap(),
    );
    let nonce = DeviceNonce::generate().unwrap();
    let nonce_value = nonce.form_value().to_owned();
    let grant = PendingDeviceGrant {
        authorization: parse_device(&device_body(|_| {})).unwrap(),
        pkce: pkce(),
        nonce,
    };
    let shown = format!("{auth:?} {grant:?} {:?}", grant.authorization());
    for secret in [
        SECRET.to_owned(),
        USER_CODE.to_owned(),
        device_code(),
        verifier(),
        challenge(),
        nonce_value,
    ] {
        assert!(!shown.contains(&secret), "{shown}");
    }
}

#[test]
fn refuses_bad_client_credentials_and_ca_bundles() {
    for id in ["", "a b", "client/../x", &"a".repeat(256)] {
        assert_eq!(ClientId::new(id).err(), Some(TransportError::BadClientId));
    }
    for secret in [String::new(), "a b".into(), "a".repeat(4_097)] {
        assert_eq!(
            ClientSecret::new(secret).err(),
            Some(TransportError::BadClientSecret)
        );
    }
    let garbage =
        b"-----BEGIN CERTIFICATE-----\nsynthetic\n-----END CERTIFICATE-----\n";
    for pem in [b"".as_slice(), b"synthetic".as_slice(), garbage.as_slice()] {
        assert_eq!(build_client(pem).err(), Some(TransportError::BadCaBundle));
    }
}
