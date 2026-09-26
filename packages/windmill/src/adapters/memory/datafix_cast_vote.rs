// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::memory::clock::FixedClock;
use crate::ports::clock::Clock;
use crate::ports::datafix_cast_vote::{
    DatafixAudit, DatafixElectionEvents, DatafixVoterDirectory, DatafixVoterLocks, DatafixVotes,
    PreparedSetVoted, VoterView,
};
use crate::services::cast_votes::{CastVote, CastVoteStatus};
use crate::services::external::datafix_types::{SoapRequestResponse, SoapRequestResult};
use crate::services::external::voterview_requests::SoapSendError;
use anyhow::anyhow;
use chrono::{DateTime, Duration, Local};
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::keycloak::User;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

/// Template hash of every request the in-memory VoterView prepares.
pub const SET_VOTED_TEMPLATE_SHA256: &str = "in-memory-set-voted-template";
/// Value of a lock held by an operation other than the one under test.
pub const ANOTHER_OPERATION: &str = "another-operation";
/// The error `PgLock` reports when another holder has the lock.
pub const LOCK_HELD_ELSEWHERE: &str = "Couldn't upsert lock";

fn fail_if_set(failure: &Option<String>) -> anyhow::Result<()> {
    match failure {
        Some(message) => Err(anyhow::Error::msg(message.clone())),
        None => Ok(()),
    }
}

/// A change another worker makes to the vote being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrentChange {
    ResolvedAfterNextLoad(CastVoteStatus),
    DeletedAfterNextLoad,
    ResolvedBeforeNextCompareAndSet(CastVoteStatus),
}

#[derive(Default)]
struct VotesState {
    votes: Vec<CastVote>,
    concurrent_change: Option<ConcurrentChange>,
    load_failure: Option<String>,
    has_valid_vote_failure: Option<String>,
    compare_and_set_failure: Option<String>,
}

impl VotesState {
    fn set_status(&mut self, id: &str, status: CastVoteStatus) {
        if let Some(vote) = self.votes.iter_mut().find(|vote| vote.id == id) {
            vote.status = status;
        }
    }
}

/// Cast votes in memory. `has_valid_vote` and `compare_and_set` follow the
/// queries in `crate::postgres::cast_vote`.
#[derive(Default)]
pub struct InMemoryDatafixVotes(Mutex<VotesState>);

impl InMemoryDatafixVotes {
    fn state(&self) -> MutexGuard<'_, VotesState> {
        self.0.lock().expect("votes lock")
    }

    /// Stores the vote, replacing the one with the same id.
    pub fn insert(&self, vote: CastVote) {
        let mut state = self.state();
        state.votes.retain(|stored| stored.id != vote.id);
        state.votes.push(vote);
    }

    pub fn status(&self, id: &str) -> Option<CastVoteStatus> {
        let state = self.state();
        state
            .votes
            .iter()
            .find(|vote| vote.id == id)
            .map(|vote| vote.status)
    }

    pub fn change_concurrently(&self, change: ConcurrentChange) {
        self.state().concurrent_change = Some(change);
    }

    pub fn fail_load(&self, message: &str) {
        self.state().load_failure = Some(message.to_string());
    }

    pub fn fail_has_valid_vote(&self, message: &str) {
        self.state().has_valid_vote_failure = Some(message.to_string());
    }

    pub fn fail_compare_and_set(&self, message: &str) {
        self.state().compare_and_set_failure = Some(message.to_string());
    }
}

impl DatafixVotes for InMemoryDatafixVotes {
    async fn load(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
    ) -> anyhow::Result<Option<CastVote>> {
        let mut state = self.state();
        fail_if_set(&state.load_failure)?;
        let id = cast_vote_id.to_string();
        let vote = state
            .votes
            .iter()
            .find(|vote| {
                vote.id == id
                    && vote.tenant_id == tenant_id
                    && vote.election_event_id == election_event_id
            })
            .cloned();
        match state.concurrent_change {
            Some(ConcurrentChange::ResolvedAfterNextLoad(status)) => {
                state.concurrent_change = None;
                state.set_status(&id, status);
            }
            Some(ConcurrentChange::DeletedAfterNextLoad) => {
                state.concurrent_change = None;
                state.votes.retain(|vote| vote.id != id);
            }
            _ => {}
        }
        Ok(vote)
    }

