// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An inert, pure model of a trustee's RFC 8628 device authorization login:
//! the timing of one challenge's polls, the binding of the login that settles
//! it to the trustee's exact identity, and the in-memory renewal of the
//! process grant it yields.
//!
//! Nothing at runtime can build it: the crate declares it under `cfg(test)`
//! only, and it does no I/O, reads no clock, keeps no device code, user code,
//! token, refresh secret, password or OTP, and persists nothing. A restart
//! makes an empty machine, and tickets of one machine are refused by any
//! other.
//!
//! Every time is an explicit input. Deadlines run on a [`Monotonic`] clock,
//! which the future adapter must take from a suspend-aware boot clock. Wall
//! time ([`UnixSeconds`]) validates the IdP's claims when a login settles a
//! poll or renews a grant, and is a second, shorten-only authority on a live
//! grant: it ends at the earlier of both clocks' deadlines, so a suspension
//! that freezes the monotonic clock can't outlast the token or the 10-hour
//! cap. Neither clock ever extends a grant, and either going backwards fails
//! closed.
//!
//! A grant is bound to the IdP session (`sid`) of the login that approved
//! it; a renewal from any other session, even of the same trustee in the
//! same second, ends it.
//!
//! A login is only a [`VerifiedLogin`], which has no constructor outside this
//! module: no caller can turn unverified input, nor an unsigned JWT, into an
//! approval or a renewal. The future IdP adapter that verifies signatures and
//! proves a fresh TOTP must supply it; until that adapter exists and the IdP's
//! device login is shown to enforce it, this module authorizes nothing.

use std::sync::Arc;

/// Wall time: seconds since the Unix epoch, as the IdP's claims carry it.
pub(crate) type UnixSeconds = u64;

/// Seconds on the process's monotonic clock, from any fixed origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Monotonic(u64);

impl Monotonic {
    pub(crate) fn from_secs(secs: u64) -> Monotonic {
        Monotonic(secs)
    }

    fn after(self, secs: u64) -> Option<Monotonic> {
        self.0.checked_add(secs).map(Monotonic)
    }

    /// Seconds from `self` until `later`; none once it passed.
    fn until(self, later: Monotonic) -> u64 {
        later.0.saturating_sub(self.0)
    }
}

/// Both clocks, read together.
#[derive(Clone, Copy)]
pub(crate) struct Clock {
    pub(crate) mono: Monotonic,
    pub(crate) wall: UnixSeconds,
}

/// RFC 8628 3.2: the poll interval when the IdP gives none, and the shortest
/// one accepted.
pub(crate) const DEFAULT_POLL_INTERVAL: u64 = 5;
/// RFC 8628 3.5: `slow_down` adds 5 seconds to this and every later poll.
pub(crate) const SLOW_DOWN_STEP: u64 = 5;
/// Longer intervals, given or reached by `slow_down`, abandon the challenge;
/// transport backoff waits at most this long.
pub(crate) const MAX_POLL_INTERVAL: u64 = 60;
/// The longest device code lifetime accepted.
pub(crate) const MAX_CHALLENGE_LIFETIME: u64 = 1_800;
/// No grant outlives its authentication by more than this many seconds.
pub(crate) const MAX_GRANT_LIFETIME: u64 = 36_000;

/// A challenge of this machine: a counter, not the device code. Two machines
/// may hold equal ids; tickets tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChallengeId(u64);

/// Why the machine refused a call. Carries no claim or secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeviceError {
    /// A scope value is empty or padded with whitespace.
    MalformedScope,
    BadInterval,
    BadLifetime,
    Overflow,
    /// A challenge is pending, or a grant is live.
    Busy,
    NoChallenge,
    WrongChallenge,
    /// The ticket belongs to another machine.
    ForeignTicket,
    /// Poll again in `wait` seconds.
    TooEarly {
        wait: u64,
    },
    /// A poll is already in flight, or the ticket is not the current one.
    OutOfOrder,
    /// The challenge or the ticket's poll was already settled.
    AlreadySettled,
    /// A clock went backwards; the challenge or grant is closed.
    ClockRegressed,
    NoGrant,
    Expired,
    /// The renewal was refused and the grant closed.
    RenewalRefused(DiscardReason),
}

