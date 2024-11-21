// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Result};
use b3::grpc::GrpcB3Message;
use b3::messages::message::Message;
use std::path::PathBuf;
use strand::signature::StrandSignatureSk;
use strand::symm::SymmetricKey;

use strand::backend::ristretto::RistrettoCtx;
use tracing::info;

use crate::protocol::trustee2::Trustee;
use crate::protocol::trustee2::TrusteeConfig;
use crate::util::ProtocolError;

use strand::context::Ctx;

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
/// (see also SessionSet::run and main_m::run).
/// SessionSets will create and drop SessionM's as necessary
/// according to updates to the bulletin board index.
pub struct SessionM<C: Ctx + 'static> {
    pub board_name: String,
    trustee: Trustee<C>,
}
impl<C: Ctx> SessionM<C> {
    /// Constructs a new SessionM to handle the requested board.
    pub fn new(board_name: &str, trustee: Trustee<C>) -> Result<SessionM<C>> {
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
    pub fn step(&mut self, messages: &Vec<GrpcB3Message>) -> Result<Vec<Message>, ProtocolError> {
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
    pub(crate) fn update_store(&self, messages: &Vec<GrpcB3Message>) -> Result<(), ProtocolError> {
        self.trustee.update_store(messages)
    }
}

#[derive(Clone)]
pub struct SessionFactory {
    pub(crate) trustee_name: String,
    signing_key: StrandSignatureSk,
    symm_key: SymmetricKey,
    store_root: PathBuf,
    max_concurrent_actions: Option<usize>,
}
impl SessionFactory {
    pub fn new(
        trustee_name: &str,
        cfg: TrusteeConfig,
        store_root: PathBuf,
        max_concurrent_actions: Option<usize>,
    ) -> Result<Self> {
        let signing_key: StrandSignatureSk =
            StrandSignatureSk::from_der_b64_string(&cfg.signing_key_sk)?;

        let bytes = crate::util::decode_base64(&cfg.encryption_key)?;
        let symm_key = strand::symm::sk_from_bytes(&bytes)?;

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

    pub fn create_session(&self, board_name: &str) -> Result<SessionM<RistrettoCtx>> {
        info!("* Creating new session for board '{}'..", board_name);

        let trustee: Trustee<RistrettoCtx> = Trustee::new(
            self.trustee_name.clone(),
            board_name.to_string(),
            self.signing_key.clone(),
            self.symm_key,
            Some(self.store_root.join(&board_name)),
            self.max_concurrent_actions,
        );

        SessionM::new(board_name, trustee)
    }
}