    async fn has_valid_vote(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        voter_id: &str,
    ) -> anyhow::Result<bool> {
        let state = self.state();
        fail_if_set(&state.has_valid_vote_failure)?;
        Ok(state.votes.iter().any(|vote| {
            vote.tenant_id == tenant_id
                && vote.election_event_id == election_event_id
                && vote.voter_id_string.as_deref() == Some(voter_id)
                && vote.status == CastVoteStatus::Valid
        }))
    }

    async fn compare_and_set(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        cast_vote_id: &Uuid,
        expected: CastVoteStatus,
        next: CastVoteStatus,
    ) -> anyhow::Result<bool> {
        let mut state = self.state();
        fail_if_set(&state.compare_and_set_failure)?;
        let id = cast_vote_id.to_string();
        if let Some(ConcurrentChange::ResolvedBeforeNextCompareAndSet(status)) =
            state.concurrent_change
        {
            state.concurrent_change = None;
            state.set_status(&id, status);
        }
        let vote = state.votes.iter_mut().find(|vote| {
            vote.id == id
                && vote.tenant_id == tenant_id
                && vote.election_event_id == election_event_id
                && vote.status == expected
        });
        match vote {
            Some(vote) => {
                vote.status = next;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

#[derive(Default)]
pub struct InMemoryDatafixElectionEvents(Mutex<Vec<ElectionEvent>>);

impl InMemoryDatafixElectionEvents {
    /// Stores the event, replacing the one with the same tenant and id.
    pub fn insert(&self, event: ElectionEvent) {
        let mut events = self.0.lock().expect("election events lock");
        events.retain(|stored| stored.tenant_id != event.tenant_id || stored.id != event.id);
        events.push(event);
    }
}

impl DatafixElectionEvents for InMemoryDatafixElectionEvents {
    async fn get(&self, tenant_id: &str, election_event_id: &str) -> anyhow::Result<ElectionEvent> {
        let events = self.0.lock().expect("election events lock");
        events
            .iter()
            .find(|event| event.tenant_id == tenant_id && event.id == election_event_id)
            .cloned()
            .ok_or_else(|| anyhow!("Election event {election_event_id} not found"))
    }
}

#[derive(Default)]
struct DirectoryState {
    voters: HashMap<(String, String), User>,
    marked_via_internet: Vec<String>,
    marking_failure: Option<String>,
}

/// Keycloak voters by realm and id.
#[derive(Default)]
pub struct InMemoryDatafixVoterDirectory(Mutex<DirectoryState>);

impl InMemoryDatafixVoterDirectory {
    fn state(&self) -> MutexGuard<'_, DirectoryState> {
        self.0.lock().expect("voter directory lock")
    }

    /// Stores the voter in `realm`, replacing the one with the same id.
    pub fn insert(&self, realm: &str, voter: User) {
        let id = voter.id.clone().expect("voter id");
        self.state().voters.insert((realm.to_string(), id), voter);
    }

    /// Voters marked as having voted via the Internet, in order.
    pub fn marked_via_internet(&self) -> Vec<String> {
        self.state().marked_via_internet.clone()
    }

    pub fn fail_marking(&self, message: &str) {
        self.state().marking_failure = Some(message.to_string());
    }
}

fn voter_not_found(realm: &str, voter_id: &str) -> anyhow::Error {
    anyhow!("Voter {voter_id} not found in realm {realm}")
}

impl DatafixVoterDirectory for InMemoryDatafixVoterDirectory {
    async fn voter(&self, realm: &str, voter_id: &str) -> anyhow::Result<User> {
        let state = self.state();
        state
            .voters
            .get(&(realm.to_string(), voter_id.to_string()))
            .cloned()
            .ok_or_else(|| voter_not_found(realm, voter_id))
    }

    async fn mark_voted_via_internet(&self, realm: &str, voter_id: &str) -> anyhow::Result<()> {
        let mut state = self.state();
        fail_if_set(&state.marking_failure)?;
        if !state
            .voters
            .contains_key(&(realm.to_string(), voter_id.to_string()))
        {
            return Err(voter_not_found(realm, voter_id));
        }
        state.marked_via_internet.push(voter_id.to_string());
        Ok(())
    }
}

/// What the in-memory VoterView answers to `SetVoted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoterViewReply {
    Response(SoapRequestResponse),
    NotDispatched(String),
    Ambiguous(String),
}

struct VoterViewState {
    reply: VoterViewReply,
    prepare_failure: Option<String>,
    sent: Vec<String>,
}

/// A VoterView that gives every `SetVoted` the same reply, `Ok` unless told
/// otherwise.
pub struct InMemoryVoterView(Mutex<VoterViewState>);

impl Default for InMemoryVoterView {
    fn default() -> Self {
        Self(Mutex::new(VoterViewState {
            reply: VoterViewReply::Response(SoapRequestResponse::Ok),
            prepare_failure: None,
            sent: Vec::new(),
        }))
    }
}

impl InMemoryVoterView {
    fn state(&self) -> MutexGuard<'_, VoterViewState> {
        self.0.lock().expect("VoterView lock")
    }

    pub fn reply_with(&self, reply: VoterViewReply) {
        self.state().reply = reply;
    }

    pub fn fail_prepare(&self, message: &str) {
        self.state().prepare_failure = Some(message.to_string());
    }

    /// Usernames `SetVoted` was sent for, in order.
    pub fn sent(&self) -> Vec<String> {
        self.state().sent.clone()
    }
}

pub struct InMemoryPreparedSetVoted {
    username: String,
}

impl PreparedSetVoted for InMemoryPreparedSetVoted {
    fn template_sha256(&self) -> &str {
        SET_VOTED_TEMPLATE_SHA256
    }
}

impl VoterView for InMemoryVoterView {
    type Prepared = InMemoryPreparedSetVoted;