/// Why a verified login was discarded. Names the check, never the value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiscardReason {
    Issuer,
    Audience,
    AuthorizedParty,
    Subject,
    Tenant,
    TrusteeName,
    Role,
    /// Times that contradict each other or the clock.
    Malformed,
    MfaAbsent,
    /// An assurance level such as ACR "gold" without proof of the factor.
    MfaAssuranceOnly,
    AuthTimeStale,
    MfaStale,
    /// No IdP session identity, or a malformed one.
    SessionMissing,
    /// A renewal from another IdP session than the grant's.
    SessionChanged,
    /// A renewal from another authentication than the grant's.
    AuthTimeChanged,
    /// A renewal with another second factor proof than the grant's.
    MfaChanged,
    Expired,
    Overflow,
}

/// How a challenge or grant ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Closure {
    Denied,
    Expired,
    /// `slow_down` pushed the interval past the cap, or a time overflowed.
    Abandoned,
    /// A clock went backwards.
    ClockFault,
    /// A verified login outside the scope. It is dropped; nothing here
    /// revokes or ends any IdP session, its subject's or anyone else's.
    Discarded(DiscardReason),
    /// A renewal that did not match the grant; the grant ended with it.
    RenewalRefused(DiscardReason),
}

/// What may be shown about the machine: no secret, claim or identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DeviceStatus {
    Idle,
    Pending {
        challenge: ChallengeId,
        poll_in: u64,
        interval: u64,
        expires_in: u64,
    },
    Approved {
        challenge: ChallengeId,
        /// Until the current token's deadline.
        expires_in: u64,
        /// Until the fixed session deadline no renewal passes.
        session_ends_in: u64,
    },
    Closed {
        challenge: ChallengeId,
        closure: Closure,
    },
}

/// The scope's values by name, so that none is passed for another.
pub(crate) struct ScopeInput<'a> {
    pub(crate) issuer: &'a str,
    pub(crate) audience: &'a str,
    pub(crate) client_id: &'a str,
    /// The IdP's immutable subject assigned to the trustee.
    pub(crate) subject: &'a str,
    pub(crate) tenant: &'a str,
    pub(crate) trustee_name: &'a str,
    pub(crate) required_role: &'a str,
}

/// The exact identity one trustee's login must carry.
pub(crate) struct DeviceScope {
    issuer: String,
    audience: String,
    client_id: String,
    subject: String,
    tenant: String,
    trustee_name: String,
    required_role: String,
}

impl DeviceScope {
    pub(crate) fn new(input: ScopeInput) -> Result<DeviceScope, DeviceError> {
        let values = [
            input.issuer,
            input.audience,
            input.client_id,
            input.subject,
            input.tenant,
            input.trustee_name,
            input.required_role,
        ];
        if values
            .iter()
            .any(|value| value.is_empty() || value.trim() != *value)
        {
            return Err(DeviceError::MalformedScope);
        }
        Ok(DeviceScope {
            issuer: input.issuer.to_owned(),
            audience: input.audience.to_owned(),
            client_id: input.client_id.to_owned(),
            subject: input.subject.to_owned(),
            tenant: input.tenant.to_owned(),
            trustee_name: input.trustee_name.to_owned(),
            required_role: input.required_role.to_owned(),
        })
    }
}

/// The second factors accepted. Only TOTP, as the trustee uses today.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecondFactor {
    Totp,
}

/// What the IdP's second factor proves.
pub(crate) enum MfaEvidence {
    Absent,
    /// Only an assurance level (ACR, LoA): not proof of a fresh factor.
    AssuranceLevelOnly,
    Verified(FreshMfa),
}

/// A second factor the trusted adapter verified, and when.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct FreshMfa {
    factor: SecondFactor,
    verified_at: UnixSeconds,
}

/// Typed claims of a login whose signature and MFA a trusted IdP adapter
/// verified. Fields are private and there is no constructor: only this
/// module and its tests build one. No `Debug`, so it can't be logged.
pub(crate) struct VerifiedLogin {
    issuer: String,
    audiences: Vec<String>,
    /// `azp`: the client the token was granted to.
    authorized_party: String,
    subject: String,
    tenant: String,
    trustee_name: String,
    roles: Vec<String>,
    issued_at: UnixSeconds,
    expires_at: UnixSeconds,
    /// The IdP session's absolute end, when known.
    session_expires_at: Option<UnixSeconds>,
    /// The IdP session's immutable identity (`sid`): an identifier the IdP
    /// signed into the token, not a bearer secret. Never shown.
    session_id: String,
    auth_time: UnixSeconds,
    mfa: MfaEvidence,
}

