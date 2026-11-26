// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Synthetic tests of the device login model. Every label is synthetic; there
//! is no real authenticator, token or account here. Tickets are only ever
//! the ones the machines issue.

use super::*;

/// Synthetic wall and monotonic origins; `at(n)` is n seconds after both.
const T0: UnixSeconds = 1_800_000_000;
const M0: u64 = 1_000;
const ISSUER: &str = "https://idp.synthetic.invalid/realms/tenant-synthetic";
const AUDIENCE: &str = "synthetic-audience";
const CLIENT: &str = "synthetic-device-client";
const SUBJECT: &str = "synthetic-subject-0001";
const OTHER_SUBJECT: &str = "synthetic-subject-0002";
const TENANT: &str = "synthetic-tenant";
const TRUSTEE: &str = "synthetic-trustee-a";
const ROLE: &str = "synthetic-trustee-role";
const SESSION: &str = "synthetic-idp-session-a";
const OTHER_SESSION: &str = "synthetic-idp-session-b";
const LIFETIME: u64 = 600;

fn mono(offset: u64) -> Monotonic {
    Monotonic::from_secs(M0 + offset)
}

fn at(offset: u64) -> Clock {
    Clock {
        mono: mono(offset),
        wall: T0 + offset,
    }
}

fn scope() -> DeviceScope {
    DeviceScope::new(ScopeInput {
        issuer: ISSUER,
        audience: AUDIENCE,
        client_id: CLIENT,
        subject: SUBJECT,
        tenant: TENANT,
        trustee_name: TRUSTEE,
        required_role: ROLE,
    })
    .unwrap()
}

fn totp(verified_at: UnixSeconds) -> MfaEvidence {
    MfaEvidence::Verified(FreshMfa {
        factor: SecondFactor::Totp,
        verified_at,
    })
}

/// A synthetic login in scope, authenticated with a fresh TOTP at `at`.
fn login(at: UnixSeconds) -> VerifiedLogin {
    VerifiedLogin {
        issuer: ISSUER.to_owned(),
        audiences: vec!["synthetic-other-audience".to_owned(), AUDIENCE.into()],
        authorized_party: CLIENT.to_owned(),
        subject: SUBJECT.to_owned(),
        tenant: TENANT.to_owned(),
        trustee_name: TRUSTEE.to_owned(),
        roles: vec![ROLE.to_owned()],
        issued_at: at,
        expires_at: at + 300,
        session_expires_at: None,
        session_id: SESSION.to_owned(),
        auth_time: at,
        mfa: totp(at),
    }
}

/// `login(T0 + 3)` changed by `edit`.
fn edited(edit: impl FnOnce(&mut VerifiedLogin)) -> VerifiedLogin {
    let mut changed = login(T0 + 3);
    edit(&mut changed);
    changed
}

/// A renewal of `login(T0 + 3)` issued at `T0 + 200`, changed by `edit`.
fn renewed(edit: impl FnOnce(&mut VerifiedLogin)) -> VerifiedLogin {
    edited(|l| {
        l.issued_at = T0 + 200;
        l.expires_at = T0 + 500;
        edit(l);
    })
}

/// A machine with a challenge begun at `at(offset)`.
fn pending_at(offset: u64) -> (DeviceLogin, ChallengeId) {
    let mut machine = DeviceLogin::new(scope());
    let challenge = machine.begin(at(offset), LIFETIME, None).unwrap();
    (machine, challenge)
}

fn poll(
    machine: &mut DeviceLogin,
    offset: u64,
    challenge: ChallengeId,
    outcome: PollOutcome,
) -> Result<DeviceStatus, DeviceError> {
    let ticket = machine.start_poll(mono(offset), challenge)?;
    machine.settle(at(offset), &ticket, outcome)
}

/// A machine whose challenge `login` settled at `at(5)`.
fn approved_with(login: VerifiedLogin) -> (DeviceLogin, ChallengeId) {
    let (mut machine, challenge) = pending_at(0);
    let status =
        poll(&mut machine, 5, challenge, PollOutcome::Approved(login));
    assert!(matches!(status, Ok(DeviceStatus::Approved { .. })));
    (machine, challenge)
}

fn approved() -> (DeviceLogin, ChallengeId) {
    approved_with(login(T0 + 3))
}