    async fn prepare_set_voted(
        &self,
        _election_event: ElectionEvent,
        username: &str,
    ) -> anyhow::Result<InMemoryPreparedSetVoted> {
        fail_if_set(&self.state().prepare_failure)?;
        Ok(InMemoryPreparedSetVoted {
            username: username.to_string(),
        })
    }

    async fn send(
        &self,
        prepared: InMemoryPreparedSetVoted,
    ) -> Result<SoapRequestResult, SoapSendError> {
        let mut state = self.state();
        state.sent.push(prepared.username);
        match state.reply.clone() {
            VoterViewReply::Response(response) => Ok(SoapRequestResult {
                response,
                template_sha256: SET_VOTED_TEMPLATE_SHA256.to_string(),
            }),
            VoterViewReply::NotDispatched(message) => {
                Err(SoapSendError::NotDispatched(anyhow::Error::msg(message)))
            }
            VoterViewReply::Ambiguous(message) => {
                Err(SoapSendError::Ambiguous(anyhow::Error::msg(message)))
            }
        }
    }
}

/// A lock taken through the port: its key, holder value and expiry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockAcquisition {
    pub key: String,
    pub value: String,
    pub expiry_date: DateTime<Local>,
}

#[derive(Default)]
struct LocksState {
    holders: HashMap<String, String>,
    expiries: HashMap<String, DateTime<Local>>,
    acquisitions: Vec<LockAcquisition>,
    released: Vec<LockAcquisition>,
    renewals: usize,
    lost_before_renewal: Option<usize>,
    release_failure: Option<String>,
}

/// Datafix voter locks by key. Acquisition and renewal can replace an expired
/// lease, while release only removes a lease with the same holder value.
pub struct InMemoryDatafixVoterLocks {
    state: Mutex<LocksState>,
    clock: FixedClock,
}

pub struct InMemoryVoterLock {
    key: String,
    value: String,
}

impl InMemoryDatafixVoterLocks {
    pub fn at(now: DateTime<Local>) -> Self {
        Self {
            state: Mutex::new(LocksState::default()),
            clock: FixedClock::at(now),
        }
    }

