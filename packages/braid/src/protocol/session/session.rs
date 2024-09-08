// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::Result;
use tracing::info;

use strand::context::Ctx;

use crate::protocol::board::{Board, BoardFactory};
use crate::protocol::trustee2::Trustee;
use crate::util::ProtocolError;

pub struct Session<C: Ctx + 'static, B: Board + 'static> {
    pub board_name: String,
    trustee: Trustee<C>,
    board: B::Factory,
}
impl<C: Ctx, B: Board> Session<C, B> {
    pub fn new(board_name: &str, trustee: Trustee<C>, board: B::Factory) -> Session<C, B> {
        Session {
            board_name: board_name.to_string(),
            trustee,
            board,
        }
    }

    pub async fn step(&mut self) -> Result<(), ProtocolError> {

        let mut board = self
            .board
            .get_board();

        let external_last_id = self.trustee.get_last_external_id()?;

        let messages = board
            .get_messages(&self.board_name, external_last_id)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()))?;

        // NOTE: we must call step even if there are no new remote messages
        // because there may be actions pending in the trustees memory board
        let step_result = self.trustee.step(&messages)?;

        info!("Posting {} messages..", step_result.messages.len());

        let result = board
            .insert_messages(&self.board_name, step_result.messages)
            .await
            .map_err(|e| ProtocolError::BoardError(e.to_string()));

        result
    }
}