/// The closure of a challenge settled at `at(5)` with `login`.
fn settle_login(login: VerifiedLogin) -> DeviceStatus {
    let (mut machine, challenge) = pending_at(0);
    let status =
        poll(&mut machine, 5, challenge, PollOutcome::Approved(login))
            .unwrap();
    assert_eq!(machine.authorize(at(5)), Err(DeviceError::NoGrant));
    status
}

fn discarded(reason: DiscardReason) -> DeviceStatus {
    DeviceStatus::Closed {
        challenge: ChallengeId(1),
        closure: Closure::Discarded(reason),
    }
}

#[test]
fn approves_an_exact_scope_with_fresh_totp_exactly_once() {
    let (mut machine, challenge) = pending_at(0);
    let ticket = machine.start_poll(mono(5), challenge).unwrap();
    assert_eq!(
        machine.settle(at(5), &ticket, PollOutcome::Approved(login(T0 + 3))),
        Ok(DeviceStatus::Approved {
            challenge,
            expires_in: 298,
            session_ends_in: MAX_GRANT_LIFETIME - 2,
        })
    );
    assert_eq!(machine.authorize(at(5)), Ok(mono(303)));
    assert_eq!(
        machine.settle(at(6), &ticket, PollOutcome::Approved(login(T0 + 6))),
        Err(DeviceError::AlreadySettled)
    );
    assert_eq!(
        machine.start_poll(mono(10), challenge).err(),
        Some(DeviceError::AlreadySettled)
    );
    assert_eq!(machine.begin(at(10), LIFETIME, None), Err(DeviceError::Busy));
    assert_eq!(machine.authorize(at(303)), Err(DeviceError::NoGrant));
    assert_eq!(
        machine.status(at(303)),
        DeviceStatus::Closed {
            challenge,
            closure: Closure::Expired,
        }
    );
    assert_eq!(machine.begin(at(303), LIFETIME, None), Ok(ChallengeId(2)));
}

#[test]
fn discards_each_scope_mismatch_distinctly() {
    let issuer = edited(|l| l.issuer = format!("{ISSUER}/"));
    assert_eq!(settle_login(issuer), discarded(DiscardReason::Issuer));
    let audience =
        edited(|l| l.audiences = vec!["synthetic-other-audience".into()]);
    assert_eq!(settle_login(audience), discarded(DiscardReason::Audience));
    let azp = edited(|l| l.authorized_party = "synthetic-other-client".into());
    assert_eq!(
        settle_login(azp),
        discarded(DiscardReason::AuthorizedParty)
    );
    let subject = edited(|l| l.subject = OTHER_SUBJECT.into());
    assert_eq!(settle_login(subject), discarded(DiscardReason::Subject));
    let tenant = edited(|l| l.tenant = "synthetic-other-tenant".into());
    assert_eq!(settle_login(tenant), discarded(DiscardReason::Tenant));
    let name = edited(|l| l.trustee_name = "synthetic-trustee-b".into());
    assert_eq!(settle_login(name), discarded(DiscardReason::TrusteeName));
    let role = edited(|l| l.roles = vec!["synthetic-other-role".into()]);
    assert_eq!(settle_login(role), discarded(DiscardReason::Role));
}

#[test]
fn a_discarded_login_closes_only_its_own_challenge() {
    let (mut machine, challenge) = pending_at(0);
    let other = edited(|l| l.subject = OTHER_SUBJECT.into());
    poll(&mut machine, 5, challenge, PollOutcome::Approved(other)).unwrap();
    assert_eq!(
        machine.start_poll(mono(10), challenge).err(),
        Some(DeviceError::AlreadySettled)
    );
    let next = machine.begin(at(10), LIFETIME, None).unwrap();
    assert_ne!(next, challenge);
    assert_eq!(
        machine.start_poll(mono(15), challenge).err(),
        Some(DeviceError::WrongChallenge)
    );
}