    fn state(&self) -> MutexGuard<'_, LocksState> {
        self.state.lock().expect("voter locks lock")
    }

    fn can_take(&self, state: &LocksState, key: &str, value: &str) -> bool {
        state.holders.get(key).is_none_or(|holder| holder == value)
            || state
                .expiries
                .get(key)
                .is_some_and(|expiry| *expiry < self.clock.now())
    }

    pub fn hold_for_another_operation(&self, key: &str) {
        let mut state = self.state();
        state
            .holders
            .insert(key.to_string(), ANOTHER_OPERATION.to_string());
        state.expiries.remove(key);
    }

    pub fn advance(&self, by: Duration) {
        self.clock.advance(by);
    }

    pub fn holder(&self, key: &str) -> Option<String> {
        self.state().holders.get(key).cloned()
    }

    /// Every lock taken, including the released ones.
    pub fn acquisitions(&self) -> Vec<LockAcquisition> {
        self.state().acquisitions.clone()
    }

    /// The lease stored when its holder released it.
    pub fn released(&self) -> Vec<LockAcquisition> {
        self.state().released.clone()
    }

    /// Another operation takes the lock over right before renewal `number`,
    /// counting from 1.
    pub fn lose_before_renewal(&self, number: usize) {
        self.state().lost_before_renewal = Some(number);
    }

    pub fn fail_release(&self, message: &str) {
        self.state().release_failure = Some(message.to_string());
    }
}

impl DatafixVoterLocks for InMemoryDatafixVoterLocks {
    type Lock = InMemoryVoterLock;

    async fn acquire(
        &self,
        key: String,
        value: String,
        expiry_date: DateTime<Local>,
    ) -> anyhow::Result<InMemoryVoterLock> {
        let mut state = self.state();
        if !self.can_take(&state, &key, &value) {
            return Err(anyhow::Error::msg(LOCK_HELD_ELSEWHERE));
        }
        state.holders.insert(key.clone(), value.clone());
        state.expiries.insert(key.clone(), expiry_date);
        state.acquisitions.push(LockAcquisition {
            key: key.clone(),
            value: value.clone(),
            expiry_date,
        });
        Ok(InMemoryVoterLock { key, value })
    }

    async fn extend(&self, lock: &InMemoryVoterLock, seconds: i64) -> anyhow::Result<()> {
        let mut state = self.state();
        state.renewals += 1;
        if state.lost_before_renewal == Some(state.renewals) {
            state
                .holders
                .insert(lock.key.clone(), ANOTHER_OPERATION.to_string());
            state.expiries.remove(&lock.key);
        }
        if self.can_take(&state, &lock.key, &lock.value) {
            state.holders.insert(lock.key.clone(), lock.value.clone());
            state.expiries.insert(
                lock.key.clone(),
                self.clock.now() + Duration::seconds(seconds),
            );
            Ok(())
        } else {
            Err(anyhow::Error::msg(LOCK_HELD_ELSEWHERE))
        }
    }

    async fn release(&self, lock: InMemoryVoterLock) -> anyhow::Result<()> {
        let mut state = self.state();
        fail_if_set(&state.release_failure)?;
        if state.holders.get(&lock.key) == Some(&lock.value) {
            state.holders.remove(&lock.key);
            let expiry_date = state
                .expiries
                .remove(&lock.key)
                .expect("owned lease expiry");
            state.released.push(LockAcquisition {
                key: lock.key,
                value: lock.value,
                expiry_date,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatafixAuditEntry {
    pub tenant_id: String,
    pub election_event_id: String,
    pub voter_id: String,
    pub username: String,
    pub operation: String,
}

#[derive(Default)]
pub struct InMemoryDatafixAudit(Mutex<Vec<DatafixAuditEntry>>);

impl InMemoryDatafixAudit {
    pub fn entries(&self) -> Vec<DatafixAuditEntry> {
        self.0.lock().expect("audit lock").clone()
    }
}

impl DatafixAudit for InMemoryDatafixAudit {
    async fn record(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        voter_id: &str,
        username: &str,
        operation: String,
    ) {
        self.0.lock().expect("audit lock").push(DatafixAuditEntry {
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            voter_id: voter_id.to_string(),
            username: username.to_string(),
            operation,
        });
    }
}
