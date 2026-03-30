// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

mod adapters;
mod services;

pub use self::adapters::ProtocolManagerBoardPort;
pub use self::services::{InsertBallotsService, TrusteePublicKeysService};

use anyhow::Result;
use async_trait::async_trait;
use b3::messages::artifact::Configuration;
use b3::messages::message::Message;
use b3::messages::newtypes::{BatchNumber, PublicKeyHash, TrusteeSet};
use b3::messages::protocol_manager::ProtocolManager;
use sequent_core::ballot::{ContestEncryptionPolicy, DelegatedVotingPolicy};
use sequent_core::types::hasura::core::{TallySession, TallySessionContest, Trustee};
use std::sync::Arc;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
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

/// Parameters used to load and cache the board context shared by all batch posts
/// in a single insert-ballots orchestration.
///
/// `trustee_public_keys` must already be validated and deserialized by the
/// application service before the port is invoked. This keeps the board port
/// focused on board-facing concerns instead of trustee data cleanup.
#[derive(Clone)]
pub struct PrepareBoardContextRequest {
    /// Tenant that owns the board configuration.
    pub tenant_id: String,
    /// Election event associated with the board.
    pub election_event_id: String,
    /// Bulletin-board name where ballots will be posted.
    pub board_name: String,
    /// Trustee keys already validated and deserialized by the application layer.
    pub trustee_public_keys: Vec<StrandSignaturePk>,
}

/// Immutable board context reused across all ballot batches within a single
/// insert-ballots execution.
///
/// The context includes the protocol manager and the already-fetched board
/// configuration needed to build and post ballot messages. Implementations may
/// fetch this data once and reuse it for every contest in the same orchestration
/// to avoid redundant board roundtrips.
#[derive(Clone)]
pub struct InsertBallotsBoardContext {
    /// Protocol manager used to sign ballot messages.
    pub protocol_manager: Arc<ProtocolManager<RistrettoCtx>>,
    /// Current board messages fetched once for the execution.
    pub messages: Arc<Vec<Message>>,
    /// Board configuration required to construct ballots messages.
    pub configuration: Configuration<RistrettoCtx>,
    /// Public-key hash referenced by ballots messages.
    pub public_key_hash: PublicKeyHash,
    /// Trustee set selected for the current board execution.
    pub selected_trustees: TrusteeSet,
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

/// Board-facing port that encapsulates all bulletin-board interactions needed by
/// the insert-ballots use case.
///
/// Implementations may depend on the caller-owned Hasura transaction in their
/// constructor when loading the protocol manager secret, but they must not create
/// their own database transactions. The port separates one-time board-context
/// loading from per-batch posting so orchestration can cache expensive board
/// setup while still posting each contest independently.
///
/// Use cases:
/// - Load protocol-manager and board configuration data once per execution.
/// - Post ballot batches idempotently to the board.
///
/// Contract:
/// - `prepare_board_context` must return all data required for subsequent calls
///   to `post_ballots`.
/// - `post_ballots` must preserve idempotency for already-posted batches.
/// - Implementations should propagate infrastructure failures rather than hiding
///   them so orchestration can roll back database work.
#[async_trait]
pub trait BallotBoardPort: Send + Sync {
    /// Loads the board context reused by all ballot batches in one execution.
    ///
    /// Implementations should perform one-time board lookups here and return a
    /// context object suitable for subsequent `post_ballots` calls.
    async fn prepare_board_context(
        &self,
        request: PrepareBoardContextRequest,
    ) -> Result<InsertBallotsBoardContext>;

    /// Posts one ballot batch to the board.
    ///
    /// Implementations must preserve board idempotency for already-posted
    /// batches and propagate infrastructure failures to the caller.
    async fn post_ballots(
        &self,
        board_name: &str,
        board_context: &InsertBallotsBoardContext,
        batch: BatchNumber,
        ciphertexts: Vec<Ciphertext<RistrettoCtx>>,
    ) -> Result<()>;
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