#[test]
fn requires_a_fresh_proven_totp_not_an_assurance_level_or_auth_time() {
    let absent = edited(|l| l.mfa = MfaEvidence::Absent);
    assert_eq!(settle_login(absent), discarded(DiscardReason::MfaAbsent));

    // A fresh auth_time with a "gold" level alone proves no second factor.
    let gold = edited(|l| l.mfa = MfaEvidence::AssuranceLevelOnly);
    assert_eq!(
        settle_login(gold),
        discarded(DiscardReason::MfaAssuranceOnly)
    );

    let stale_mfa = edited(|l| l.mfa = totp(T0 - 1));
    assert_eq!(settle_login(stale_mfa), discarded(DiscardReason::MfaStale));

    // An SSO session from before the challenge.
    let stale_auth = edited(|l| l.auth_time = T0 - 1);
    assert_eq!(
        settle_login(stale_auth),
        discarded(DiscardReason::AuthTimeStale)
    );
}

#[test]
fn fails_closed_on_malformed_and_expired_claims() {
    let malformed = [
        edited(|l| l.issued_at = T0 + 6),
        edited(|l| l.auth_time = l.issued_at + 1),
        edited(|l| l.expires_at = l.issued_at),
        edited(|l| l.mfa = totp(T0 + 6)),
    ];
    for corrupt in malformed {
        assert_eq!(settle_login(corrupt), discarded(DiscardReason::Malformed));
    }

    let expired = edited(|l| l.expires_at = T0 + 5);
    assert_eq!(settle_login(expired), discarded(DiscardReason::Expired));

    let session_over = edited(|l| l.session_expires_at = Some(T0 + 4));
    assert_eq!(
        settle_login(session_over),
        discarded(DiscardReason::Expired)
    );
}

#[test]
fn caps_the_session_at_auth_time_plus_ten_hours_or_upstream() {
    let (mut machine, _) =
        approved_with(edited(|l| l.expires_at = T0 + 100_000));
    assert_eq!(
        machine.authorize(at(5)),
        Ok(mono(3 + MAX_GRANT_LIFETIME))
    );

    let (mut machine, _) = approved_with(edited(|l| {
        l.expires_at = T0 + 100_000;
        l.session_expires_at = Some(T0 + 1_000);
    }));
    assert_eq!(machine.authorize(at(5)), Ok(mono(1_000)));
    assert_eq!(machine.authorize(at(1_000)), Err(DeviceError::NoGrant));
}

#[test]
fn discards_a_login_whose_cap_overflows() {
    let start = UnixSeconds::MAX - 1_000;
    let mut machine = DeviceLogin::new(scope());
    let clock = |offset| Clock {
        mono: mono(offset),
        wall: start + offset,
    };
    let challenge = machine.begin(clock(0), LIFETIME, None).unwrap();
    let ticket = machine.start_poll(mono(5), challenge).unwrap();
    assert_eq!(
        machine.settle(
            clock(5),
            &ticket,
            PollOutcome::Approved(login(start + 3))
        ),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: Closure::Discarded(DiscardReason::Overflow),
        })
    );
    assert_eq!(machine.authorize(clock(5)), Err(DeviceError::NoGrant));
}

#[test]
fn a_wall_clock_rollback_after_approval_closes_the_grant() {
    let (mut machine, challenge) = approved();
    let rolled_back = Clock {
        mono: mono(100),
        wall: T0 - 1_000,
    };
    assert_eq!(
        machine.renew(rolled_back, renewed(|_| {})),
        Err(DeviceError::ClockRegressed)
    );
    assert_eq!(
        machine.status(at(100)),
        DeviceStatus::Closed {
            challenge,
            closure: Closure::ClockFault,
        }
    );
    assert_eq!(machine.authorize(at(100)), Err(DeviceError::NoGrant));
    assert_eq!(
        machine.renew(at(200), renewed(|_| {})),
        Err(DeviceError::NoGrant)
    );
}

#[test]
fn a_monotonic_regression_never_resurrects_or_extends_a_grant() {
    let (mut machine, challenge) = approved();
    assert_eq!(machine.authorize(at(303)), Err(DeviceError::NoGrant));
    assert_eq!(
        machine.authorize(at(100)),
        Err(DeviceError::ClockRegressed)
    );
    assert_eq!(
        machine.status(at(100)),
        DeviceStatus::Closed {
            challenge,
            closure: Closure::Expired,
        }
    );

    let (mut machine, challenge) = approved();
    assert_eq!(machine.authorize(at(100)), Ok(mono(303)));
    assert_eq!(
        machine.authorize(at(50)),
        Err(DeviceError::ClockRegressed)
    );
    assert_eq!(
        machine.status(at(150)),
        DeviceStatus::Closed {
            challenge,
            closure: Closure::ClockFault,
        }
    );
    assert_eq!(machine.authorize(at(150)), Err(DeviceError::NoGrant));
}