/// The token endpoint's answer to one poll, as RFC 8628 3.5 names them.
pub(crate) enum PollOutcome {
    AuthorizationPending,
    SlowDown,
    AccessDenied,
    ExpiredToken,
    Approved(VerifiedLogin),
}

/// The identity of one machine: an allocation kept alive by its tickets, so
/// its address is never reused while a ticket could name it. No `Debug`.
#[derive(Clone)]
struct Owner(Arc<()>);

/// The right to settle one poll of one machine. Not `Clone`, no `Debug`.
pub(crate) struct PollTicket {
    owner: Owner,
    challenge: ChallengeId,
    sequence: u64,
}

/// An opaque binding to one challenge of one machine. Only
/// [`DeviceLogin::begin_bound`] makes one, as it begins that challenge; it
/// is not `Clone` and shows nothing. A device grant bound to it can only
/// poll with tickets of that machine for that challenge: an equal-looking
/// challenge of another machine, or a later challenge of the same machine,
/// never matches. Inert: nothing at runtime begins a bound challenge yet.
pub(crate) struct ChallengeBinding {
    owner: Owner,
    challenge: ChallengeId,
}

impl ChallengeBinding {
    /// Whether `ticket` is this machine's, for this challenge.
    pub(crate) fn matches(&self, ticket: &PollTicket) -> bool {
        Arc::ptr_eq(&self.owner.0, &ticket.owner.0)
            && self.challenge == ticket.challenge
    }
}

impl std::fmt::Debug for ChallengeBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChallengeBinding(..)")
    }
}

struct Pending {
    challenge: ChallengeId,
    /// Wall time at the start: no authentication before it counts.
    started_wall: UnixSeconds,
    expires_at: Monotonic,
    interval: u64,
    next_poll_at: Monotonic,
    sequence: u64,
    in_flight: bool,
    /// Transport failures in a row.
    failures: u32,
}

struct Grant {
    challenge: ChallengeId,
    started_wall: UnixSeconds,
    session_id: String,
    auth_time: UnixSeconds,
    mfa: FreshMfa,
    /// The latest token's, to refuse an older one.
    issued_at: UnixSeconds,
    /// Fixed at approval; renewals may only bring it closer.
    session_deadline: Monotonic,
    /// Never after `session_deadline`.
    token_deadline: Monotonic,
    /// The wall-time twins of the two deadlines, under the same rules.
    session_end_wall: UnixSeconds,
    token_end_wall: UnixSeconds,
}

enum State {
    Idle,
    Pending(Pending),
    Approved(Grant),
    Closed {
        challenge: ChallengeId,
        closure: Closure,
    },
}

/// What a login within the scope proves, at the wall time it was checked.
struct Proof {
    mfa: FreshMfa,
    token_left: u64,
    session_left: u64,
    /// Wall times: the token's end, never after the session's.
    token_end: UnixSeconds,
    session_end: UnixSeconds,
}

/// One trustee's device login, held in process memory only.
pub(crate) struct DeviceLogin {
    scope: DeviceScope,
    owner: Owner,
    state: State,
    next_challenge: u64,
    observed_mono: Monotonic,
    observed_wall: UnixSeconds,
}

impl DeviceLogin {
    /// An empty machine: no challenge, no grant.
    pub(crate) fn new(scope: DeviceScope) -> DeviceLogin {
        DeviceLogin {
            scope,
            owner: Owner(Arc::new(())),
            state: State::Idle,
            next_challenge: 1,
            observed_mono: Monotonic(0),
            observed_wall: 0,
        }
    }

