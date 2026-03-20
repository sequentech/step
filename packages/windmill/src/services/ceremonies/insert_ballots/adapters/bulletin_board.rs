// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::ceremonies::insert_ballots::domain::generate_trustee_set;
use crate::services::ceremonies::insert_ballots::ports::BulletinBoardPort;
use crate::services::protocol_manager::{
    get_b3_pgsql_client, get_board_messages, get_configuration, get_protocol_manager,
    get_public_key_hash,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use b3::client::pgsql::PgsqlB3Client;
use b3::messages::artifact::{Ballots, Configuration};
use b3::messages::message::Message;
use b3::messages::newtypes::{BatchNumber, PublicKeyHash, TrusteeSet};
use b3::messages::protocol_manager::ProtocolManager;
use b3::messages::statement::StatementType;
use deadpool_postgres::Transaction;
use std::sync::Arc;
use strand::backend::ristretto::RistrettoCtx;
use strand::elgamal::Ciphertext;
use strand::signature::StrandSignaturePk;
use tracing::{event, info, instrument, Level};

/// Adapter for the bulletin board port backed by the B3 tamper-evident PostgreSQL board.
///
/// Holds all the crypto state needed to sign and submit ballot batches. Constructed once per
/// `insert_ballots_messages` call and shared (via `Arc`) across per-contest tasks.
pub struct B3BulletinBoardAdapter {
    protocol_manager: Arc<ProtocolManager<RistrettoCtx>>,
    configuration: Configuration<RistrettoCtx>,
    public_key_hash: PublicKeyHash,
    selected_trustees: TrusteeSet,
    board_messages: Arc<Vec<Message>>,
}

impl B3BulletinBoardAdapter {
    /// Build the adapter by fetching the protocol manager, board messages, and deriving all
    /// crypto state needed to sign ballot batches.
    pub async fn new(
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event_id: &str,
        board_name: &str,
        trustee_pks: Vec<StrandSignaturePk>,
    ) -> Result<Self> {
        let protocol_manager = get_protocol_manager::<RistrettoCtx>(
            hasura_transaction,
            tenant_id,
            Some(election_event_id),
            board_name,
        )
        .await?;

        let mut board_client = get_b3_pgsql_client().await?;
        let board_messages = get_board_messages::<RistrettoCtx>(board_name, &board_client).await?;
        let configuration = get_configuration(&board_messages)?;
        let public_key_hash = get_public_key_hash::<RistrettoCtx>(&board_messages)?;
        let selected_trustees = generate_trustee_set(&configuration, trustee_pks);

        Ok(Self {
            protocol_manager: Arc::new(protocol_manager),
            configuration,
            public_key_hash,
            selected_trustees,
            board_messages: Arc::new(board_messages),
        })
    }
}

#[async_trait]
impl BulletinBoardPort for B3BulletinBoardAdapter {
    #[instrument(skip(self, ciphertexts), err)]
    async fn add_ballots(
        &self,
        board_name: &str,
        ciphertexts: Vec<Ciphertext<RistrettoCtx>>,
        batch: BatchNumber,
    ) -> Result<()> {
        let mut board = get_b3_pgsql_client().await?;
        submit_ballots_to_board(
            &self.protocol_manager,
            &mut board,
            board_name,
            &self.board_messages,
            &self.configuration,
            self.public_key_hash.clone(),
            self.selected_trustees.clone(),
            ciphertexts,
            batch,
        )
        .await
    }
}

/// Signs and inserts an encrypted ballot batch into the B3 board, skipping if the batch already
/// exists (idempotent).
#[instrument(
    skip(
        messages,
        configuration,
        public_key_hash,
        selected_trustees,
        ballots,
        b3_client
    ),
    err
)]
async fn submit_ballots_to_board(
    pm: &ProtocolManager<RistrettoCtx>,
    b3_client: &mut PgsqlB3Client,
    board_name: &str,
    messages: &Vec<Message>,
    configuration: &Configuration<RistrettoCtx>,
    public_key_hash: PublicKeyHash,
    selected_trustees: TrusteeSet,
    ballots: Vec<Ciphertext<RistrettoCtx>>,
    batch: BatchNumber,
) -> Result<()> {
    let existing_message = messages.iter().find(|message| {
        let batch_number = message.statement.get_batch_number();
        let kind = message.statement.get_kind();
        batch_number == batch && StatementType::Ballots == kind
    });
    if let Some(_message) = existing_message {
        event!(
            Level::INFO,
            "Not adding Ballot to board {} as it already exists for batch {}",
            board_name,
            batch
        );
        return Ok(());
    }

    let ballots_len = ballots.len();

    let message = Message::ballots_msg::<RistrettoCtx, ProtocolManager<RistrettoCtx>>(
        configuration,
        batch,
        &Ballots::<RistrettoCtx>::new(ballots),
        selected_trustees,
        public_key_hash,
        pm,
    )?;
    info!(
        "Adding configuration to the board for batch {} and number of ballots {}",
        batch, ballots_len
    );
    b3_client
        .insert_ballots::<RistrettoCtx>(board_name, message)
        .await
}