#[test]
fn renews_the_short_token_only_up_to_the_original_session_deadline() {
    let (mut machine, challenge) = approved();
    assert_eq!(
        machine.renew(at(200), renewed(|_| {})),
        Ok(DeviceStatus::Approved {
            challenge,
            expires_in: 300,
            session_ends_in: 3 + MAX_GRANT_LIFETIME - 200,
        })
    );
    assert_eq!(machine.authorize(at(200)), Ok(mono(500)));

    // A long token is cut at auth_time + 10 hours.
    let long = renewed(|l| {
        l.issued_at = T0 + 450;
        l.expires_at = T0 + 100_000;
    });
    machine.renew(at(450), long).unwrap();
    assert_eq!(
        machine.authorize(at(450)),
        Ok(mono(3 + MAX_GRANT_LIFETIME))
    );

    // An upstream session end known now brings the deadline closer...
    let shorter = renewed(|l| {
        l.issued_at = T0 + 500;
        l.expires_at = T0 + 100_000;
        l.session_expires_at = Some(T0 + 2_000);
    });
    machine.renew(at(500), shorter).unwrap();
    assert_eq!(machine.authorize(at(500)), Ok(mono(2_000)));

    // ...and a later one never pushes it back.
    let later = renewed(|l| {
        l.issued_at = T0 + 600;
        l.expires_at = T0 + 100_000;
        l.session_expires_at = Some(T0 + 30_000);
    });
    machine.renew(at(600), later).unwrap();
    assert_eq!(machine.authorize(at(600)), Ok(mono(2_000)));

    assert_eq!(machine.authorize(at(2_000)), Err(DeviceError::NoGrant));
    let too_late = renewed(|l| {
        l.issued_at = T0 + 2_001;
        l.expires_at = T0 + 3_000;
    });
    assert_eq!(
        machine.renew(at(2_001), too_late),
        Err(DeviceError::NoGrant)
    );
}

/// A grant approved with a token issued at `T0 + 5`, renewed at `at(200)`.
fn assert_renewal_refused(renewal: VerifiedLogin, reason: DiscardReason) {
    let (mut machine, challenge) =
        approved_with(edited(|l| l.issued_at = T0 + 5));
    assert_eq!(
        machine.renew(at(200), renewal),
        Err(DeviceError::RenewalRefused(reason))
    );
    assert_eq!(
        machine.status(at(200)),
        DeviceStatus::Closed {
            challenge,
            closure: Closure::RenewalRefused(reason),
        }
    );
    assert_eq!(machine.authorize(at(200)), Err(DeviceError::NoGrant));
}

#[test]
fn refuses_a_renewal_of_another_actor_or_authentication() {
    assert_renewal_refused(
        renewed(|l| l.subject = OTHER_SUBJECT.into()),
        DiscardReason::Subject,
    );
    assert_renewal_refused(
        renewed(|l| l.authorized_party = "synthetic-other-client".into()),
        DiscardReason::AuthorizedParty,
    );
    // A re-authentication is a new login, not a renewal.
    assert_renewal_refused(
        renewed(|l| {
            l.auth_time = T0 + 4;
            l.mfa = totp(T0 + 4);
        }),
        DiscardReason::AuthTimeChanged,
    );
    assert_renewal_refused(
        renewed(|l| l.mfa = totp(T0 + 4)),
        DiscardReason::MfaChanged,
    );
    assert_renewal_refused(
        renewed(|l| l.mfa = MfaEvidence::AssuranceLevelOnly),
        DiscardReason::MfaAssuranceOnly,
    );
}

#[test]
fn refuses_expired_or_contradictory_renewal_evidence() {
    assert_renewal_refused(
        renewed(|l| {
            l.issued_at = T0 + 100;
            l.expires_at = T0 + 200;
        }),
        DiscardReason::Expired,
    );
    // Older than the token it would replace.
    assert_renewal_refused(
        renewed(|l| l.issued_at = T0 + 4),
        DiscardReason::Malformed,
    );
    assert_renewal_refused(
        renewed(|l| l.issued_at = T0 + 201),
        DiscardReason::Malformed,
    );
}