    /// Selects a new challenge from the IdP's `expires_in` and `interval`.
    pub(crate) fn begin(
        &mut self,
        clock: Clock,
        expires_in: u64,
        interval: Option<u64>,
    ) -> Result<ChallengeId, DeviceError> {
        self.observe(clock.mono)?;
        self.observe_wall(clock.wall)?;
        if matches!(self.state, State::Pending(_) | State::Approved(_)) {
            return Err(DeviceError::Busy);
        }
        let interval = interval.unwrap_or(DEFAULT_POLL_INTERVAL);
        if !(DEFAULT_POLL_INTERVAL..=MAX_POLL_INTERVAL).contains(&interval) {
            return Err(DeviceError::BadInterval);
        }
        if expires_in <= interval || expires_in > MAX_CHALLENGE_LIFETIME {
            return Err(DeviceError::BadLifetime);
        }
        let expires_at =
            clock.mono.after(expires_in).ok_or(DeviceError::Overflow)?;
        let next_poll_at =
            clock.mono.after(interval).ok_or(DeviceError::Overflow)?;
        let challenge = ChallengeId(self.next_challenge);
        self.next_challenge = self
            .next_challenge
            .checked_add(1)
            .ok_or(DeviceError::Overflow)?;
        self.state = State::Pending(Pending {
            challenge,
            started_wall: clock.wall,
            expires_at,
            interval,
            next_poll_at,
            sequence: 0,
            in_flight: false,
            failures: 0,
        });
        Ok(challenge)
    }

    /// Begins a challenge as [`Self::begin`] does, and binds it: the binding
    /// names this machine and this challenge alone.
    pub(crate) fn begin_bound(
        &mut self,
        clock: Clock,
        expires_in: u64,
        interval: Option<u64>,
    ) -> Result<ChallengeBinding, DeviceError> {
        let challenge = self.begin(clock, expires_in, interval)?;
        Ok(ChallengeBinding {
            owner: self.owner.clone(),
            challenge,
        })
    }

    /// The ticket for the challenge's next poll, once its wait passed.
    pub(crate) fn start_poll(
        &mut self,
        mono: Monotonic,
        challenge: ChallengeId,
    ) -> Result<PollTicket, DeviceError> {
        self.observe(mono)?;
        let pending = pending_in(&mut self.state, challenge)?;
        if pending.in_flight {
            return Err(DeviceError::OutOfOrder);
        }
        if mono < pending.next_poll_at {
            return Err(DeviceError::TooEarly {
                wait: mono.until(pending.next_poll_at),
            });
        }
        pending.sequence = pending
            .sequence
            .checked_add(1)
            .ok_or(DeviceError::Overflow)?;
        pending.in_flight = true;
        Ok(PollTicket {
            owner: self.owner.clone(),
            challenge,
            sequence: pending.sequence,
        })
    }

    /// Settles the ticket's poll with the IdP's answer, once.
    pub(crate) fn settle(
        &mut self,
        clock: Clock,
        ticket: &PollTicket,
        outcome: PollOutcome,
    ) -> Result<DeviceStatus, DeviceError> {
        self.claim(ticket)?;
        self.observe_both(clock)?;
        let State::Pending(pending) = &mut self.state else {
            // Expired while the poll was in flight.
            return Ok(self.view(clock));
        };
        pending.in_flight = false;
        let challenge = pending.challenge;
        let next = match outcome {
            PollOutcome::AuthorizationPending => {
                pending.failures = 0;
                let interval = pending.interval;
                reschedule(pending, clock.mono, interval, interval)
            }
            PollOutcome::SlowDown => {
                pending.failures = 0;
                match pending
                    .interval
                    .checked_add(SLOW_DOWN_STEP)
                    .filter(|interval| *interval <= MAX_POLL_INTERVAL)
                {
                    Some(interval) => {
                        reschedule(pending, clock.mono, interval, interval)
                    }
                    None => Some(closed(challenge, Closure::Abandoned)),
                }
            }
            PollOutcome::AccessDenied => {
                Some(closed(challenge, Closure::Denied))
            }
            PollOutcome::ExpiredToken => {
                Some(closed(challenge, Closure::Expired))
            }
            PollOutcome::Approved(login) => {
                Some(match approve(&self.scope, pending, clock, &login) {
                    Ok(grant) => State::Approved(grant),
                    Err(reason) => {
                        closed(challenge, Closure::Discarded(reason))
                    }
                })
            }
        };
        if let Some(state) = next {
            self.state = state;
        }
        Ok(self.view(clock))
    }

