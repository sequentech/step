// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Infrastructure adapters for the insert-ballots ports.
//!
//! These adapters bridge the insert-ballots contracts with existing Hasura,
//! Keycloak, protocol-manager, and CSV-processing helpers. They are intentionally
//! thin and rely on caller-owned transactions supplied at construction time.

use super::{BallotBoardPort, InsertBallotsBoardContext, PrepareBoardContextRequest};
use crate::services::join::merge_join_csv;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BallotProcessingOutput {
    pub ballot_contents: Vec<String>,
    pub elegible_voters: u64,
    pub ballots_without_voter: u64,
    pub casted_ballots: u64,
}

pub(super) fn merge_ballots_and_voters(
    ballots_file: &File,
    voters_file: &File,
    delegated_voting_enabled: bool,
) -> Result<BallotProcessingOutput> {
    let delegate_count_index = delegated_voting_enabled.then_some(1);
    let (ballot_contents, elegible_voters, ballots_without_voter, casted_ballots) =
        merge_join_csv(ballots_file, voters_file, 0, 0, 1, delegate_count_index)?;

    Ok(BallotProcessingOutput {
        ballot_contents,
        elegible_voters,
        ballots_without_voter,
        casted_ballots,
    })
}

pub(super) fn extract_ciphertexts(
    ballot_contents: Vec<String>,
    contest_encryption_policy: &ContestEncryptionPolicy,
    contest_id: Option<&str>,
) -> Result<Vec<Ciphertext<RistrettoCtx>>> {
    ballot_contents
        .into_iter()
        .map(|ballot_str| {
            let ciphertext =
                if ContestEncryptionPolicy::MULTIPLE_CONTESTS == *contest_encryption_policy {
                    let hashable_multi_ballot: HashableMultiBallot = deserialize_str(&ballot_str)?;
                    let contests = hashable_multi_ballot
                        .deserialize_contests()
                        .map_err(|err| anyhow!("{:?}", err))?;
                    Some(contests.ciphertext)
                } else {
                    let hashable_ballot: HashableBallot = deserialize_str(&ballot_str)?;
                    let contests = hashable_ballot
                        .deserialize_contests()
                        .map_err(|err| anyhow!("{:?}", err))?;
                    contests
                        .iter()
                        .find(|contest| contest.contest_id == contest_id.unwrap_or_default())
                        .map(|contest| contest.ciphertext.clone())
                };

            ciphertext.ok_or(anyhow!("Could not get ciphertext"))
        })
        .collect()
}

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
