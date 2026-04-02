// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Infrastructure adapters for the insert-ballots ports.
//!
//! These adapters bridge the insert-ballots contracts with existing Hasura,
//! Keycloak, protocol-manager, and CSV-processing helpers. They are intentionally
//! thin and rely on caller-owned transactions supplied at construction time.

use super::{BallotBoardPort, InsertBallotsBoardContext, PrepareBoardContextRequest};

use crate::services::protocol_manager::{
    add_ballots_to_board, generate_trustee_set, get_b3_pgsql_client, get_board_messages,
    get_configuration, get_protocol_manager, get_public_key_hash,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use b3::messages::newtypes::BatchNumber;
use deadpool_postgres::Transaction;
use sequent_core::ballot::{ContestEncryptionPolicy, HashableBallot};
use sequent_core::multi_ballot::HashableMultiBallot;
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::signature::StrandSignaturePk;

/// Protocol-manager-backed implementation of `BallotBoardPort`.
///
/// The adapter performs one-time board setup and per-batch ballot posting while
/// preserving the caller-provided Hasura transaction for protocol-manager
/// configuration lookup.
pub struct ProtocolManagerBoardPort<'a> {
    transaction: &'a Transaction<'a>,
}

impl<'a> ProtocolManagerBoardPort<'a> {
    /// Creates a board port bound to the provided Hasura transaction.
    pub fn new(transaction: &'a Transaction<'a>) -> Self {
        Self { transaction }
    }
}

#[async_trait]
impl BallotBoardPort for ProtocolManagerBoardPort<'_> {
    async fn prepare_board_context(
        &self,
        request: PrepareBoardContextRequest,
    ) -> Result<InsertBallotsBoardContext> {
        let protocol_manager = Arc::new(
            get_protocol_manager(
                self.transaction,
                &request.tenant_id,
                Some(&request.election_event_id),
                &request.board_name,
            )
            .await?,
        );
        let mut board_client = get_b3_pgsql_client().await?;
        let messages = Arc::new(
            get_board_messages::<RistrettoCtx>(&request.board_name, &mut board_client).await?,
        );
        let configuration = get_configuration(&messages)?;
        let public_key_hash = get_public_key_hash::<RistrettoCtx>(&messages)?;
        let selected_trustees = generate_trustee_set(&configuration, request.trustee_public_keys);

        Ok(InsertBallotsBoardContext {
            protocol_manager,
            messages,
            configuration,
            public_key_hash,
            selected_trustees,
        })
    }

    async fn post_ballots(
        &self,
        board_name: &str,
        board_context: &InsertBallotsBoardContext,
        batch: BatchNumber,
        ciphertexts: Vec<Ciphertext<RistrettoCtx>>,
    ) -> Result<()> {
        let mut board = get_b3_pgsql_client().await?;
        add_ballots_to_board(
            &board_context.protocol_manager,
            &mut board,
            board_name,
            &board_context.messages,
            &board_context.configuration,
            board_context.public_key_hash.clone(),
            board_context.selected_trustees,
            ciphertexts,
            batch,
        )
        .await
    }
}
