// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use tracing::info;

use cryptography::context::Context;

use crate::protocol::board::{Board, BoardFactory};
use crate::protocol::trustee::{Trustee, StepResult};
use crate::util::ProtocolError;

/// A protocol session.
///
/// A protocol session handles one board in the
/// bulletin board.
pub struct Session<C: Context + 'static, B: Board<C> + 'static, S: crate::protocol::board::LocalBoardStorage> {
    pub board_name: String,
    pub trustee: Trustee<C, S>,
    board_factory: B::Factory,
}
impl<C: Context, B: Board<C>, S: crate::protocol::board::LocalBoardStorage> Session<C, B, S> {
    /// Constructs a new SessionM to handle the requested board.
    ///
    /// The board_factory parameter is used at each step to perform
    /// messaging to/from the remote bulletin board.
    pub fn new(board_name: &str, trustee: Trustee<C, S>, board_factory: B::Factory) -> Session<C, B, S> {
        Session {
            board_name: board_name.to_string(),
            trustee,
            board_factory,
        }
    }

    /// Performs one step of the protocol for this session.
    ///
    /// A step performs the following operations
    ///
    /// 1) Retrieve new messages from the remote board (as per
    /// trustee::get_last_external_id)
    /// 2) Run the trustee step
    /// 3) Post the messages returned by the trustee
    /// to the remote board
    pub async fn step(&mut self) -> Result<(usize, StepResult<C>), ProtocolError> {
        let mut board = self.board_factory.get_board();

        let external_last_id = self.trustee.get_last_external_id()?;

        let messages = board
            .get_messages(&self.board_name, external_last_id)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()))?;

        // NOTE: we must call step even if there are no new remote messages
        // because there may be actions pending in the trustee's LocalBoard.
        let mut step_result = self.trustee.step(&messages)?;

        let posted_count = step_result.messages.len();
        info!("Posting {} messages..", posted_count);

        // Convert protocol messages to wire format before posting
        let http_messages: Vec<b4::HttpB4Message> = std::mem::take(&mut step_result.messages)
            .into_iter()
            .map(b4::HttpB4Message::from_protocol_message)
            .collect();

        board
            .post_messages(&self.board_name, http_messages)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()))?;

        Ok((posted_count, step_result))
    }
}
