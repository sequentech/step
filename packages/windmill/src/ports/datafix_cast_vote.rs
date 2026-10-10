// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What resolving a Datafix vote needs from outside: the event's cast votes
//! and configuration, its Keycloak voters, VoterView, the per-voter lock and
//! the electoral log.

use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::external::datafix_types::SoapRequestResult;
use crate::services::external::voterview_requests::SoapSendError;
use chrono::{DateTime, Local};
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::User;
use std::future::Future;
use uuid::Uuid;

pub trait DatafixVotes: Sync {
    fn load(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
    ) -> impl Future<Output = anyhow::Result<Option<CastVote>>> + Send;

    /// Whether the voter already has a valid vote in the event.
    fn has_valid_vote(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        voter_id: &str,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send;

    /// Moves the vote from `expected` to `next` and commits, returning whether
    /// it moved: `false` means it was no longer `expected`.
    fn compare_and_set(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
        expected: CastVoteStatus,
        next: CastVoteStatus,
    ) -> impl Future<Output = anyhow::Result<bool>> + Send;
}

pub trait DatafixElectionEvents: Sync {
    fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> impl Future<Output = anyhow::Result<ElectionEvent>> + Send;
}

/// Voters of an election event's Keycloak realm.
pub trait DatafixVoterDirectory: Sync {
    fn voter(
        &self,
        realm: &str,
        voter_id: &str,
    ) -> impl Future<Output = anyhow::Result<User>> + Send;

    fn mark_voted_via_internet(
        &self,
        realm: &str,
        voter_id: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// A rendered `SetVoted` request that has not been sent yet.
pub trait PreparedSetVoted: Send + Sync {
    /// SHA-256 of the template that rendered the request, for the audit trail.
    fn template_sha256(&self) -> &str;
}

/// VoterView's `SetVoted`, in two steps so the voter lock can be renewed
/// between rendering the request and sending it.
pub trait VoterView: Sync {
    type Prepared: PreparedSetVoted;

    fn prepare_set_voted(
        &self,
        election_event: ElectionEvent,
        username: &str,
    ) -> impl Future<Output = anyhow::Result<Self::Prepared>> + Send;

    fn send(
        &self,
        prepared: Self::Prepared,
    ) -> impl Future<Output = Result<SoapRequestResult, SoapSendError>> + Send;
}

/// The event-wide per-voter lock that serializes every Datafix operation on
/// a voter.
pub trait DatafixVoterLocks: Sync {
    type Lock: Send + Sync;

    /// Fails while another holder's lock on `key` has not expired, or when the
    /// lock cannot be written.
    fn acquire(
        &self,
        key: String,
        value: String,
        expiry_date: DateTime<Local>,
    ) -> impl Future<Output = anyhow::Result<Self::Lock>> + Send;

    /// Renews the lock for `seconds` from now; fails if it was lost.
    fn extend(
        &self,
        lock: &Self::Lock,
        seconds: i64,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn release(&self, lock: Self::Lock) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// The electoral log of outbound Datafix operations. Recording failures are
/// logged, never returned, so auditing cannot fail the vote.
pub trait DatafixAudit: Sync {
    fn record(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        voter_id: &str,
        username: &str,
        operation: String,
    ) -> impl Future<Output = ()> + Send;
}
