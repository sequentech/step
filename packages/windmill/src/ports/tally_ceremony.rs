// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::tally_ceremony::TallyExecuter;
use b4::messages::newtypes::BatchNumber;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::ceremonies::{TallyCeremonyStatus, TallyExecutionStatus, TallyRunReason};
use sequent_core::types::hasura::core::{
    Area, AreaContest, BallotStyle, Contest, Election, ElectionEvent, KeysCeremony, TallySession,
    TallySessionConfiguration, TallySessionExecution, TallySheet,
};
use serde_json::Value;
use std::future::Future;

/// The election, area and, when each contest is encrypted on its own,
/// contest whose ballots are decrypted together.
pub type DecryptionSet = (String, String, Option<String>);

/// A tally session to create.
pub struct NewTallySession {
    pub id: String,
    pub election_ids: Vec<String>,
    pub area_ids: Vec<String>,
    pub keys_ceremony_id: String,
    pub execution_status: TallyExecutionStatus,
    pub threshold: i32,
    pub configuration: Option<TallySessionConfiguration>,
    pub tally_type: String,
    pub annotations: Value,
    pub permission_labels: Vec<String>,
}

/// The election event data a tally session is created from, read together.
pub struct TallyEventSnapshot {
    pub election_event: ElectionEvent,
    pub elections: Vec<Election>,
    pub contests: Vec<Contest>,
    pub areas: Vec<Area>,
    pub area_contests: Vec<AreaContest>,
}

pub trait TallyCreationReader: Sync {
    fn event_snapshot(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> impl Future<Output = anyhow::Result<TallyEventSnapshot>> + Send;

    fn published_ballot_styles(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: &[String],
    ) -> impl Future<Output = anyhow::Result<Vec<BallotStyle>>> + Send;

    fn approved_tally_sheets(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> impl Future<Output = anyhow::Result<Vec<TallySheet>>> + Send;
}

/// Tally sessions and their execution history. Each execution is a snapshot
/// of the ceremony status; the newest one is the current state.
pub trait TallySessions: Sync {
    fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> impl Future<Output = anyhow::Result<TallySession>> + Send;

    /// Locks the session row until the transaction ends.
    fn lock_for_update(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn last_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
    ) -> impl Future<Output = anyhow::Result<Option<TallySessionExecution>>> + Send;

    /// The newest execution and the session as read after it, or `None` if
    /// the session has no execution.
    fn last_execution_and_session(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        election_ids: Vec<String>,
    ) -> impl Future<Output = anyhow::Result<Option<(TallySessionExecution, TallySession)>>> + Send;

    fn append_execution(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        current_message_id: i32,
        status: TallyCeremonyStatus,
        run_reason: TallyRunReason,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn set_status(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        execution_status: TallyExecutionStatus,
        is_execution_completed: bool,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Marks the execution completed and its post-tally task pending.
    fn mark_completed(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        execution_status: TallyExecutionStatus,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    fn insert(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session: NewTallySession,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// The first board batch after the runs already allocated to the
    /// election event's tally session contests.
    fn next_batch(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> impl Future<Output = anyhow::Result<BatchNumber>> + Send;

    /// Records that `decryption_set` is tallied from the board batch run
    /// starting at `batch`.
    fn insert_contest(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        tally_session_id: &str,
        decryption_set: &DecryptionSet,
        batch: BatchNumber,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub trait KeysCeremonyReader: Sync {
    fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        keys_ceremony_id: &str,
    ) -> impl Future<Output = anyhow::Result<KeysCeremony>> + Send;
}

/// The private keys trustees generated in a keys ceremony, as stored
/// (encrypted) on its bulletin board.
pub trait TrusteePrivateKeys: Sync {
    fn encrypted_private_key(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        trustee_name: &str,
        keys_ceremony: &KeysCeremony,
    ) -> impl Future<Output = anyhow::Result<String>> + Send;
}

pub trait ElectionsById: Sync {
    fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: &[String],
    ) -> impl Future<Output = anyhow::Result<Vec<Election>>> + Send;
}

pub trait ElectionEventReader: Sync {
    fn get(
        &self,
        tenant_id: &str,
        election_event_id: &str,
    ) -> impl Future<Output = anyhow::Result<ElectionEvent>> + Send;
}

/// The deployment's `ENV_SLUG`, which prefixes bulletin board names.
pub trait EnvironmentSlug: Sync {
    fn env_slug(&self) -> anyhow::Result<String>;
}

/// Tally ceremony entries of the electoral log, posted on `board_name`.
pub trait TallyCeremonyAudit: Sync {
    /// Signed by the administrator who started the tally.
    fn tally_opened(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Option<Vec<String>>,
        user_id: &str,
        username: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Signed by the trustee's user, identified by `claims`.
    fn key_restored(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Option<Vec<String>>,
        trustee_name: &str,
        claims: &JwtClaims,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Signed by the administrator who created the tally session.
    fn key_insertion_started(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Vec<String>,
        user_id: &str,
        username: &str,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;

    /// Signed by the system, naming the user who created the session.
    fn tally_closed(
        &self,
        board_name: &str,
        tenant_id: &str,
        election_event_id: &str,
        election_ids: Option<Vec<String>>,
        executer: TallyExecuter,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}