    /// Settles the ticket's poll that failed in transport: RFC 8628 3.5's
    /// backoff doubles the wait after each failure in a row, up to
    /// [`MAX_POLL_INTERVAL`], and the challenge stays pending.
    pub(crate) fn poll_failed(
        &mut self,
        mono: Monotonic,
        ticket: &PollTicket,
    ) -> Result<DeviceStatus, DeviceError> {
        self.claim(ticket)?;
        self.observe(mono)?;
        // No new wall reading: only a pending or closed challenge is shown.
        let clock = Clock {
            mono,
            wall: self.observed_wall,
        };
        let State::Pending(pending) = &mut self.state else {
            return Ok(self.view(clock));
        };
        pending.in_flight = false;
        pending.failures = pending.failures.saturating_add(1);
        let interval = pending.interval;
        let wait = 1u64
            .checked_shl(pending.failures)
            .and_then(|factor| interval.checked_mul(factor))
            .map_or(MAX_POLL_INTERVAL, |wait| wait.min(MAX_POLL_INTERVAL))
            .max(interval);
        if let Some(state) = reschedule(pending, mono, interval, wait) {
            self.state = state;
        }
        Ok(self.view(clock))
    }

    /// Rolls the grant's token deadline to a new verified login of the same
    /// authentication, within the session deadline fixed at approval. Any
    /// mismatch closes the grant.
    pub(crate) fn renew(
        &mut self,
        clock: Clock,
        login: VerifiedLogin,
    ) -> Result<DeviceStatus, DeviceError> {
        self.observe_both(clock)?;
        let State::Approved(grant) = &mut self.state else {
            return Err(DeviceError::NoGrant);
        };
        let challenge = grant.challenge;
        match renewal(&self.scope, grant, clock, &login) {
            Ok(renewed) => {
                *grant = renewed;
                Ok(self.view(clock))
            }
            Err(reason) => {
                self.state = closed(challenge, Closure::RenewalRefused(reason));
                Err(DeviceError::RenewalRefused(reason))
            }
        }
    }

    /// The monotonic deadline of the live grant's current token. The grant
    /// also ends once wall time reaches the token's or the session's end.
    pub(crate) fn authorize(
        &mut self,
        clock: Clock,
    ) -> Result<Monotonic, DeviceError> {
        self.observe_both(clock)?;
        match &self.state {
            State::Approved(grant) => Ok(grant.token_deadline),
            _ => Err(DeviceError::NoGrant),
        }
    }

    pub(crate) fn status(&mut self, clock: Clock) -> DeviceStatus {
        // A regression closes what was open; the view then shows it.
        let _ = self.observe_both(clock);
        self.view(clock)
    }

    fn view(&self, clock: Clock) -> DeviceStatus {
        let (mono, wall) = (clock.mono, clock.wall);
        match &self.state {
            State::Idle => DeviceStatus::Idle,
            State::Pending(pending) => DeviceStatus::Pending {
                challenge: pending.challenge,
                poll_in: mono.until(pending.next_poll_at),
                interval: pending.interval,
                expires_in: mono.until(pending.expires_at),
            },
            State::Approved(grant) => DeviceStatus::Approved {
                challenge: grant.challenge,
                expires_in: mono
                    .until(grant.token_deadline)
                    .min(grant.token_end_wall.saturating_sub(wall)),
                session_ends_in: mono
                    .until(grant.session_deadline)
                    .min(grant.session_end_wall.saturating_sub(wall)),
            },
            State::Closed { challenge, closure } => DeviceStatus::Closed {
                challenge: *challenge,
                closure: *closure,
            },
        }
    }

    /// Records the monotonic time, closing what it ended. A regression
    /// closes the open challenge or grant instead.
    fn observe(&mut self, mono: Monotonic) -> Result<(), DeviceError> {
        if mono < self.observed_mono {
            self.fail_closed();
            return Err(DeviceError::ClockRegressed);
        }
        self.observed_mono = mono;
        let ended = match &self.state {
            State::Pending(pending) if mono >= pending.expires_at => {
                Some(pending.challenge)
            }
            State::Approved(grant) if mono >= grant.token_deadline => {
                Some(grant.challenge)
            }
            _ => None,
        };
        if let Some(challenge) = ended {
            self.state = closed(challenge, Closure::Expired);
        }
        Ok(())
    }

