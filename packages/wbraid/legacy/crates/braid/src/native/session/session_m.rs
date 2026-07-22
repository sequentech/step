// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use b4::HttpB4Message;
use b4::messages::message::Message;
use std::path::PathBuf;
use cryptography::utils::symm::SymmetricKey;

// use cryptography::context::RistrettoCtx;
use cryptography::utils::signatures::SignatureScheme;
use tracing::info;

use crate::protocol::trustee::Trustee;
use crate::protocol::trustee::TrusteeConfig;
use crate::native::board::SqliteStorage;
use crate::util::ProtocolError;

use cryptography::context::Context;

/// A protocol session, M version.
///
/// As opposed to the non-M implementation, SessionM's
/// support concurrency, multiplexing and chunking.
///
/// A protocol session handles one board in the
/// bulletin board. Unlike the other Session implementation,
/// this implementation does not handle any messaging to/from
/// the remote bulletin board, this is done by SessionSet;
/// a SessionM step only calls its trustee's step.
///
/// SessionM's belong to a SessionSet, which
/// is the unit of both session concurrency and multiplexing
/// (see also SessionSet::run and main_concurrent::run).
/// SessionSets will create and drop SessionM's as necessary
/// according to updates to the bulletin board index.
pub struct SessionM<C: Context + 'static, S: crate::protocol::board::LocalBoardStorage> {
    pub board_name: String,
    trustee: Trustee<C, S>,
}
impl<C: Context, S: crate::protocol::board::LocalBoardStorage> SessionM<C, S> {
    /// Constructs a new SessionM to handle the requested board.
    pub fn new(board_name: &str, trustee: Trustee<C, S>) -> Result<SessionM<C, S>> {
        let ret = SessionM {
            board_name: board_name.to_string(),
            trustee,
        };

        Ok(ret)
    }

    /// Executes one step of the protocol main loop.
    ///
    /// Not all calls of step will supply new messages, but the
    /// call is still required because there may be messages in the
    /// message_store whose required Actions have not yet executed,
    /// leading to a possible protocol hang.
    pub fn step(&mut self, messages: &Vec<HttpB4Message>) -> Result<Vec<Message<C>>, ProtocolError> {
        // NOTE: we must call step even if there are no new remote messages
        // because there may be actions pending in the trustees LocalBoard.
        let step_result = self.trustee.step(messages)?;

        Ok(step_result.messages)
    }

    /// Returns the largest id stored in the local message store
    ///
    /// The session will requests messages for id > last_external_id from
    /// the bulletin board.
    pub fn get_last_external_id(&mut self) -> Result<i64, ProtocolError> {
        self.trustee.get_last_external_id()
    }

    /// Updates the trustees message store only, not its local board.
    ///
    /// Used when the remote bulletin board returns a truncated response
    /// indicating that a further request must be made before inferring any
    /// new Actions.
    pub(crate) fn update_store(&self, messages: &Vec<HttpB4Message>) -> Result<(), ProtocolError> {
        self.trustee.update_store(messages)
    }
}

#[derive(Clone)]
pub struct SessionFactory<C: Context> {
    pub(crate) trustee_name: String,
    signing_key: <<C as Context>::SignatureScheme as SignatureScheme<<C as Context>::Rng>>::Signer,
    symm_key: SymmetricKey,
    store_root: PathBuf,
    max_concurrent_actions: Option<usize>,
}
impl<C: Context> SessionFactory<C> {
    pub fn new(
        trustee_name: &str,
        cfg: TrusteeConfig,
        store_root: PathBuf,
        max_concurrent_actions: Option<usize>,
    ) -> Result<Self> {
        let signing_key = <<C as Context>::SignatureScheme as SignatureScheme<_>>::signer_from_base64_string(&cfg.signing_key_sk)
            .map_err(|e| anyhow!("Failed to decode signing key: {}", e))?;

        let bytes = crate::util::decode_base64(&cfg.encryption_key)?;
        let symm_key = cryptography::utils::symm::sk_from_bytes(&bytes)?;

        if !store_root.is_dir() {
            return Err(anyhow!("Invalid store root {:?}", store_root));
        }

        Ok(SessionFactory {
            trustee_name: trustee_name.to_string(),
            symm_key,
            signing_key,
            store_root,
            max_concurrent_actions,
        })
    }

    pub fn create_session(&self, board_name: &str) -> Result<SessionM<C, SqliteStorage>> {
        info!("* Creating new session for board '{}'..", board_name);

        let storage = SqliteStorage::new(self.store_root.join(&board_name), None);
        let trustee = Trustee::new(
            self.trustee_name.clone(),
            board_name.to_string(),
            self.signing_key.clone(),
            self.symm_key,
            storage,
            self.max_concurrent_actions,
        );

        SessionM::new(board_name, trustee)
    }
}