#[test]
fn binds_renewal_to_the_approving_idp_session() {
    for sid in ["", " synthetic-idp-session-a", "synthetic idp session"] {
        let login = edited(|l| l.session_id = sid.into());
        assert_eq!(
            settle_login(login),
            discarded(DiscardReason::SessionMissing)
        );
    }
    // The same trustee, authenticated in the same second with the same TOTP
    // proof, but in another IdP session.
    assert_renewal_refused(
        renewed(|l| l.session_id = OTHER_SESSION.into()),
        DiscardReason::SessionChanged,
    );
    assert_renewal_refused(
        renewed(|l| l.session_id = String::new()),
        DiscardReason::SessionMissing,
    );
    let (mut machine, _) = approved();
    assert!(machine.renew(at(200), renewed(|_| {})).is_ok());
}

#[test]
fn wall_time_ends_a_grant_while_the_monotonic_clock_is_frozen() {
    // A suspension the monotonic clock does not see.
    let suspended = |wall_offset| Clock {
        mono: mono(6),
        wall: T0 + wall_offset,
    };

    // Past the token's wall expiry.
    let (mut machine, challenge) = approved();
    let expired = DeviceStatus::Closed {
        challenge,
        closure: Closure::Expired,
    };
    assert_eq!(machine.authorize(at(6)), Ok(mono(303)));
    assert_eq!(machine.authorize(suspended(303)), Err(DeviceError::NoGrant));
    assert_eq!(machine.status(suspended(303)), expired);
    // Rolling the wall clock back afterwards resurrects nothing.
    assert_eq!(
        machine.authorize(suspended(10)),
        Err(DeviceError::ClockRegressed)
    );
    assert_eq!(machine.status(at(7)), expired);
    assert_eq!(
        machine.renew(at(303), renewed(|_| {})),
        Err(DeviceError::NoGrant)
    );

    // Past auth_time + 10 hours with a long token.
    let (mut machine, challenge) =
        approved_with(edited(|l| l.expires_at = T0 + 100_000));
    let left = 3 + MAX_GRANT_LIFETIME - 1_000;
    assert_eq!(
        machine.status(suspended(1_000)),
        DeviceStatus::Approved {
            challenge,
            expires_in: left,
            session_ends_in: left,
        }
    );
    assert_eq!(
        machine.authorize(suspended(3 + MAX_GRANT_LIFETIME)),
        Err(DeviceError::NoGrant)
    );
    assert_eq!(
        machine.status(suspended(3 + MAX_GRANT_LIFETIME)),
        DeviceStatus::Closed {
            challenge,
            closure: Closure::Expired,
        }
    );
}

#[test]
fn authorization_pending_waits_the_interval() {
    let (mut machine, challenge) = pending_at(0);
    assert_eq!(
        machine.start_poll(mono(4), challenge).err(),
        Some(DeviceError::TooEarly { wait: 1 })
    );
    assert_eq!(
        poll(&mut machine, 5, challenge, PollOutcome::AuthorizationPending),
        Ok(DeviceStatus::Pending {
            challenge,
            poll_in: 5,
            interval: 5,
            expires_in: LIFETIME - 5,
        })
    );
    assert_eq!(
        machine.start_poll(mono(9), challenge).err(),
        Some(DeviceError::TooEarly { wait: 1 })
    );
    assert!(machine.start_poll(mono(10), challenge).is_ok());
}

#[test]
fn slow_down_adds_five_seconds_and_abandons_past_the_cap() {
    let (mut machine, challenge) = pending_at(0);
    assert_eq!(
        poll(&mut machine, 5, challenge, PollOutcome::SlowDown),
        Ok(DeviceStatus::Pending {
            challenge,
            poll_in: 10,
            interval: 10,
            expires_in: LIFETIME - 5,
        })
    );
    assert_eq!(
        machine.start_poll(mono(14), challenge).err(),
        Some(DeviceError::TooEarly { wait: 1 })
    );
    // The slower interval holds for later pending answers too.
    assert_eq!(
        poll(&mut machine, 15, challenge, PollOutcome::AuthorizationPending),
        Ok(DeviceStatus::Pending {
            challenge,
            poll_in: 10,
            interval: 10,
            expires_in: LIFETIME - 15,
        })
    );

    let mut now = 25;
    let mut status = Ok(DeviceStatus::Idle);
    for _ in 0..20 {
        status = poll(&mut machine, now, challenge, PollOutcome::SlowDown);
        match status {
            Ok(DeviceStatus::Pending { interval, .. }) => now += interval,
            _ => break,
        }
    }
    assert_eq!(
        status,
        Ok(DeviceStatus::Closed {
            challenge,
            closure: Closure::Abandoned,
        })
    );
}