    /// Records the wall time, ending a grant whose token or session it
    /// reached even while the monotonic clock stood still. A regression
    /// closes the open challenge or grant instead.
    fn observe_wall(&mut self, wall: UnixSeconds) -> Result<(), DeviceError> {
        if wall < self.observed_wall {
            self.fail_closed();
            return Err(DeviceError::ClockRegressed);
        }
        self.observed_wall = wall;
        let ended = match &self.state {
            State::Approved(grant) if wall >= grant.token_end_wall => {
                Some(grant.challenge)
            }
            _ => None,
        };
        if let Some(challenge) = ended {
            self.state = closed(challenge, Closure::Expired);
        }
        Ok(())
    }

    fn observe_both(&mut self, clock: Clock) -> Result<(), DeviceError> {
        self.observe(clock.mono)?;
        self.observe_wall(clock.wall)
    }

    fn fail_closed(&mut self) {
        let challenge = match &self.state {
            State::Pending(pending) => pending.challenge,
            State::Approved(grant) => grant.challenge,
            State::Idle | State::Closed { .. } => return,
        };
        self.state = closed(challenge, Closure::ClockFault);
    }

    /// Whether the ticket is this machine's for the poll in flight. Changes
    /// nothing, so a foreign or stale ticket leaves every outcome alone.
    fn claim(&self, ticket: &PollTicket) -> Result<(), DeviceError> {
        if !Arc::ptr_eq(&self.owner.0, &ticket.owner.0) {
            return Err(DeviceError::ForeignTicket);
        }
        match &self.state {
            State::Pending(pending)
                if pending.challenge == ticket.challenge =>
            {
                if !pending.in_flight {
                    Err(DeviceError::AlreadySettled)
                } else if pending.sequence != ticket.sequence {
                    Err(DeviceError::OutOfOrder)
                } else {
                    Ok(())
                }
            }
            State::Idle => Err(DeviceError::NoChallenge),
            State::Approved(Grant { challenge, .. })
            | State::Closed { challenge, .. }
                if *challenge == ticket.challenge =>
            {
                Err(DeviceError::AlreadySettled)
            }
            _ => Err(DeviceError::WrongChallenge),
        }
    }
}

fn closed(challenge: ChallengeId, closure: Closure) -> State {
    State::Closed { challenge, closure }
}

fn pending_in(
    state: &mut State,
    challenge: ChallengeId,
) -> Result<&mut Pending, DeviceError> {
    match state {
        State::Idle => Err(DeviceError::NoChallenge),
        State::Pending(pending) if pending.challenge == challenge => {
            Ok(pending)
        }
        State::Closed {
            challenge: own,
            closure: Closure::Expired,
        } if *own == challenge => Err(DeviceError::Expired),
        State::Approved(Grant { challenge: own, .. })
        | State::Closed { challenge: own, .. }
            if *own == challenge =>
        {
            Err(DeviceError::AlreadySettled)
        }
        _ => Err(DeviceError::WrongChallenge),
    }
}

/// Keeps the challenge pending with `interval` for later polls and the next
/// one `wait` seconds from now.
fn reschedule(
    pending: &mut Pending,
    mono: Monotonic,
    interval: u64,
    wait: u64,
) -> Option<State> {
    match mono.after(wait) {
        Some(next_poll_at) => {
            pending.interval = interval;
            pending.next_poll_at = next_poll_at;
            None
        }
        None => Some(closed(pending.challenge, Closure::Abandoned)),
    }
}

/// The grant for a login that settles `pending`, or why it is discarded.
fn approve(
    scope: &DeviceScope,
    pending: &Pending,
    clock: Clock,
    login: &VerifiedLogin,
) -> Result<Grant, DiscardReason> {
    let proof = verify(scope, pending.started_wall, clock.wall, login)?;
    let session_deadline = clock
        .mono
        .after(proof.session_left)
        .ok_or(DiscardReason::Overflow)?;
    let token_deadline = clock
        .mono
        .after(proof.token_left)
        .ok_or(DiscardReason::Overflow)?;
    Ok(Grant {
        challenge: pending.challenge,
        started_wall: pending.started_wall,
        session_id: login.session_id.clone(),
        auth_time: login.auth_time,
        mfa: proof.mfa,
        issued_at: login.issued_at,
        session_deadline,
        token_deadline,
        session_end_wall: proof.session_end,
        token_end_wall: proof.token_end,
    })
}

