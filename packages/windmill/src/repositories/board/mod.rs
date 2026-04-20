// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod postgres;

use anyhow::Result;
use async_trait::async_trait;
use b3::messages::artifact::Configuration;
use b3::messages::message::Message;
use b3::messages::newtypes::{BatchNumber, PublicKeyHash, TrusteeSet};
use b3::messages::protocol_manager::ProtocolManager;
use std::sync::Arc;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::signature::StrandSignaturePk;

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
pub trait BallotBoardRepository: Send + Sync {
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