#[test]
fn transport_failures_back_off_exponentially_up_to_the_cap() {
    let (mut machine, challenge) = pending_at(0);
    let mut now = 5;
    for wait in [10, 20, 40, 60, 60] {
        let ticket = machine.start_poll(mono(now), challenge).unwrap();
        assert_eq!(
            machine.poll_failed(mono(now), &ticket),
            Ok(DeviceStatus::Pending {
                challenge,
                poll_in: wait,
                interval: 5,
                expires_in: LIFETIME - now,
            })
        );
        assert_eq!(
            machine.poll_failed(mono(now), &ticket),
            Err(DeviceError::AlreadySettled)
        );
        assert_eq!(
            machine.start_poll(mono(now + wait - 1), challenge).err(),
            Some(DeviceError::TooEarly { wait: 1 })
        );
        now += wait;
    }
    // An answer from the IdP ends the backoff.
    let ticket = machine.start_poll(mono(now), challenge).unwrap();
    assert_eq!(
        machine.settle(at(now), &ticket, PollOutcome::AuthorizationPending),
        Ok(DeviceStatus::Pending {
            challenge,
            poll_in: 5,
            interval: 5,
            expires_in: LIFETIME - now,
        })
    );
}

#[test]
fn a_clock_regression_under_a_poll_in_flight_fails_closed() {
    let regressions = [
        Clock {
            mono: mono(4),
            wall: T0 + 5,
        },
        Clock {
            mono: mono(5),
            wall: T0 - 1,
        },
    ];
    for regressed in regressions {
        let (mut machine, challenge) = pending_at(0);
        let ticket = machine.start_poll(mono(5), challenge).unwrap();
        assert_eq!(
            machine.settle(
                regressed,
                &ticket,
                PollOutcome::Approved(login(T0 + 3))
            ),
            Err(DeviceError::ClockRegressed)
        );
        let closed = DeviceStatus::Closed {
            challenge,
            closure: Closure::ClockFault,
        };
        assert_eq!(machine.status(at(6)), closed);
        let late = PollOutcome::Approved(login(T0 + 3));
        assert_eq!(
            machine.settle(at(6), &ticket, late),
            Err(DeviceError::AlreadySettled)
        );
        assert_eq!(machine.authorize(at(6)), Err(DeviceError::NoGrant));
        // Not left busy: a new challenge may begin.
        assert_eq!(machine.begin(at(6), LIFETIME, None), Ok(ChallengeId(2)));
    }
}

#[test]
fn tickets_are_bound_to_the_machine_that_issued_them() {
    let (mut first, one) = pending_at(0);
    let (mut second, two) = pending_at(0);
    assert_eq!(one, two);
    let first_ticket = first.start_poll(mono(5), one).unwrap();
    let second_ticket = second.start_poll(mono(5), two).unwrap();
    let regressed = Clock {
        mono: mono(1),
        wall: T0,
    };
    assert_eq!(
        second.settle(
            regressed,
            &first_ticket,
            PollOutcome::Approved(login(T0 + 3))
        ),
        Err(DeviceError::ForeignTicket)
    );
    assert_eq!(
        second.settle(at(5), &first_ticket, PollOutcome::AccessDenied),
        Err(DeviceError::ForeignTicket)
    );
    assert_eq!(
        second.poll_failed(mono(5), &first_ticket),
        Err(DeviceError::ForeignTicket)
    );
    assert_eq!(
        second.status(at(5)),
        DeviceStatus::Pending {
            challenge: two,
            poll_in: 0,
            interval: 5,
            expires_in: LIFETIME - 5,
        }
    );
    assert!(matches!(
        second.settle(
            at(5),
            &second_ticket,
            PollOutcome::Approved(login(T0 + 3))
        ),
        Ok(DeviceStatus::Approved { .. })
    ));
    assert_eq!(
        first.settle(at(5), &first_ticket, PollOutcome::AccessDenied),
        Ok(DeviceStatus::Closed {
            challenge: one,
            closure: Closure::Denied,
        })
    );
    assert_eq!(
        first.settle(at(6), &second_ticket, PollOutcome::AccessDenied),
        Err(DeviceError::ForeignTicket)
    );
    assert!(second.authorize(at(6)).is_ok());
}