/// The grant as a renewal leaves it, or why the renewal is refused. Every
/// deadline may only come closer.
fn renewal(
    scope: &DeviceScope,
    grant: &Grant,
    clock: Clock,
    login: &VerifiedLogin,
) -> Result<Grant, DiscardReason> {
    let proof = verify(scope, grant.started_wall, clock.wall, login)?;
    if login.session_id != grant.session_id {
        return Err(DiscardReason::SessionChanged);
    }
    if login.auth_time != grant.auth_time {
        return Err(DiscardReason::AuthTimeChanged);
    }
    if proof.mfa != grant.mfa {
        return Err(DiscardReason::MfaChanged);
    }
    if login.issued_at < grant.issued_at {
        return Err(DiscardReason::Malformed);
    }
    let session_deadline = clock
        .mono
        .after(proof.session_left)
        .ok_or(DiscardReason::Overflow)?
        .min(grant.session_deadline);
    let token_deadline = clock
        .mono
        .after(proof.token_left)
        .ok_or(DiscardReason::Overflow)?
        .min(session_deadline);
    let session_end_wall = proof.session_end.min(grant.session_end_wall);
    let token_end_wall = proof.token_end.min(session_end_wall);
    if token_deadline <= clock.mono || token_end_wall <= clock.wall {
        return Err(DiscardReason::Expired);
    }
    Ok(Grant {
        challenge: grant.challenge,
        started_wall: grant.started_wall,
        session_id: grant.session_id.clone(),
        auth_time: grant.auth_time,
        mfa: grant.mfa,
        issued_at: login.issued_at,
        session_deadline,
        token_deadline,
        session_end_wall,
        token_end_wall,
    })
}

/// What a login within the scope proves at `wall`, or why it is discarded.
fn verify(
    scope: &DeviceScope,
    started_wall: UnixSeconds,
    wall: UnixSeconds,
    login: &VerifiedLogin,
) -> Result<Proof, DiscardReason> {
    if login.issuer != scope.issuer {
        return Err(DiscardReason::Issuer);
    }
    if !login.audiences.iter().any(|aud| *aud == scope.audience) {
        return Err(DiscardReason::Audience);
    }
    if login.authorized_party != scope.client_id {
        return Err(DiscardReason::AuthorizedParty);
    }
    if login.subject != scope.subject {
        return Err(DiscardReason::Subject);
    }
    if login.tenant != scope.tenant {
        return Err(DiscardReason::Tenant);
    }
    if login.trustee_name != scope.trustee_name {
        return Err(DiscardReason::TrusteeName);
    }
    if !login.roles.iter().any(|role| *role == scope.required_role) {
        return Err(DiscardReason::Role);
    }
    let sid = login.session_id.as_str();
    if sid.is_empty()
        || sid.len() > 255
        || !sid.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(DiscardReason::SessionMissing);
    }
    if login.issued_at > wall
        || login.auth_time > login.issued_at
        || login.expires_at <= login.issued_at
    {
        return Err(DiscardReason::Malformed);
    }
    let mfa = match &login.mfa {
        MfaEvidence::Absent => return Err(DiscardReason::MfaAbsent),
        MfaEvidence::AssuranceLevelOnly => {
            return Err(DiscardReason::MfaAssuranceOnly)
        }
        MfaEvidence::Verified(mfa) => *mfa,
    };
    if mfa.verified_at > wall {
        return Err(DiscardReason::Malformed);
    }
    if login.auth_time < started_wall {
        return Err(DiscardReason::AuthTimeStale);
    }
    if mfa.verified_at < started_wall {
        return Err(DiscardReason::MfaStale);
    }
    let mut session_end = login
        .auth_time
        .checked_add(MAX_GRANT_LIFETIME)
        .ok_or(DiscardReason::Overflow)?;
    if let Some(upstream) = login.session_expires_at {
        session_end = session_end.min(upstream);
    }
    let token_end = login.expires_at.min(session_end);
    if token_end <= wall {
        return Err(DiscardReason::Expired);
    }
    Ok(Proof {
        mfa,
        token_left: token_end - wall,
        session_left: (session_end - wall).min(MAX_GRANT_LIFETIME),
        token_end,
        session_end,
    })
}

#[cfg(test)]
#[path = "../tests/support/device_tests.rs"]
mod tests;

/// The token pair verifier: a child, so it alone can build a login.
#[cfg(test)]
#[path = "device_verify.rs"]
pub(crate) mod verify;
