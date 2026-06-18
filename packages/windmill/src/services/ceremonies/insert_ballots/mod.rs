// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod services;

pub use self::services::{InsertBallotsService, TrusteePublicKeysService};
pub use crate::repositories::board::postgres::ProtocolManagerBoardPort;
pub use crate::repositories::board::{
    BallotBoardRepository, InsertBallotsBoardContext, PrepareBoardContextRequest,
};

use anyhow::Result;
use async_trait::async_trait;
use b3::messages::message::Message;
use b3::messages::newtypes::TrusteeSet;
use sequent_core::types::hasura::core::{TallySession, TallySessionContest};
use strand::signature::StrandSignaturePk;

/// Input contract for the orchestration service that reconciles board state with
/// tally-session contest annotations.
///
/// The upsert service uses the existing board messages to decide whether each
/// contest requires a full ballot insertion or only annotation recovery. The
/// service must preserve idempotency by never reposting a batch that already has
/// ballots on the board.
pub struct UpsertBallotsMessagesRequest<'a> {
    /// Tenant that owns the tally execution.
    pub tenant_id: String,
    /// Election event whose tally session is being reconciled.
    pub election_event_id: String,
    /// Bulletin-board name associated with the tally session.
    pub board_name: String,
    /// Trustees selected for the current execution.
    pub trustee_names: Vec<String>,
    /// Current board messages used to detect already-posted batches.
    pub messages: &'a [Message],
    /// Contest batches that should exist for the tally session.
    pub tally_session_contests: Vec<TallySessionContest>,
    /// Tally-session configuration used to recover execution policies.
    pub tally_session: TallySession,
}

/// Service contract for resolving trustees into deserialized public keys ready
/// for board-context preparation.
///
/// This port sits above `TrusteeRepository` and separates persistence concerns
/// from cryptographic key preparation. Implementations are expected to be built
/// from transaction-bound repository adapters supplied by the orchestration
/// layer, which keeps transaction propagation under caller control.
///
/// Use cases:
/// - Load trustee key material once before preparing the board context.
/// - Validate that every selected trustee has a persisted public key.
/// - Normalize trustee ordering so the resolved keys match the caller-provided
///   trustee sequence.
///
/// Contract:
/// - Returns one deserialized public key per requested trustee name, in the
///   same order as `trustee_names`.
/// - Must fail when a requested trustee cannot be found, lacks a persisted
///   public key, or contains an invalid serialized key.
/// - Must not open its own transaction or bypass the injected repository.
#[async_trait]
pub trait TrusteePublicKeyResolver: Send + Sync {
    /// Resolves the selected trustees into deserialized public keys.
    ///
    /// The returned vector must align with the caller-provided trustee order.
    /// Missing trustees, missing public keys, and invalid serialized keys are
    /// contract violations and must be reported as errors.
    async fn resolve_public_keys(
        &self,
        tenant_id: &str,
        trustee_names: &[String],
    ) -> Result<Vec<StrandSignaturePk>>;
}

/// Application-level contract for reconciling the board state with tally-session
/// contest batches.
///
/// This port is the orchestration boundary consumed by the tally-session task.
/// It decides which contests need a full ballot insertion and which only need
/// annotation recovery.
///
/// Use cases:
/// - Detect missing ballot batches before plaintext processing continues.
/// - Recover annotations when ballots were already posted but the database
///   transaction previously rolled back.
///
/// Contract:
/// - Must preserve board idempotency by never requiring reposting for batches
///   that are already present on the board.
/// - Must return the contests whose annotations should be persisted by the
///   caller.
/// - Must be fully testable with fake inserters and board-message fixtures.
#[async_trait]
pub trait ContestBallotUpserter: Send + Sync {
    /// Reconciles board state with tally-session contests.
    ///
    /// Implementations decide whether each contest needs a full insertion or an
    /// annotation-only recovery and return the contests whose annotations must
    /// be persisted by the caller.
    async fn upsert_ballots(
        &self,
        request: UpsertBallotsMessagesRequest<'_>,
    ) -> Result<Vec<TallySessionContest>>;
}