#[test]
fn a_restarted_machine_refuses_the_old_machines_ticket() {
    let (mut old, challenge) = pending_at(0);
    let stale = old.start_poll(mono(5), challenge).unwrap();
    drop(old);

    let (mut restarted, fresh) = pending_at(10);
    assert_eq!(fresh, challenge);
    let live = restarted.start_poll(mono(15), fresh).unwrap();
    assert_eq!(
        restarted.settle(
            at(15),
            &stale,
            PollOutcome::Approved(login(T0 + 12))
        ),
        Err(DeviceError::ForeignTicket)
    );
    assert_eq!(
        restarted.settle(at(15), &live, PollOutcome::AuthorizationPending),
        Ok(DeviceStatus::Pending {
            challenge: fresh,
            poll_in: 5,
            interval: 5,
            expires_in: LIFETIME - 5,
        })
    );
    assert_eq!(restarted.authorize(at(15)), Err(DeviceError::NoGrant));
}

#[test]
fn access_denied_closes_without_a_grant() {
    let (mut machine, challenge) = pending_at(0);
    assert_eq!(
        poll(&mut machine, 5, challenge, PollOutcome::AccessDenied),
        Ok(DeviceStatus::Closed {
            challenge,
            closure: Closure::Denied,
        })
    );
    assert_eq!(
        machine.start_poll(mono(10), challenge).err(),
        Some(DeviceError::AlreadySettled)
    );
    assert_eq!(machine.authorize(at(10)), Err(DeviceError::NoGrant));
}

#[test]
fn expiry_closes_the_challenge_however_it_is_reached() {
    let expired = |challenge| DeviceStatus::Closed {
        challenge,
        closure: Closure::Expired,
    };

    let (mut machine, challenge) = pending_at(0);
    assert_eq!(machine.status(at(LIFETIME)), expired(challenge));
    assert_eq!(
        machine.start_poll(mono(LIFETIME), challenge).err(),
        Some(DeviceError::Expired)
    );

    let (mut machine, challenge) = pending_at(0);
    assert_eq!(
        poll(&mut machine, 5, challenge, PollOutcome::ExpiredToken),
        Ok(expired(challenge))
    );

    // An approval answered after the device code's end grants nothing.
    let (mut machine, challenge) = pending_at(0);
    let ticket = machine.start_poll(mono(LIFETIME - 1), challenge).unwrap();
    assert_eq!(
        machine.settle(
            at(LIFETIME),
            &ticket,
            PollOutcome::Approved(login(T0 + 3))
        ),
        Ok(expired(challenge))
    );
    assert_eq!(
        machine.authorize(at(LIFETIME)),
        Err(DeviceError::NoGrant)
    );
}

#[test]
fn refuses_out_of_order_repeated_and_earlier_challenge_tickets() {
    let (mut machine, first) = pending_at(0);
    let first_ticket = machine.start_poll(mono(5), first).unwrap();
    assert_eq!(
        machine.start_poll(mono(5), first).err(),
        Some(DeviceError::OutOfOrder)
    );
    machine
        .settle(at(5), &first_ticket, PollOutcome::AccessDenied)
        .unwrap();
    assert_eq!(
        machine.settle(
            at(5),
            &first_ticket,
            PollOutcome::Approved(login(T0 + 3))
        ),
        Err(DeviceError::AlreadySettled)
    );

    let second = machine.begin(at(6), LIFETIME, None).unwrap();
    assert_eq!(
        machine.settle(
            at(11),
            &first_ticket,
            PollOutcome::Approved(login(T0 + 7))
        ),
        Err(DeviceError::WrongChallenge)
    );
    let earlier = machine.start_poll(mono(11), second).unwrap();
    machine
        .settle(at(11), &earlier, PollOutcome::AuthorizationPending)
        .unwrap();
    let current = machine.start_poll(mono(16), second).unwrap();
    assert_eq!(
        machine.settle(
            at(16),
            &earlier,
            PollOutcome::Approved(login(T0 + 7))
        ),
        Err(DeviceError::OutOfOrder)
    );
    assert_eq!(
        machine.settle(at(16), &current, PollOutcome::AuthorizationPending),
        Ok(DeviceStatus::Pending {
            challenge: second,
            poll_in: 5,
            interval: 5,
            expires_in: LIFETIME - 10,
        })
    );
    assert_eq!(machine.begin(at(16), LIFETIME, None), Err(DeviceError::Busy));
}

#[test]
fn refuses_bad_intervals_lifetimes_and_overflow() {
    let mut machine = DeviceLogin::new(scope());
    for interval in [0, DEFAULT_POLL_INTERVAL - 1, MAX_POLL_INTERVAL + 1] {
        assert_eq!(
            machine.begin(at(0), LIFETIME, Some(interval)),
            Err(DeviceError::BadInterval)
        );
    }
    for lifetime in [0, DEFAULT_POLL_INTERVAL, MAX_CHALLENGE_LIFETIME + 1] {
        assert_eq!(
            machine.begin(at(0), lifetime, None),
            Err(DeviceError::BadLifetime)
        );
    }
    assert_eq!(machine.status(at(0)), DeviceStatus::Idle);
    assert_eq!(
        machine.begin(at(0), MAX_CHALLENGE_LIFETIME, Some(MAX_POLL_INTERVAL)),
        Ok(ChallengeId(1))
    );

    let mut machine = DeviceLogin::new(scope());
    let end_of_time = Clock {
        mono: Monotonic::from_secs(u64::MAX - 10),
        wall: T0,
    };
    assert_eq!(
        machine.begin(end_of_time, LIFETIME, None),
        Err(DeviceError::Overflow)
    );
}

#[test]
fn refuses_a_malformed_scope() {
    let input = |issuer, subject| ScopeInput {
        issuer,
        audience: AUDIENCE,
        client_id: CLIENT,
        subject,
        tenant: TENANT,
        trustee_name: TRUSTEE,
        required_role: ROLE,
    };
    let cases = [("", SUBJECT), (ISSUER, " synthetic-subject-0001")];
    for (issuer, subject) in cases {
        assert!(DeviceScope::new(input(issuer, subject))
            .is_err_and(|err| err == DeviceError::MalformedScope));
    }
}

#[test]
fn a_fresh_instance_carries_no_old_authorization() {
    let (mut old, challenge) = approved();
    assert!(old.authorize(at(6)).is_ok());

    let mut restarted = DeviceLogin::new(scope());
    assert_eq!(restarted.status(at(6)), DeviceStatus::Idle);
    assert_eq!(restarted.authorize(at(6)), Err(DeviceError::NoGrant));
    assert_eq!(
        restarted.start_poll(mono(6), challenge).err(),
        Some(DeviceError::NoChallenge)
    );
    assert_eq!(
        restarted.renew(at(6), renewed(|l| l.issued_at = T0 + 6)),
        Err(DeviceError::NoGrant)
    );
}

#[test]
fn status_and_errors_show_no_identity_or_claim() {
    let (mut machine, challenge) = pending_at(0);
    let mut shown = vec![format!("{:?}", machine.status(at(0)))];
    shown.push(format!(
        "{:?}",
        poll(&mut machine, 5, challenge, PollOutcome::Approved(login(T0 + 3)))
    ));
    shown.push(format!("{:?}", machine.renew(at(200), renewed(|_| {}))));
    shown.push(format!("{:?}", machine.status(at(600))));
    let (mut machine, _) = approved();
    let foreign = renewed(|l| l.subject = OTHER_SUBJECT.into());
    shown.push(format!("{:?}", machine.renew(at(200), foreign)));
    shown.push(format!("{:?}", machine.status(at(200))));
    for text in shown {
        for value in [
            ISSUER,
            AUDIENCE,
            CLIENT,
            SUBJECT,
            OTHER_SUBJECT,
            TENANT,
            TRUSTEE,
            ROLE,
            SESSION,
            OTHER_SESSION,
        ] {
            assert!(!text.contains(value), "{text}");
        }
    }
}
